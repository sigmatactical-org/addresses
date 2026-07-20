//! [`AddressRow`].

/// A row in the billing/shipping address list.
pub struct AddressRow {
    pub label: String,
    pub recipient_name: String,
    pub line1: String,
    pub line2: String,
    pub city_line: String,
    pub country: String,
    pub is_default: bool,
    pub edit_url: String,
    pub delete_url: String,
    pub default_url: String,
}
