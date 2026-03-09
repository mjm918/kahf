//! Notification API routes for preferences, in-app notifications, and
//! push subscription management.
//!
//! ## router
//!
//! Mounts all notification endpoints under `/api/notifications`.
//!
//! ## GET /api/notifications
//!
//! Returns paginated in-app notifications for the authenticated user.
//! Query params: `limit` (default 20), `offset` (default 0).
//!
//! ## GET /api/notifications/unread-count
//!
//! Returns the number of unread notifications.
//!
//! ## PATCH /api/notifications/:id/read
//!
//! Marks a single notification as read.
//!
//! ## POST /api/notifications/read-all
//!
//! Marks all notifications as read.
//!
//! ## DELETE /api/notifications/:id
//!
//! Deletes a single notification.
//!
//! ## GET /api/notifications/preferences
//!
//! Returns the user's notification preferences for all channels.
//!
//! ## PUT /api/notifications/preferences/:channel
//!
//! Enables or disables a notification channel.
//!
//! ## POST /api/notifications/preferences/:channel/snooze
//!
//! Snoozes a channel for a given duration in minutes.
//!
//! ## DELETE /api/notifications/preferences/:channel/snooze
//!
//! Unsnoozes a channel.
//!
//! ## POST /api/notifications/snooze-all
//!
//! Snoozes all channels for a given duration.
//!
//! ## DELETE /api/notifications/snooze-all
//!
//! Unsnoozes all channels.
//!
//! ## POST /api/push/subscribe
//!
//! Saves a browser push subscription for the authenticated user.
//!
//! ## DELETE /api/push/subscribe
//!
//! Removes a browser push subscription.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use kahf_auth::AuthUser;

use crate::app_state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/unread-count", get(unread_count))
        .route("/api/notifications/{id}/read", patch(mark_read))
        .route("/api/notifications/read-all", post(mark_all_read))
        .route("/api/notifications/{id}", delete(delete_notification))
        .route("/api/notifications/preferences", get(get_preferences))
        .route("/api/notifications/preferences/{channel}", put(set_preference))
        .route("/api/notifications/preferences/{channel}/snooze", post(snooze_channel))
        .route("/api/notifications/preferences/{channel}/snooze", delete(unsnooze_channel))
        .route("/api/notifications/snooze-all", post(snooze_all))
        .route("/api/notifications/snooze-all", delete(unsnooze_all))
        .route("/api/push/subscribe", post(push_subscribe))
        .route("/api/push/subscribe", delete(push_unsubscribe))
}

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize)]
struct CountResponse {
    count: i64,
}

async fn list_notifications(
    State(state): State<AppState>,
    user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<kahf_db::notification_repo::Notification>>, StatusCode> {
    let notifications = kahf_db::notification_repo::get_notifications(
        state.pool(),
        user.claims.sub,
        pagination.limit,
        pagination.offset,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(notifications))
}

async fn unread_count(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<CountResponse>, StatusCode> {
    let count = kahf_db::notification_repo::get_unread_count(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CountResponse { count }))
}

async fn mark_read(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    kahf_db::notification_repo::mark_read(state.pool(), id, user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn mark_all_read(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    kahf_db::notification_repo::mark_all_read(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_notification(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    kahf_db::notification_repo::delete_notification(state.pool(), id, user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
struct PreferenceResponse {
    channel: String,
    enabled: bool,
    snoozed_until: Option<chrono::DateTime<Utc>>,
}

async fn get_preferences(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<PreferenceResponse>>, StatusCode> {
    let prefs = kahf_db::notification_pref_repo::get_preferences(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let all_channels = kahf_notify::NotificationChannel::all();
    let responses: Vec<PreferenceResponse> = all_channels
        .iter()
        .map(|ch| {
            let existing = prefs.iter().find(|p| p.channel == ch.as_str());
            PreferenceResponse {
                channel: ch.as_str().to_owned(),
                enabled: existing.map_or(true, |p| p.enabled),
                snoozed_until: existing.and_then(|p| p.snoozed_until),
            }
        })
        .collect();

    Ok(Json(responses))
}

#[derive(Debug, Deserialize)]
struct SetPreferenceBody {
    enabled: bool,
}

async fn set_preference(
    State(state): State<AppState>,
    user: AuthUser,
    Path(channel): Path<String>,
    Json(body): Json<SetPreferenceBody>,
) -> Result<StatusCode, StatusCode> {
    channel.parse::<kahf_notify::NotificationChannel>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    kahf_db::notification_pref_repo::upsert_preference(
        state.pool(),
        user.claims.sub,
        &channel,
        body.enabled,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct SnoozeBody {
    duration_minutes: i64,
}

async fn snooze_channel(
    State(state): State<AppState>,
    user: AuthUser,
    Path(channel): Path<String>,
    Json(body): Json<SnoozeBody>,
) -> Result<StatusCode, StatusCode> {
    channel.parse::<kahf_notify::NotificationChannel>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let until = Utc::now() + Duration::minutes(body.duration_minutes);
    kahf_db::notification_pref_repo::snooze_channel(state.pool(), user.claims.sub, &channel, until)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn unsnooze_channel(
    State(state): State<AppState>,
    user: AuthUser,
    Path(channel): Path<String>,
) -> Result<StatusCode, StatusCode> {
    channel.parse::<kahf_notify::NotificationChannel>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    kahf_db::notification_pref_repo::unsnooze_channel(state.pool(), user.claims.sub, &channel)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn snooze_all(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<SnoozeBody>,
) -> Result<StatusCode, StatusCode> {
    let until = Utc::now() + Duration::minutes(body.duration_minutes);
    kahf_db::notification_pref_repo::snooze_all(state.pool(), user.claims.sub, until)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn unsnooze_all(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    kahf_db::notification_pref_repo::unsnooze_all(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct PushSubscribeBody {
    endpoint: String,
    p256dh: String,
    auth: String,
    user_agent: Option<String>,
}

async fn push_subscribe(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PushSubscribeBody>,
) -> Result<StatusCode, StatusCode> {
    kahf_db::push_subscription_repo::save_subscription(
        state.pool(),
        user.claims.sub,
        &body.endpoint,
        &body.p256dh,
        &body.auth,
        body.user_agent.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

#[derive(Debug, Deserialize)]
struct PushUnsubscribeBody {
    endpoint: String,
}

async fn push_unsubscribe(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PushUnsubscribeBody>,
) -> Result<StatusCode, StatusCode> {
    kahf_db::push_subscription_repo::remove_subscription(
        state.pool(),
        user.claims.sub,
        &body.endpoint,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
