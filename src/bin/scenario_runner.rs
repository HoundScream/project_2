//! Ad-hoc CLI for replaying one or more scenario files against a running
//! Geonetix server. Meant for manual/exploratory runs (local dev, staging);
//! `cargo test` (see `tests/run_scenarios.rs`) is the automated entry point.
//!
//! Usage:
//!   cargo run --bin scenario-runner -- scenarios/*.json
//!   cargo run --bin scenario-runner -- --base-url http://staging.example.com scenarios/login_existing_user.json

use std::path::PathBuf;
use std::process::ExitCode;

use geonetix::scenario_lib::{ensure_server_reachable, print_report, run_scenario_file};

#[tokio::main]
async fn main() -> ExitCode {
    let mut base_url = std::env::var("GEONETIX_TEST_BASE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let mut scenario_paths: Vec<PathBuf> = Vec::new();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--base-url" => {
                base_url = args
                    .next()
                    .unwrap_or_else(|| panic!("--base-url requires a value"));
            }
            path => scenario_paths.push(PathBuf::from(path)),
        }
    }

    if scenario_paths.is_empty() {
        eprintln!("usage: scenario-runner [--base-url URL] <scenario.json> [more.json ...]");
        return ExitCode::FAILURE;
    }

    if let Err(error) = ensure_server_reachable(&base_url).await {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }

    let mut all_passed = true;
    for path in scenario_paths {
        match run_scenario_file(&path, &base_url).await {
            Ok(report) => {
                all_passed &= report.passed;
                print_report(&report);
            }
            Err(error) => {
                all_passed = false;
                eprintln!("error running {}: {error}", path.display());
            }
        }
    }

    if all_passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
