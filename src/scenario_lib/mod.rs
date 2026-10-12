//! Scenario-replay toolkit for exercising Geonetix's server-rendered pages
//! the way a real user's browser would: real HTTP requests, a shared
//! cookie jar across steps, and assertions against the *rendered* page
//! (redirects, visible text, CSS selectors) rather than internal state.
//!
//! Scenarios are plain JSON files (see `scenarios/` at the repo root and
//! `scenarios/README.md` for the format) so they can be replayed by this
//! Rust HTTP-level runner and, optionally, by a real-browser Playwright
//! runner (`ui-tests/`) without maintaining two divergent step lists.
//!
//! Entry points:
//! - [`run_scenario_file`] / [`run_scenario`] to execute a scenario against
//!   a running server and get back a [`ScenarioReport`].
//! - [`print_report`] for a human-readable summary (used by both the
//!   `scenario-runner` CLI binary and the `cargo test` harness).

mod client;
mod steps;
mod template;

pub use client::{PageResponse, Session};
pub use steps::{
    ScenarioFile, ScenarioReport, StepDef, StepReport, run_scenario, run_scenario_file,
};
pub use template::TemplateContext;

/// Checks that a server is actually reachable at `base_url` before running
/// scenarios against it, so a misconfigured `base_url` fails fast with a
/// clear message instead of every step timing out one by one.
pub async fn ensure_server_reachable(base_url: &str) -> Result<(), String> {
    let session = Session::new(base_url)?;
    session
        .get("/health")
        .await
        .map(|_| ())
        .map_err(|error| {
            format!(
                "no Geonetix server reachable at {base_url} ({error}). \
                 Start it first (`cargo run`) or point GEONETIX_TEST_BASE_URL at a running instance."
            )
        })
}

/// Renders a [`ScenarioReport`] as readable, indented text.
pub fn format_report(report: &ScenarioReport) -> String {
    let mut out = String::new();
    let status = if report.passed { "PASSED" } else { "FAILED" };
    out.push_str(&format!(
        "[{status}] {} ({:.2?})\n",
        report.scenario_name, report.duration
    ));
    for step in &report.steps {
        let mark = if step.passed { "ok  " } else { "FAIL" };
        out.push_str(&format!(
            "  {mark} {} ({:.2?})\n",
            step.label, step.duration
        ));
        if let Some(error) = &step.error {
            out.push_str(&format!("       -> {error}\n"));
        }
    }
    out
}

pub fn print_report(report: &ScenarioReport) {
    print!("{}", format_report(report));
}
