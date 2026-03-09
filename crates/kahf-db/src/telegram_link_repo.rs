//! Telegram account linking repository.
//!
//! ## TelegramLink
//!
//! Row struct mapping to the `telegram_links` table. Associates a KahfLane
//! user with their Telegram chat ID for notification delivery.
//!
//! ## link_telegram
//!
//! Upserts a Telegram chat ID for a user. If the user already has a link,
//! it is replaced with the new chat ID.
//!
//! ## unlink_telegram
//!
//! Removes the Telegram link for a user.
//!
//! ## get_link_by_user
//!
//! Returns the Telegram link for a user, if one exists.
//!
//! ## get_link_by_chat_id
//!
//! Returns the Telegram link for a given chat ID. Used by the webhook
//! handler to resolve commands from Telegram back to KahfLane users.
//!
//! ## get_chat_id
//!
//! Returns just the Telegram chat ID for a user, or `None` if unlinked.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TelegramLink {
    pub id: Uuid,
    pub user_id: Uuid,
    pub telegram_chat_id: i64,
    pub telegram_username: Option<String>,
    pub linked_at: DateTime<Utc>,
}

pub async fn link_telegram(
    pool: &PgPool,
    user_id: Uuid,
    chat_id: i64,
    username: Option<&str>,
) -> eyre::Result<TelegramLink> {
    let row = sqlx::query_as::<_, TelegramLink>(
        "INSERT INTO telegram_links (user_id, telegram_chat_id, telegram_username)
         VALUES ($1, $2, $3)
         ON CONFLICT (user_id)
         DO UPDATE SET telegram_chat_id = $2, telegram_username = $3, linked_at = NOW()
         RETURNING id, user_id, telegram_chat_id, telegram_username, linked_at"
    )
    .bind(user_id)
    .bind(chat_id)
    .bind(username)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn unlink_telegram(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query("DELETE FROM telegram_links WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_link_by_user(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<Option<TelegramLink>> {
    let row = sqlx::query_as::<_, TelegramLink>(
        "SELECT id, user_id, telegram_chat_id, telegram_username, linked_at
         FROM telegram_links WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_link_by_chat_id(
    pool: &PgPool,
    chat_id: i64,
) -> eyre::Result<Option<TelegramLink>> {
    let row = sqlx::query_as::<_, TelegramLink>(
        "SELECT id, user_id, telegram_chat_id, telegram_username, linked_at
         FROM telegram_links WHERE telegram_chat_id = $1"
    )
    .bind(chat_id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_chat_id(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<Option<i64>> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT telegram_chat_id FROM telegram_links WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.0))
}
