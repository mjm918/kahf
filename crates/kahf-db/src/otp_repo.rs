//! OTP repository for the `email_otps` table.
//!
//! ## OtpRow
//!
//! Database row struct: `id`, `user_id`, `code` (6-digit string),
//! `expires_at`, `used`, `created_at`.
//!
//! ## create_otp
//!
//! Inserts a new OTP for the given user. Caller provides the code and
//! expiration time.
//!
//! ## get_valid_otp
//!
//! Fetches the most recent unused, non-expired OTP for a user matching
//! the given code. Returns `None` if no valid OTP exists.
//!
//! ## mark_otp_used
//!
//! Sets `used = true` on the given OTP row.
//!
//! ## invalidate_user_otps
//!
//! Marks all unused OTPs for a user as used, called before issuing a
//! new OTP to prevent stale codes from being accepted.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OtpRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_otp(
    pool: &PgPool,
    user_id: Uuid,
    code: &str,
    expires_at: DateTime<Utc>,
) -> eyre::Result<OtpRow> {
    let row = sqlx::query_as::<_, OtpRow>(
        "INSERT INTO email_otps (user_id, code, expires_at)
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

pub async fn get_valid_otp(
    pool: &PgPool,
    user_id: Uuid,
    code: &str,
) -> eyre::Result<Option<OtpRow>> {
    let row = sqlx::query_as::<_, OtpRow>(
        "SELECT id, user_id, code, expires_at, used, created_at
         FROM email_otps
         WHERE user_id = $1 AND code = $2 AND used = false AND expires_at > now()
         ORDER BY created_at DESC
         LIMIT 1"
    )
    .bind(user_id)
    .bind(code)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn mark_otp_used(pool: &PgPool, otp_id: Uuid) -> eyre::Result<()> {
    sqlx::query("UPDATE email_otps SET used = true WHERE id = $1")
        .bind(otp_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn invalidate_user_otps(pool: &PgPool, user_id: Uuid) -> eyre::Result<()> {
    sqlx::query("UPDATE email_otps SET used = true WHERE user_id = $1 AND used = false")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}
