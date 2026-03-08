//! Password history repository for the `password_history` table.
//!
//! Stores hashed passwords to prevent reuse within a configurable time
//! window. Used during password reset to reject recently used passwords.
//!
//! ## save_password
//!
//! Records a password hash in the history for the given user.
//!
//! ## get_recent_hashes
//!
//! Returns all password hashes for a user created within the last N months.

use sqlx::PgPool;
use uuid::Uuid;

pub async fn save_password(pool: &PgPool, user_id: Uuid, password_hash: &str) -> eyre::Result<()> {
    sqlx::query("INSERT INTO password_history (user_id, password) VALUES ($1, $2)")
        .bind(user_id)
        .bind(password_hash)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_recent_hashes(pool: &PgPool, user_id: Uuid, months: i32) -> eyre::Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT password FROM password_history
         WHERE user_id = $1 AND created_at > now() - make_interval(months => $2)
         ORDER BY created_at DESC"
    )
    .bind(user_id)
    .bind(months)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(h,)| h).collect())
}
