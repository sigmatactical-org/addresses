mod address_form_values;
mod address_row;
mod form_template;
mod index_template;
pub use address_form_values::AddressFormValues;
pub use address_row::AddressRow;
pub(crate) use form_template::FormTemplate;
pub(crate) use index_template::IndexTemplate;

use askama::Template;

use crate::config;
use crate::model::{Address, AddressCategory};
use sigma_theme::copyright_years;
use sigma_theme::nav::{SiteHeader, site_menu};
use sigma_theme::site_nav::{AppSiteNav, render_app_site_nav};

fn page_header() -> SiteHeader {
    SiteHeader::new().with_menu(site_menu(None))
}

fn site_nav(return_path: &str) -> Result<String, askama::Error> {
    render_app_site_nav(&AppSiteNav {
        identity_base: &config::identity_public_base_url(),
        app_base: &config::public_base_url(),
        contact_base: &config::contact_public_base_url(),
        cart_url: &config::cart_public_base_url(),
        cart_count: 0,
        return_path,
        show_cart: true,
        show_contact_us: false,
        leading_html: "",
    })
}

fn category_label(category: AddressCategory) -> &'static str {
    match category {
        AddressCategory::Billing => "Billing",
        AddressCategory::Shipping => "Shipping",
    }
}

fn address_row(address: &Address) -> AddressRow {
    let city_line = [
        address.city.clone(),
        address.region.clone().unwrap_or_default(),
        address.postal_code.clone(),
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    AddressRow {
        label: address.label.clone().unwrap_or_default(),
        recipient_name: address.recipient_name.clone().unwrap_or_default(),
        line1: address.line1.clone(),
        line2: address.line2.clone().unwrap_or_default(),
        city_line,
        country: address.country.clone(),
        is_default: address.is_default,
        edit_url: format!("/{}/edit", address.id),
        delete_url: format!("/{}/delete", address.id),
        default_url: format!("/{}/default", address.id),
    }
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_index_html(
    addresses: Vec<Address>,
    message: Option<String>,
) -> Result<String, askama::Error> {
    let mut billing_rows = Vec::new();
    let mut shipping_rows = Vec::new();
    for address in &addresses {
        match address.category {
            AddressCategory::Billing => billing_rows.push(address_row(address)),
            AddressCategory::Shipping => shipping_rows.push(address_row(address)),
        }
    }
    IndexTemplate {
        billing_rows,
        shipping_rows,
        message,
        site_header: page_header(),
        site_nav: site_nav("/")?,
        copyright_years: copyright_years(),
    }
    .render()
}

fn render_form(
    address: Option<&Address>,
    category: AddressCategory,
    error: Option<String>,
    values: AddressFormValues,
) -> Result<String, askama::Error> {
    let is_edit = address.is_some();
    let address_id = address.map(|a| a.id.clone()).unwrap_or_default();
    let return_path = match address {
        Some(a) => format!("/{}/edit", a.id),
        None => format!("/new?category={}", category.as_str()),
    };
    FormTemplate {
        is_edit,
        address_id,
        category: category.as_str().to_string(),
        category_label: category_label(category).to_string(),
        category_label_lower: category.as_str().to_string(),
        label: values.label,
        recipient_name: values.recipient_name,
        line1: values.line1,
        line2: values.line2,
        city: values.city,
        region: values.region,
        postal_code: values.postal_code,
        country: values.country,
        error,
        site_header: page_header(),
        site_nav: site_nav(&return_path)?,
        copyright_years: copyright_years(),
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_form_html(
    address: Option<Address>,
    category: AddressCategory,
    error: Option<String>,
) -> Result<String, askama::Error> {
    let values = address
        .as_ref()
        .map(AddressFormValues::from_address)
        .unwrap_or_default();
    render_form(address.as_ref(), category, error, values)
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_form_html_with_values(
    address: Option<Address>,
    category: AddressCategory,
    error: Option<String>,
    values: AddressFormValues,
) -> Result<String, askama::Error> {
    render_form(address.as_ref(), category, error, values)
}
