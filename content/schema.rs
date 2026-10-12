//! Reusable content schema shared across pages.
//!
//! AI assistants: extend this schema before introducing ad-hoc page-specific
//! text shapes. This keeps content predictable for CMS and localization.

use crate::content::{metadata::PageMetadata, seo::SeoContent};

#[derive(Clone, Copy)]
pub struct HeroContent {
    pub eyebrow: Option<&'static str>,
    pub title: &'static str,
    pub description: &'static str,
}

#[derive(Clone, Copy)]
pub struct ActionLink {
    pub label: &'static str,
    pub href: &'static str,
    pub variant: ActionVariant,
}

#[derive(Clone, Copy)]
pub enum ActionVariant {
    Primary,
    Secondary,
}

#[derive(Clone, Copy)]
pub struct FeatureItem {
    pub title: Option<&'static str>,
    pub description: &'static str,
}

#[derive(Clone, Copy)]
pub struct FaqItem {
    pub question: &'static str,
    pub answer: &'static str,
}

#[derive(Clone, Copy)]
pub struct CtaContent {
    pub title: &'static str,
    pub description: &'static str,
    pub primary_action: ActionLink,
    pub secondary_action: Option<ActionLink>,
}

#[derive(Clone, Copy)]
pub struct FooterContent {
    pub note: &'static str,
    pub links: &'static [ActionLink],
}

#[derive(Clone, Copy)]
pub struct SearchFormContent {
    pub action: &'static str,
    pub method: &'static str,
    pub field_name: &'static str,
    pub input_type: &'static str,
    pub placeholder: &'static str,
    pub aria_label: &'static str,
    pub submit_label: &'static str,
}

#[derive(Clone, Copy)]
pub struct SelectOption {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy)]
pub struct FormFieldContent {
    pub name: &'static str,
    pub label: &'static str,
    pub input_type: &'static str,
    pub placeholder: &'static str,
}

#[derive(Clone, Copy)]
pub struct SelectFieldContent {
    pub name: &'static str,
    pub label: &'static str,
    pub options: &'static [SelectOption],
}

#[derive(Clone, Copy)]
pub struct StatusMessageContent {
    pub title: &'static str,
    pub success_text: &'static str,
    pub error_text: &'static str,
    pub next_step_title: &'static str,
    pub next_step_description: &'static str,
}

#[derive(Clone, Copy)]
pub struct PageContent {
    pub metadata: PageMetadata,
    pub seo: SeoContent,
}
