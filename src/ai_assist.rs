//! Cabinet-facing endpoint that actually calls Claude, using the shared key
//! from `settings` and spending one unit of the user's existing analysis
//! quota per call — the same `analysis_quota_used` counter the free-tier
//! audits use, so usage shows up in one place instead of a second limit
//! system. Scope is intentionally narrow: answer a short question about one
//! of the user's own business profiles. Swap in a different prompt/shape
//! once there's a concrete second use case.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::claude_client::ClaudeClient;
use crate::db::fetch_session_user;
use crate::models::AppState;
use crate::settings::{resolve_claude_api_key, resolve_claude_model};

const SYSTEM_PROMPT: &str = "You are an assistant inside Geonetix, a tool that audits \
business listing cards (Yandex Maps, 2GIS). Answer the user's question about their own \
business profile briefly, in Russian, in plain language. Do not invent data you were not \
given.";

#[derive(Debug, Deserialize)]
pub struct AiAssistRequest {
    pub question: String,
    #[serde(default)]
    pub business_profile_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct AiAssistResponse {
    pub answer: String,
}

#[derive(Debug, Serialize)]
pub struct AiAssistError {
    pub error: &'static str,
    pub detail: String,
}

fn error_response(status: StatusCode, error: &'static str, detail: impl Into<String>) -> Response {
    (
        status,
        Json(AiAssistError {
            error,
            detail: detail.into(),
        }),
    )
        .into_response()
}

/// `POST /cabinet/{user_id}/ai/ask`
pub async fn ask_ai_assistant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AiAssistRequest>,
) -> Response {
    let session_user = match fetch_session_user(&state.pool, &headers).await {
        Ok(Some(user)) if user.id == user_id => user,
        Ok(_) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };

    let question = payload.question.trim();
    if question.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "empty_question", "question is required");
    }

    if session_user.analysis_quota_used >= session_user.analysis_quota_limit {
        return error_response(
            StatusCode::PAYMENT_REQUIRED,
            "quota_exhausted",
            "Free analysis quota is used up; upgrade to PRO to keep using the AI assistant.",
        );
    }

    let Ok(Some(api_key)) = resolve_claude_api_key(&state.pool).await else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "claude_not_configured",
            "No Claude API key is configured yet. An admin needs to set one via POST /admin/settings.",
        );
    };

    let Ok(Some(model)) = resolve_claude_model(&state.pool).await else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "claude_model_not_configured",
            "No Claude model id is configured yet. An admin needs to set one via POST /admin/settings.",
        );
    };

    let request_record: Result<(Uuid,), sqlx::Error> = sqlx::query_as(
        r#"
        insert into ai_assist_requests (app_user_id, business_profile_id, prompt, status)
        values ($1, $2, $3, 'queued')
        returning id
        "#,
    )
    .bind(session_user.id)
    .bind(payload.business_profile_id)
    .bind(question)
    .fetch_one(&state.pool)
    .await;

    let Ok((request_id,)) = request_record else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let client = ClaudeClient::new();
    let result = client
        .generate_text(&api_key, &model, SYSTEM_PROMPT, question)
        .await;

    match result {
        Ok(answer) => {
            let _ = sqlx::query(
                r#"
                update ai_assist_requests
                set response = $2, model = $3, status = 'completed', completed_at = now()
                where id = $1
                "#,
            )
            .bind(request_id)
            .bind(&answer)
            .bind(&model)
            .execute(&state.pool)
            .await;

            let _ = sqlx::query(
                r#"
                update app_users
                set analysis_quota_used = analysis_quota_used + 1,
                    updated_at = now()
                where id = $1
                "#,
            )
            .bind(session_user.id)
            .execute(&state.pool)
            .await;

            Json(AiAssistResponse { answer }).into_response()
        }
        Err(err) => {
            let _ = sqlx::query(
                r#"
                update ai_assist_requests
                set status = 'failed', error_message = $2
                where id = $1
                "#,
            )
            .bind(request_id)
            .bind(err.to_string())
            .execute(&state.pool)
            .await;

            error_response(StatusCode::BAD_GATEWAY, "claude_request_failed", err.to_string())
        }
    }
}
