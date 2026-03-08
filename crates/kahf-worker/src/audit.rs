//! Job audit trail recorder backed by TimescaleDB.
//!
//! Every job state transition (enqueued, started, completed, failed, retried)
//! is appended as a row in the `job_audit` hypertable. This provides a full
//! time-series history of all background jobs with no updates — append-only.
//!
//! ## JobAuditor
//!
//! Holds an optional `PgPool` reference and provides `record` for inserting
//! audit rows. Use `new` with a pool for production, `noop` without a pool
//! for tests. Used by `JobProducer` on enqueue and by handlers on
//! start/complete/fail transitions.
//!
//! ## JobStatus
//!
//! Enum of possible job states: `Enqueued`, `Started`, `Completed`, `Failed`,
//! `Retried`.

use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct JobAuditor {
    pool: Option<PgPool>,
}

pub enum JobStatus {
    Enqueued,
    Started,
    Completed,
    Failed,
    Retried,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Enqueued => "enqueued",
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Retried => "retried",
        }
    }
}

impl JobAuditor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool: Some(pool) }
    }

    pub fn noop() -> Self {
        Self { pool: None }
    }

    pub async fn record(
        &self,
        job_id: Uuid,
        job_type: &str,
        status: JobStatus,
        payload: serde_json::Value,
        error: Option<&str>,
        attempt: i32,
    ) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };

        let result = sqlx::query(
            "INSERT INTO job_audit (job_id, job_type, status, payload, error, attempt) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(job_id)
        .bind(job_type)
        .bind(status.as_str())
        .bind(&payload)
        .bind(error)
        .bind(attempt)
        .execute(pool)
        .await;

        if let Err(e) = result {
            tracing::error!(job_id = %job_id, job_type = %job_type, error = %e, "failed to write job audit");
        }
    }
}
