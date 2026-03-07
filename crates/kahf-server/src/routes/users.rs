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

use axum::extract::State;
use axum::routing::get;
use axum::Router;
use kahf_auth::AuthUser;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users/me", get(get_me).patch(update_me))
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

    Ok(axum::Json(serde_json::json!({
        "id": user.id,
        "email": user.email,
        "first_name": first_name,
        "last_name": last_name,
        "avatar_url": avatar_url,
    })))
}
