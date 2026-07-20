//! [`AddressFormValues`].

use crate::model::{Address, AddressForm};

/// Preserved (possibly invalid) form input, re-rendered on a validation error
/// so the visitor doesn't lose what they typed.
#[derive(Default)]
pub struct AddressFormValues {
    pub label: String,
    pub recipient_name: String,
    pub line1: String,
    pub line2: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
}
impl AddressFormValues {
    #[must_use]
    pub fn from_form(form: &AddressForm) -> Self {
        Self {
            label: form.label.clone(),
            recipient_name: form.recipient_name.clone(),
            line1: form.line1.clone(),
            line2: form.line2.clone(),
            city: form.city.clone(),
            region: form.region.clone(),
            postal_code: form.postal_code.clone(),
            country: form.country.clone(),
        }
    }

    #[must_use]
    pub fn from_address(address: &Address) -> Self {
        Self {
            label: address.label.clone().unwrap_or_default(),
            recipient_name: address.recipient_name.clone().unwrap_or_default(),
            line1: address.line1.clone(),
            line2: address.line2.clone().unwrap_or_default(),
            city: address.city.clone(),
            region: address.region.clone().unwrap_or_default(),
            postal_code: address.postal_code.clone(),
            country: address.country.clone(),
        }
    }
}
