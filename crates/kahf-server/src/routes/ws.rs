//! WebSocket upgrade endpoint.
//!
//! ## GET /ws
//!
//! Upgrades an HTTP connection to a WebSocket. Requires a `token` query
//! parameter containing a valid JWT access token. Extracts `user_id`
//! and `workspace_id` from the token claims and hands the connection
//! to `kahf_realtime::handle_connection`.

use axum::extract::{Query, State};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use axum::extract::ws::WebSocketUpgrade;
use serde::Deserialize;

use crate::app_state::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(ws_handler))
}

#[derive(Deserialize)]
struct WsQuery {
    token: String,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsQuery>,
) -> Result<Response, AppError> {
    let claims = kahf_auth::jwt::verify_token(&state.jwt, &params.token)
        .map_err(|_| kahf_core::KahfError::unauthorized())?;

    if claims.token_type != "access" {
        return Err(kahf_core::KahfError::unauthorized().into());
    }

    let user_id = claims.sub;
    let workspace_id = claims
        .workspace_id
        .ok_or_else(|| kahf_core::KahfError::validation("workspace_id missing from token"))?;

    let hub = state.hub.clone();

    Ok(ws.on_upgrade(move |socket| {
        kahf_realtime::handle_connection(socket, hub, user_id, workspace_id)
    }))
}
