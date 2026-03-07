//! Email delivery and HTML templating for the KahfLane platform.
//!
//! Provides SMTP email transport, Tera-based HTML template rendering, and
//! OTP generation. Templates are embedded at compile time from the `templates/`
//! directory using Tera's inheritance system (base layout + per-email blocks).
//!
//! ## EmailSender
//!
//! Trait abstracting email delivery. Methods: `send_otp` for email verification,
//! `send_password_reset_otp` for password reset codes, and `send_invite` for
//! tenant-level invitation links. Implemented by `SmtpEmailSender` for
//! production use. Tests can supply a no-op implementation.
//!
//! ## SmtpEncryption
//!
//! Transport encryption mode: `Tls` (implicit TLS), `StartTls`
//! (upgrade-to-TLS), or `None` (plaintext). Configured via the
//! `EMAIL_ENCRYPTION` env var (values: `tls`, `starttls`, `none`;
//! defaults to `tls`).
//!
//! ## SmtpConfig
//!
//! SMTP connection configuration: `host`, `port`, `username`, `password`,
//! `from_email`, `sender_email`, `encryption`. Loaded from environment
//! variables by `SmtpConfig::from_env()`.
//!
//! ## SmtpEmailSender
//!
//! Production email sender combining `SmtpConfig` with a compiled `Tera`
//! template engine. Constructed via `SmtpEmailSender::new()`.
//!
//! ## generate_otp
//!
//! Generates a cryptographically random 6-digit numeric OTP string.
//!
//! ## OTP_TTL_MINUTES
//!
//! OTP expiration time in minutes (10).
//!
//! ## INVITE_TTL_DAYS
//!
//! Invitation expiration time in days (7).

use eyre::WrapErr;
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::Rng;
use tera::{Context, Tera};

pub const OTP_TTL_MINUTES: i64 = 10;
pub const INVITE_TTL_DAYS: i64 = 7;

pub trait EmailSender: Send + Sync {
    fn send_otp(&self, to_email: &str, otp_code: &str) -> eyre::Result<()>;
    fn send_password_reset_otp(&self, to_email: &str, otp_code: &str) -> eyre::Result<()>;
    fn send_invite(&self, to_email: &str, inviter_name: &str, invite_token: &str) -> eyre::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtpEncryption {
    Tls,
    StartTls,
    None,
}

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub sender_email: String,
    pub encryption: SmtpEncryption,
}

impl SmtpConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let encryption = match std::env::var("EMAIL_ENCRYPTION")
            .unwrap_or_else(|_| "tls".into())
            .to_lowercase()
            .as_str()
        {
            "starttls" => SmtpEncryption::StartTls,
            "none" => SmtpEncryption::None,
            _ => SmtpEncryption::Tls,
        };

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
            encryption,
        })
    }
}

pub fn generate_otp() -> String {
    let mut rng = rand::rng();
    let code: u32 = rng.random_range(100_000..1_000_000);
    code.to_string()
}

fn build_tera() -> eyre::Result<Tera> {
    let mut tera = Tera::default();
    tera.add_raw_templates(vec![
        ("base.html", include_str!("../templates/base.html")),
        ("otp_verification.html", include_str!("../templates/otp_verification.html")),
        ("password_reset.html", include_str!("../templates/password_reset.html")),
        ("invitation.html", include_str!("../templates/invitation.html")),
    ])
    .wrap_err("failed to compile email templates")?;
    Ok(tera)
}

fn current_year() -> i32 {
    chrono_free_year()
}

fn chrono_free_year() -> i32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (1970 + secs / 31_557_600) as i32
}

pub struct SmtpEmailSender {
    config: SmtpConfig,
    tera: Tera,
}

impl SmtpEmailSender {
    pub fn new(config: SmtpConfig) -> eyre::Result<Self> {
        let tera = build_tera()?;
        Ok(Self { config, tera })
    }

    fn send_html(&self, to_email: &str, subject: &str, html_body: &str) -> eyre::Result<()> {
        let email = Message::builder()
            .from(
                format!("KahfLane <{}>", self.config.sender_email)
                    .parse()
                    .wrap_err("invalid from address")?,
            )
            .to(to_email.parse().wrap_err("invalid recipient address")?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_owned())
            .wrap_err("failed to build email message")?;

        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());

        let mailer = match self.config.encryption {
            SmtpEncryption::StartTls => SmtpTransport::starttls_relay(&self.config.host)
                .wrap_err("failed to create SMTP STARTTLS transport")?
                .port(self.config.port)
                .credentials(creds)
                .build(),
            SmtpEncryption::None => SmtpTransport::builder_dangerous(&self.config.host)
                .port(self.config.port)
                .credentials(creds)
                .build(),
            SmtpEncryption::Tls => SmtpTransport::relay(&self.config.host)
                .wrap_err("failed to create SMTP TLS transport")?
                .port(self.config.port)
                .credentials(creds)
                .build(),
        };

        mailer.send(&email).wrap_err("failed to send email")?;

        Ok(())
    }

    fn render(&self, template: &str, context: &Context) -> eyre::Result<String> {
        self.tera
            .render(template, context)
            .wrap_err_with(|| format!("failed to render template: {}", template))
    }
}

impl EmailSender for SmtpEmailSender {
    fn send_otp(&self, to_email: &str, otp_code: &str) -> eyre::Result<()> {
        let mut ctx = Context::new();
        ctx.insert("subject", "KahfLane — Email Verification Code");
        ctx.insert("year", &current_year());
        ctx.insert("otp_code", otp_code);
        ctx.insert("ttl_minutes", &OTP_TTL_MINUTES);

        let html = self.render("otp_verification.html", &ctx)?;
        self.send_html(to_email, "KahfLane — Email Verification Code", &html)
    }

    fn send_password_reset_otp(&self, to_email: &str, otp_code: &str) -> eyre::Result<()> {
        let mut ctx = Context::new();
        ctx.insert("subject", "KahfLane — Password Reset Code");
        ctx.insert("year", &current_year());
        ctx.insert("otp_code", otp_code);
        ctx.insert("ttl_minutes", &OTP_TTL_MINUTES);

        let html = self.render("password_reset.html", &ctx)?;
        self.send_html(to_email, "KahfLane — Password Reset Code", &html)
    }

    fn send_invite(&self, to_email: &str, inviter_name: &str, invite_token: &str) -> eyre::Result<()> {
        let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:4200".into());
        let invite_link = format!("{}/signup?invite={}", frontend_url, invite_token);

        let mut ctx = Context::new();
        ctx.insert("subject", "KahfLane — You've been invited to join");
        ctx.insert("year", &current_year());
        ctx.insert("inviter_name", inviter_name);
        ctx.insert("invite_link", &invite_link);
        ctx.insert("ttl_days", &INVITE_TTL_DAYS);

        let html = self.render("invitation.html", &ctx)?;
        self.send_html(to_email, "KahfLane — You've been invited to join", &html)
    }
}
