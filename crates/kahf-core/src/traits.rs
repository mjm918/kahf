//! Core trait contracts.
//!
//! These traits define the boundaries between KahfLane crates. Each trait
//! is implemented by a downstream crate (e.g. kahf-db, kahf-search)
//! and consumed via dependency injection through `dyn Trait` or generics.
//!
//! ## EventStore
//!
//! Append-only event store backed by the `tx_log` hypertable.
//! Implemented by kahf-db. Methods: `append`, `history`, `history_at`.
//!
//! ## EntityRepository
//!
//! CRUD operations on the materialized `entities` table.
//! Implemented by kahf-db. Methods: `get`, `list`, `upsert`, `soft_delete`.
//!
//! ## EventBus
//!
//! In-process event bus for broadcasting domain events.
//! Implemented by kahf-server using `tokio::sync::broadcast`.
//! Methods: `publish`.
//!
//! ## SearchIndex
//!
//! Full-text search index for entities.
//! Implemented by kahf-search via Meilisearch.
//! Methods: `index`, `remove`, `search`.
//!
//! ## FileStorage
//!
//! Object storage for file uploads and downloads.
//! Implemented by kahf-storage via MinIO/S3.
//! Methods: `upload`, `download`, `presign_url`, `delete`.
//!
//! ## TypedEntityData
//!
//! Helper trait for deserializing entity JSONB payloads into typed
//! domain structs (e.g. `TaskData`, `ContactData`). Provides
//! `from_value` and `to_value` with automatic error wrapping.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::entity::{Entity, EntityType};
use crate::event::Event;
use crate::id::{EntityId, UserId, WorkspaceId};
use crate::pagination::Pagination;

pub trait EventStore: Send + Sync {
    fn append(
        &self,
        workspace_id: WorkspaceId,
        user_id: UserId,
        event: Event,
    ) -> impl std::future::Future<Output = crate::Result<Event>> + Send;

    fn history(
        &self,
        entity_id: EntityId,
    ) -> impl std::future::Future<Output = crate::Result<Vec<Event>>> + Send;

    fn history_at(
        &self,
        entity_id: EntityId,
        at: DateTime<Utc>,
    ) -> impl std::future::Future<Output = crate::Result<Vec<Event>>> + Send;
}

pub trait EntityRepository: Send + Sync {
    fn get(
        &self,
        id: EntityId,
    ) -> impl std::future::Future<Output = crate::Result<Option<Entity>>> + Send;

    fn list(
        &self,
        workspace_id: WorkspaceId,
        entity_type: EntityType,
        pagination: Pagination,
    ) -> impl std::future::Future<Output = crate::Result<Vec<Entity>>> + Send;

    fn upsert(
        &self,
        entity: &Entity,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;

    fn soft_delete(
        &self,
        id: EntityId,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;
}

pub trait EventBus: Send + Sync {
    fn publish(
        &self,
        event: Event,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;
}

pub trait SearchIndex: Send + Sync {
    fn index(
        &self,
        entity: &Entity,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;

    fn remove(
        &self,
        id: EntityId,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;

    fn search(
        &self,
        workspace_id: WorkspaceId,
        query: &str,
        entity_type: Option<EntityType>,
        pagination: Pagination,
    ) -> impl std::future::Future<Output = crate::Result<Vec<Entity>>> + Send;
}

pub trait FileStorage: Send + Sync {
    fn upload(
        &self,
        workspace_id: WorkspaceId,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> impl std::future::Future<Output = crate::Result<String>> + Send;

    fn download(
        &self,
        key: &str,
    ) -> impl std::future::Future<Output = crate::Result<Vec<u8>>> + Send;

    fn presign_url(
        &self,
        key: &str,
        expires_secs: u64,
    ) -> impl std::future::Future<Output = crate::Result<String>> + Send;

    fn delete(
        &self,
        key: &str,
    ) -> impl std::future::Future<Output = crate::Result<()>> + Send;
}

pub trait TypedEntityData: Sized + serde::Serialize + serde::de::DeserializeOwned {
    fn entity_type() -> EntityType;

    fn from_value(value: Value) -> crate::Result<Self> {
        serde_json::from_value(value).map_err(|e| eyre::eyre!("failed to deserialize {}: {e}", Self::entity_type()))
    }

    fn to_value(&self) -> crate::Result<Value> {
        serde_json::to_value(self).map_err(|e| eyre::eyre!("failed to serialize {}: {e}", Self::entity_type()))
    }
}
