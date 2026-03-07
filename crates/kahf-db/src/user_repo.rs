//! User repository for the `users` table.
//!
//! ## UserRow
//!
//! Database row struct matching the `users` table columns: `id`, `email`,
//! `password` (argon2 hash), `name`, `avatar_url`, `email_verified`,
//! `created_at`.
//!
//! ## create_user
//!
//! Inserts a new user with `email_verified = false`. The `password` field
//! must already be hashed by the caller (kahf-auth). Returns the created
//! `UserRow`.
//!
//! ## get_user_by_id / get_user_by_email
//!
//! Fetches a user by UUID primary key or unique email.
//!
//! ## update_user
//!
//! Updates mutable user fields: `name` and `avatar_url`.
//!
//! ## update_password
//!
//! Updates the password hash for a given user ID.
//!
//! ## mark_email_verified
//!
//! Sets `email_verified = true` for the given user ID.
//!
//! ## count_users
//!
//! Returns the total number of users in the system. Used to determine
//! whether open registration is allowed (only when zero users exist).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub email_verified: bool,
    pub created_at: DateTime<Utc>,
}

pub async fn create_user(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    name: &str,
) -> eyre::Result<UserRow> {
    let row = sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (email, password, name) VALUES ($1, $2, $3)
         RETURNING id, email, password, name, avatar_url, email_verified, created_at"
    )
    .bind(email)
    .bind(password_hash)
    .bind(name)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> eyre::Result<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password, name, avatar_url, email_verified, created_at
         FROM users WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_user_by_email(pool: &PgPool, email: &str) -> eyre::Result<Option<UserRow>> {
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, password, name, avatar_url, email_verified, created_at
         FROM users WHERE email = $1"
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn update_user(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    avatar_url: Option<&str>,
) -> eyre::Result<()> {
    sqlx::query("UPDATE users SET name = $1, avatar_url = $2 WHERE id = $3")
        .bind(name)
        .bind(avatar_url)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_password(pool: &PgPool, user_id: Uuid, password_hash: &str) -> eyre::Result<()> {
    sqlx::query("UPDATE users SET password = $1 WHERE id = $2")
        .bind(password_hash)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn mark_email_verified(pool: &PgPool, user_id: Uuid) -> eyre::Result<()> {
    sqlx::query("UPDATE users SET email_verified = true WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn count_users(pool: &PgPool) -> eyre::Result<i64> {
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    Ok(count)
}
