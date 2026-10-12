//! "Organization search" has two different meanings in the product, so this
//! module covers both:
//!
//! 1. **Internal search** — a user's own saved business cards
//!    (`business_profiles`), e.g. when they manage several locations and want
//!    to find one by name instead of scrolling the cabinet.
//! 2. **External lookup** — looking up a real company by name or INN via
//!    DaData, for onboarding (verifying who you're adding before you paste a
//!    Yandex Maps / 2GIS link for them).
//!
//! Both are read-only JSON endpoints; neither writes to `business_profiles`.

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::fetch_session_user;
use crate::models::AppState;

#[derive(Debug, Deserialize)]
pub struct OrgSearchParams {
    #[serde(default)]
    pub q: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OwnedOrganization {
    pub id: Uuid,
    pub source_platform: String,
    pub canonical_url: String,
    pub profile_title: Option<String>,
    pub profile_address: Option<String>,
    pub ingestion_status: String,
}

/// `GET /cabinet/{user_id}/organizations/search?q=...`
///
/// Searches only the signed-in user's own business profiles. Requires a
/// valid session matching `user_id` in the path, mirroring
/// `require_authenticated_user` in `handlers.rs` — this is not public data.
pub async fn search_own_organizations(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(user_id): axum::extract::Path<Uuid>,
    Query(params): Query<OrgSearchParams>,
) -> Response {
    match fetch_session_user(&state.pool, &headers).await {
        Ok(Some(session_user)) if session_user.id == user_id => {}
        Ok(_) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }

    let query = params.q.trim();
    let like_pattern = format!("%{query}%");

    let rows: Vec<OwnedOrganization> = match sqlx::query_as(
        r#"
        select id, source_platform, canonical_url, profile_title, profile_address, ingestion_status
        from business_profiles
        where app_user_id = $1
          and (
              $2 = ''
              or profile_title ilike $3
              or profile_address ilike $3
              or canonical_url ilike $3
          )
        order by created_at desc
        limit 50
        "#,
    )
    .bind(user_id)
    .bind(query)
    .bind(&like_pattern)
    .fetch_all(&state.pool)
    .await
    {
        Ok(rows) => rows,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    Json(rows).into_response()
}

#[derive(Debug, Serialize)]
pub struct ExternalOrganizationSuggestion {
    pub name: String,
    pub inn: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExternalLookupResponse {
    pub query: String,
    pub suggestions: Vec<ExternalOrganizationSuggestion>,
}

#[derive(Debug, Serialize)]
pub struct ExternalLookupUnavailable {
    pub error: &'static str,
    pub detail: &'static str,
}

/// `GET /organizations/lookup?q=...`
///
/// Looks up real companies by name or INN through DaData's `suggest/party`
/// endpoint, for verifying a prospect before onboarding them. Needs
/// `DADATA_API_KEY` set; returns 501 with a clear reason if it isn't, rather
/// than crashing or silently returning nothing.
pub async fn lookup_external_organizations(Query(params): Query<OrgSearchParams>) -> Response {
    let query = params.q.trim();
    if query.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ExternalLookupResponse {
                query: query.to_string(),
                suggestions: vec![],
            }),
        )
            .into_response();
    }

    let Ok(api_key) = std::env::var("DADATA_API_KEY") else {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(ExternalLookupUnavailable {
                error: "dadata_not_configured",
                detail: "Set DADATA_API_KEY to enable company lookup by name or INN.",
            }),
        )
            .into_response();
    };

    let client = reqwest::Client::new();
    let body = serde_json::json!({ "query": query, "count": 10 });

    let response = client
        .post("https://suggestions.dadata.ru/suggestions/api/4_1/rs/suggest/party")
        .header("Authorization", format!("Token {api_key}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };

    if !response.status().is_success() {
        return StatusCode::BAD_GATEWAY.into_response();
    }

    let parsed: DadataSuggestResponse = match response.json().await {
        Ok(parsed) => parsed,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };

    let suggestions = parsed
        .suggestions
        .into_iter()
        .map(|item| ExternalOrganizationSuggestion {
            name: item.value,
            inn: item.data.inn,
            address: item.data.address.and_then(|a| a.value),
        })
        .collect();

    Json(ExternalLookupResponse {
        query: query.to_string(),
        suggestions,
    })
    .into_response()
}

#[derive(Debug, Deserialize)]
struct DadataSuggestResponse {
    suggestions: Vec<DadataSuggestion>,
}

#[derive(Debug, Deserialize)]
struct DadataSuggestion {
    value: String,
    data: DadataPartyData,
}

#[derive(Debug, Deserialize)]
struct DadataPartyData {
    inn: Option<String>,
    address: Option<DadataAddress>,
}

#[derive(Debug, Deserialize)]
struct DadataAddress {
    value: Option<String>,
}
