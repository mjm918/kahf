//! High-level authentication service operations.
//!
//! ## signup
//!
//! Creates a new user account with `email_verified = false`. Hashes the
//! password with Argon2id, inserts the user, generates a 6-digit OTP,
//! stores it in the database, and sends it via SMTP. Returns
//! `SignupResponse` with `user_id` and `email` (no tokens yet — user
//! must verify email first).
//!
//! ## verify_otp
//!
//! Validates a 6-digit OTP code for a given email. If valid, marks the
//! user's email as verified, marks the OTP as used, and issues
//! access + refresh tokens. Returns `AuthResponse`.
//!
//! ## resend_otp
//!
//! Invalidates all existing OTPs for a user, generates a new one, and
//! sends it via SMTP. Requires the user to exist and not yet be verified.
//!
//! ## login
//!
//! Authenticates an existing user by email and password. Rejects users
//! whose email is not yet verified. Returns `AuthResponse` on success.
//!
//! ## refresh
//!
//! Exchanges a valid refresh token for a new access token.
//!
//! ## AuthResponse
//!
//! Response payload containing `access_token`, `refresh_token`, and
//! basic user info (`user_id`, `email`, `name`).
//!
//! ## SignupResponse
//!
//! Response payload for signup containing `user_id` and `email`.
//! Tokens are NOT included — user must verify email first.

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::email::{EmailSender, OTP_TTL_MINUTES, generate_otp};
use crate::jwt::{JwtConfig, issue_access_token, issue_refresh_token, verify_token};
use crate::password::{hash_password, verify_password};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignupResponse {
    pub user_id: Uuid,
    pub email: String,
    pub message: String,
}

pub async fn signup(
    pool: &PgPool,
    mailer: &dyn EmailSender,
    email: &str,
    password: &str,
    name: &str,
) -> eyre::Result<SignupResponse> {
    let password_hash = hash_password(password)?;
    let user = kahf_db::user_repo::create_user(pool, email, &password_hash, name).await?;

    let otp_code = generate_otp();
    let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(pool, user.id, &otp_code, expires_at).await?;

    mailer.send_otp(email, &otp_code)?;

    Ok(SignupResponse {
        user_id: user.id,
        email: user.email,
        message: "verification code sent to your email".into(),
    })
}

pub async fn verify_otp(
    pool: &PgPool,
    config: &JwtConfig,
    email: &str,
    code: &str,
) -> eyre::Result<AuthResponse> {
    let user = kahf_db::user_repo::get_user_by_email(pool, email)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("user", email))?;

    if user.email_verified {
        return Err(kahf_core::KahfError::validation("email already verified"));
    }

    let otp = kahf_db::otp_repo::get_valid_otp(pool, user.id, code)
        .await?
        .ok_or_else(|| kahf_core::KahfError::validation("invalid or expired verification code"))?;

    kahf_db::otp_repo::mark_otp_used(pool, otp.id).await?;
    kahf_db::user_repo::mark_email_verified(pool, user.id).await?;

    let access_token = issue_access_token(config, user.id, None, None)?;
    let refresh_token = issue_refresh_token(config, user.id)?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        user_id: user.id,
        email: user.email,
        name: user.name,
    })
}

pub async fn resend_otp(
    pool: &PgPool,
    mailer: &dyn EmailSender,
    email: &str,
) -> eyre::Result<SignupResponse> {
    let user = kahf_db::user_repo::get_user_by_email(pool, email)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("user", email))?;

    if user.email_verified {
        return Err(kahf_core::KahfError::validation("email already verified"));
    }

    kahf_db::otp_repo::invalidate_user_otps(pool, user.id).await?;

    let otp_code = generate_otp();
    let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(pool, user.id, &otp_code, expires_at).await?;

    mailer.send_otp(email, &otp_code)?;

    Ok(SignupResponse {
        user_id: user.id,
        email: user.email,
        message: "new verification code sent to your email".into(),
    })
}

pub async fn login(
    pool: &PgPool,
    config: &JwtConfig,
    email: &str,
    password: &str,
) -> eyre::Result<AuthResponse> {
    let user = kahf_db::user_repo::get_user_by_email(pool, email)
        .await?
        .ok_or_else(|| kahf_core::KahfError::unauthorized())?;

    if !user.email_verified {
        return Err(kahf_core::KahfError::validation("email not verified"));
    }

    verify_password(password, &user.password)?;

    let access_token = issue_access_token(config, user.id, None, None)?;
    let refresh_token = issue_refresh_token(config, user.id)?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        user_id: user.id,
        email: user.email,
        name: user.name,
    })
}

pub async fn refresh(
    pool: &PgPool,
    config: &JwtConfig,
    refresh_token_str: &str,
) -> eyre::Result<String> {
    let claims = verify_token(config, refresh_token_str)?;

    if claims.token_type != "refresh" {
        return Err(kahf_core::KahfError::unauthorized());
    }

    kahf_db::user_repo::get_user_by_id(pool, claims.sub)
        .await?
        .ok_or_else(|| kahf_core::KahfError::unauthorized())?;

    let access_token = issue_access_token(config, claims.sub, None, None)?;
    Ok(access_token)
}
