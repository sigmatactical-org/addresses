use std::convert::Infallible;

use warp::http::StatusCode;
use warp::reply::Response;
use warp::{Filter, Rejection, Reply};

use crate::SharedStore;
use crate::model::{Address, AddressCategory, AddressForm};
use crate::store::StoreError;
use crate::templates::{self, AddressFormValues};

pub fn routes(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    index(store.clone())
        .or(new_address(store.clone()))
        .or(create_address(store.clone()))
        .or(edit_address(store.clone()))
        .or(update_address(store.clone()))
        .or(delete_address(store.clone()))
        .or(set_default_address(store))
}

// ---------------------------------------------------------------------------
// Session gate + redirect helpers
// ---------------------------------------------------------------------------

/// Resolve the signed-in identity user id from the session cookie, or `Err`
/// with a 303 redirect to identity sign-in (returning to `return_path` after
/// login). Every route in this service is gated on this — addresses are
/// strictly per-user, and there is no anonymous or admin-visible view.
async fn require_user(cookie: Option<String>, return_path: &str) -> Result<String, Response> {
    let status = sigma_pg::clients::session::fetch_identity_status(
        &crate::config::identity_internal_base_url(),
        cookie.as_deref(),
    )
    .await;
    let user_id = match status {
        Ok(Some(status)) => status.user_id.filter(|id| !id.trim().is_empty()),
        Ok(None) => None,
        Err(error) => {
            tracing::error!("web: fetch_identity_status failed: {error:?}");
            None
        }
    };
    user_id.ok_or_else(|| sign_in_redirect(return_path))
}

fn sign_in_redirect(return_path: &str) -> Response {
    let links = sigma_identity_nav::auth_links(
        &crate::config::identity_public_base_url(),
        &crate::config::public_base_url(),
        return_path,
    );
    redirect(&links.sign_in_url)
}

/// 303 redirect (PRG pattern for form POSTs, also used for the sign-in bounce).
// `to_string` is required, not redundant: `Uri::from_maybe_shared` needs an
// owned buffer it can turn into `Bytes` without borrowing past this call.
#[allow(clippy::unnecessary_to_owned)]
fn redirect(location: &str) -> Response {
    match warp::http::Uri::from_maybe_shared(location.to_string()) {
        Ok(uri) => warp::redirect::see_other(uri).into_response(),
        Err(_) => warp::reply::with_status(warp::reply(), StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}

fn cookie_filter() -> impl Filter<Extract = (Option<String>,), Error = Rejection> + Clone {
    warp::header::optional::<String>("cookie")
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

fn index(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path::end()
        .and(warp::get())
        .and(cookie_filter())
        .and(store)
        .and_then(|cookie: Option<String>, store: SharedStore| async move {
            let user_id = match require_user(cookie, "/").await {
                Ok(user_id) => user_id,
                Err(resp) => return Ok::<_, Rejection>(resp),
            };
            let addresses = store
                .list_for_user(&user_id)
                .await
                .map_err(|_| warp::reject::not_found())?;
            templates::render_index_html(addresses, None)
                .map(|html| warp::reply::html(html).into_response())
                .map_err(|_| warp::reject::not_found())
        })
}

#[derive(serde::Deserialize)]
struct NewQuery {
    category: Option<String>,
}

fn new_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path("new")
        .and(warp::path::end())
        .and(warp::get())
        .and(cookie_filter())
        .and(warp::query::<NewQuery>())
        .and(store)
        .and_then(
            |cookie: Option<String>, query: NewQuery, _store: SharedStore| async move {
                let category = match query.category.as_deref().map(AddressCategory::parse) {
                    Some(Ok(category)) => category,
                    _ => return Err(warp::reject::not_found()),
                };
                let return_path = format!("/new?category={}", category.as_str());
                let _user_id = match require_user(cookie, &return_path).await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                templates::render_form_html(None, category, None)
                    .map(|html| warp::reply::html(html).into_response())
                    .map_err(|_| warp::reject::not_found())
            },
        )
}

fn create_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path::end()
        .and(warp::post())
        .and(cookie_filter())
        .and(warp::body::form())
        .and(store)
        .and_then(
            |cookie: Option<String>, form: AddressForm, store: SharedStore| async move {
                let user_id = match require_user(cookie, "/").await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                let category = match AddressCategory::parse(&form.category) {
                    Ok(category) => category,
                    Err(_) => return Err(warp::reject::not_found()),
                };
                let values = AddressFormValues::from_form(&form);
                let response = match form.into_create(category) {
                    Ok(input) => match store.create(&user_id, input).await {
                        Ok(_) => redirect("/"),
                        Err(e) => render_form_error(None, category, values, e),
                    },
                    Err(e) => {
                        render_form_error(None, category, values, StoreError::InvalidInput(e))
                    }
                };
                Ok(response)
            },
        )
}

fn edit_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path::param::<String>()
        .and(warp::path("edit"))
        .and(warp::path::end())
        .and(warp::get())
        .and(cookie_filter())
        .and(store)
        .and_then(
            |id: String, cookie: Option<String>, store: SharedStore| async move {
                let return_path = format!("/{id}/edit");
                let user_id = match require_user(cookie, &return_path).await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                match store.get_for_user(&user_id, &id).await {
                    Ok(address) => {
                        templates::render_form_html(Some(address.clone()), address.category, None)
                            .map(|html| warp::reply::html(html).into_response())
                            .map_err(|_| warp::reject::not_found())
                    }
                    Err(StoreError::NotFound) => Err(warp::reject::not_found()),
                    Err(_) => Err(warp::reject::not_found()),
                }
            },
        )
}

