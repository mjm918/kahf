//! High-level authentication service operations.
//!
//! ## signup
//!
//! Creates a new user account with `email_verified = false`. Hashes the
//! password with Argon2id, inserts the user, generates a 6-digit OTP,
//! stores it in the database, and sends it via SMTP. Accepts an optional
//! `invite_token` — if provided, validates it and marks the invitation
//! as accepted after user creation. Returns `SignupResponse` with
//! `user_id` and `email` (no tokens yet — user must verify email first).
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
//! ## forgot_password
//!
//! Sends a password reset OTP to the given email. Returns a generic
//! success message regardless of whether the email exists (prevents
//! email enumeration). Only works for verified accounts.
//!
//! ## reset_password
//!
//! Validates a password reset OTP and updates the user's password.
//! Hashes the new password with Argon2id, marks OTP as used, and
//! invalidates remaining reset OTPs.
//!
//! ## invite_user
//!
//! Creates a tenant-level invitation for the given email. Generates a
//! unique token, stores the invitation, and sends an invite email with
//! a signup link. Rejects if the email is already registered or has a
//! pending invitation.
//!
//! ## validate_invite
//!
//! Validates an invitation token and returns the invitee email. Used by
//! the frontend to pre-fill the signup form.
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
//!
//! ## MessageResponse
//!
//! Generic response payload with a `message` field.
//!
//! ## InviteResponse
//!
//! Response payload for invite containing `invitation_id`, `email`,
//! and `expires_at`.
//!
//! ## InviteValidation
//!
//! Response payload for invite validation containing `email` and
//! `expires_at`.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use kahf_email::{EmailSender, INVITE_TTL_DAYS, OTP_TTL_MINUTES, generate_otp};
use crate::jwt::{JwtConfig, issue_access_token, issue_refresh_token, verify_token};
use crate::password::{hash_password, verify_password};

const PURPOSE_EMAIL_VERIFICATION: &str = "email_verification";
const PURPOSE_PASSWORD_RESET: &str = "password_reset";

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteResponse {
    pub invitation_id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteValidation {
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn signup(
    pool: &PgPool,
    mailer: &dyn EmailSender,
    email: &str,
    password: &str,
    name: &str,
    invite_token: Option<&str>,
) -> eyre::Result<SignupResponse> {
    if let Some(token) = invite_token {
        let invitation = kahf_db::invite_repo::get_invitation_by_token(pool, token)
            .await?
            .ok_or_else(|| kahf_core::KahfError::validation("invalid or expired invitation"))?;

        if invitation.email != email {
            return Err(kahf_core::KahfError::validation("email does not match invitation"));
        }

        kahf_db::invite_repo::mark_invitation_accepted(pool, invitation.id).await?;
    }

    let password_hash = hash_password(password)?;
    let user = kahf_db::user_repo::create_user(pool, email, &password_hash, name).await?;

    let otp_code = generate_otp();
    let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(pool, user.id, &otp_code, expires_at, PURPOSE_EMAIL_VERIFICATION).await?;

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

    let otp = kahf_db::otp_repo::get_valid_otp(pool, user.id, code, PURPOSE_EMAIL_VERIFICATION)
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

    kahf_db::otp_repo::invalidate_user_otps(pool, user.id, PURPOSE_EMAIL_VERIFICATION).await?;

    let otp_code = generate_otp();
    let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
    kahf_db::otp_repo::create_otp(pool, user.id, &otp_code, expires_at, PURPOSE_EMAIL_VERIFICATION).await?;

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

pub async fn forgot_password(
    pool: &PgPool,
    mailer: &dyn EmailSender,
    email: &str,
) -> eyre::Result<MessageResponse> {
    let user = kahf_db::user_repo::get_user_by_email(pool, email).await?;

    if let Some(user) = user {
        if user.email_verified {
            kahf_db::otp_repo::invalidate_user_otps(pool, user.id, PURPOSE_PASSWORD_RESET).await?;

            let otp_code = generate_otp();
            let expires_at = Utc::now() + Duration::minutes(OTP_TTL_MINUTES);
            kahf_db::otp_repo::create_otp(pool, user.id, &otp_code, expires_at, PURPOSE_PASSWORD_RESET).await?;

            mailer.send_password_reset_otp(email, &otp_code)?;
        }
    }

    Ok(MessageResponse {
        message: "if an account exists with this email, a reset code has been sent".into(),
    })
}

pub async fn reset_password(
    pool: &PgPool,
    email: &str,
    code: &str,
    new_password: &str,
) -> eyre::Result<MessageResponse> {
    let user = kahf_db::user_repo::get_user_by_email(pool, email)
        .await?
        .ok_or_else(|| kahf_core::KahfError::validation("invalid or expired reset code"))?;

    let otp = kahf_db::otp_repo::get_valid_otp(pool, user.id, code, PURPOSE_PASSWORD_RESET)
        .await?
        .ok_or_else(|| kahf_core::KahfError::validation("invalid or expired reset code"))?;

    let password_hash = hash_password(new_password)?;
    kahf_db::user_repo::update_password(pool, user.id, &password_hash).await?;

    kahf_db::otp_repo::mark_otp_used(pool, otp.id).await?;
    kahf_db::otp_repo::invalidate_user_otps(pool, user.id, PURPOSE_PASSWORD_RESET).await?;

    Ok(MessageResponse {
        message: "password reset successfully".into(),
    })
}

pub async fn invite_user(
    pool: &PgPool,
    mailer: &dyn EmailSender,
    inviter_user_id: Uuid,
    invitee_email: &str,
) -> eyre::Result<InviteResponse> {
    let existing = kahf_db::user_repo::get_user_by_email(pool, invitee_email).await?;
    if existing.is_some() {
        return Err(kahf_core::KahfError::conflict("user with this email already exists"));
    }

    let pending = kahf_db::invite_repo::get_pending_by_email(pool, invitee_email).await?;
    if pending.is_some() {
        return Err(kahf_core::KahfError::conflict("a pending invitation already exists for this email"));
    }

    let inviter = kahf_db::user_repo::get_user_by_id(pool, inviter_user_id)
        .await?
        .ok_or_else(|| kahf_core::KahfError::not_found("user", inviter_user_id.to_string()))?;

    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + Duration::days(INVITE_TTL_DAYS);

    let invitation = kahf_db::invite_repo::create_invitation(
        pool,
        invitee_email,
        inviter_user_id,
        &token,
        expires_at,
    )
    .await?;

    mailer.send_invite(invitee_email, &inviter.name, &token)?;

    Ok(InviteResponse {
        invitation_id: invitation.id,
        email: invitation.email,
        expires_at: invitation.expires_at,
    })
}

pub async fn validate_invite(
    pool: &PgPool,
    token: &str,
) -> eyre::Result<InviteValidation> {
    let invitation = kahf_db::invite_repo::get_invitation_by_token(pool, token)
        .await?
        .ok_or_else(|| kahf_core::KahfError::validation("invalid or expired invitation"))?;

    Ok(InviteValidation {
        email: invitation.email,
        expires_at: invitation.expires_at,
    })
}
