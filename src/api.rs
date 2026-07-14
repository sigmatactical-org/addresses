mod error_body;
mod list_query;
pub(crate) use error_body::ErrorBody;
pub(crate) use list_query::ListQuery;

use std::convert::Infallible;

use warp::http::StatusCode;
use warp::reply::Response;
use warp::{Filter, Rejection, Reply};

use crate::SharedStore;
use crate::model::AddressCategory;
use crate::store::StoreError;

fn json_error(status: StatusCode, message: impl Into<String>) -> Response {
    warp::reply::with_status(
        warp::reply::json(&ErrorBody {
            error: message.into(),
        }),
        status,
    )
    .into_response()
}

fn store_error_status(err: &StoreError) -> StatusCode {
    match err {
        StoreError::NotFound => StatusCode::NOT_FOUND,
        StoreError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        StoreError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn internal_auth() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::optional::<String>("authorization")
        .and(warp::header::optional::<String>("x-sigma-internal-token"))
        .and_then(
            |authorization: Option<String>, internal_token: Option<String>| async move {
                if sigma_pg::clients::internal::authorize_internal(
                    authorization.as_deref(),
                    internal_token.as_deref(),
                ) {
                    Ok::<_, Rejection>(())
                } else {
                    Err(warp::reject::not_found())
                }
            },
        )
        .untuple_one()
}

/// Internal-token-gated JSON API, mounted at `/api` by [`crate::routes`]. Used
/// by other services (e.g. payments, validating that a `billing_address_id`
/// belongs to the expected user) rather than by browsers.
pub fn routes(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    list_user_addresses(store.clone()).or(get_user_address(store))
}

fn list_user_addresses(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path!("users" / String / "addresses")
        .and(warp::path::end())
        .and(warp::get())
        .and(warp::query::<ListQuery>())
        .and(internal_auth())
        .and(store)
        .and_then(
            |user_id: String, query: ListQuery, store: SharedStore| async move {
                let category = match query.category.as_deref().map(AddressCategory::parse) {
                    Some(Ok(category)) => Some(category),
                    Some(Err(e)) => {
                        return Ok::<_, Rejection>(json_error(StatusCode::BAD_REQUEST, e));
                    }
                    None => None,
                };
                let addresses = match store.list_for_user(&user_id).await {
                    Ok(addresses) => addresses,
                    Err(e) => return Ok(json_error(store_error_status(&e), e.to_string())),
                };
                let filtered: Vec<_> = addresses
                    .into_iter()
                    .filter(|a| category.is_none_or(|c| a.category == c))
                    .collect();
                Ok(warp::reply::json(&filtered).into_response())
            },
        )
}

fn get_user_address(
    store: impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone + Send + 'static,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone + Send + 'static {
    warp::path!("users" / String / "addresses" / String)
        .and(warp::path::end())
        .and(warp::get())
        .and(internal_auth())
        .and(store)
        .and_then(
            |user_id: String, id: String, store: SharedStore| async move {
                let address = match store.get_any(&id).await {
                    Ok(Some(address)) if address.user_id == user_id => address,
                    Ok(_) => return Err(warp::reject::not_found()),
                    Err(e) => return Ok(json_error(store_error_status(&e), e.to_string())),
                };
                Ok(warp::reply::json(&address).into_response())
            },
        )
}
