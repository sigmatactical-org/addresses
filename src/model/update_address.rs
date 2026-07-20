//! [`UpdateAddress`].

/// Fields accepted when updating an address. `category` cannot be changed
/// after creation — it is fixed at creation time via `CreateAddress`.
#[derive(Debug, Clone)]
pub struct UpdateAddress {
    pub label: Option<String>,
    pub recipient_name: Option<String>,
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub region: Option<String>,
    pub postal_code: String,
    pub country: String,
}
