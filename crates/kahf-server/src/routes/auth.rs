//! Authentication endpoints: signup, login, refresh, logout, verify-otp,
//! resend-otp.
//!
//! ## POST /api/auth/signup
//!
//! Creates a new user account and sends a 6-digit OTP to the provided
//! email. Body: `{ email, password, name }`. Returns `{ user_id, email,
//! message }`. Tokens are NOT returned — user must verify email first.
//!
//! ## POST /api/auth/verify-otp
//!
//! Verifies the OTP code for the given email. Body: `{ email, code }`.
//! On success, marks email as verified and returns `AuthResponse` with
//! access/refresh tokens.
//!
//! ## POST /api/auth/resend-otp
//!
//! Invalidates existing OTPs and sends a new one. Body: `{ email }`.
//! Returns `{ user_id, email, message }`.
//!
//! ## POST /api/auth/login
//!
//! Authenticates by email and password. Rejects unverified emails.
//! Body: `{ email, password }`. Returns `AuthResponse`.
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
        .route("/api/auth/verify-otp", post(verify_otp))
        .route("/api/auth/resend-otp", post(resend_otp))
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
        &*state.mailer,
        &body.email,
        &body.password,
        &body.name,
    )
    .await?;

    Ok((StatusCode::CREATED, axum::Json(serde_json::to_value(resp)?)))
}

#[derive(Deserialize)]
struct VerifyOtpRequest {
    email: String,
    code: String,
}

async fn verify_otp(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<VerifyOtpRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let resp = kahf_auth::service::verify_otp(
        state.pool(),
        &state.jwt,
        &body.email,
        &body.code,
    )
    .await?;

    Ok(axum::Json(serde_json::to_value(resp)?))
}

#[derive(Deserialize)]
struct ResendOtpRequest {
    email: String,
}

async fn resend_otp(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<ResendOtpRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let resp = kahf_auth::service::resend_otp(
        state.pool(),
        &*state.mailer,
        &body.email,
    )
    .await?;

    Ok(axum::Json(serde_json::to_value(resp)?))
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
