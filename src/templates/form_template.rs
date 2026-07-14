//! [`FormTemplate`].

#[allow(unused_imports)]
use super::*;
use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "form.html")]
pub(crate) struct FormTemplate {
    pub(crate) is_edit: bool,
    pub(crate) address_id: String,
    pub(crate) category: String,
    pub(crate) category_label: String,
    pub(crate) category_label_lower: String,
    pub(crate) label: String,
    pub(crate) recipient_name: String,
    pub(crate) line1: String,
    pub(crate) line2: String,
    pub(crate) city: String,
    pub(crate) region: String,
    pub(crate) postal_code: String,
    pub(crate) country: String,
    pub(crate) error: Option<String>,
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
}
