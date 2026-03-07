//! Strongly-typed identifier newtypes.
//!
//! Wraps `uuid::Uuid` in distinct types so the compiler prevents mixing
//! up user IDs, workspace IDs, and entity IDs at call sites.
//!
//! ## Types
//!
//! - `UserId` — unique identifier for a user account
//! - `WorkspaceId` — unique identifier for a workspace
//! - `EntityId` — unique identifier for any domain entity
//!
//! Each type supports `new()` (random v4), `from_uuid()`, `as_uuid()`,
//! `Display`, and bidirectional `From<Uuid>` conversion.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(id: Uuid) -> Self {
                Self(id)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<Uuid> for $name {
            fn from(id: Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

define_id!(UserId);
define_id!(WorkspaceId);
define_id!(EntityId);
