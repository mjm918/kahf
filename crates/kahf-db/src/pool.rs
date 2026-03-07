//! Database connection pool initialization and migration runner.
//!
//! ## DbPool
//!
//! Thin wrapper holding a `sqlx::PgPool`. Provides `pool()` accessor
//! and runs embedded migrations via `migrate()`.
//!
//! ## connect
//!
//! Creates a `PgPool` from a database URL string. Configures a max of
//! 20 connections via `PgPoolOptions`.
//!
//! ## migrate
//!
//! Runs all SQL migrations embedded at compile time from the
//! `migrations/` directory using `sqlx::migrate!()`.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

pub struct DbPool {
    pool: PgPool,
}

impl DbPool {
    pub async fn connect(database_url: &str) -> eyre::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> eyre::Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
