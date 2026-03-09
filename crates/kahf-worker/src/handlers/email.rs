//! Email job handlers.
//!
//! Processes email delivery jobs by delegating to `kahf_notify::EmailSender`.
//! Each handler runs on a blocking thread pool via `spawn_blocking` to avoid
//! blocking the tokio runtime (lettre uses synchronous SMTP transport).
//!
//! ## handle_send_otp
//!
//! Sends an OTP verification email. Extracts `EmailSender` from apalis
//! data context and calls `send_otp`.
//!
//! ## handle_send_password_reset
//!
//! Sends a password reset OTP email via `send_password_reset_otp`.
//!
//! ## handle_send_invite
//!
//! Sends a tenant invitation email via `send_invite`.

use std::sync::Arc;

use apalis::prelude::Data;
use kahf_notify::EmailSender;

use crate::audit::{JobAuditor, JobStatus};
use crate::jobs::email::{SendInviteEmail, SendOtpEmail, SendPasswordResetEmail};

pub async fn handle_send_otp(
    job: SendOtpEmail,
    mailer: Data<Arc<dyn EmailSender>>,
    auditor: Data<JobAuditor>,
) -> Result<(), String> {
    tracing::info!(email = %job.email, "processing OTP email job");

    let mailer = mailer.clone();
    let email = job.email.clone();
    let otp = job.otp_code.clone();

    let result = tokio::task::spawn_blocking(move || mailer.send_otp(&email, &otp))
        .await
        .map_err(|e| format!("spawn_blocking failed: {}", e))?
        .map_err(|e| format!("send_otp failed: {:?}", e));

    match &result {
        Ok(()) => {
            tracing::info!(email = %job.email, "OTP email sent successfully");
            auditor
                .record(uuid::Uuid::new_v4(), "SendOtpEmail", JobStatus::Completed, serde_json::json!({"email": job.email}), None, 1)
                .await;
        }
        Err(e) => {
            tracing::error!(email = %job.email, error = %e, "OTP email failed");
            auditor
                .record(uuid::Uuid::new_v4(), "SendOtpEmail", JobStatus::Failed, serde_json::json!({"email": job.email}), Some(&e), 1)
                .await;
        }
    }

    result
}

pub async fn handle_send_password_reset(
    job: SendPasswordResetEmail,
    mailer: Data<Arc<dyn EmailSender>>,
    auditor: Data<JobAuditor>,
) -> Result<(), String> {
    tracing::info!(email = %job.email, "processing password reset email job");

    let mailer = mailer.clone();
    let email = job.email.clone();
    let otp = job.otp_code.clone();

    let result = tokio::task::spawn_blocking(move || mailer.send_password_reset_otp(&email, &otp))
        .await
        .map_err(|e| format!("spawn_blocking failed: {}", e))?
        .map_err(|e| format!("send_password_reset_otp failed: {:?}", e));

    match &result {
        Ok(()) => {
            tracing::info!(email = %job.email, "password reset email sent successfully");
            auditor
                .record(uuid::Uuid::new_v4(), "SendPasswordResetEmail", JobStatus::Completed, serde_json::json!({"email": job.email}), None, 1)
                .await;
        }
        Err(e) => {
            tracing::error!(email = %job.email, error = %e, "password reset email failed");
            auditor
                .record(uuid::Uuid::new_v4(), "SendPasswordResetEmail", JobStatus::Failed, serde_json::json!({"email": job.email}), Some(&e), 1)
                .await;
        }
    }

    result
}

pub async fn handle_send_invite(
    job: SendInviteEmail,
    mailer: Data<Arc<dyn EmailSender>>,
    auditor: Data<JobAuditor>,
) -> Result<(), String> {
    tracing::info!(email = %job.email, "processing invite email job");

    let mailer = mailer.clone();
    let email = job.email.clone();
    let inviter = job.inviter_name.clone();
    let token = job.invite_token.clone();

    let result = tokio::task::spawn_blocking(move || mailer.send_invite(&email, &inviter, &token))
        .await
        .map_err(|e| format!("spawn_blocking failed: {}", e))?
        .map_err(|e| format!("send_invite failed: {:?}", e));

    match &result {
        Ok(()) => {
            tracing::info!(email = %job.email, "invite email sent successfully");
            auditor
                .record(uuid::Uuid::new_v4(), "SendInviteEmail", JobStatus::Completed, serde_json::json!({"email": job.email}), None, 1)
                .await;
        }
        Err(e) => {
            tracing::error!(email = %job.email, error = %e, "invite email failed");
            auditor
                .record(uuid::Uuid::new_v4(), "SendInviteEmail", JobStatus::Failed, serde_json::json!({"email": job.email}), Some(&e), 1)
                .await;
        }
    }

    result
}
