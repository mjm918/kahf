//! Shared application state passed to all axum handlers.
//!
//! ## AppState
//!
//! Holds the database pool (`DbPool`) wrapped in `Arc`, JWT
//! configuration (`JwtConfig`), SMTP configuration (`SmtpConfig`),
//! WebSocket hub (`Hub`), the in-process event bus
//! (`BroadcastEventBus`), and the RBAC enforcer (`RbacEnforcer`).
//! Extracted by handlers via axum's `State` extractor.

use std::sync::Arc;

use kahf_auth::JwtConfig;
use kahf_email::EmailSender;
use kahf_db::DbPool;
use kahf_rbac::RbacEnforcer;
use kahf_realtime::{BroadcastEventBus, Hub};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub jwt: JwtConfig,
    pub mailer: Arc<dyn EmailSender>,
    pub hub: Hub,
    pub event_bus: BroadcastEventBus,
    pub rbac: RbacEnforcer,
}

impl AppState {
    pub fn new(
        db: DbPool,
        jwt: JwtConfig,
        mailer: Arc<dyn EmailSender>,
        hub: Hub,
        event_bus: BroadcastEventBus,
        rbac: RbacEnforcer,
    ) -> Self {
        Self {
            db: Arc::new(db),
            jwt,
            mailer,
            hub,
            event_bus,
            rbac,
        }
    }

    pub fn pool(&self) -> &PgPool {
        self.db.pool()
    }
}
