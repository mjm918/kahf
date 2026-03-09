//! Application configuration loaded from environment variables.
//!
//! ## Config
//!
//! Holds all runtime configuration: `database_url`, `jwt_secret`,
//! `smtp` (email settings), `redis_url`, `host`, `port`. Loaded via
//! `Config::from_env()` which reads `DATABASE_URL`, `JWT_SECRET`,
//! `REDIS_URL` (default `redis://localhost:6379`), SMTP env vars,
//! `HOST` (default `0.0.0.0`), and `PORT` (default `3000`).

use eyre::WrapErr;
use kahf_email::SmtpConfig;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub redis_url: String,
    pub smtp: SmtpConfig,
    pub frontend_url: String,
    pub host: String,
    pub port: u16,
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

        Ok(Self {
            database_url,
            jwt_secret,
            redis_url,
            smtp,
            frontend_url,
            host,
            port,
        })
    }
}
