//! Non-SEO metadata used to identify and route content blocks.
//!
//! AI assistants: keep ids stable. Other agents may depend on them to update
//! content safely without touching UI code.

#[derive(Clone, Copy)]
pub struct PageMetadata {
    pub id: &'static str,
    pub route: &'static str,
    pub locale: &'static str,
    pub cms_collection: &'static str,
}
