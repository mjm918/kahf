//! Authentication endpoints: signup, login, refresh, logout, verify-otp,
//! resend-otp, forgot-password, reset-password, registration-status,
//! validate-invite.
//!
//! All mutating actions emit non-blocking audit log events via the
//! background job queue for security forensics and compliance.
//!
//! ## POST /api/auth/signup
//!
//! Creates a new user account and sends a 6-digit OTP to the provided
//! email. Body: `{ email, password, first_name, last_name, company_name?,
//! invite_token? }`. Returns `{ user_id, email, message }`. Tokens are
//! NOT returned — user must verify email first. Optional `invite_token`
//! validates and accepts a pending invitation. `company_name` is required
//! for the first registration (owner).
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
//!
//! ## POST /api/auth/forgot-password
//!
//! Sends a password reset OTP to the given email. Body: `{ email }`.
//! Returns a generic success message regardless of account existence.
//!
//! ## POST /api/auth/reset-password
//!
//! Validates reset OTP and updates password. Body: `{ email, code,
//! new_password }`. Returns `{ message }`.
//!
//! ## GET /api/auth/registration-status
//!
//! Returns whether open registration is available. Returns
//! `{ open: true }` if no users exist yet (first user becomes tenant
//! owner), `{ open: false }` if users already exist (invite-only).
//!
//! ## GET /api/auth/invite/validate/:token
//!
//! Validates an invitation token and returns the invitee email.
//! Public endpoint used by frontend to pre-fill signup.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::audit::{self, RequestContext};
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/signup", post(signup))
        .route("/api/auth/verify-otp", post(verify_otp))
        .route("/api/auth/resend-otp", post(resend_otp))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/registration-status", get(registration_status))
        .route("/api/auth/invite/validate/{token}", get(validate_invite))
}

fn ctx(headers: &HeaderMap, ci: &ConnectInfo<SocketAddr>) -> RequestContext {
    RequestContext::extract(headers, Some(ci))
}

#[derive(Deserialize)]
struct SignupRequest {
    email: String,
    password: String,
    first_name: String,
    last_name: String,
    company_name: Option<String>,
    invite_token: Option<String>,
}

async fn signup(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<SignupRequest>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), AppError> {
    let ctx = ctx(&headers, &ci);

    let result = kahf_auth::service::signup(
        state.pool(),
        &state.jobs,
        &body.email,
        &body.password,
        &body.first_name,
        &body.last_name,
        body.company_name.as_deref(),
        body.invite_token.as_deref(),
    )
    .await;

    match &result {
        Ok(resp) => {
            audit::emit(
                &state.jobs, &ctx, Some(resp.user_id),
                "auth.signup", Some(format!("user:{}", resp.user_id)),
                "success", Some(serde_json::json!({"email": body.email})),
            ).await;
        }
        Err(e) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.signup", None,
                "failure", Some(serde_json::json!({"email": body.email, "error": e.to_string()})),
            ).await;
        }
    }

    let resp = result?;
    Ok((StatusCode::CREATED, axum::Json(serde_json::to_value(resp)?)))
}

#[derive(Deserialize)]
struct VerifyOtpRequest {
    email: String,
    code: String,
}

async fn verify_otp(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<VerifyOtpRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ctx = ctx(&headers, &ci);

    let result = kahf_auth::service::verify_otp(
        state.pool(),
        &state.jwt,
        &body.email,
        &body.code,
    )
    .await;

    match &result {
        Ok(resp) => {
            audit::emit(
                &state.jobs, &ctx, Some(resp.user_id),
                "auth.verify_otp", Some(format!("user:{}", resp.user_id)),
                "success", Some(serde_json::json!({"email": body.email})),
            ).await;
        }
        Err(e) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.verify_otp", None,
                "failure", Some(serde_json::json!({"email": body.email, "error": e.to_string()})),
            ).await;
        }
    }

    let resp = result?;
    Ok(axum::Json(serde_json::to_value(resp)?))
}

#[derive(Deserialize)]
struct ResendOtpRequest {
    email: String,
}

