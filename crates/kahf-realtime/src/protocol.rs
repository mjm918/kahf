//! WebSocket protocol message types.
//!
//! All messages are JSON-encoded with a `type` tag discriminator.
//! CRDT binary data is base64-encoded in the `payload` field.
//!
//! ## Inbound (client → server)
//!
//! - `CrdtJoin` — join a document room for CRDT sync
//! - `CrdtLeave` — leave a document room
//! - `CrdtSync` — send a yrs update (base64-encoded)
//! - `CrdtAwareness` — send cursor/selection state
//! - `ChatMessage` — send a chat message to a channel
//!
//! ## Outbound (server → client, or broadcast)
//!
//! - `EntityCreated` / `EntityUpdated` / `EntityDeleted` — entity mutations
//! - `CrdtSync` — broadcast yrs update to other doc room members
//! - `CrdtState` — full document state sent on join
//! - `CrdtAwareness` — broadcast awareness to other doc room members
//! - `PresenceUpdate` — user online/offline status
//! - `ChatMessage` — broadcast chat message
//! - `Notification` — push notification

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "entity.created")]
    EntityCreated {
        entity_type: String,
        id: Uuid,
        data: Value,
    },

    #[serde(rename = "entity.updated")]
    EntityUpdated {
        entity_type: String,
        id: Uuid,
        patch: Value,
    },

    #[serde(rename = "entity.deleted")]
    EntityDeleted {
        entity_type: String,
        id: Uuid,
    },

    #[serde(rename = "crdt.join")]
    CrdtJoin {
        doc_id: Uuid,
    },

    #[serde(rename = "crdt.leave")]
    CrdtLeave {
        doc_id: Uuid,
    },

    #[serde(rename = "crdt.sync")]
    CrdtSync {
        doc_id: Uuid,
        payload: String,
    },

    #[serde(rename = "crdt.state")]
    CrdtState {
        doc_id: Uuid,
        payload: String,
    },

    #[serde(rename = "crdt.awareness")]
    CrdtAwareness {
        doc_id: Uuid,
        user: Uuid,
        state: Value,
    },

    #[serde(rename = "presence.update")]
    PresenceUpdate {
        user: Uuid,
        status: String,
    },

    #[serde(rename = "chat.message")]
    ChatMessage {
        channel_id: Uuid,
        text: String,
        user: Uuid,
    },

    #[serde(rename = "notification")]
    Notification {
        kind: String,
        #[serde(rename = "ref")]
        reference: Uuid,
        text: String,
    },
}
