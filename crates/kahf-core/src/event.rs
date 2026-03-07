//! Event (transaction log) model.
//!
//! Every mutation in Kahf is recorded as an immutable event in the
//! `tx_log` TimescaleDB hypertable. Events are the source of truth;
//! the `entities` table is a materialized projection.
//!
//! ## Operation
//!
//! Enum of mutation kinds: `Create`, `Update`, `Delete`.
//!
//! ## Event
//!
//! Mirrors a `tx_log` row. For `Create` operations, `data` holds the
//! full entity payload. For `Update`, a JSON patch (partial fields).
//! For `Delete`, typically an empty object. The optional `metadata`
//! field carries context like plugin ID or sync source.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::entity::EntityType;
use crate::id::{EntityId, UserId, WorkspaceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Create,
    Update,
    Delete,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Create => write!(f, "create"),
            Self::Update => write!(f, "update"),
            Self::Delete => write!(f, "delete"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: EntityId,
    pub ts: DateTime<Utc>,
    pub workspace_id: WorkspaceId,
    pub user_id: UserId,
    pub op: Operation,
    pub entity_type: EntityType,
    pub entity_id: EntityId,
    pub data: Value,
    pub metadata: Option<Value>,
}
