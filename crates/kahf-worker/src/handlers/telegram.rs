//! Telegram message delivery handler.
//!
//! ## handle_send_telegram
//!
//! Sends a text message to a Telegram chat via `TelegramSender`. Records
//! job audit on completion or failure.

use std::sync::Arc;

use apalis::prelude::Data;
use kahf_notify::TelegramSender;

use crate::audit::{JobAuditor, JobStatus};
use crate::jobs::telegram::SendTelegramMessage;

pub async fn handle_send_telegram(
    job: SendTelegramMessage,
    sender: Data<Arc<TelegramSender>>,
    auditor: Data<JobAuditor>,
) -> Result<(), String> {
    tracing::info!(chat_id = job.chat_id, "processing Telegram message job");

    let result = sender
        .send_message(job.chat_id, &job.text)
        .await
        .map_err(|e| format!("send_telegram failed: {:?}", e));

    match &result {
        Ok(()) => {
            tracing::info!(chat_id = job.chat_id, "Telegram message sent");
            auditor
                .record(
                    uuid::Uuid::new_v4(),
                    "SendTelegramMessage",
                    JobStatus::Completed,
                    serde_json::json!({"chat_id": job.chat_id}),
                    None,
                    1,
                )
                .await;
        }
        Err(e) => {
            tracing::error!(chat_id = job.chat_id, error = %e, "Telegram message failed");
            auditor
                .record(
                    uuid::Uuid::new_v4(),
                    "SendTelegramMessage",
                    JobStatus::Failed,
                    serde_json::json!({"chat_id": job.chat_id}),
                    Some(e),
                    1,
                )
                .await;
        }
    }

    result
}
