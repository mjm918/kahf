//! Telegram bot webhook handler and link code generation API.
//!
//! ## router
//!
//! Mounts the Telegram webhook at `/api/telegram/webhook` (unauthenticated,
//! verified by secret token header) and link management at
//! `/api/telegram/link`.
//!
//! ## POST /api/telegram/webhook
//!
//! Receives Telegram updates. Verifies the `X-Telegram-Bot-Api-Secret-Token`
//! header matches the configured secret. Processes bot commands: `/start`,
//! `/link`, `/unlink`, `/status`, `/mute`, `/unmute`, `/help`.
//!
//! ## POST /api/telegram/link
//!
//! Generates a 6-character link code for the authenticated user. Returns
//! the code and the bot username. The user sends `/link CODE` to the bot.
//!
//! ## DELETE /api/telegram/link
//!
//! Unlinks the authenticated user's Telegram account.
//!
//! ## GET /api/telegram/link
//!
//! Returns the current Telegram link status for the authenticated user.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Serialize;

use kahf_auth::AuthUser;
use kahf_notify::telegram_bot::responses;

use crate::app_state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/telegram/webhook", post(handle_webhook))
        .route("/api/telegram/link", post(generate_link_code))
        .route("/api/telegram/link", delete(unlink_telegram))
        .route("/api/telegram/link", get(link_status))
}

async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<kahf_notify::TelegramUpdate>,
) -> StatusCode {
    let secret = state.telegram_webhook_secret.as_deref().unwrap_or("");
    let header_secret = headers
        .get("x-telegram-bot-api-secret-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if secret.is_empty() || header_secret != secret {
        return StatusCode::UNAUTHORIZED;
    }

    let message = match update.message {
        Some(m) => m,
        None => return StatusCode::OK,
    };

    let text = match message.text.as_deref() {
        Some(t) => t,
        None => return StatusCode::OK,
    };

    let chat_id = message.chat.id;
    let username = message.from.as_ref().and_then(|u| u.username.clone());
    let command = kahf_notify::parse_command(text);
    let pool = state.pool();

    let reply = match command {
        kahf_notify::BotCommand::Start => responses::WELCOME.to_owned(),

        kahf_notify::BotCommand::Help => responses::HELP.to_owned(),

        kahf_notify::BotCommand::Link { code } => {
            if code.is_empty() {
                responses::LINK_MISSING_CODE.to_owned()
            } else {
                handle_link_command(pool, chat_id, username.as_deref(), &code).await
            }
        }

        kahf_notify::BotCommand::Unlink => {
            handle_unlink_command(pool, chat_id).await
        }

        kahf_notify::BotCommand::Status => {
            handle_status_command(pool, chat_id).await
        }

        kahf_notify::BotCommand::Mute { minutes } => {
            handle_mute_command(pool, chat_id, minutes).await
        }

        kahf_notify::BotCommand::Unmute => {
            handle_unmute_command(pool, chat_id).await
        }

        kahf_notify::BotCommand::Unknown { .. } => {
            responses::UNKNOWN_COMMAND.to_owned()
        }
    };

    if let Some(ref sender) = state.telegram_sender {
        if let Err(e) = sender.send_message(chat_id, &reply).await {
            tracing::error!(chat_id = chat_id, error = %e, "failed to send bot reply");
        }
    }

    StatusCode::OK
}

async fn handle_link_command(
    pool: &sqlx::PgPool,
    chat_id: i64,
    username: Option<&str>,
    code: &str,
) -> String {
    let existing = kahf_db::telegram_link_repo::get_link_by_chat_id(pool, chat_id).await;
    if let Ok(Some(_)) = existing {
        return responses::LINK_ALREADY_LINKED.to_owned();
    }

    let user_id = match kahf_db::telegram_link_code_repo::validate_and_consume_code(pool, code).await {
        Ok(Some(uid)) => uid,
        Ok(None) => return responses::LINK_INVALID.to_owned(),
        Err(e) => {
            tracing::error!(error = %e, "failed to validate link code");
            return responses::LINK_INVALID.to_owned();
        }
    };

    if let Err(e) = kahf_db::telegram_link_repo::link_telegram(pool, user_id, chat_id, username).await {
        tracing::error!(error = %e, "failed to save telegram link");
        return "An error occurred. Please try again.".to_owned();
    }

    let _ = kahf_db::notification_pref_repo::upsert_preference(pool, user_id, "telegram", true).await;

    responses::LINK_SUCCESS.to_owned()
}

