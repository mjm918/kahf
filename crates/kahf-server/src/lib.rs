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
//! (origin-restricted CORS, tracing, JWT extension) and route modules
//! attached. Accepts the frontend URL for CORS origin whitelisting.
//! Used by both `main.rs` and integration tests.
//!
//! ## audit
//!
//! Helpers for enqueuing non-blocking audit log events from route
//! handlers.

pub mod app_state;
pub mod audit;
pub mod config;
pub mod error;
pub mod routes;

use app_state::AppState;
use axum::http::{HeaderValue, Method, header};
use kahf_auth::JwtConfig;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn build_app(state: AppState, jwt: JwtConfig, frontend_url: &str) -> axum::Router {
    let cors = CorsLayer::new()
        .allow_origin(
            frontend_url
                .parse::<HeaderValue>()
                .expect("FRONTEND_URL must be a valid header value"),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .allow_credentials(true);

    routes::api_router()
        .layer(axum::Extension(jwt))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
