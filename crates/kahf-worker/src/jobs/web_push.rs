//! Web push notification job definition.
//!
//! ## SendWebPush
//!
//! Sends a VAPID-encrypted push notification to a browser endpoint.
//! Fields: `endpoint`, `p256dh`, `auth`, `title`, `body`, `icon`, `url`, `tag`.

use serde::{Deserialize, Serialize};

use crate::Job;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendWebPush {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub title: String,
    pub body: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub tag: Option<String>,
}

impl Job for SendWebPush {
    const JOB_TYPE: &'static str = "SendWebPush";
}
