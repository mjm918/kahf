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

pub use pool::DbPool;
