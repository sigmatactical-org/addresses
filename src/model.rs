mod address;
mod address_category;
mod address_form;
mod create_address;
mod update_address;
pub use address::Address;
pub use address_category::AddressCategory;
pub use address_form::AddressForm;
pub use create_address::CreateAddress;
pub use update_address::UpdateAddress;

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
