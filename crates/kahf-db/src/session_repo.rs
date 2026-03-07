//! Session repository for the `sessions` table.
//!
//! ## SessionRow
//!
//! Database row struct for sessions. Fields: `id`, `user_id`,
//! `token_hash`, `expires_at`, `created_at`.
//!
//! ## create_session
//!
//! Inserts a new session with a hashed token and expiry time.
//! Token hashing is the caller's responsibility (kahf-auth).
//!
//! ## get_session
//!
//! Fetches a session by ID. Used for token validation lookups.
//!
//! ## delete_session
//!
//! Removes a session (logout).
//!
//! ## delete_expired_sessions
//!
//! Bulk removes all sessions past their `expires_at`. Called by
//! the background worker (kahf-worker) on a schedule. Returns
//! the number of deleted rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

pub async fn create_session(
    pool: &PgPool,
    user_id: Uuid,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> eyre::Result<SessionRow> {
    let row = sqlx::query_as::<_, SessionRow>(
        "INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)
         RETURNING id, user_id, token_hash, expires_at, created_at"
    )
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_session(pool: &PgPool, id: Uuid) -> eyre::Result<Option<SessionRow>> {
    let row = sqlx::query_as::<_, SessionRow>(
        "SELECT id, user_id, token_hash, expires_at, created_at FROM sessions WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn delete_session(pool: &PgPool, id: Uuid) -> eyre::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn delete_expired_sessions(pool: &PgPool) -> eyre::Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE expires_at < now()")
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
