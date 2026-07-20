mod new_query;
pub(crate) use new_query::NewQuery;

use std::convert::Infallible;

use sigma_pg::clients::cart;
use warp::http::StatusCode;
use warp::reply::Response;
use warp::{Filter, Rejection, Reply};

use crate::SharedStore;
use crate::config;
use crate::model::{Address, AddressCategory, AddressForm};
use crate::store::StoreError;
use crate::templates::{self, AddressFormValues};

/// Build this module's routes.
pub fn routes(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    index(store.clone())
        .or(new_address())
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
async fn require_user(cookie: Option<&str>, return_path: &str) -> Result<String, Response> {
    let status = sigma_pg::clients::session::fetch_identity_status(
        &config::identity_internal_base_url(),
        cookie,
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
        &config::identity_public_base_url(),
        &config::public_base_url(),
        return_path,
    );
    redirect(&links.sign_in_url)
}

/// 303 redirect (PRG pattern for form POSTs, also used for the sign-in bounce).
// `to_string` is required, not redundant: `Uri::from_maybe_shared` needs an
// owned buffer it can turn into `Bytes` without borrowing past this call.
fn redirect(location: &str) -> Response {
    match warp::http::Uri::from_maybe_shared(location.to_string()) {
        Ok(uri) => warp::redirect::see_other(uri).into_response(),
        Err(_) => internal_error(),
    }
}

fn cookie_filter() -> impl Filter<Extract = (Option<String>,), Error = Rejection> + Clone {
    warp::header::optional::<String>("cookie")
}

/// Themed 500 response for unexpected failures (database or template render
/// errors) — distinct from [`StoreError::NotFound`], which stays a 404.
fn internal_error() -> Response {
    warp::reply::with_status(
        warp::reply::html(sigma_theme::errors::internal_server_error_html()),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
    .into_response()
}

/// Live cart item count for the navbar badge (0 when unconfigured).
async fn nav_cart_count(cookie: Option<&str>) -> u32 {
    cart::nav_cart_count(config::cart_base_url().as_deref(), cookie).await
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
            let user_id = match require_user(cookie.as_deref(), "/").await {
                Ok(user_id) => user_id,
                Err(resp) => return Ok::<_, Rejection>(resp),
            };
            let (addresses, cart_count) = tokio::join!(
                store.list_for_user(&user_id, None),
                nav_cart_count(cookie.as_deref()),
            );
            let addresses = match addresses {
                Ok(addresses) => addresses,
                Err(error) => {
                    tracing::error!("web: list_for_user failed for {user_id}: {error:?}");
                    return Ok(internal_error());
                }
            };
            match templates::render_index_html(addresses, None, cart_count) {
                Ok(html) => Ok(warp::reply::html(html).into_response()),
                Err(error) => {
                    tracing::error!("web: index render failed: {error:?}");
                    Ok(internal_error())
                }
            }
        })
}

