//! In-app notification handler.
//!
//! ## handle_create_in_app_notification
//!
//! Persists a notification record to the database. The WebSocket broadcast
//! is handled separately by the caller after enqueueing. Records job audit
//! on completion or failure.

use apalis::prelude::Data;
use sqlx::PgPool;

use crate::audit::{JobAuditor, JobStatus};
use crate::jobs::notification::CreateInAppNotification;

pub async fn handle_create_in_app_notification(
    job: CreateInAppNotification,
    pool: Data<PgPool>,
    auditor: Data<JobAuditor>,
) -> Result<(), String> {
    tracing::info!(user_id = %job.user_id, category = %job.category, "processing in-app notification job");

    let result = kahf_db::notification_repo::create_notification(
        &pool,
        job.user_id,
        &job.title,
        &job.body,
        &job.category,
        job.data.clone(),
    )
    .await
    .map(|_| ())
    .map_err(|e| format!("create_notification failed: {:?}", e));

    match &result {
        Ok(()) => {
            tracing::info!(user_id = %job.user_id, "in-app notification created");
            auditor
                .record(
                    uuid::Uuid::new_v4(),
                    "CreateInAppNotification",
                    JobStatus::Completed,
                    serde_json::json!({"user_id": job.user_id, "category": job.category}),
                    None,
                    1,
                )
                .await;
        }
        Err(e) => {
            tracing::error!(user_id = %job.user_id, error = %e, "in-app notification failed");
            auditor
                .record(
                    uuid::Uuid::new_v4(),
                    "CreateInAppNotification",
                    JobStatus::Failed,
                    serde_json::json!({"user_id": job.user_id, "category": job.category}),
                    Some(e),
                    1,
                )
                .await;
        }
    }

    result
}
