//! Minimal client for the Claude Messages API (api.anthropic.com/v1/messages).
//!
//! Intentionally small: one function, no retries, no streaming. This exists
//! to prove the shared-key wiring end to end (settings -> client -> a quota-
//! checked cabinet endpoint); it is not a general-purpose SDK. Reach for the
//! official Anthropic SDK instead if this needs to grow.

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use serde_json::json;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 1024;

#[derive(Clone)]
pub struct ClaudeClient {
    http: reqwest::Client,
}

impl ClaudeClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    /// Sends a single-turn request and returns the concatenated text of the
    /// response. `api_key` and `model` come from `crate::settings` so the
    /// caller controls where they're sourced from (DB setting vs env var).
    pub async fn generate_text(
        &self,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
    ) -> anyhow::Result<String> {
        let body = json!({
            "model": model,
            "max_tokens": DEFAULT_MAX_TOKENS,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": user_message }
            ]
        });

        let response = self
            .http
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("failed to reach the Claude API")?;

        let status = response.status();
        let raw = response
            .text()
            .await
            .context("failed to read the Claude API response body")?;

        if !status.is_success() {
            bail!("Claude API returned {status}: {raw}");
        }

        let parsed: ClaudeMessageResponse =
            serde_json::from_str(&raw).context("failed to parse the Claude API response")?;

        let text = parsed
            .content
            .into_iter()
            .filter_map(|block| block.text)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();

        if text.is_empty() {
            return Err(anyhow!("Claude API returned no text content"));
        }

        Ok(text)
    }
}

impl Default for ClaudeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct ClaudeMessageResponse {
    content: Vec<ClaudeContentBlock>,
}

#[derive(Deserialize)]
struct ClaudeContentBlock {
    #[serde(default)]
    text: Option<String>,
}
