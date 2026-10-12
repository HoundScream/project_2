use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub mod heuristics;
pub mod platform;
pub mod text;

pub const SESSION_COOKIE_NAME: &str = "geonetix_session";
pub const SESSION_TTL_SECONDS: i64 = 60 * 60 * 24 * 30;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegisterForm {
    pub full_name: String,
    pub phone: String,
    pub telegram_handle: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoginForm {
    pub identifier: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CabinetAnalysisForm {
    pub source_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProPackageForm {
    pub business_query: String,
    pub business_category: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleEightForm {
    pub source_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleTwelveForm {
    pub source_url: String,
    pub business_category: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SubscriptionActionForm {
    pub action: String,
}

#[derive(Debug, FromRow)]
pub struct AuthUserRecord {
    pub id: Uuid,
    pub password_hash: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct GeoObject {
    pub name: String,
    pub object_type: String,
    pub city: String,
}

#[derive(Debug, FromRow)]
pub struct RegisteredUser {
    pub id: Uuid,
    pub full_name: String,
    pub phone: String,
    pub telegram_handle: String,
    pub email: String,
    pub account_tier: String,
    pub subscription_plan: String,
    pub subscription_expires_at_label: Option<String>,
    pub subscription_auto_renew: bool,
    pub subscription_cancel_at_period_end: bool,
    pub subscription_is_active: bool,
    pub analysis_quota_limit: i32,
    pub analysis_quota_used: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum MapPlatform {
    YandexMaps,
    GoogleMaps,
    TwoGis,
}

impl MapPlatform {
    pub fn as_db_value(self) -> &'static str {
        match self {
            MapPlatform::YandexMaps => "yandex_maps",
            MapPlatform::GoogleMaps => "google_maps",
            MapPlatform::TwoGis => "two_gis",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "yandex_maps" => Some(MapPlatform::YandexMaps),
            "google_maps" => Some(MapPlatform::GoogleMaps),
            "two_gis" => Some(MapPlatform::TwoGis),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ParsedBusinessUrl {
    pub source_platform: MapPlatform,
    pub source_url: String,
    pub canonical_url: String,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub title: String,
    pub detail: String,
    pub upgrade_to_pro: bool,
}

#[derive(Debug, Clone)]
pub struct SubscriptionFeedback {
    pub title: String,
    pub detail: String,
    pub is_error: bool,
}

#[derive(Debug, FromRow)]
pub struct CabinetAnalysisRecord {
    pub request_id: Uuid,
    pub request_status: String,
    pub analysis_kind: String,
    pub requested_at: String,
    pub source_platform: String,
    pub canonical_url: String,
    pub profile_title: Option<String>,
    pub profile_address: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct AnalysisStatusRecord {
    pub request_id: Uuid,
    pub request_status: String,
    pub analysis_kind: String,
    pub requested_at_label: String,
    pub current_stage: Option<String>,
    pub stage_index: Option<i32>,
    pub total_stages: i32,
    pub progress_percent: Option<i32>,
    pub status_message: Option<String>,
    pub status_detail: Option<String>,
    pub error_message: Option<String>,
    pub source_platform: String,
    pub canonical_url: String,
    pub profile_title: Option<String>,
    pub profile_address: Option<String>,
    pub report_summary: Option<String>,
    pub report_strengths: Option<String>,
    pub report_risks: Option<String>,
}

#[derive(Debug)]
pub struct AnalysisReportDraft {
    pub summary: String,
    pub strengths: String,
    pub risks: String,
}

#[derive(Debug, FromRow)]
pub struct ProMonitoringScenario {
    pub id: Uuid,
    pub business_query: String,
    pub business_category: String,
    pub monitoring_status: String,
    pub check_frequency: String,
    pub report_format: String,
    pub created_at_label: String,
}

#[derive(Debug, FromRow)]
pub struct ProMonitoringEvent {
    pub changed_at_label: String,
    pub competitor_name: String,
    pub changed_field: String,
    pub change_summary: String,
    pub recommendation: String,
}

#[derive(Debug, FromRow)]
pub struct ModuleEightAuditSummary {
    pub audit_id: Uuid,
    pub request_status: String,
    pub source_platform: String,
    pub requested_at_label: String,
    pub card_title: Option<String>,
    pub status_message: Option<String>,
    pub completeness_score: Option<i32>,
}

#[derive(Debug, FromRow)]
pub struct ModuleEightAuditRecord {
    pub source_platform: String,
    pub source_url: String,
    pub canonical_url: String,
    pub request_status: String,
    pub current_stage: Option<String>,
    pub progress_percent: Option<i32>,
    pub status_message: Option<String>,
    pub error_summary: Option<String>,
    pub card_title: Option<String>,
    pub completeness_score: Option<i32>,
    pub photo_count: Option<i32>,
    pub menu_items_count: Option<i32>,
    pub contact_count: Option<i32>,
    pub social_count: Option<i32>,
    pub feature_count: Option<i32>,
    pub positive_review_share: Option<i32>,
    pub negative_review_share: Option<i32>,
    pub website_url: Option<String>,
    pub social_links: Option<String>,
    pub has_delivery: Option<bool>,
    pub ai_recommendations: Option<String>,
    pub competitor_snapshot: Option<String>,
    pub request_diagnostics: Option<String>,
    pub requested_at_label: String,
}

#[derive(Debug, FromRow)]
pub struct ModuleEightCompletedAuditRow {
    pub audit_id: Uuid,
    pub source_platform: String,
    pub source_url: String,
    pub canonical_url: String,
    pub request_status: String,
    pub current_stage: Option<String>,
    pub progress_percent: Option<i32>,
    pub status_message: Option<String>,
    pub error_summary: Option<String>,
    pub card_title: Option<String>,
    pub completeness_score: Option<i32>,
    pub photo_count: Option<i32>,
    pub menu_items_count: Option<i32>,
    pub contact_count: Option<i32>,
    pub social_count: Option<i32>,
    pub feature_count: Option<i32>,
    pub positive_review_share: Option<i32>,
    pub negative_review_share: Option<i32>,
    pub website_url: Option<String>,
    pub social_links: Option<String>,
    pub has_delivery: Option<bool>,
    pub ai_recommendations: Option<String>,
    pub competitor_snapshot: Option<String>,
    pub request_diagnostics: Option<String>,
    pub requested_at_label: String,
}

#[derive(Debug)]
pub struct ModuleEightReportDraft {
    pub card_title: String,
    pub completeness_score: i32,
    pub photo_count: i32,
    pub menu_items_count: i32,
    pub contact_count: i32,
    pub social_count: i32,
    pub feature_count: i32,
    pub positive_review_share: i32,
    pub negative_review_share: i32,
    pub website_url: Option<String>,
    pub social_links: String,
    pub has_delivery: bool,
    pub ai_recommendations: String,
    pub competitor_snapshot: String,
    pub request_diagnostics: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiveMapCardData {
    pub card_title: String,
    pub address: String,
    pub description: String,
    pub rating: Option<f32>,
    pub review_count: Option<i32>,
    pub photo_count: i32,
    pub menu_items_count: i32,
    pub contact_count: i32,
    pub social_count: i32,
    pub feature_count: i32,
    pub positive_review_share: i32,
    pub negative_review_share: i32,
    pub category_hint: String,
    pub website_url: Option<String>,
    pub social_links: Vec<String>,
    pub has_delivery: bool,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BrowserWorkerCardData {
    pub final_url: Option<String>,
    pub card_title: String,
    pub address: String,
    pub description: String,
    pub rating: Option<f32>,
    pub review_count: Option<i32>,
    pub photo_count: i32,
    pub menu_items_count: i32,
    pub contact_count: i32,
    pub social_count: i32,
    #[serde(default)]
    pub social_links: Vec<String>,
    pub feature_count: i32,
    #[serde(default)]
    pub has_delivery: bool,
    pub positive_review_share: i32,
    pub negative_review_share: i32,
    pub category_hint: String,
    pub website_url: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug)]
pub struct RemotePageSnapshot {
    pub final_url: String,
    pub body: String,
}

#[derive(Debug, FromRow)]
pub struct PendingAnalysisJob {
    pub request_id: Uuid,
    pub app_user_id: Uuid,
    pub business_profile_id: Uuid,
    pub source_platform: String,
    pub canonical_url: String,
}

#[derive(Debug, FromRow)]
pub struct PendingModuleEightJob {
    pub audit_id: Uuid,
    pub scenario_id: Uuid,
    pub source_platform: String,
    pub canonical_url: String,
}

#[derive(Debug, FromRow)]
pub struct ModuleNineReportSummary {
    pub report_id: Uuid,
    pub created_at_label: String,
    pub business_name: String,
    pub source_platform: String,
    pub card_rating_label: String,
    pub seo_score: i32,
    pub pdf_status: String,
    pub pdf_generated_at_label: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct ModuleNineReportRecord {
    pub report_id: Uuid,
    pub created_at_label: String,
    pub business_name: String,
    pub source_platform: String,
    pub card_rating_label: String,
    pub seo_score: i32,
    pub competitors_summary: String,
    pub recommendations: String,
    pub strengths: String,
    pub weaknesses: String,
    pub seo_analysis: String,
    pub pdf_status: String,
    pub pdf_file_name: Option<String>,
    pub pdf_disk_path: Option<String>,
    pub pdf_generated_at_label: Option<String>,
}

#[derive(Debug)]
pub struct ModuleNineReportDraft {
    pub business_name: String,
    pub source_platform: String,
    pub card_rating_label: String,
    pub seo_score: i32,
    pub competitors_summary: String,
    pub recommendations: String,
    pub strengths: String,
    pub weaknesses: String,
    pub seo_analysis: String,
}

#[derive(Debug, Clone, Copy)]
pub enum AnalysisStageKey {
    ValidateLink,
    LoadData,
    AnalyzeReviews,
    CompareCompetitors,
    BuildReport,
    SaveResult,
}

impl AnalysisStageKey {
    pub fn as_db_value(self) -> &'static str {
        match self {
            AnalysisStageKey::ValidateLink => "validate_link",
            AnalysisStageKey::LoadData => "load_data",
            AnalysisStageKey::AnalyzeReviews => "analyze_reviews",
            AnalysisStageKey::CompareCompetitors => "compare_competitors",
            AnalysisStageKey::BuildReport => "build_report",
            AnalysisStageKey::SaveResult => "save_result",
        }
    }

    pub fn index(self) -> i32 {
        match self {
            AnalysisStageKey::ValidateLink => 1,
            AnalysisStageKey::LoadData => 2,
            AnalysisStageKey::AnalyzeReviews => 3,
            AnalysisStageKey::CompareCompetitors => 4,
            AnalysisStageKey::BuildReport => 5,
            AnalysisStageKey::SaveResult => 6,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ModuleEightStageKey {
    ValidateRequest,
    OpenChrome,
    InspectCard,
    AnalyzeReviews,
    CompareCompetitors,
    PublishReport,
}

impl ModuleEightStageKey {
    pub fn as_db_value(self) -> &'static str {
        match self {
            ModuleEightStageKey::ValidateRequest => "validate_request",
            ModuleEightStageKey::OpenChrome => "open_chrome",
            ModuleEightStageKey::InspectCard => "inspect_card",
            ModuleEightStageKey::AnalyzeReviews => "analyze_reviews",
            ModuleEightStageKey::CompareCompetitors => "compare_competitors",
            ModuleEightStageKey::PublishReport => "publish_report",
        }
    }
}

#[derive(Debug)]
pub struct QueryAnalysis {
    pub normalized_query: String,
    pub result_count: usize,
    pub exact_match_count: usize,
    pub unique_cities: usize,
    pub unique_types: usize,
    pub lead_city: Option<String>,
    pub lead_type: Option<String>,
    pub focus_object_name: Option<String>,
    pub focus_object_city: Option<String>,
    pub confidence_label: &'static str,
    pub confidence_note: &'static str,
}
