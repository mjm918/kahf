//! Shared application state passed to all axum handlers.
//!
//! ## AppState
//!
//! Holds the database pool (`DbPool`) wrapped in `Arc`, JWT
//! configuration (`JwtConfig`), email sender, job producer for
//! background tasks, WebSocket hub (`Hub`), the in-process event bus
//! (`BroadcastEventBus`), the RBAC enforcer (`RbacEnforcer`), and
//! optional Telegram sender with webhook secret for bot integration.
//! Extracted by handlers via axum's `State` extractor.

use std::sync::Arc;

use kahf_auth::JwtConfig;
use kahf_notify::{EmailSender, TelegramSender};
use kahf_db::DbPool;
use kahf_rbac::RbacEnforcer;
use kahf_realtime::{BroadcastEventBus, Hub};
use kahf_worker::JobProducer;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DbPool>,
    pub jwt: JwtConfig,
    pub mailer: Arc<dyn EmailSender>,
    pub jobs: JobProducer,
    pub hub: Hub,
    pub event_bus: BroadcastEventBus,
    pub rbac: RbacEnforcer,
    pub telegram_sender: Option<Arc<TelegramSender>>,
    pub telegram_webhook_secret: Option<String>,
}

impl AppState {
    pub fn new(
        db: DbPool,
        jwt: JwtConfig,
        mailer: Arc<dyn EmailSender>,
        jobs: JobProducer,
        hub: Hub,
        event_bus: BroadcastEventBus,
        rbac: RbacEnforcer,
    ) -> Self {
        Self {
            db: Arc::new(db),
            jwt,
            mailer,
            jobs,
            hub,
            event_bus,
            rbac,
            telegram_sender: None,
            telegram_webhook_secret: None,
        }
    }

    pub fn with_telegram(
        mut self,
        sender: Arc<TelegramSender>,
        webhook_secret: String,
    ) -> Self {
        self.telegram_sender = Some(sender);
        self.telegram_webhook_secret = Some(webhook_secret);
        self
    }

    pub fn pool(&self) -> &PgPool {
        self.db.pool()
    }
}
