//! Sigma Addresses: billing/shipping addresses for identity users.

#![forbid(unsafe_code)]

mod api;
pub mod config;
mod model;
pub mod store;
mod templates;
mod web;

use std::convert::Infallible;
use std::sync::Arc;

use warp::Filter;
use warp::Reply;

/// Shared address store handle (`PgPool` is internally concurrent).
pub type SharedStore = Arc<store::AddressStore>;

/// Site routes: session-gated web UI, internal-token JSON API (under `/api`),
/// `/up`, health routes, theme static assets, and themed error recovery, with
/// the shared security header set (CSP `connect-src` extended with the
/// identity BFF origin).
pub fn routes(
    store: store::AddressStore,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + Sync + 'static {
    let health_pool = Arc::new(store.pool().clone());
    let store = Arc::new(store);

    let site = sigma_theme::warp::site_routes(
        web::routes(sigma_theme::warp::with_state(store.clone())),
        sigma_pg::health::warp::health_routes("addresses", Some(health_pool))
            .or(warp::path("api").and(api::routes(sigma_theme::warp::with_state(store)))),
    );
    sigma_theme::warp::security_headers(site, config::identity_public_origin())
}

#[cfg(test)]
mod tests {
    use warp::http::StatusCode;

    use super::routes;
    use crate::store;

    async fn test_store() -> store::AddressStore {
        sigma_pg::clients::internal::ensure_test_internal_token();
        store::AddressStore::connect_empty()
            .await
            .expect("PostgreSQL required for tests")
    }

    #[tokio::test]
    async fn up_returns_ok() {
        let res = warp::test::request()
            .method("GET")
            .path("/up")
            .reply(&routes(test_store().await))
            .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn index_without_session_redirects_to_sign_in() {
        let res = warp::test::request()
            .method("GET")
            .path("/")
            .reply(&routes(test_store().await))
            .await;
        assert_eq!(res.status(), StatusCode::SEE_OTHER);
        let location = res
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(location.contains("/auth/login"));
    }

    #[tokio::test]
    async fn api_list_addresses_requires_internal_token() {
        let res = warp::test::request()
            .method("GET")
            .path("/api/users/user-1/addresses")
            .reply(&routes(test_store().await))
            .await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn api_list_addresses_with_internal_token() {
        let res = warp::test::request()
            .method("GET")
            .path("/api/users/user-1/addresses")
            .header(
                "x-sigma-internal-token",
                sigma_pg::clients::internal::TEST_INTERNAL_TOKEN,
            )
            .reply(&routes(test_store().await))
            .await;
        assert_eq!(res.status(), StatusCode::OK);
        let body: Vec<serde_json::Value> = serde_json::from_slice(res.body()).unwrap();
        assert!(body.is_empty());
    }
}
