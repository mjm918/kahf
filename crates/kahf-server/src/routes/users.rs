//! User profile endpoints.
//!
//! ## GET /api/users/me
//!
//! Returns the authenticated user's profile. Requires a valid access
//! token via the `AuthUser` extractor.
//!
//! ## PATCH /api/users/me
//!
//! Updates the authenticated user's `first_name`, `last_name`, and/or
//! `avatar_url`. Body: `{ first_name?, last_name?, avatar_url? }`.
//! Emits an audit event on profile update.
//!
//! ## PATCH /api/users/me/password
//!
//! Changes the authenticated user's password. Requires `current_password`
//! and `new_password`. Enforces password reuse prevention. Revokes all
//! sessions on success.
//!
//! ## DELETE /api/users/me
//!
//! Deletes the authenticated user's account. Requires `password` for
//! confirmation. Removes the user and all associated data. Emits an
//! audit event.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, patch};
use axum::Router;
use kahf_auth::AuthUser;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::audit::{self, RequestContext};
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users/me", get(get_me).patch(update_me).delete(delete_me))
        .route("/api/users/me/password", patch(change_password))
}

async fn get_me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let user = kahf_db::user_repo::get_user_by_id(state.pool(), auth.claims.sub)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("user", auth.claims.sub.to_string()))?;

    Ok(axum::Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "first_name": user.first_name,
        "last_name": user.last_name,
        "avatar_url": user.avatar_url,
        "created_at": user.created_at,
    })))
}

#[derive(Deserialize)]
struct UpdateMeRequest {
    first_name: Option<String>,
    last_name: Option<String>,
    avatar_url: Option<String>,
}

async fn update_me(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: AuthUser,
    axum::Json(body): axum::Json<UpdateMeRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let user = kahf_db::user_repo::get_user_by_id(state.pool(), auth.claims.sub)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("user", auth.claims.sub.to_string()))?;

    let first_name = body.first_name.as_deref().unwrap_or(&user.first_name);
    let last_name = body.last_name.as_deref().unwrap_or(&user.last_name);
    let avatar_url = body.avatar_url.as_deref().or(user.avatar_url.as_deref());

    kahf_db::user_repo::update_user(state.pool(), auth.claims.sub, first_name, last_name, avatar_url).await?;

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "user.profile_update", Some(format!("user:{}", auth.claims.sub)),
        "success", Some(serde_json::json!({
            "first_name": first_name,
            "last_name": last_name,
        })),
    ).await;

    Ok(axum::Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "first_name": first_name,
        "last_name": last_name,
        "avatar_url": avatar_url,
    })))
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

async fn change_password(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: AuthUser,
    axum::Json(body): axum::Json<ChangePasswordRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let result = kahf_auth::service::change_password(
        state.pool(),
        auth.claims.sub,
        &body.current_password,
        &body.new_password,
    )
    .await?;

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "user.password_change", Some(format!("user:{}", auth.claims.sub)),
        "success", None,
    ).await;

    Ok(axum::Json(serde_json::json!({ "message": result.message })))
}

#[derive(Deserialize)]
struct DeleteMeRequest {
    password: String,
}

async fn delete_me(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth: AuthUser,
    axum::Json(body): axum::Json<DeleteMeRequest>,
) -> Result<StatusCode, AppError> {
    let user = kahf_db::user_repo::get_user_by_id(state.pool(), auth.claims.sub)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("user", auth.claims.sub.to_string()))?;

    kahf_auth::password::verify_password(&body.password, &user.password)?;

    kahf_db::refresh_token_repo::revoke_all_user_tokens(state.pool(), auth.claims.sub).await?;
    kahf_db::user_repo::delete_user(state.pool(), auth.claims.sub).await?;

    let ctx = RequestContext::extract(&headers, Some(&ci));
    audit::emit(
        &state.jobs, &ctx, Some(auth.claims.sub),
        "user.account_delete", Some(format!("user:{}", auth.claims.sub)),
        "success", None,
    ).await;

    Ok(StatusCode::NO_CONTENT)
}
