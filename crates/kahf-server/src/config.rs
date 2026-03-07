//! Application configuration loaded from environment variables.
//!
//! ## Config
//!
//! Holds all runtime configuration: `database_url`, `jwt_secret`,
//! `smtp` (email settings), `host`, `port`. Loaded via
//! `Config::from_env()` which reads `DATABASE_URL`, `JWT_SECRET`,
//! SMTP env vars, `HOST` (default `0.0.0.0`), and `PORT` (default
//! `3000`).

use eyre::WrapErr;
use kahf_auth::SmtpConfig;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub smtp: SmtpConfig,
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> eyre::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .wrap_err("DATABASE_URL must be set")?;

        let jwt_secret = std::env::var("JWT_SECRET")
            .wrap_err("JWT_SECRET must be set")?;

        let smtp = SmtpConfig::from_env()?;

        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".into())
            .parse::<u16>()
            .wrap_err("PORT must be a valid u16")?;

        Ok(Self {
            database_url,
            jwt_secret,
            smtp,
            host,
            port,
        })
    }
}
