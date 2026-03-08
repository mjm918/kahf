//! Email job definitions for background delivery.
//!
//! ## SendOtpEmail
//!
//! Sends a 6-digit OTP verification code to the given email address.
//! Fields: `email`, `otp_code`.
//!
//! ## SendPasswordResetEmail
//!
//! Sends a password reset OTP code to the given email address.
//! Fields: `email`, `otp_code`.
//!
//! ## SendInviteEmail
//!
//! Sends a tenant invitation email with a signup link.
//! Fields: `email`, `inviter_name`, `invite_token`.

use serde::{Deserialize, Serialize};

use crate::Job;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendOtpEmail {
    pub email: String,
    pub otp_code: String,
}

impl Job for SendOtpEmail {
    const JOB_TYPE: &'static str = "SendOtpEmail";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendPasswordResetEmail {
    pub email: String,
    pub otp_code: String,
}

impl Job for SendPasswordResetEmail {
    const JOB_TYPE: &'static str = "SendPasswordResetEmail";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendInviteEmail {
    pub email: String,
    pub inviter_name: String,
    pub invite_token: String,
}

impl Job for SendInviteEmail {
    const JOB_TYPE: &'static str = "SendInviteEmail";
}
