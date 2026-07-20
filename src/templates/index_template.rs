//! [`IndexTemplate`].

use askama::Template;
use sigma_theme::nav::SiteHeader;

use super::AddressRow;

#[derive(Template)]
#[template(path = "index.html")]
pub(crate) struct IndexTemplate {
    pub(crate) billing_rows: Vec<AddressRow>,
    pub(crate) shipping_rows: Vec<AddressRow>,
    pub(crate) message: Option<String>,
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
}
