//! KahfLane server binary entry point.
//!
//! Loads configuration from environment variables (with `.env` fallback),
//! initializes tracing, connects to the database, runs migrations,
//! creates the job producer and background workers, builds the axum
//! application with all route modules, and starts the HTTP server.
//! Workers and the HTTP server run concurrently via `tokio::select!`.
//! Optional notification channels (Telegram, web push) are initialized
//! when their env vars are present. The Telegram bot webhook and command
//! menu are registered on startup.
//!
//! ## Environment Variables
//!
//! - `DATABASE_URL` — PostgreSQL connection string (required)
//! - `JWT_SECRET` — Secret key for JWT token signing (required)
//! - `REDIS_URL` — Redis connection string (default `redis://localhost:6379`)
//! - `HOST` — Bind address (default `0.0.0.0`)
//! - `PORT` — Bind port (default `3000`)
//! - `RUST_LOG` — Tracing filter (default `kahf=debug,tower_http=debug`)
//! - `TELEGRAM_BOT_TOKEN` — Telegram bot token (optional, enables Telegram notifications)
//! - `BACKEND_URL` — Public URL of this server for Telegram webhook (optional)
//! - `VAPID_PRIVATE_KEY` — VAPID private key base64url (optional, enables web push)
//! - `VAPID_PUBLIC_KEY` — VAPID public key base64url (optional, enables web push)

use std::net::SocketAddr;
use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use kahf_server::app_state::AppState;
use kahf_server::config::Config;
use kahf_worker::WorkerDeps;

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
    let mailer: Arc<dyn kahf_notify::EmailSender> = Arc::new(kahf_notify::SmtpEmailSender::new(config.smtp)?);

    let telegram = kahf_notify::TelegramConfig::from_env_optional().map(|cfg| {
        tracing::info!("Telegram bot configured");
        Arc::new(kahf_notify::TelegramSender::new(cfg))
    });

    let web_push = kahf_notify::VapidConfig::from_env_optional()
        .and_then(|cfg| match kahf_notify::WebPushSender::new(&cfg) {
            Ok(sender) => {
                tracing::info!("VAPID web push configured");
                Some(Arc::new(sender))
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to initialize web push sender, skipping");
                None
            }
        });

    tracing::info!("connecting to Redis for job queue");
    let jobs = kahf_worker::JobProducer::new(&config.redis_url, db.pool().clone()).await?;

    tracing::info!("starting background workers");
    let worker_deps = WorkerDeps {
        mailer: mailer.clone(),
        telegram: telegram.clone(),
        web_push,
    };
    let monitor = kahf_worker::start_workers(&config.redis_url, db.pool().clone(), worker_deps).await?;

    let hub = kahf_realtime::Hub::new(db.pool().clone());
    let event_bus = kahf_realtime::BroadcastEventBus::new(1024);
    let mut state = AppState::new(db, jwt.clone(), mailer, jobs, hub, event_bus, rbac);

    if let (Some(tg_sender), Some(tg_config)) = (&telegram, &config.telegram_bot) {
        state = state.with_telegram(tg_sender.clone(), tg_config.webhook_secret.clone());

        let http_client = reqwest::Client::new();

        let webhook_url = format!("{}/api/telegram/webhook", tg_config.webhook_base_url);
        match kahf_notify::register_webhook(
            &http_client,
            &tg_config.bot_token,
            &webhook_url,
            &tg_config.webhook_secret,
        ).await {
            Ok(()) => tracing::info!(url = %webhook_url, "Telegram webhook registered"),
            Err(e) => tracing::warn!(error = %e, "failed to register Telegram webhook — bot commands will not work until a public URL is configured via BACKEND_URL"),
        }

        if let Err(e) = kahf_notify::set_bot_commands(&http_client, &tg_config.bot_token).await {
            tracing::warn!(error = %e, "failed to set Telegram bot commands");
        }
    }

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
