//! Admin settings panel: just enough to let the product owner set the
//! shared Claude API key and model without redeploying.
//!
//! Auth is deliberately minimal — a bearer token compared to `ADMIN_TOKEN` —
//! rather than a full roles/permissions system. `app_users` has no admin
//! role yet (see the project's own status notes on `account_role`); wiring
//! this to a real role belongs with that work, not here. Treat `ADMIN_TOKEN`
//! like any other production secret: long, random, not reused elsewhere.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::models::AppState;
use crate::settings::{
    self, CLAUDE_API_KEY_SETTING, CLAUDE_MODEL_SETTING, mask_secret,
};

fn is_authorized_admin(headers: &HeaderMap) -> bool {
    let Ok(expected) = std::env::var("ADMIN_TOKEN") else {
        // No ADMIN_TOKEN configured means the admin panel is off, not open.
        return false;
    };
    if expected.trim().is_empty() {
        return false;
    }

    let Some(header_value) = headers.get(header::AUTHORIZATION) else {
        return false;
    };
    let Ok(header_value) = header_value.to_str() else {
        return false;
    };

    header_value
        .strip_prefix("Bearer ")
        .map(|token| token == expected)
        .unwrap_or(false)
}

#[derive(Debug, Serialize)]
pub struct AdminSettingsView {
    pub claude_api_key_configured: bool,
    pub claude_api_key_masked: Option<String>,
    pub claude_api_key_source: Option<&'static str>,
    pub claude_model: Option<String>,
    pub claude_model_source: Option<&'static str>,
}

/// `GET /admin/settings` — view current config (key is masked, never returned raw).
pub async fn get_admin_settings(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if !is_authorized_admin(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let db_key = settings::get_setting(&state.pool, CLAUDE_API_KEY_SETTING)
        .await
        .ok()
        .flatten();
    let db_model = settings::get_setting(&state.pool, CLAUDE_MODEL_SETTING)
        .await
        .ok()
        .flatten();

    let (key_masked, key_configured, key_source) = match db_key {
        Some(key) if !key.trim().is_empty() => {
            (Some(mask_secret(&key)), true, Some("database"))
        }
        _ => match std::env::var("CLAUDE_API_KEY").ok().filter(|v| !v.trim().is_empty()) {
            Some(key) => (Some(mask_secret(&key)), true, Some("environment")),
            None => (None, false, None),
        },
    };

    let (model, model_source) = match db_model {
        Some(model) if !model.trim().is_empty() => (Some(model), Some("database")),
        _ => match std::env::var("CLAUDE_MODEL").ok().filter(|v| !v.trim().is_empty()) {
            Some(model) => (Some(model), Some("environment")),
            None => (None, None),
        },
    };

    Json(AdminSettingsView {
        claude_api_key_configured: key_configured,
        claude_api_key_masked: key_masked,
        claude_api_key_source: key_source,
        claude_model: model,
        claude_model_source: model_source,
    })
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct UpdateAdminSettingsRequest {
    #[serde(default)]
    pub claude_api_key: Option<String>,
    #[serde(default)]
    pub claude_model: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateAdminSettingsResponse {
    pub updated: Vec<&'static str>,
}

/// `POST /admin/settings` — set the shared Claude API key and/or model.
/// Either field may be omitted to leave it unchanged.
pub async fn update_admin_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateAdminSettingsRequest>,
) -> Response {
    if !is_authorized_admin(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mut updated = Vec::new();

    if let Some(key) = payload.claude_api_key {
        let key = key.trim();
        if key.is_empty() {
            return (StatusCode::BAD_REQUEST, "claude_api_key cannot be empty").into_response();
        }
        if settings::set_setting(&state.pool, CLAUDE_API_KEY_SETTING, key, "admin_panel")
            .await
            .is_err()
        {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        updated.push("claude_api_key");
    }

    if let Some(model) = payload.claude_model {
        let model = model.trim();
        if model.is_empty() {
            return (StatusCode::BAD_REQUEST, "claude_model cannot be empty").into_response();
        }
        if settings::set_setting(&state.pool, CLAUDE_MODEL_SETTING, model, "admin_panel")
            .await
            .is_err()
        {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }
        updated.push("claude_model");
    }

    Json(UpdateAdminSettingsResponse { updated }).into_response()
}