async fn handle_unlink_command(pool: &sqlx::PgPool, chat_id: i64) -> String {
    let link = kahf_db::telegram_link_repo::get_link_by_chat_id(pool, chat_id).await;
    match link {
        Ok(Some(link)) => {
            if let Err(e) = kahf_db::telegram_link_repo::unlink_telegram(pool, link.user_id).await {
                tracing::error!(error = %e, "failed to unlink telegram");
                return "An error occurred. Please try again.".to_owned();
            }
            responses::UNLINK_SUCCESS.to_owned()
        }
        _ => responses::UNLINK_NOT_LINKED.to_owned(),
    }
}

async fn handle_status_command(pool: &sqlx::PgPool, chat_id: i64) -> String {
    let link = kahf_db::telegram_link_repo::get_link_by_chat_id(pool, chat_id).await;
    match link {
        Ok(Some(link)) => {
            match kahf_db::user_repo::get_user_by_id(pool, link.user_id).await {
                Ok(Some(user)) => {
                    responses::status_linked(&user.first_name, &user.last_name, &user.email)
                }
                _ => responses::STATUS_NOT_LINKED.to_owned(),
            }
        }
        _ => responses::STATUS_NOT_LINKED.to_owned(),
    }
}

async fn handle_mute_command(pool: &sqlx::PgPool, chat_id: i64, minutes: Option<i64>) -> String {
    let link = kahf_db::telegram_link_repo::get_link_by_chat_id(pool, chat_id).await;
    match link {
        Ok(Some(link)) => {
            match minutes {
                Some(m) if m > 0 => {
                    let until = Utc::now() + Duration::minutes(m);
                    let _ = kahf_db::notification_pref_repo::snooze_channel(
                        pool, link.user_id, "telegram", until,
                    ).await;
                    responses::mute_success(m)
                }
                _ => {
                    let _ = kahf_db::notification_pref_repo::upsert_preference(
                        pool, link.user_id, "telegram", false,
                    ).await;
                    responses::MUTE_INDEFINITE.to_owned()
                }
            }
        }
        _ => responses::UNLINK_NOT_LINKED.to_owned(),
    }
}

async fn handle_unmute_command(pool: &sqlx::PgPool, chat_id: i64) -> String {
    let link = kahf_db::telegram_link_repo::get_link_by_chat_id(pool, chat_id).await;
    match link {
        Ok(Some(link)) => {
            let _ = kahf_db::notification_pref_repo::upsert_preference(
                pool, link.user_id, "telegram", true,
            ).await;
            let _ = kahf_db::notification_pref_repo::unsnooze_channel(
                pool, link.user_id, "telegram",
            ).await;
            responses::UNMUTE_SUCCESS.to_owned()
        }
        _ => responses::UNLINK_NOT_LINKED.to_owned(),
    }
}

#[derive(Debug, Serialize)]
struct LinkCodeResponse {
    code: String,
    bot_username: String,
    expires_in_minutes: i64,
}

async fn generate_link_code(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<LinkCodeResponse>, StatusCode> {
    let existing = kahf_db::telegram_link_repo::get_link_by_user(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing.is_some() {
        return Err(StatusCode::CONFLICT);
    }

    let code = kahf_notify::generate_link_code();
    let expires_at = Utc::now() + Duration::minutes(kahf_notify::LINK_CODE_TTL_MINUTES);

    kahf_db::telegram_link_code_repo::create_link_code(
        state.pool(),
        user.claims.sub,
        &code,
        expires_at,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LinkCodeResponse {
        code,
        bot_username: "KahfLaneBot".to_owned(),
        expires_in_minutes: kahf_notify::LINK_CODE_TTL_MINUTES,
    }))
}

#[derive(Debug, Serialize)]
struct LinkStatusResponse {
    linked: bool,
    telegram_username: Option<String>,
}

async fn link_status(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<LinkStatusResponse>, StatusCode> {
    let link = kahf_db::telegram_link_repo::get_link_by_user(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LinkStatusResponse {
        linked: link.is_some(),
        telegram_username: link.and_then(|l| l.telegram_username),
    }))
}

async fn unlink_telegram(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<StatusCode, StatusCode> {
    kahf_db::telegram_link_repo::unlink_telegram(state.pool(), user.claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}
