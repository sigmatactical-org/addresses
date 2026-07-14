//! [`Address`].

#[allow(unused_imports)]
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    pub id: String,
    pub user_id: String,
    pub category: AddressCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recipient_name: Option<String>,
    pub line1: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<String>,
    pub city: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub postal_code: String,
    pub country: String,
    pub is_default: bool,
    pub updated_at: String,
}
impl Address {
    #[must_use]
    pub fn new(user_id: &str, input: CreateAddress) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.trim().to_string(),
            category: input.category,
            label: input.label,
            recipient_name: input.recipient_name,
            line1: input.line1,
            line2: input.line2,
            city: input.city,
            region: input.region,
            postal_code: input.postal_code,
            country: input.country,
            is_default: false,
            updated_at: now,
        }
    }

    /// Apply a partial update in place.
    pub fn apply_update(&mut self, input: UpdateAddress) {
        self.label = input.label;
        self.recipient_name = input.recipient_name;
        self.line1 = input.line1;
        self.line2 = input.line2;
        self.city = input.city;
        self.region = input.region;
        self.postal_code = input.postal_code;
        self.country = input.country;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}
