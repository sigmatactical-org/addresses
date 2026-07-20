//! [`NewQuery`].

use serde::Deserialize;

/// `?category=billing|shipping` selector for the create-address form.
#[derive(Debug, Deserialize)]
pub(crate) struct NewQuery {
    pub(crate) category: Option<String>,
}
