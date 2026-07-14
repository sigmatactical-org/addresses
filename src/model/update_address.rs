//! [`UpdateAddress`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

/// Fields accepted when updating an address. `category` cannot be changed
/// after creation — it is fixed at creation time via `CreateAddress`.
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAddress {
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub recipient_name: Option<String>,
    pub line1: String,
    #[serde(default)]
    pub line2: Option<String>,
    pub city: String,
    #[serde(default)]
    pub region: Option<String>,
    pub postal_code: String,
    pub country: String,
}
