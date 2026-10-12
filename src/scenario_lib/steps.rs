use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::scenario_lib::client::{PageResponse, Session};
use crate::scenario_lib::template::TemplateContext;

/// A scenario file: a named, ordered list of steps that reproduce one
/// real user journey (e.g. "register, then run a free analysis").
#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    pub steps: Vec<StepDef>,
}

/// One step. Not every field applies to every `action`; unused fields are
/// simply ignored (see `run_step`/`run_wait_for_text` for exactly what each
/// action reads). Kept as one flat struct rather than an enum-per-action so
/// scenario JSON stays easy to hand-write and the same optional
/// assertions (`expect_status`, `expect_text`, `capture`, ...) can be
/// reused after any action.
#[derive(Debug, Clone, Deserialize)]
pub struct StepDef {
    pub action: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: String,
    /// Form fields for `fill_form`, rendered through the template context.
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
    #[serde(default)]
    pub expect_status: Option<u16>,
    #[serde(default)]
    pub expect_text: Vec<String>,
    #[serde(default)]
    pub expect_selector: Vec<String>,
    #[serde(default)]
    pub expect_redirect_contains: Option<String>,
    /// `{ "captured_name": "text immediately before the id to capture" }`
    /// Looked up in the `Location` header for redirects, in the body text
    /// otherwise.
    #[serde(default)]
    pub capture: BTreeMap<String, String>,
    /// Text to poll for, `wait_for_text` only.
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub poll_ms: Option<u64>,
}

#[derive(Debug)]
pub struct StepReport {
    pub label: String,
    pub passed: bool,
    pub duration: Duration,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct ScenarioReport {
    pub scenario_name: String,
    pub steps: Vec<StepReport>,
    pub passed: bool,
    pub duration: Duration,
}

pub async fn run_scenario_file(path: &Path, base_url: &str) -> Result<ScenarioReport, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read scenario file {}: {error}", path.display()))?;
    let scenario: ScenarioFile = serde_json::from_str(&raw)
        .map_err(|error| format!("invalid scenario JSON in {}: {error}", path.display()))?;
    run_scenario(scenario, base_url).await
}

pub async fn run_scenario(
    scenario: ScenarioFile,
    base_url: &str,
) -> Result<ScenarioReport, String> {
    let session = Session::new(base_url)?;
    let mut ctx = TemplateContext::new(scenario.vars);
    let started = Instant::now();
    let mut step_reports = Vec::with_capacity(scenario.steps.len());
    let mut overall_passed = true;

    for (index, step) in scenario.steps.iter().enumerate() {
        let label = step
            .name
            .clone()
            .unwrap_or_else(|| format!("step {} ({} {})", index + 1, step.action, step.path));
        let step_started = Instant::now();
        let outcome = run_step(&session, &mut ctx, step).await;
        let duration = step_started.elapsed();

        match outcome {
            Ok(()) => step_reports.push(StepReport {
                label,
                passed: true,
                duration,
                error: None,
            }),
            Err(error) => {
                step_reports.push(StepReport {
                    label,
                    passed: false,
                    duration,
                    error: Some(error),
                });
                overall_passed = false;
                break; // later steps typically depend on captures from this one
            }
        }
    }

    Ok(ScenarioReport {
        scenario_name: scenario.name,
        steps: step_reports,
        passed: overall_passed,
        duration: started.elapsed(),
    })
}

async fn run_step(
    session: &Session,
    ctx: &mut TemplateContext,
    step: &StepDef,
) -> Result<(), String> {
    match step.action.as_str() {
        "navigate" => {
            let path = ctx.render(&step.path)?;
            let response = session.get(&path).await?;
            apply_common_assertions(step, ctx, &response)
        }
        "fill_form" => {
            let path = ctx.render(&step.path)?;
            let mut rendered_fields = Vec::with_capacity(step.fields.len());
            for (key, value) in &step.fields {
                rendered_fields.push((key.clone(), ctx.render(value)?));
            }
            let response = session.post_form(&path, &rendered_fields).await?;
            apply_common_assertions(step, ctx, &response)
        }
        "wait_for_text" => run_wait_for_text(session, ctx, step).await,
        other => Err(format!("unknown step action `{other}`")),
    }
}

async fn run_wait_for_text(
    session: &Session,
    ctx: &mut TemplateContext,
    step: &StepDef,
) -> Result<(), String> {
    let path = ctx.render(&step.path)?;
    let needle = step
        .text
        .as_deref()
        .ok_or_else(|| "wait_for_text step is missing `text`".to_string())?;
    let needle = ctx.render(needle)?;
    let timeout = Duration::from_millis(step.timeout_ms.unwrap_or(8_000));
    let poll = Duration::from_millis(step.poll_ms.unwrap_or(400));
    let deadline = Instant::now() + timeout;

    let mut last_response: Option<PageResponse> = None;
    loop {
        let response = session.get(&path).await?;
        let found = response.visible_text().contains(&needle);
        let is_last_attempt = Instant::now() >= deadline;
        if found || is_last_attempt {
            last_response = Some(response);
            if found {
                break;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(poll).await;
    }

    let response = last_response
        .ok_or_else(|| format!("wait_for_text on {path} never received a response"))?;
    if !response.visible_text().contains(&needle) {
        return Err(format!(
            "wait_for_text timed out after {:?} waiting for text `{needle}` on {path}",
            timeout
        ));
    }
    apply_common_assertions(step, ctx, &response)
}

fn apply_common_assertions(
    step: &StepDef,
    ctx: &mut TemplateContext,
    response: &PageResponse,
) -> Result<(), String> {
    if let Some(expected) = step.expect_status
        && response.status != expected
    {
        return Err(format!(
            "expected HTTP status {expected}, got {} for {}",
            response.status, step.path
        ));
    }

    if let Some(fragment) = &step.expect_redirect_contains {
        let fragment = ctx.render(fragment)?;
        match &response.location {
            Some(location) if location.contains(fragment.as_str()) => {}
            Some(location) => {
                return Err(format!(
                    "expected redirect Location to contain `{fragment}`, got `{location}`"
                ));
            }
            None => {
                return Err(format!(
                    "expected a redirect containing `{fragment}`, but response had no Location header (status {})",
                    response.status
                ));
            }
        }
    }

    let visible_text = response.visible_text();
    for expected in &step.expect_text {
        let expected = ctx.render(expected)?;
        if !visible_text.contains(&expected) {
            return Err(format!(
                "expected page text to contain `{expected}`, it did not (path: {})",
                step.path
            ));
        }
    }

    for selector in &step.expect_selector {
        if !response.matches_selector(selector) {
            return Err(format!(
                "expected an element matching selector `{selector}` on {}, found none",
                step.path
            ));
        }
    }

    for (name, after) in &step.capture {
        let after = ctx.render(after)?;
        let source = response.location.as_deref().unwrap_or(&response.body);
        let captured = PageResponse::capture_after(source, &after).ok_or_else(|| {
            format!(
                "capture `{name}`: could not find an id-like value after `{after}` in {} (path: {})",
                if response.location.is_some() { "redirect Location" } else { "response body" },
                step.path
            )
        })?;
        ctx.set(name.clone(), captured);
    }

    Ok(())
}
