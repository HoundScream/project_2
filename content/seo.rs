//! SEO content is stored separately from visible UI copy.

#[derive(Clone, Copy)]
pub struct SeoContent {
    pub title: &'static str,
    pub description: &'static str,
    pub canonical_path: &'static str,
    pub robots: &'static str,
    pub keywords: &'static [&'static str],
}
