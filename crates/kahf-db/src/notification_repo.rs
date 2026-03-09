//! In-app notification persistence for the notifications bell/inbox.
//!
//! ## Notification
//!
//! Row struct for the `notifications` table. Each notification has a `title`,
//! `body`, `category` (e.g. "system", "mention", "assignment"), optional
//! `data` JSON payload, and a nullable `read_at` timestamp.
//!
//! ## create_notification
//!
//! Inserts a new unread notification for a user.
//!
//! ## get_notifications
//!
//! Returns paginated notifications for a user, newest first.
//!
//! ## get_unread_count
//!
//! Returns the number of unread notifications for a user.
//!
//! ## mark_read
//!
//! Marks a single notification as read.
//!
//! ## mark_all_read
//!
//! Marks all unread notifications for a user as read.
//!
//! ## delete_notification
//!
//! Hard-deletes a single notification.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub category: String,
    pub read_at: Option<DateTime<Utc>>,
    pub data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

pub async fn create_notification(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    body: &str,
    category: &str,
    data: Option<serde_json::Value>,
) -> eyre::Result<Notification> {
    let row = sqlx::query_as::<_, Notification>(
        "INSERT INTO notifications (user_id, title, body, category, data)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, user_id, title, body, category, read_at, data, created_at"
    )
    .bind(user_id)
    .bind(title)
    .bind(body)
    .bind(category)
    .bind(data)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn get_notifications(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> eyre::Result<Vec<Notification>> {
    let rows = sqlx::query_as::<_, Notification>(
        "SELECT id, user_id, title, body, category, read_at, data, created_at
         FROM notifications
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3"
    )
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_unread_count(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<i64> {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL"
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(count)
}

pub async fn mark_read(
    pool: &PgPool,
    notification_id: Uuid,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query(
        "UPDATE notifications SET read_at = NOW() WHERE id = $1 AND user_id = $2 AND read_at IS NULL"
    )
    .bind(notification_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_all_read(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query(
        "UPDATE notifications SET read_at = NOW() WHERE user_id = $1 AND read_at IS NULL"
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_notification(
    pool: &PgPool,
    notification_id: Uuid,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
        .bind(notification_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}
