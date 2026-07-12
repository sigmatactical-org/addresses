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

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "billing" => Ok(AddressCategory::Billing),
            "shipping" => Ok(AddressCategory::Shipping),
            _ => Err("category must be billing or shipping".to_string()),
        }
    }
}

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

/// Raw `application/x-www-form-urlencoded` body for the create/edit web form.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AddressForm {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub recipient_name: String,
    #[serde(default)]
    pub line1: String,
    #[serde(default)]
    pub line2: String,
    #[serde(default)]
    pub city: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub postal_code: String,
    #[serde(default)]
    pub country: String,
}

impl AddressForm {
    pub fn into_create(self, category: AddressCategory) -> Result<CreateAddress, String> {
        Ok(CreateAddress {
            category,
            label: empty_to_none(self.label),
            recipient_name: empty_to_none(self.recipient_name),
            line1: required(self.line1, "line1")?,
            line2: empty_to_none(self.line2),
            city: required(self.city, "city")?,
            region: empty_to_none(self.region),
            postal_code: required(self.postal_code, "postal_code")?,
            country: required(self.country, "country")?,
        })
    }

    pub fn into_update(self) -> Result<UpdateAddress, String> {
        Ok(UpdateAddress {
            label: empty_to_none(self.label),
            recipient_name: empty_to_none(self.recipient_name),
            line1: required(self.line1, "line1")?,
            line2: empty_to_none(self.line2),
            city: required(self.city, "city")?,
            region: empty_to_none(self.region),
            postal_code: required(self.postal_code, "postal_code")?,
            country: required(self.country, "country")?,
        })
    }
}

fn empty_to_none(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn required(value: String, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(trimmed.to_string())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_category_accepts_known_values() {
        assert_eq!(
            AddressCategory::parse("billing").unwrap(),
            AddressCategory::Billing
        );
        assert_eq!(
            AddressCategory::parse("Shipping").unwrap(),
            AddressCategory::Shipping
        );
        assert!(AddressCategory::parse("bogus").is_err());
    }

    #[test]
    fn form_into_create_requires_line1() {
        let form = AddressForm {
            city: "Springfield".to_string(),
            postal_code: "12345".to_string(),
            country: "US".to_string(),
            ..Default::default()
        };
        assert!(form.into_create(AddressCategory::Shipping).is_err());
    }
}
