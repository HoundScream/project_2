//! Runs every scenario under `scenarios/*.json` against a live Geonetix
//! server and fails the test if any step in any scenario fails.
//!
//! Requires a running server (`cargo run`, with Postgres and Redis
//! reachable per `.env`) at `GEONETIX_TEST_BASE_URL`
//! (default `http://127.0.0.1:8080`). This is a deliberate choice over
//! spawning the server in-process: replaying scenarios as real HTTP
//! requests against a real running instance is what actually reproduces
//! what a user's browser does (cookies, redirects, timing), and the same
//! suite can then be pointed at a staging/CI deployment unchanged.

use std::path::Path;

use geonetix::scenario_lib::{ensure_server_reachable, format_report, run_scenario_file};

fn base_url() -> String {
    std::env::var("GEONETIX_TEST_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string())
}

fn scenario_files() -> Vec<std::path::PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("cannot read scenarios dir {}: {error}", dir.display()))
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();
    files.sort();
    files
}

#[tokio::test]
async fn all_scenarios_pass() {
    let base_url = base_url();
    if let Err(error) = ensure_server_reachable(&base_url).await {
        panic!("{error}");
    }

    let files = scenario_files();
    assert!(
        !files.is_empty(),
        "no scenario files found under scenarios/*.json"
    );

    let mut failures = Vec::new();
    for path in &files {
        let report = run_scenario_file(path, &base_url)
            .await
            .unwrap_or_else(|error| panic!("failed to load scenario {}: {error}", path.display()));
        print!("{}", format_report(&report));
        if !report.passed {
            failures.push(report.scenario_name);
        }
    }

    assert!(
        failures.is_empty(),
        "scenario(s) failed: {}",
        failures.join(", ")
    );
}
