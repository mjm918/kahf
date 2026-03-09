//! KahfLane server binary entry point.
//!
//! Loads configuration from environment variables (with `.env` fallback),
//! initializes tracing, connects to the database, runs migrations,
//! creates the job producer and background workers, builds the axum
//! application with all route modules, and starts the HTTP server.
//! Workers and the HTTP server run concurrently via `tokio::select!`.
//!
//! ## Environment Variables
//!
//! - `DATABASE_URL` — PostgreSQL connection string (required)
//! - `JWT_SECRET` — Secret key for JWT token signing (required)
//! - `REDIS_URL` — Redis connection string (default `redis://localhost:6379`)
//! - `HOST` — Bind address (default `0.0.0.0`)
//! - `PORT` — Bind port (default `3000`)
//! - `RUST_LOG` — Tracing filter (default `kahf=debug,tower_http=debug`)

use std::net::SocketAddr;
use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use kahf_server::app_state::AppState;
use kahf_server::config::Config;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("kahf=debug,tower_http=debug,apalis=debug")),
        )
        .json()
        .init();

    let config = Config::from_env()?;

    tracing::info!("connecting to database");
    let db = kahf_db::DbPool::connect(&config.database_url).await?;

    tracing::info!("running migrations");
    db.migrate().await?;

    tracing::info!("initializing RBAC enforcer");
    let rbac = kahf_rbac::RbacEnforcer::new(&config.database_url).await?;

    let jwt = kahf_auth::JwtConfig::new(config.jwt_secret);
    let mailer: Arc<dyn kahf_email::EmailSender> = Arc::new(kahf_email::SmtpEmailSender::new(config.smtp)?);

    tracing::info!("connecting to Redis for job queue");
    let jobs = kahf_worker::JobProducer::new(&config.redis_url, db.pool().clone()).await?;

    tracing::info!("starting background workers");
    let monitor = kahf_worker::start_workers(&config.redis_url, db.pool().clone(), mailer.clone()).await?;

    let hub = kahf_realtime::Hub::new(db.pool().clone());
    let event_bus = kahf_realtime::BroadcastEventBus::new(1024);
    let state = AppState::new(db, jwt.clone(), mailer, jobs, hub, event_bus, rbac);

    let app = kahf_server::build_app(state, jwt, &config.frontend_url);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");

    let worker_handle = tokio::spawn(async move {
        if let Err(e) = monitor.run().await {
            tracing::error!(error = %e, "worker monitor exited with error");
        }
    });

    tokio::select! {
        res = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()) => {
            res?;
        }
        _ = worker_handle => {
            tracing::warn!("worker monitor stopped unexpectedly");
        }
    }

    Ok(())
}
