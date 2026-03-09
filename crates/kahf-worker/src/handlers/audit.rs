//! Audit log job handler.
//!
//! ## handle_audit_log
//!
//! Inserts an audit event into the `audit_log` TimescaleDB hypertable.
//! Receives the `AuditLog` job payload from Redis and persists it using
//! the shared `PgPool`. Failures are logged but do not retry — audit
//! events are best-effort to avoid blocking the job queue.

use apalis::prelude::Data;
use sqlx::PgPool;

use crate::jobs::audit::AuditLog;

pub async fn handle_audit_log(
    job: AuditLog,
    pool: Data<PgPool>,
) -> Result<(), String> {
    let ip_str = job.ip_addr.as_deref();

    let result = sqlx::query(
        "INSERT INTO audit_log (user_id, action, resource, outcome, detail, ip_addr, user_agent) VALUES ($1, $2, $3, $4, $5, $6::inet, $7)",
    )
    .bind(job.user_id)
    .bind(&job.action)
    .bind(&job.resource)
    .bind(&job.outcome)
    .bind(&job.detail)
    .bind(ip_str)
    .bind(&job.user_agent)
    .execute(&*pool)
    .await;

    match result {
        Ok(_) => {
            tracing::debug!(action = %job.action, "audit event recorded");
            Ok(())
        }
        Err(e) => {
            tracing::error!(action = %job.action, error = %e, "failed to write audit log");
            Err(format!("audit insert failed: {e}"))
        }
    }
}
