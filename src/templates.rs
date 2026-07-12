use askama::Template;

use crate::config;
use crate::model::{Address, AddressCategory, AddressForm};
use sigma_theme::copyright_years;
use sigma_theme::nav::SiteHeader;
use sigma_theme::site_nav::{AppSiteNav, render_app_site_nav};

fn page_header(brand: &str) -> SiteHeader {
    SiteHeader::new(brand)
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
        show_contact_us: true,
        leading_html: "",
    })
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    billing_rows: Vec<AddressRow>,
    shipping_rows: Vec<AddressRow>,
    message: Option<String>,
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
}

#[derive(Template)]
#[template(path = "form.html")]
struct FormTemplate {
    is_edit: bool,
    address_id: String,
    category: String,
    category_label: String,
    category_label_lower: String,
    label: String,
    recipient_name: String,
    line1: String,
    line2: String,
    city: String,
    region: String,
    postal_code: String,
    country: String,
    error: Option<String>,
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
}

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
        site_header: page_header("Sigma Addresses"),
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
        site_header: page_header("Sigma Addresses"),
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
