//! Shared app settings: today this is just the Claude API key and model,
//! stored once (DB-backed, admin-editable) instead of per-user.
//!
//! Precedence: a value saved in `app_settings` (via the admin panel) wins;
//! otherwise we fall back to the matching environment variable so the app
//! still works on a box where nobody has opened the admin panel yet.

use sqlx::PgPool;

pub const CLAUDE_API_KEY_SETTING: &str = "claude_api_key";
pub const CLAUDE_MODEL_SETTING: &str = "claude_model";

pub async fn get_setting(pool: &PgPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> =
        sqlx::query_as("select value from app_settings where key = $1")
            .bind(key)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|(value,)| value))
}

pub async fn set_setting(
    pool: &PgPool,
    key: &str,
    value: &str,
    changed_by: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        insert into app_settings (key, value, updated_at)
        values ($1, $2, now())
        on conflict (key) do update
        set value = excluded.value,
            updated_at = now()
        "#,
    )
    .bind(key)
    .bind(value)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        insert into app_settings_audit (setting_key, changed_by)
        values ($1, $2)
        "#,
    )
    .bind(key)
    .bind(changed_by)
    .execute(&mut *tx)
    .await?;

    tx.commit().await
}

/// Resolves the Claude API key: DB setting first, then `CLAUDE_API_KEY` env.
pub async fn resolve_claude_api_key(pool: &PgPool) -> Result<Option<String>, sqlx::Error> {
    if let Some(value) = get_setting(pool, CLAUDE_API_KEY_SETTING).await? {
        if !value.trim().is_empty() {
            return Ok(Some(value));
        }
    }
    Ok(std::env::var("CLAUDE_API_KEY").ok().filter(|v| !v.trim().is_empty()))
}

/// Resolves the Claude model id: DB setting first, then `CLAUDE_MODEL` env.
/// Deliberately has no hardcoded fallback — model ids change over time and a
/// stale default would fail silently in a confusing way. Whoever wires this
/// up picks the current model id from docs.claude.com/en/docs/about-claude/models.
pub async fn resolve_claude_model(pool: &PgPool) -> Result<Option<String>, sqlx::Error> {
    if let Some(value) = get_setting(pool, CLAUDE_MODEL_SETTING).await? {
        if !value.trim().is_empty() {
            return Ok(Some(value));
        }
    }
    Ok(std::env::var("CLAUDE_MODEL").ok().filter(|v| !v.trim().is_empty()))
}

/// Masks a secret for display: keeps the first 4 and last 4 characters.
pub fn mask_secret(secret: &str) -> String {
    let len = secret.chars().count();
    if len <= 8 {
        return "*".repeat(len);
    }
    let chars: Vec<char> = secret.chars().collect();
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[len - 4..].iter().collect();
    format!("{head}{}{tail}", "*".repeat(len - 8))
}
