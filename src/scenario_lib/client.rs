use std::sync::Arc;
use std::time::Duration;

use reqwest::cookie::Jar;
use reqwest::redirect::Policy;
use scraper::{Html, Selector};

/// A single simulated browser session: keeps cookies across requests (like a
/// real user's browser would) and never follows redirects automatically, so
/// scenario steps can assert on the redirect itself before deciding whether
/// to navigate onward.
pub struct Session {
    client: reqwest::Client,
    base_url: String,
}

/// One HTTP response, with helpers matching what a scenario step needs to
/// assert against: status, redirect target, and rendered (tag-stripped)
/// visible text, the same way a user reading the page would experience it.
pub struct PageResponse {
    pub status: u16,
    pub location: Option<String>,
    pub body: String,
}

impl Session {
    pub fn new(base_url: impl Into<String>) -> Result<Self, String> {
        let jar = Arc::new(Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(jar)
            .redirect(Policy::none())
            .timeout(Duration::from_secs(20))
            .user_agent("geonetix-scenario-runner/1.0")
            .build()
            .map_err(|error| format!("failed to build HTTP client: {error}"))?;
        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    fn full_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        }
    }

    pub async fn get(&self, path: &str) -> Result<PageResponse, String> {
        let response = self
            .client
            .get(self.full_url(path))
            .send()
            .await
            .map_err(|error| format!("GET {path} failed: {error}"))?;
        PageResponse::from_response(response).await
    }

    pub async fn post_form(
        &self,
        path: &str,
        fields: &[(String, String)],
    ) -> Result<PageResponse, String> {
        let response = self
            .client
            .post(self.full_url(path))
            .form(fields)
            .send()
            .await
            .map_err(|error| format!("POST {path} failed: {error}"))?;
        PageResponse::from_response(response).await
    }
}

impl PageResponse {
    async fn from_response(response: reqwest::Response) -> Result<Self, String> {
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = response
            .text()
            .await
            .map_err(|error| format!("failed to read response body: {error}"))?;
        Ok(Self {
            status,
            location,
            body,
        })
    }

    /// The page's visible text, the way a user reading the rendered page
    /// would experience it: the browser tab title plus the rendered body
    /// text, tags stripped, whitespace collapsed. Used for substring
    /// assertions and for `wait_for_text` polling.
    pub fn visible_text(&self) -> String {
        let document = Html::parse_document(&self.body);
        let mut parts = Vec::new();
        for selector_str in ["title", "body"] {
            let Ok(selector) = Selector::parse(selector_str) else {
                continue;
            };
            if let Some(node) = document.select(&selector).next() {
                parts.push(node.text().collect::<Vec<_>>().join(" "));
            }
        }
        parts
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn matches_selector(&self, css: &str) -> bool {
        let document = Html::parse_document(&self.body);
        match Selector::parse(css) {
            Ok(selector) => document.select(&selector).next().is_some(),
            Err(_) => false,
        }
    }

    /// Extracts the run of id-like characters (`[A-Za-z0-9-]`) immediately
    /// following the first occurrence of `after` inside `source`. Covers
    /// the UUID-in-path/redirect pattern used throughout Geonetix
    /// (`/register/success/{id}`, `/cabinet/{id}/analysis/{id}`, ...)
    /// without pulling in a regex dependency for a single fixed shape.
    pub fn capture_after(source: &str, after: &str) -> Option<String> {
        let start = source.find(after)? + after.len();
        let tail = &source[start..];
        let end = tail
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-'))
            .unwrap_or(tail.len());
        let captured = &tail[..end];
        if captured.is_empty() {
            None
        } else {
            Some(captured.to_string())
        }
    }
}
