//! Telegram link code repository for the secure bot-linking flow.
//!
//! ## TelegramLinkCode
//!
//! Row struct for the `telegram_link_codes` table. Each code is a short
//! alphanumeric string with a 10-minute expiry, tied to a single user.
//!
//! ## create_link_code
//!
//! Invalidates any existing unused codes for the user, then inserts a
//! fresh code. Returns the new row.
//!
//! ## validate_and_consume_code
//!
//! Finds an unused, non-expired code matching the given string. If found,
//! marks it as used and returns the associated user_id. Returns `None` if
//! the code is invalid, expired, or already used.
//!
//! ## cleanup_expired_codes
//!
//! Deletes expired or used codes older than 1 hour. Called periodically
//! by the background worker.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TelegramLinkCode {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_link_code(
    pool: &PgPool,
    user_id: Uuid,
    code: &str,
    expires_at: DateTime<Utc>,
) -> eyre::Result<TelegramLinkCode> {
    sqlx::query(
        "UPDATE telegram_link_codes SET used = TRUE WHERE user_id = $1 AND used = FALSE"
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    let row = sqlx::query_as::<_, TelegramLinkCode>(
        "INSERT INTO telegram_link_codes (user_id, code, expires_at)
         VALUES ($1, $2, $3)
         RETURNING id, user_id, code, expires_at, used, created_at"
    )
    .bind(user_id)
    .bind(code)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn validate_and_consume_code(
    pool: &PgPool,
    code: &str,
) -> eyre::Result<Option<Uuid>> {
    let row = sqlx::query_as::<_, TelegramLinkCode>(
        "UPDATE telegram_link_codes
         SET used = TRUE
         WHERE code = $1 AND used = FALSE AND expires_at > NOW()
         RETURNING id, user_id, code, expires_at, used, created_at"
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.user_id))
}

pub async fn cleanup_expired_codes(pool: &PgPool) -> eyre::Result<u64> {
    let result = sqlx::query(
        "DELETE FROM telegram_link_codes
         WHERE (used = TRUE OR expires_at < NOW())
         AND created_at < NOW() - INTERVAL '1 hour'"
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
