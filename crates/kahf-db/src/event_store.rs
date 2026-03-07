//! EventStore trait implementation backed by the `tx_log` hypertable.
//!
//! ## EventStore for DbPool
//!
//! Implements the `EventStore` trait from `kahf-core` against PostgreSQL.
//!
//! ## append
//!
//! Inserts a new event into `tx_log` and returns the event with its
//! server-assigned `id` and `ts`. The caller provides workspace/user
//! context; the event carries entity details and payload.
//!
//! ## history
//!
//! Returns all events for an entity ordered by timestamp ascending.
//!
//! ## history_at
//!
//! Returns events for an entity up to a given timestamp for time-travel.
//!
//! ## parse_op
//!
//! Converts a text column value back into an `Operation` enum variant.
//!
//! ## parse_entity_type
//!
//! Converts a text column value back into an `EntityType` enum variant.
//! Falls back to `EntityType::Custom` for unrecognized values.
//!
//! ## row_to_event
//!
//! Maps a `sqlx::postgres::PgRow` to a `kahf_core::event::Event`.

use chrono::{DateTime, Utc};
use sqlx::Row;

use kahf_core::entity::EntityType;
use kahf_core::event::{Event, Operation};
use kahf_core::id::{EntityId, UserId, WorkspaceId};
use kahf_core::traits::EventStore;

use crate::pool::DbPool;

fn parse_op(s: &str) -> eyre::Result<Operation> {
    match s {
        "create" => Ok(Operation::Create),
        "update" => Ok(Operation::Update),
        "delete" => Ok(Operation::Delete),
        other => Err(eyre::eyre!("unknown operation: {other}")),
    }
}

pub fn parse_entity_type(s: &str) -> EntityType {
    match s {
        "task" => EntityType::Task,
        "contact" => EntityType::Contact,
        "company" => EntityType::Company,
        "document" => EntityType::Document,
        "message" => EntityType::Message,
        "channel" => EntityType::Channel,
        "calendar_event" => EntityType::CalendarEvent,
        "file" => EntityType::File,
        "employee" => EntityType::Employee,
        "department" => EntityType::Department,
        "lead" => EntityType::Lead,
        "deal" => EntityType::Deal,
        other => EntityType::Custom(other.to_string()),
    }
}

fn row_to_event(row: sqlx::postgres::PgRow) -> eyre::Result<Event> {
    Ok(Event {
        id: EntityId::from_uuid(row.try_get("id")?),
        ts: row.try_get("ts")?,
        workspace_id: WorkspaceId::from_uuid(row.try_get("workspace_id")?),
        user_id: UserId::from_uuid(row.try_get("user_id")?),
        op: parse_op(row.try_get("op")?)?,
        entity_type: parse_entity_type(row.try_get("entity_type")?),
        entity_id: EntityId::from_uuid(row.try_get("entity_id")?),
        data: row.try_get("data")?,
        metadata: row.try_get("metadata")?,
    })
}

impl EventStore for DbPool {
    async fn append(
        &self,
        workspace_id: WorkspaceId,
        user_id: UserId,
        event: Event,
    ) -> kahf_core::Result<Event> {
        let row = sqlx::query(
            "INSERT INTO tx_log (workspace_id, user_id, op, entity_type, entity_id, data, metadata)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, ts, workspace_id, user_id, op, entity_type, entity_id, data, metadata"
        )
        .bind(workspace_id.0)
        .bind(user_id.0)
        .bind(event.op.to_string())
        .bind(event.entity_type.to_string())
        .bind(event.entity_id.0)
        .bind(&event.data)
        .bind(&event.metadata)
        .fetch_one(self.pool())
        .await?;

        row_to_event(row)
    }

    async fn history(
        &self,
        entity_id: EntityId,
    ) -> kahf_core::Result<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT id, ts, workspace_id, user_id, op, entity_type, entity_id, data, metadata
             FROM tx_log
             WHERE entity_id = $1
             ORDER BY ts ASC"
        )
        .bind(entity_id.0)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }

    async fn history_at(
        &self,
        entity_id: EntityId,
        at: DateTime<Utc>,
    ) -> kahf_core::Result<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT id, ts, workspace_id, user_id, op, entity_type, entity_id, data, metadata
             FROM tx_log
             WHERE entity_id = $1 AND ts <= $2
             ORDER BY ts ASC"
        )
        .bind(entity_id.0)
        .bind(at)
        .fetch_all(self.pool())
        .await?;

        rows.into_iter().map(row_to_event).collect()
    }
}
