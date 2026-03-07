//! Kahf server binary entry point.
//!
//! Loads configuration from environment variables (with `.env` fallback),
//! initializes tracing, connects to the database, runs migrations,
//! creates the WebSocket hub and event bus, builds the axum application
//! with all route modules, and starts the HTTP server.
//!
//! ## Environment Variables
//!
//! - `DATABASE_URL` — PostgreSQL connection string (required)
//! - `JWT_SECRET` — Secret key for JWT token signing (required)
//! - `HOST` — Bind address (default `0.0.0.0`)
//! - `PORT` — Bind port (default `3000`)
//! - `RUST_LOG` — Tracing filter (default `kahf=debug,tower_http=debug`)

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
                .unwrap_or_else(|_| EnvFilter::new("kahf=debug,tower_http=debug")),
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
    let hub = kahf_realtime::Hub::new(db.pool().clone());
    let event_bus = kahf_realtime::BroadcastEventBus::new(1024);
    let state = AppState::new(db, jwt.clone(), hub, event_bus, rbac);

    let app = kahf_server::build_app(state, jwt);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");

    axum::serve(listener, app).await?;

    Ok(())
}
