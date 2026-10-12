use sqlx::PgPool;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: redis::aio::ConnectionManager,
    pub report_storage_dir: PathBuf,
}
