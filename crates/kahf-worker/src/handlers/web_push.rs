//! Web push notification delivery handler.
//!
//! ## handle_send_web_push
//!
//! Sends a VAPID-encrypted push notification to a browser endpoint via
//! `WebPushSender`. Records job audit on completion or failure.

use std::sync::Arc;

use apalis::prelude::Data;
use kahf_notify::{PushPayload, WebPushSender};

use crate::audit::{JobAuditor, JobStatus};
use crate::jobs::web_push::SendWebPush;

pub async fn handle_send_web_push(
    job: SendWebPush,
    sender: Data<Arc<WebPushSender>>,
    auditor: Data<JobAuditor>,
) -> Result<(), String> {
    tracing::info!(endpoint = %job.endpoint, "processing web push job");

    let payload = PushPayload {
        title: job.title.clone(),
        body: job.body.clone(),
        icon: job.icon.clone(),
        url: job.url.clone(),
        tag: job.tag.clone(),
    };

    let result = sender
        .send(&job.endpoint, &job.p256dh, &job.auth, &payload)
        .await
        .map_err(|e| format!("send_web_push failed: {:?}", e));

    match &result {
        Ok(()) => {
            tracing::info!(endpoint = %job.endpoint, "web push sent");
            auditor
                .record(
                    uuid::Uuid::new_v4(),
                    "SendWebPush",
                    JobStatus::Completed,
                    serde_json::json!({"endpoint": job.endpoint}),
                    None,
                    1,
                )
                .await;
        }
        Err(e) => {
            tracing::error!(endpoint = %job.endpoint, error = %e, "web push failed");
            auditor
                .record(
                    uuid::Uuid::new_v4(),
                    "SendWebPush",
                    JobStatus::Failed,
                    serde_json::json!({"endpoint": job.endpoint}),
                    Some(e),
                    1,
                )
                .await;
        }
    }

    result
}