fn new_address() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static
{
    warp::path("new")
        .and(warp::path::end())
        .and(warp::get())
        .and(cookie_filter())
        .and(warp::query::<NewQuery>())
        .and_then(|cookie: Option<String>, query: NewQuery| async move {
            let category = match query.category.as_deref().map(AddressCategory::parse) {
                Some(Ok(category)) => category,
                _ => return Err(warp::reject::not_found()),
            };
            let return_path = format!("/new?category={}", category.as_str());
            if let Err(resp) = require_user(cookie.as_deref(), &return_path).await {
                return Ok::<_, Rejection>(resp);
            }
            let cart_count = nav_cart_count(cookie.as_deref()).await;
            match templates::render_form_html(None, category, None, cart_count) {
                Ok(html) => Ok(warp::reply::html(html).into_response()),
                Err(error) => {
                    tracing::error!("web: new form render failed: {error:?}");
                    Ok(internal_error())
                }
            }
        })
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
                let user_id = match require_user(cookie.as_deref(), "/").await {
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
                        Err(e) => {
                            render_form_error(None, category, values, e, cookie.as_deref()).await
                        }
                    },
                    Err(e) => {
                        render_form_error(
                            None,
                            category,
                            values,
                            StoreError::InvalidInput(e),
                            cookie.as_deref(),
                        )
                        .await
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
                let user_id = match require_user(cookie.as_deref(), &return_path).await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                let address = match store.get_for_user(&user_id, &id).await {
                    Ok(address) => address,
                    Err(StoreError::NotFound(_)) => return Err(warp::reject::not_found()),
                    Err(error) => {
                        tracing::error!("web: get_for_user failed for {user_id}/{id}: {error:?}");
                        return Ok(internal_error());
                    }
                };
                let cart_count = nav_cart_count(cookie.as_deref()).await;
                match templates::render_form_html(
                    Some(&address),
                    address.category,
                    None,
                    cart_count,
                ) {
                    Ok(html) => Ok(warp::reply::html(html).into_response()),
                    Err(error) => {
                        tracing::error!("web: edit form render failed: {error:?}");
                        Ok(internal_error())
                    }
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
                let user_id = match require_user(cookie.as_deref(), &return_path).await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                let existing: Address = match store.get_for_user(&user_id, &id).await {
                    Ok(address) => address,
                    Err(StoreError::NotFound(_)) => return Err(warp::reject::not_found()),
                    Err(error) => {
                        tracing::error!("web: get_for_user failed for {user_id}/{id}: {error:?}");
                        return Ok(internal_error());
                    }
                };
                let category = existing.category;
                let values = AddressFormValues::from_form(&form);
                let response = match form.into_update() {
                    Ok(input) => match store.update(&user_id, &id, input).await {
                        Ok(_) => redirect("/"),
                        Err(e) => {
                            render_form_error(
                                Some(&existing),
                                category,
                                values,
                                e,
                                cookie.as_deref(),
                            )
                            .await
                        }
                    },
                    Err(e) => {
                        render_form_error(
                            Some(&existing),
                            category,
                            values,
                            StoreError::InvalidInput(e),
                            cookie.as_deref(),
                        )
                        .await
                    }
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
                let user_id = match require_user(cookie.as_deref(), "/").await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                match store.delete(&user_id, &id).await {
                    Ok(()) => Ok(redirect("/")),
                    Err(StoreError::NotFound(_)) => Err(warp::reject::not_found()),
                    Err(error) => {
                        tracing::error!("web: delete failed for {user_id}/{id}: {error:?}");
                        Ok(internal_error())
                    }
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
                let user_id = match require_user(cookie.as_deref(), "/").await {
                    Ok(user_id) => user_id,
                    Err(resp) => return Ok::<_, Rejection>(resp),
                };
                match store.set_default(&user_id, &id).await {
                    Ok(()) => Ok(redirect("/")),
                    Err(StoreError::NotFound(_)) => Err(warp::reject::not_found()),
                    Err(error) => {
                        tracing::error!("web: set_default failed for {user_id}/{id}: {error:?}");
                        Ok(internal_error())
                    }
                }
            },
        )
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Re-render the form with the visitor's input preserved and `err` shown,
/// fetching the nav cart count only now that we know HTML will be rendered
/// (successful POSTs redirect and never need it).
async fn render_form_error(
    address: Option<&Address>,
    category: AddressCategory,
    values: AddressFormValues,
    err: StoreError,
    cookie: Option<&str>,
) -> Response {
    let cart_count = nav_cart_count(cookie).await;
    let message = err.to_string();
    match templates::render_form_html_with_values(
        address,
        category,
        Some(message),
        values,
        cart_count,
    ) {
        Ok(html) => warp::reply::with_status(warp::reply::html(html), StatusCode::BAD_REQUEST)
            .into_response(),
        Err(_) => internal_error(),
    }
}
