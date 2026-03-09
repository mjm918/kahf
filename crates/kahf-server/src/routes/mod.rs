//! Route module aggregation and top-level router construction.
//!
//! ## api_router
//!
//! Builds the complete router by merging all sub-routers:
//! auth, users, workspaces, entities, notifications, telegram, health,
//! and WebSocket.

pub mod auth;
pub mod entities;
pub mod health;
pub mod notifications;
pub mod telegram;
pub mod users;
pub mod workspaces;
pub mod ws;

use axum::Router;

use crate::app_state::AppState;

pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(users::router())
        .merge(workspaces::router())
        .merge(entities::router())
        .merge(notifications::router())
        .merge(telegram::router())
        .merge(ws::router())
}
