#[path = "../content/mod.rs"]
mod content;

mod analysis;
mod db;
mod handlers;
mod models;
mod pages;
mod utils;

use anyhow::Context;
use axum::{Router, routing::get};
use models::AppState;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::path::PathBuf;

use handlers::{
    analysis_status_page, cabinet_page, create_analysis_request, download_module_nine_pdf, health,
    login_page, login_user, logout_user, module_eight_audit_page, module_eight_page,
    module_eight_submit, module_eleven_page, module_nine_page, module_nine_report_page,
    module_ten_page, module_twelve_page, module_twelve_submit, pricing_page,
    pro_monitoring_notifications_page, pro_monitoring_page, pro_package_page, pro_package_submit,
    register_page, register_success, register_user, root, search, subscription_action,
    subscription_page,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL is not set. Copy .env.example to .env and adjust it if needed.")?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;

    let row: (i64,) = sqlx::query_as("select count(*) from app_healthcheck")
        .fetch_one(&pool)
        .await
        .context("failed to run healthcheck query")?;

    println!(
        "PostgreSQL connection is ready. app_healthcheck rows: {}",
        row.0
    );

    let redis_url = std::env::var("REDIS_URL")
        .context("REDIS_URL is not set. Copy .env.example to .env and adjust it if needed.")?;
    let redis = redis::Client::open(redis_url)
        .context("failed to parse REDIS_URL")?
        .get_connection_manager()
        .await
        .context("failed to connect to Redis")?;

    println!("Redis connection is ready.");

    let report_storage_dir = PathBuf::from("storage/reports/module9");
    tokio::fs::create_dir_all(&report_storage_dir)
        .await
        .context("failed to create module 9 report storage directory")?;

    let state = AppState {
        pool,
        redis,
        report_storage_dir,
    };

    tokio::spawn(analysis::resume_background_jobs(state.clone()));

    let app = Router::new()
        .route("/", get(root))
        .route("/login", get(login_page).post(login_user))
        .route("/logout", get(logout_user))
        .route("/pricing", get(pricing_page))
        .route(
            "/pricing/pro",
            get(pro_package_page).post(pro_package_submit),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}",
            get(pro_monitoring_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/notifications",
            get(pro_monitoring_notifications_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-8",
            get(module_eight_page).post(module_eight_submit),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-8/{audit_id}",
            get(module_eight_audit_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-9",
            get(module_nine_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-9/{report_id}",
            get(module_nine_report_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-9/{report_id}/download",
            get(download_module_nine_pdf),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-10",
            get(module_ten_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-11",
            get(module_eleven_page),
        )
        .route(
            "/pricing/pro/monitor/{scenario_id}/module-12",
            get(module_twelve_page).post(module_twelve_submit),
        )
        .route("/search", get(search))
        .route("/register", get(register_page).post(register_user))
        .route("/register/success/{user_id}", get(register_success))
        .route(
            "/cabinet/{user_id}",
            get(cabinet_page).post(create_analysis_request),
        )
        .route(
            "/cabinet/{user_id}/subscription",
            get(subscription_page).post(subscription_action),
        )
        .route(
            "/cabinet/{user_id}/analysis/{request_id}",
            get(analysis_status_page),
        )
        .route("/health", get(health))
        .with_state(state);

    let address = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("HTTP server is listening on http://{address}");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .context("failed to bind HTTP listener on 0.0.0.0:8080")?;

    axum::serve(listener, app)
        .await
        .context("HTTP server stopped unexpectedly")?;

    Ok(())
}
