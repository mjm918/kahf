//! KahfLane database layer.
//!
//! Provides PostgreSQL + TimescaleDB integration for the KahfLane platform.
//! Owns all database tables and migrations. Implements `EventStore` and
//! `EntityRepository` traits from `kahf-core`. Provides standalone
//! repository functions for users, workspaces, sessions, CRDT docs,
//! and email OTPs.
//!
//! ## Modules
//!
//! - `pool` — Connection pool creation and migration runner
//! - `event_store` — `EventStore` impl: append, history, time-travel
//! - `entity_repo` — `EntityRepository` impl: get, list, upsert, soft_delete
//! - `user_repo` — User CRUD: create, get by id/email, update, mark_email_verified
//! - `workspace_repo` — Workspace + member management
//! - `session_repo` — Session create/validate/delete/cleanup
//! - `crdt_repo` — CRDT document snapshot save/load
//! - `otp_repo` — Email OTP create, validate, invalidate
//! - `invite_repo` — Tenant-level invitation create, validate, accept, cancel
//! - `tenant_repo` — Tenant create, get by owner/id
//! - `password_history_repo` — Password history for reuse prevention
//! - `refresh_token_repo` — Server-side refresh token storage, validation, and revocation
//! - `notification_pref_repo` — Per-user, per-channel notification preference CRUD
//! - `telegram_link_repo` — Telegram account linking for notification delivery
//! - `telegram_link_code_repo` — Temporary link codes for secure bot-linking flow
//! - `push_subscription_repo` — Web push subscription storage for browser push delivery
//! - `notification_repo` — In-app notification persistence and read/unread management

pub mod pool;
pub mod event_store;
pub mod entity_repo;
pub mod user_repo;
pub mod workspace_repo;
pub mod session_repo;
pub mod crdt_repo;
pub mod otp_repo;
pub mod invite_repo;
pub mod tenant_repo;
pub mod password_history_repo;
pub mod refresh_token_repo;
pub mod notification_pref_repo;
pub mod telegram_link_repo;
pub mod telegram_link_code_repo;
pub mod push_subscription_repo;
pub mod notification_repo;

pub use pool::DbPool;
