//! Presence tracking for online users per workspace.
//!
//! ## PresenceTracker
//!
//! Maintains a mapping of workspace IDs to sets of online user IDs.
//! Updated when users connect/disconnect via WebSocket.
//!
//! ## user_online / user_offline
//!
//! Adds or removes a user from the online set for a workspace.
//! Returns `true` if the set was actually modified (first join or last leave).
//!
//! ## online_users
//!
//! Returns the set of currently online user IDs for a workspace.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

pub struct PresenceTracker {
    online: HashMap<Uuid, HashSet<Uuid>>,
}

impl PresenceTracker {
    pub fn new() -> Self {
        Self {
            online: HashMap::new(),
        }
    }

    pub fn user_online(&mut self, workspace_id: Uuid, user_id: Uuid) -> bool {
        self.online
            .entry(workspace_id)
            .or_default()
            .insert(user_id)
    }

    pub fn user_offline(&mut self, workspace_id: Uuid, user_id: Uuid) -> bool {
        if let Some(users) = self.online.get_mut(&workspace_id) {
            let removed = users.remove(&user_id);
            if users.is_empty() {
                self.online.remove(&workspace_id);
            }
            removed
        } else {
            false
        }
    }

    pub fn online_users(&self, workspace_id: Uuid) -> HashSet<Uuid> {
        self.online
            .get(&workspace_id)
            .cloned()
            .unwrap_or_default()
    }
}
