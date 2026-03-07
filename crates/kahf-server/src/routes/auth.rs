//! Authentication endpoints: signup, login, refresh, logout, verify-otp,
//! resend-otp, forgot-password, reset-password, registration-status,
//! invite, validate-invite, list invitations, cancel invitation.
//!
//! ## POST /api/auth/signup
//!
//! Creates a new user account and sends a 6-digit OTP to the provided
//! email. Body: `{ email, password, name, invite_token? }`. Returns
//! `{ user_id, email, message }`. Tokens are NOT returned — user must
//! verify email first. Optional `invite_token` validates and accepts
//! a pending invitation.
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
//! ## POST /api/auth/invite
//!
//! Sends a tenant-level invitation to the given email. Requires
//! authentication. Body: `{ email }`. Returns `{ invitation_id,
//! email, expires_at }`.
//!
//! ## GET /api/auth/invite/validate/:token
//!
//! Validates an invitation token and returns the invitee email.
//! Public endpoint used by frontend to pre-fill signup.
//!
//! ## GET /api/auth/invitations
//!
//! Lists all pending invitations. Requires authentication.
//!
//! ## DELETE /api/auth/invitations/:id
//!
//! Cancels a pending invitation by ID. Requires authentication.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::Router;
use serde::Deserialize;
use uuid::Uuid;

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
        .route("/api/auth/forgot-password", post(forgot_password))
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/registration-status", get(registration_status))
        .route("/api/auth/invite", post(invite_user))
        .route("/api/auth/invite/validate/{token}", get(validate_invite))
        .route("/api/auth/invitations", get(list_invitations))
        .route("/api/auth/invitations/{id}", delete(cancel_invitation))
}

#[derive(Deserialize)]
struct SignupRequest {
    email: String,
    password: String,
    name: String,
    invite_token: Option<String>,
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
        body.invite_token.as_deref(),
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

#[derive(Deserialize)]
struct ForgotPasswordRequest {
    email: String,
}

async fn forgot_password(
    State(state): State<AppState>,
    axum::Json(body): axum::Json<ForgotPasswordRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let resp = kahf_auth::service::forgot_password(
        state.pool(),
        &*state.mailer,
        &body.email,
    )
    .await?;

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
    axum::Json(body): axum::Json<ResetPasswordRequest>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let resp = kahf_auth::service::reset_password(
        state.pool(),
        &body.email,
        &body.code,
        &body.new_password,
    )
    .await?;

    Ok(axum::Json(serde_json::to_value(resp)?))
}

async fn registration_status(
    State(state): State<AppState>,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let open = kahf_auth::service::registration_open(state.pool()).await?;
    Ok(axum::Json(serde_json::json!({ "open": open })))
}

#[derive(Deserialize)]
struct InviteRequest {
    email: String,
}

async fn invite_user(
    State(state): State<AppState>,
    auth_user: kahf_auth::AuthUser,
    axum::Json(body): axum::Json<InviteRequest>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), AppError> {
    let resp = kahf_auth::service::invite_user(
        state.pool(),
        &*state.mailer,
        auth_user.claims.sub,
        &body.email,
    )
    .await?;

    Ok((StatusCode::CREATED, axum::Json(serde_json::to_value(resp)?)))
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

async fn list_invitations(
    State(state): State<AppState>,
    _auth_user: kahf_auth::AuthUser,
) -> Result<axum::Json<serde_json::Value>, AppError> {
    let invitations = kahf_db::invite_repo::list_pending_invitations(state.pool()).await?;
    Ok(axum::Json(serde_json::to_value(invitations)?))
}

async fn cancel_invitation(
    State(state): State<AppState>,
    _auth_user: kahf_auth::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    kahf_db::invite_repo::cancel_invitation(state.pool(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
