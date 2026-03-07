//! Authentication endpoints: signup, login, refresh, logout.
//!
//! ## POST /api/auth/signup
//!
//! Creates a new user account. Body: `{ email, password, name }`.
//! Returns `AuthResponse` with access/refresh tokens and user info.
//!
//! ## POST /api/auth/login
//!
//! Authenticates by email and password. Body: `{ email, password }`.
//! Returns `AuthResponse` on success, 401 on bad credentials.
//!
//! ## POST /api/auth/refresh
//!
//! Exchanges a refresh token for a new access token.
//! Body: `{ refresh_token }`. Returns `{ access_token }`.
//!
//! ## POST /api/auth/logout
//!
//! Placeholder for future session invalidation. Currently returns 200.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::Router;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/signup", post(signup))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
}

#[derive(Deserialize)]
struct SignupRequest {
    email: String,
    password: String,
    name: String,
}

async fn signup(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SignupRequest>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), AppError> {
    let resp = kahf_auth::service::signup(
        state.pool(),
        &state.jwt,
        &body.email,
        &body.password,
        &body.name,
    )
    .await?;

    Ok((StatusCode::CREATED, axum::Json(serde_json::to_value(resp)?)))
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn login(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<LoginRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let resp = kahf_auth::service::login(
        state.pool(),
        &state.jwt,
        &body.email,
        &body.password,
    )
    .await?;

    Ok(axum::Json(serde_json::to_value(resp)?))
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

async fn refresh(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<RefreshRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let access_token = kahf_auth::service::refresh(
        state.pool(),
        &state.jwt,
        &body.refresh_token,
    )
    .await?;

    Ok(axum::Json(serde_json::json!({ "access_token": access_token })))
}

async fn logout() -> StatusCode {
    StatusCode::OK
}
