//! Central content registry for AI-friendly editing.
//!
//! AI assistants: prefer editing files in `content/pages/*` for text changes.
//! UI renderers should import data from here and avoid embedding copy inline.

pub mod metadata;
pub mod pages;
pub mod schema;
pub mod seo;
