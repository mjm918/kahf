//! Application configuration loaded from environment variables.
//!
//! ## Config
//!
//! Holds all runtime configuration: `database_url`, `jwt_secret`,
//! `smtp` (email settings), `redis_url`, `host`, `port`, and optional
//! Telegram bot settings. Loaded via `Config::from_env()`.
//!
//! ## TelegramBotConfig
//!
//! Optional Telegram bot configuration: `bot_token`, `webhook_secret`,
//! and `webhook_base_url`. Present only when `TELEGRAM_BOT_TOKEN` is set.
//! The `webhook_secret` is derived from the JWT secret via HMAC to avoid
//! requiring yet another env var. The `webhook_base_url` defaults to
//! `BACKEND_URL` or `http://localhost:3000`.

use eyre::WrapErr;
use kahf_notify::SmtpConfig;

#[derive(Debug, Clone)]
pub struct TelegramBotConfig {
    pub bot_token: String,
    pub webhook_secret: String,
    pub webhook_base_url: String,
}

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub redis_url: String,
    pub smtp: SmtpConfig,
    pub frontend_url: String,
    pub host: String,
    pub port: u16,
    pub telegram_bot: Option<TelegramBotConfig>,
}

impl Config {
    pub fn from_env() -> eyre::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .wrap_err("DATABASE_URL must be set")?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .wrap_err("JWT_SECRET must be set")?;

        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".into());

        let smtp = SmtpConfig::from_env()?;

        let frontend_url = std::env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:4200".into());

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .wrap_err("PORT must be a valid u16")?;

        let telegram_bot = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|t| !t.is_empty())
            .map(|bot_token| {
                let webhook_secret = format!("tg-webhook-{}", &jwt_secret[..16.min(jwt_secret.len())]);

                let webhook_base_url = std::env::var("BACKEND_URL")
                    .unwrap_or_else(|_| format!("http://localhost:{}", port));

                TelegramBotConfig {
                    bot_token,
                    webhook_secret,
                    webhook_base_url,
                }
            });

        Ok(Self {
            database_url,
            jwt_secret,
            redis_url,
            smtp,
            frontend_url,
            host,
            port,
            telegram_bot,
        })
    }
}
