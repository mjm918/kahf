//! In-app notification job definition.
//!
//! ## CreateInAppNotification
//!
//! Persists a notification record and broadcasts it over WebSocket.
//! Fields: `user_id`, `title`, `body`, `category`, `data`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Job;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInAppNotification {
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub category: String,
    pub data: Option<serde_json::Value>,
}

impl Job for CreateInAppNotification {
    const JOB_TYPE: &'static str = "CreateInAppNotification";
}
