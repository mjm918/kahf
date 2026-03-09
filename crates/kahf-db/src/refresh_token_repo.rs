//! Refresh token repository for the `refresh_tokens` table.
//!
//! ## RefreshTokenRow
//!
//! Database row struct: `id`, `user_id`, `token_hash`, `expires_at`,
//! `revoked`, `created_at`.
//!
//! ## store_refresh_token
//!
//! Inserts a new refresh token record with a SHA-256 hash of the token
//! string. The raw token is never stored.
//!
//! ## validate_refresh_token
//!
//! Looks up a non-revoked, non-expired refresh token by its hash.
//! Returns the row if valid, None otherwise.
//!
//! ## revoke_token
//!
//! Marks a single refresh token as revoked.
//!
//! ## revoke_all_user_tokens
//!
//! Revokes all active refresh tokens for a user (used on logout).
//!
//! ## delete_expired_tokens
//!
//! Bulk removes all expired or revoked tokens. Called by background
//! worker on a schedule. Returns number of deleted rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
    pub created_at: DateTime<Utc>,
}

pub fn hash_token(token: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub async fn store_refresh_token(
    pool: &PgPool,
    user_id: Uuid,
    token: &str,
    expires_at: DateTime<Utc>,
) -> eyre::Result<RefreshTokenRow> {
    let token_hash = hash_token(token);
    let row = sqlx::query_as::<_, RefreshTokenRow>(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)
         RETURNING id, user_id, token_hash, expires_at, revoked, created_at"
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn validate_refresh_token(
    pool: &PgPool,
    token: &str,
) -> eyre::Result<Option<RefreshTokenRow>> {
    let token_hash = hash_token(token);
    let row = sqlx::query_as::<_, RefreshTokenRow>(
        "SELECT id, user_id, token_hash, expires_at, revoked, created_at
         FROM refresh_tokens
         WHERE token_hash = $1 AND revoked = false AND expires_at > now()"
    )
    .bind(&token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn revoke_token(pool: &PgPool, id: Uuid) -> eyre::Result<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked = true WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn revoke_all_user_tokens(pool: &PgPool, user_id: Uuid) -> eyre::Result<()> {
    sqlx::query("UPDATE refresh_tokens SET revoked = true WHERE user_id = $1 AND revoked = false")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_expired_tokens(pool: &PgPool) -> eyre::Result<u64> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < now() OR revoked = true")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
