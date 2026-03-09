//! Notification preference CRUD for per-user, per-channel settings.
//!
//! ## NotificationPreference
//!
//! Row struct mapping to the `notification_preferences` table. Tracks
//! whether a channel is enabled and an optional snooze expiry.
//!
//! ## get_preferences
//!
//! Returns all notification preferences for a user. If a channel has no
//! row, it is treated as enabled by default by the caller.
//!
//! ## upsert_preference
//!
//! Inserts or updates the enabled state for a user + channel pair.
//!
//! ## snooze_channel
//!
//! Sets `snoozed_until` for a specific channel. The channel remains
//! enabled but notifications are suppressed until the snooze expires.
//!
//! ## unsnooze_channel
//!
//! Clears the `snoozed_until` timestamp for a channel.
//!
//! ## snooze_all
//!
//! Snoozes all existing preference rows for a user until the given time.
//!
//! ## unsnooze_all
//!
//! Clears snooze on all channels for a user.
//!
//! ## is_channel_active
//!
//! Returns `true` if the channel is both enabled and not currently snoozed.
//! Channels with no preference row are considered active.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NotificationPreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub channel: String,
    pub enabled: bool,
    pub snoozed_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<Vec<NotificationPreference>> {
    let rows = sqlx::query_as::<_, NotificationPreference>(
        "SELECT id, user_id, channel, enabled, snoozed_until, created_at, updated_at
         FROM notification_preferences
         WHERE user_id = $1
         ORDER BY channel"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn upsert_preference(
    pool: &PgPool,
    user_id: Uuid,
    channel: &str,
    enabled: bool,
) -> eyre::Result<NotificationPreference> {
    let row = sqlx::query_as::<_, NotificationPreference>(
        "INSERT INTO notification_preferences (user_id, channel, enabled)
         VALUES ($1, $2, $3)
         ON CONFLICT (user_id, channel)
         DO UPDATE SET enabled = $3, updated_at = NOW()
         RETURNING id, user_id, channel, enabled, snoozed_until, created_at, updated_at"
    )
    .bind(user_id)
    .bind(channel)
    .bind(enabled)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn snooze_channel(
    pool: &PgPool,
    user_id: Uuid,
    channel: &str,
    until: DateTime<Utc>,
) -> eyre::Result<()> {
    sqlx::query(
        "INSERT INTO notification_preferences (user_id, channel, snoozed_until)
         VALUES ($1, $2, $3)
         ON CONFLICT (user_id, channel)
         DO UPDATE SET snoozed_until = $3, updated_at = NOW()"
    )
    .bind(user_id)
    .bind(channel)
    .bind(until)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn unsnooze_channel(
    pool: &PgPool,
    user_id: Uuid,
    channel: &str,
) -> eyre::Result<()> {
    sqlx::query(
        "UPDATE notification_preferences
         SET snoozed_until = NULL, updated_at = NOW()
         WHERE user_id = $1 AND channel = $2"
    )
    .bind(user_id)
    .bind(channel)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn snooze_all(
    pool: &PgPool,
    user_id: Uuid,
    until: DateTime<Utc>,
) -> eyre::Result<()> {
    sqlx::query(
        "UPDATE notification_preferences
         SET snoozed_until = $2, updated_at = NOW()
         WHERE user_id = $1"
    )
    .bind(user_id)
    .bind(until)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn unsnooze_all(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query(
        "UPDATE notification_preferences
         SET snoozed_until = NULL, updated_at = NOW()
         WHERE user_id = $1"
    )
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn is_channel_active(
    pool: &PgPool,
    user_id: Uuid,
    channel: &str,
) -> eyre::Result<bool> {
    let row = sqlx::query_as::<_, NotificationPreference>(
        "SELECT id, user_id, channel, enabled, snoozed_until, created_at, updated_at
         FROM notification_preferences
         WHERE user_id = $1 AND channel = $2"
    )
    .bind(user_id)
    .bind(channel)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Ok(true),
        Some(pref) => {
            if !pref.enabled {
                return Ok(false);
            }
            if let Some(until) = pref.snoozed_until {
                return Ok(Utc::now() >= until);
            }
            Ok(true)
        }
    }
}
