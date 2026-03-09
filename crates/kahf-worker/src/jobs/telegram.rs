//! Telegram notification job definition.
//!
//! ## SendTelegramMessage
//!
//! Sends a text message to a Telegram chat via the Bot API.
//! Fields: `chat_id`, `text`.

use serde::{Deserialize, Serialize};

use crate::Job;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendTelegramMessage {
    pub chat_id: i64,
    pub text: String,
}

impl Job for SendTelegramMessage {
    const JOB_TYPE: &'static str = "SendTelegramMessage";
}
