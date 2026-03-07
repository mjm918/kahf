//! Kahf core domain library.
//!
//! Provides the foundational types, traits, and error definitions shared
//! across all Kahf crates. This crate is intentionally free of I/O — it
//! contains only pure domain models and trait contracts.
//!
//! ## Modules
//!
//! - `entity` — `Entity` (materialized state) and `EntityType` enum
//! - `error` — `KahfError` variants with `eyre::Report` constructors
//! - `event` — `Event` (tx_log row) and `Operation` enum
//! - `id` — Newtype wrappers: `UserId`, `WorkspaceId`, `EntityId`
//! - `pagination` — `Pagination` and `SortOrder` for list queries
//! - `traits` — Trait contracts: `EventStore`, `EntityRepository`,
//!   `EventBus`, `SearchIndex`, `FileStorage`, `TypedEntityData`
//!
//! ## Result Type
//!
//! `kahf_core::Result<T>` is an alias for `eyre::Result<T>`.

pub mod entity;
pub mod error;
pub mod event;
pub mod id;
pub mod pagination;
pub mod traits;

pub use entity::{Entity, EntityType};
pub use error::KahfError;
pub use event::{Event, Operation};
pub use id::{EntityId, UserId, WorkspaceId};
pub use pagination::{Pagination, SortOrder};

pub type Result<T> = eyre::Result<T>;