fn update_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path::param::<String>()
        .and(warp::path::end())
        .and(warp::post())
        .and(cookie_filter())
        .and(warp::body::form())
        .and(store)
        .and_then(
            |id: String, cookie: Option<String>, form: AddressForm, store: SharedStore| async move {
                let return_path = format!("/{id}/edit");
                let user_id = match require_user(cookie, &return_path).await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                let existing: Address = match store.get_for_user(&user_id, &id).await {
                    Ok(address) => address,
                    Err(StoreError::NotFound) => return Err(warp::reject::not_found()),
                    Err(_) => return Err(warp::reject::not_found()),
                };
                let category = existing.category;
                let values = AddressFormValues::from_form(&form);
                let response = match form.into_update() {
                    Ok(input) => match store.update(&user_id, &id, input).await {
                        Ok(_) => redirect("/"),
                        Err(e) => render_form_error(Some(existing), category, values, e),
                    },
                    Err(e) => render_form_error(
                        Some(existing),
                        category,
                        values,
                        StoreError::InvalidInput(e),
                    ),
                };
                Ok(response)
            },
        )
}

fn delete_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path::param::<String>()
        .and(warp::path("delete"))
        .and(warp::path::end())
        .and(warp::post())
        .and(cookie_filter())
        .and(store)
        .and_then(
            |id: String, cookie: Option<String>, store: SharedStore| async move {
                let user_id = match require_user(cookie, "/").await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                match store.delete(&user_id, &id).await {
                    Ok(()) => Ok(redirect("/")),
                    Err(StoreError::NotFound) => Err(warp::reject::not_found()),
                    Err(_) => Err(warp::reject::not_found()),
                }
            },
        )
}

fn set_default_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path::param::<String>()
        .and(warp::path("default"))
        .and(warp::path::end())
        .and(warp::post())
        .and(cookie_filter())
        .and(store)
        .and_then(
            |id: String, cookie: Option<String>, store: SharedStore| async move {
                let user_id = match require_user(cookie, "/").await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                match store.set_default(&user_id, &id).await {
                    Ok(()) => Ok(redirect("/")),
                    Err(StoreError::NotFound) => Err(warp::reject::not_found()),
                    Err(_) => Err(warp::reject::not_found()),
                }
            },
        )
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn render_form_error(
    address: Option<Address>,
    category: AddressCategory,
    values: AddressFormValues,
    err: StoreError,
) -> Response {
    let message = err.to_string();
    match templates::render_form_html_with_values(address, category, Some(message), values) {
        Ok(html) => warp::reply::with_status(warp::reply::html(html), StatusCode::BAD_REQUEST)
            .into_response(),
        Err(_) => warp::reply::with_status(warp::reply(), StatusCode::INTERNAL_SERVER_ERROR)
            .into_response(),
    }
}
