//! KahfLane server library.
//!
//! Exposes the application state, configuration, error mapping, and
//! route construction so that `main.rs` composes them into the binary
//! and integration tests can build a test app without starting a
//! real TCP listener.
//!
//! ## build_app
//!
//! Constructs the full axum `Router` with all middleware layers
//! (CORS, tracing, JWT extension) and route modules attached.
//! Used by both `main.rs` and integration tests.

pub mod app_state;
pub mod config;
pub mod error;
pub mod routes;

use app_state::AppState;
use kahf_auth::JwtConfig;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn build_app(state: AppState, jwt: JwtConfig) -> axum::Router {
    routes::api_router()
        .layer(axum::Extension(jwt))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
