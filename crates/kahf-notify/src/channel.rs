//! Notification channel definitions.
//!
//! ## NotificationChannel
//!
//! Enum of all supported delivery channels. Serialized as lowercase strings
//! for database storage and API responses. Variants: `Email`, `Telegram`,
//! `WebPush`, `InApp`. The `all()` method returns a slice of every channel
//! for initializing default user preferences.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationChannel {
    Email,
    Telegram,
    WebPush,
    InApp,
}

impl NotificationChannel {
    pub fn all() -> &'static [NotificationChannel] {
        &[
            NotificationChannel::Email,
            NotificationChannel::Telegram,
            NotificationChannel::WebPush,
            NotificationChannel::InApp,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationChannel::Email => "email",
            NotificationChannel::Telegram => "telegram",
            NotificationChannel::WebPush => "web_push",
            NotificationChannel::InApp => "in_app",
        }
    }
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NotificationChannel {
    type Err = eyre::Report;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email" => Ok(NotificationChannel::Email),
            "telegram" => Ok(NotificationChannel::Telegram),
            "web_push" => Ok(NotificationChannel::WebPush),
            "in_app" => Ok(NotificationChannel::InApp),
            _ => Err(eyre::eyre!("unknown notification channel: {}", s)),
        }
    }
}
