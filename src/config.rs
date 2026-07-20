//! Environment-driven configuration for the addresses service.

use sigma_pg::clients::http::{env_url, normalize_base_url};

/// PostgreSQL connection URL (shared Sigma database).
#[must_use]
pub fn database_url() -> String {
    std::env::var("DATABASE_URL").unwrap_or_else(|_| sigma_pg::service_database_url("addresses"))
}

/// Canonical public URL of this addresses service, for sign-in return links
/// (e.g. `http://127.0.0.1:8089/`).
#[must_use]
pub fn public_base_url() -> String {
    env_url("ADDRESSES_PUBLIC_BASE_URL", "http://127.0.0.1:8089/")
}

/// Public base URL of the identity BFF (e.g. `http://127.0.0.1:3000/`).
#[must_use]
pub fn identity_public_base_url() -> String {
    env_url("ADDRESSES_IDENTITY_PUBLIC_URL", "http://127.0.0.1:3000/")
}

/// Browser origin of the identity BFF for CSP `connect-src` (no trailing slash).
#[must_use]
pub fn identity_public_origin() -> String {
    identity_public_base_url().trim_end_matches('/').to_string()
}

/// Base URL for server-to-server calls to the identity BFF (session status
/// checks gating every addresses page). Must be reachable from this pod,
/// unlike `identity_public_base_url`, which is the browser-facing ingress
/// host and does not resolve back to identity from inside the cluster
/// network. Falls back to the public URL for non-cluster local dev.
#[must_use]
pub fn identity_internal_base_url() -> String {
    std::env::var("ADDRESSES_IDENTITY_INTERNAL_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| normalize_base_url(&s))
        .unwrap_or_else(identity_public_base_url)
}

/// Public base URL of the contact service for the navbar link.
#[must_use]
pub fn contact_public_base_url() -> String {
    env_url("ADDRESSES_CONTACT_PUBLIC_URL", "http://127.0.0.1:8083/")
}

/// Public base URL of the cart service for the navbar link.
#[must_use]
pub fn cart_public_base_url() -> String {
    env_url("ADDRESSES_CART_PUBLIC_URL", "http://127.0.0.1:8084/")
}

/// Base URL of the cart service over the mesh, used server-side to read the
/// live item count for the navbar badge (e.g. `http://127.0.0.1:8084/`).
/// `None` (unset) means cart integration is not configured.
#[must_use]
pub fn cart_base_url() -> Option<String> {
    std::env::var("ADDRESSES_CART_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| normalize_base_url(&s))
}
