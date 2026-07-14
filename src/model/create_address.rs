//! [`CreateAddress`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

/// Fields accepted when creating an address. `is_default` is deliberately
/// absent: new addresses are never created as the default directly. Promoting
/// an address to default requires the clear-then-set transaction implemented
/// by `AddressStore::set_default`, which the UI calls as a separate action
/// after creation.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAddress {
    pub category: AddressCategory,
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
