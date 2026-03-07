//! Kahf database layer.
//!
//! Provides PostgreSQL + TimescaleDB integration for the Kahf platform.
//! Owns all database tables and migrations. Implements `EventStore` and
//! `EntityRepository` traits from `kahf-core`. Provides standalone
//! repository functions for users, workspaces, sessions, and CRDT docs.
//!
//! ## Modules
//!
//! - `pool` — Connection pool creation and migration runner
//! - `event_store` — `EventStore` impl: append, history, time-travel
//! - `entity_repo` — `EntityRepository` impl: get, list, upsert, soft_delete
//! - `user_repo` — User CRUD: create, get by id/email, update
//! - `workspace_repo` — Workspace + member management
//! - `session_repo` — Session create/validate/delete/cleanup
//! - `crdt_repo` — CRDT document snapshot save/load

pub mod pool;
pub mod event_store;
pub mod entity_repo;
pub mod user_repo;
pub mod workspace_repo;
pub mod session_repo;
pub mod crdt_repo;

pub use pool::DbPool;
