//! Unified notification dispatch that fans out to active channels.
//!
//! ## Notification
//!
//! A channel-agnostic notification payload with `title`, `body`, `category`,
//! recipient `user_id`, and optional `data`. Passed to `dispatch` which
//! checks user preferences and enqueues jobs for each active channel.
//!
//! ## NotificationKind
//!
//! Distinguishes transactional notifications (OTP, password reset) from
//! regular ones. Transactional notifications bypass user preferences and
//! are always delivered via email.
//!
//! ## dispatch
//!
//! Checks the user's notification preferences for each channel, skips
//! disabled or snoozed channels, and returns a list of `DispatchAction`
//! values indicating which channels should receive the notification.
//! The caller is responsible for enqueuing the actual worker jobs, since
//! this module does not depend on `kahf-worker`.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::channel::NotificationChannel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub category: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Transactional,
    Regular,
}

#[derive(Debug, Clone)]
pub struct DispatchAction {
    pub channel: NotificationChannel,
    pub notification: Notification,
}

#[derive(Debug, Clone)]
pub struct ChannelPreference {
    pub channel: NotificationChannel,
    pub enabled: bool,
    pub snoozed_until: Option<chrono::DateTime<Utc>>,
}

pub fn resolve_channels(
    kind: NotificationKind,
    preferences: &[ChannelPreference],
) -> Vec<NotificationChannel> {
    if kind == NotificationKind::Transactional {
        return vec![NotificationChannel::Email];
    }

    let all_channels = NotificationChannel::all();

    all_channels
        .iter()
        .filter(|ch| {
            match preferences.iter().find(|p| p.channel == **ch) {
                None => true,
                Some(pref) => {
                    if !pref.enabled {
                        return false;
                    }
                    if let Some(until) = pref.snoozed_until {
                        return Utc::now() >= until;
                    }
                    true
                }
            }
        })
        .copied()
        .collect()
}
