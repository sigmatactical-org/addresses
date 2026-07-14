//! [`AddressForm`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

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
    /// Validate the form into a create request.
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

    /// Validate the form into an update request.
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
