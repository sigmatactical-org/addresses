//! Sigma Addresses: billing/shipping addresses for identity users.

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

pub use model::{Address, AddressCategory, CreateAddress, UpdateAddress};

/// Shared address store handle (`PgPool` is internally concurrent).
pub type SharedStore = Arc<store::AddressStore>;

/// Resolve listen address from **`PORT`** (default **8080**).
#[must_use]
pub fn listen_socket_addr_from_env() -> std::net::SocketAddr {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
}

fn with_store(
    store: SharedStore,
) -> impl Filter<Extract = (SharedStore,), Error = Infallible> + Clone {
    warp::any().map(move || store.clone())
}

fn content_security_policy() -> String {
    let identity_origin = config::identity_public_origin();
    format!(
        "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; \
         img-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self'; \
         font-src 'self'; connect-src 'self' {identity_origin}; form-action 'self'"
    )
}

/// Site routes: session-gated web UI, internal-token JSON API (under `/api`),
/// `/up`, theme static assets, error recovery.
pub fn routes(
    store: store::AddressStore,
) -> impl Filter<Extract = (impl Reply,), Error = Infallible> + Clone + Send + 'static {
    use warp::reply::with::header;

    let health_pool = Arc::new(store.pool().clone());
    let store = Arc::new(store);

    warp::path("up")
        .and(warp::get())
        .map(|| warp::reply::with_status("up", warp::http::StatusCode::OK))
        .or(sigma_pg::health::warp::health_routes(
            "addresses",
            Some(health_pool),
        ))
        .or(web::routes(with_store(store.clone())))
        .or(warp::path("api").and(api::routes(with_store(store))))
        .or(sigma_theme::warp::static_files())
        .or(sigma_theme::warp::favicon())
        .recover(sigma_theme::warp::handle_rejection)
        .with(header("content-security-policy", content_security_policy()))
        .with(header("x-content-type-options", "nosniff"))
        .with(header("x-frame-options", "DENY"))
        .with(header("referrer-policy", "strict-origin-when-cross-origin"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;

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
