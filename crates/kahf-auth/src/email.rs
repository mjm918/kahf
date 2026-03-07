//! Email sending via SMTP and OTP generation for email verification.
//!
//! ## EmailSender
//!
//! Trait abstracting OTP email delivery. Implemented by `SmtpConfig` for
//! production use. Tests can supply a no-op implementation.
//!
//! ## SmtpConfig
//!
//! SMTP connection configuration: `host`, `port`, `username`, `password`,
//! `from_email`, `sender_email`. Loaded from environment variables by
//! `SmtpConfig::from_env()`. Implements `EmailSender`.
//!
//! ## generate_otp
//!
//! Generates a cryptographically random 6-digit numeric OTP string using
//! `rand::Rng`.
//!
//! ## OTP_TTL_MINUTES
//!
//! OTP expiration time in minutes. Set to 10.

use eyre::WrapErr;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::Rng;

pub const OTP_TTL_MINUTES: i64 = 10;

pub trait EmailSender: Send + Sync {
    fn send_otp(&self, to_email: &str, otp_code: &str) -> eyre::Result<()>;
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub sender_email: String,
}

impl SmtpConfig {
    pub fn from_env() -> eyre::Result<Self> {
        Ok(Self {
            host: std::env::var("EMAIL_HOST").wrap_err("EMAIL_HOST must be set")?,
            port: std::env::var("EMAIL_PORT")
                .unwrap_or_else(|_| "587".into())
                .parse::<u16>()
                .wrap_err("EMAIL_PORT must be a valid u16")?,
            username: std::env::var("EMAIL_USER").wrap_err("EMAIL_USER must be set")?,
            password: std::env::var("EMAIL_PW").wrap_err("EMAIL_PW must be set")?,
            from_email: std::env::var("EMAIL_FROM").wrap_err("EMAIL_FROM must be set")?,
            sender_email: std::env::var("SENDER_EMAIL").wrap_err("SENDER_EMAIL must be set")?,
        })
    }
}

pub fn generate_otp() -> String {
    let mut rng = rand::rng();
    let code: u32 = rng.random_range(100_000..1_000_000);
    code.to_string()
}

impl EmailSender for SmtpConfig {
    fn send_otp(&self, to_email: &str, otp_code: &str) -> eyre::Result<()> {
        let email = Message::builder()
            .from(
                format!("Kahf <{}>", self.sender_email)
                    .parse()
                    .wrap_err("invalid from address")?,
            )
            .to(to_email.parse().wrap_err("invalid recipient address")?)
            .subject("Kahf — Email Verification Code")
            .header(ContentType::TEXT_HTML)
            .body(format!(
                r#"<div style="font-family: Segoe UI, sans-serif; max-width: 480px; margin: 0 auto; padding: 32px;">
  <h2 style="color: #0078D4; margin-bottom: 24px;">Verify your email</h2>
  <p style="color: #323130; font-size: 14px;">Your verification code is:</p>
  <div style="background: #F3F2F1; border: 1px solid #EDEBE9; border-radius: 4px; padding: 16px; text-align: center; margin: 16px 0;">
    <span style="font-size: 32px; font-weight: 600; letter-spacing: 8px; color: #323130;">{otp_code}</span>
  </div>
  <p style="color: #605E5C; font-size: 13px;">This code expires in {OTP_TTL_MINUTES} minutes. If you did not request this, ignore this email.</p>
</div>"#
            ))
            .wrap_err("failed to build email message")?;

        let creds = Credentials::new(self.username.clone(), self.password.clone());

        let mailer = SmtpTransport::starttls_relay(&self.host)
            .wrap_err("failed to create SMTP transport")?
            .port(self.port)
            .credentials(creds)
            .build();

        mailer.send(&email).wrap_err("failed to send OTP email")?;

        Ok(())
    }
}
