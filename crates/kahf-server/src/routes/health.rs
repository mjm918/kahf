//! Health check endpoint.
//!
//! ## GET /api/health
//!
//! Returns JSON with server status, version, and database connectivity.
//! Attempts a `SELECT 1` query to verify the database connection.

use axum::extract::State;
use axum::routing::get;
use axum::Router;

use crate::app_state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/health", get(health_check))
}

async fn health_check(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(state.pool())
        .await
        .is_ok();

    let status = if db_ok { "ok" } else { "degraded" };

    axum::Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "postgres": if db_ok { "ok" } else { "error" },
    }))
}
