//! Materialized entity model.
//!
//! Represents the current state of any domain object, materialized from
//! the event log. Stored in the `entities` table with JSONB `data`.
//!
//! ## EntityType
//!
//! Enum of built-in domain object kinds: `Task`, `Contact`, `Company`,
//! `Document`, `Message`, `Channel`, `CalendarEvent`, `File`, `Employee`,
//! `Department`, `Lead`, `Deal`. The `Custom(String)` variant allows
//! plugins to register new entity types without schema migrations.
//!
//! ## Entity
//!
//! Mirrors the `entities` table. Fields: `id`, `workspace_id`,
//! `entity_type`, `data` (JSONB value), `created_at`, `updated_at`,
//! `created_by`, `deleted`. Typed helper structs (e.g. `TaskData`) can
//! be deserialized from the `data` field when needed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::id::{EntityId, UserId, WorkspaceId};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Task,
    Contact,
    Company,
    Document,
    Message,
    Channel,
    CalendarEvent,
    File,
    Employee,
    Department,
    Lead,
    Deal,
    #[serde(untagged)]
    Custom(String),
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Task => write!(f, "task"),
            Self::Contact => write!(f, "contact"),
            Self::Company => write!(f, "company"),
            Self::Document => write!(f, "document"),
            Self::Message => write!(f, "message"),
            Self::Channel => write!(f, "channel"),
            Self::CalendarEvent => write!(f, "calendar_event"),
            Self::File => write!(f, "file"),
            Self::Employee => write!(f, "employee"),
            Self::Department => write!(f, "department"),
            Self::Lead => write!(f, "lead"),
            Self::Deal => write!(f, "deal"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub workspace_id: WorkspaceId,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub data: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: UserId,
    pub deleted: bool,
}
