//! [`CreateAddress`].

use super::AddressCategory;

/// Fields accepted when creating an address. `is_default` is deliberately
/// absent: the caller never chooses it. `AddressStore::create` makes the
/// user's first address in a category its default automatically; any later
/// promotion goes through the clear-then-set transaction in
/// `AddressStore::set_default`, which the UI calls as a separate action.
#[derive(Debug, Clone)]
pub struct CreateAddress {
    pub category: AddressCategory,
    pub label: Option<String>,
    pub recipient_name: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub region: Option<String>,
    pub postal_code: String,
    pub country: String,
}
