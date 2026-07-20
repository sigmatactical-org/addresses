//! [`ListQuery`].

use serde::Deserialize;

/// `?category=billing|shipping` filter for the address listing endpoint.
#[derive(Debug, Deserialize)]
pub(crate) struct ListQuery {
    pub(crate) category: Option<String>,
}
