//! Web push subscription repository for browser push notification delivery.
//!
//! ## PushSubscription
//!
//! Row struct mapping to the `push_subscriptions` table. Stores the browser
//! push endpoint, p256dh key, and auth secret per user per device.
//!
//! ## save_subscription
//!
//! Upserts a push subscription for a user + endpoint pair. If the endpoint
//! already exists, the keys are updated.
//!
//! ## remove_subscription
//!
//! Deletes a specific subscription by endpoint (used when a subscription
//! becomes invalid or the user unsubscribes).
//!
//! ## remove_all_subscriptions
//!
//! Removes all push subscriptions for a user.
//!
//! ## get_subscriptions
//!
//! Returns all active push subscriptions for a user (one per device/browser).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PushSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn save_subscription(
    pool: &PgPool,
    user_id: Uuid,
    endpoint: &str,
    p256dh: &str,
    auth: &str,
    user_agent: Option<&str>,
) -> eyre::Result<PushSubscription> {
    let row = sqlx::query_as::<_, PushSubscription>(
        "INSERT INTO push_subscriptions (user_id, endpoint, p256dh, auth, user_agent)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, endpoint)
         DO UPDATE SET p256dh = $3, auth = $4, user_agent = $5, created_at = NOW()
         RETURNING id, user_id, endpoint, p256dh, auth, user_agent, created_at"
    )
    .bind(user_id)
    .bind(endpoint)
    .bind(p256dh)
    .bind(auth)
    .bind(user_agent)
    .fetch_one(pool)
    .await?;

    Ok(row)
}

pub async fn remove_subscription(
    pool: &PgPool,
    user_id: Uuid,
    endpoint: &str,
) -> eyre::Result<()> {
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id = $1 AND endpoint = $2")
        .bind(user_id)
        .bind(endpoint)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn remove_all_subscriptions(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<()> {
    sqlx::query("DELETE FROM push_subscriptions WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_subscriptions(
    pool: &PgPool,
    user_id: Uuid,
) -> eyre::Result<Vec<PushSubscription>> {
    let rows = sqlx::query_as::<_, PushSubscription>(
        "SELECT id, user_id, endpoint, p256dh, auth, user_agent, created_at
         FROM push_subscriptions WHERE user_id = $1"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
