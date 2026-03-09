//! Audit log job definition for non-blocking security audit trail.
//!
//! ## AuditLog
//!
//! Represents a single audit event to be persisted to the `audit_log`
//! TimescaleDB hypertable. Enqueued from route handlers and processed
//! by a background worker so audit writes never block HTTP responses.
//! Fields: `user_id` (optional for unauthenticated actions like failed
//! login), `action` (dot-notation category), `resource` (target
//! identifier), `outcome` (success/failure), `detail` (arbitrary JSON
//! context), `ip_addr` (client IP), `user_agent` (client identifier).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Job;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource: Option<String>,
    pub outcome: String,
    pub detail: Option<serde_json::Value>,
    pub ip_addr: Option<String>,
    pub user_agent: Option<String>,
}

impl Job for AuditLog {
    const JOB_TYPE: &'static str = "AuditLog";
}
