//! [`ListQuery`].

#[allow(unused_imports)]
use super::*;

#[derive(serde::Deserialize)]
pub(crate) struct ListQuery {
    pub(crate) category: Option<String>,
}