async fn resend_otp(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<ResendOtpRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ctx = ctx(&headers, &ci);

    let result = kahf_auth::service::resend_otp(
        state.pool(),
        &state.jobs,
        &body.email,
    )
    .await;

    match &result {
        Ok(resp) => {
            audit::emit(
                &state.jobs, &ctx, Some(resp.user_id),
                "auth.resend_otp", Some(format!("user:{}", resp.user_id)),
                "success", Some(serde_json::json!({"email": body.email})),
            ).await;
        }
        Err(e) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.resend_otp", None,
                "failure", Some(serde_json::json!({"email": body.email, "error": e.to_string()})),
            ).await;
        }
    }

    let resp = result?;
    Ok(axum::Json(serde_json::to_value(resp)?))
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

async fn login(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<LoginRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ctx = ctx(&headers, &ci);

    let result = kahf_auth::service::login(
        state.pool(),
        &state.jwt,
        &body.email,
        &body.password,
    )
    .await;

    match &result {
        Ok(resp) => {
            audit::emit(
                &state.jobs, &ctx, Some(resp.user_id),
                "auth.login", Some(format!("user:{}", resp.user_id)),
                "success", Some(serde_json::json!({"email": body.email})),
            ).await;
        }
        Err(e) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.login_failed", None,
                "failure", Some(serde_json::json!({"email": body.email, "error": e.to_string()})),
            ).await;
        }
    }

    let resp = result?;
    Ok(axum::Json(serde_json::to_value(resp)?))
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

async fn refresh(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<RefreshRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ctx = ctx(&headers, &ci);

    let result = kahf_auth::service::refresh(
        state.pool(),
        &state.jwt,
        &body.refresh_token,
    )
    .await;

    match &result {
        Ok(resp) => {
            audit::emit(
                &state.jobs, &ctx, Some(resp.user_id),
                "auth.refresh", Some(format!("user:{}", resp.user_id)),
                "success", None,
            ).await;
        }
        Err(e) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.refresh", None,
                "failure", Some(serde_json::json!({"error": e.to_string()})),
            ).await;
        }
    }

    let resp = result?;
    Ok(axum::Json(serde_json::to_value(resp)?))
}

async fn logout(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    auth_user: kahf_auth::AuthUser,
) -> Result<StatusCode, AppError> {
    let ctx = ctx(&headers, &ci);

    kahf_auth::service::logout(state.pool(), auth_user.claims.sub).await?;

    audit::emit(
        &state.jobs, &ctx, Some(auth_user.claims.sub),
        "auth.logout", Some(format!("user:{}", auth_user.claims.sub)),
        "success", None,
    ).await;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
struct ForgotPasswordRequest {
    email: String,
}

async fn forgot_password(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<ForgotPasswordRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ctx = ctx(&headers, &ci);

    let resp = kahf_auth::service::forgot_password(
        state.pool(),
        &state.jobs,
        &body.email,
    )
    .await?;

    audit::emit(
        &state.jobs, &ctx, None,
        "auth.forgot_password", None,
        "success", Some(serde_json::json!({"email": body.email})),
    ).await;

    Ok(axum::Json(serde_json::to_value(resp)?))
}

#[derive(Deserialize)]
struct ResetPasswordRequest {
    email: String,
    code: String,
    new_password: String,
}

async fn reset_password(
    State(state): State<AppState>,
    ci: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<ResetPasswordRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let ctx = ctx(&headers, &ci);

    let result = kahf_auth::service::reset_password(
        state.pool(),
        &body.email,
        &body.code,
        &body.new_password,
    )
    .await;

    match &result {
        Ok(_) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.reset_password", None,
                "success", Some(serde_json::json!({"email": body.email})),
            ).await;
        }
        Err(e) => {
            audit::emit(
                &state.jobs, &ctx, None,
                "auth.reset_password", None,
                "failure", Some(serde_json::json!({"email": body.email, "error": e.to_string()})),
            ).await;
        }
    }

    let resp = result?;
    Ok(axum::Json(serde_json::to_value(resp)?))
}

async fn registration_status(
    State(state): State<AppState>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let open = kahf_auth::service::registration_open(state.pool()).await?;
    Ok(axum::Json(serde_json::json!({ "open": open })))
}

async fn validate_invite(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let resp = kahf_auth::service::validate_invite(
        state.pool(),
        &token,
    )
    .await?;

    Ok(axum::Json(serde_json::to_value(resp)?))
}

