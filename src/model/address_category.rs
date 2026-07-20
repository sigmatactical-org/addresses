//! [`AddressCategory`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressCategory {
    Billing,
    Shipping,
}
impl AddressCategory {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AddressCategory::Billing => "billing",
            AddressCategory::Shipping => "shipping",
        }
    }

    /// Parse from its stored/wire string.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "billing" => Ok(AddressCategory::Billing),
            "shipping" => Ok(AddressCategory::Shipping),
            _ => Err("category must be billing or shipping".to_string()),
        }
    }
}
