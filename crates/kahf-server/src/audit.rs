//! Audit logging helpers for route handlers.
//!
//! Provides `RequestContext` for extracting client IP and user-agent
//! from axum requests, and `emit` for enqueuing audit events to the
//! background job queue without blocking the HTTP response.
//!
//! ## RequestContext
//!
//! Axum extractor that captures the client IP address and user-agent.
//! Checks `x-forwarded-for` and `x-real-ip` headers first (for proxied
//! requests), then falls back to `ConnectInfo<SocketAddr>` for the
//! direct TCP peer address.
//!
//! ## emit
//!
//! Enqueues an `AuditLog` job via `JobProducer`. Fire-and-forget —
//! failures are logged but never bubble up to the caller.

use std::net::SocketAddr;

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use kahf_worker::JobProducer;
use kahf_worker::jobs::AuditLog;
use uuid::Uuid;

pub struct RequestContext {
    pub ip_addr: Option<String>,
    pub user_agent: Option<String>,
}

impl RequestContext {
    pub fn extract(headers: &HeaderMap, connect_info: Option<&ConnectInfo<SocketAddr>>) -> Self {
        let ip_addr = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_owned())
            .or_else(|| {
                headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from)
            })
            .or_else(|| connect_info.map(|ci| ci.0.ip().to_string()));

        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Self { ip_addr, user_agent }
    }
}

pub async fn emit(
    jobs: &JobProducer,
    ctx: &RequestContext,
    user_id: Option<Uuid>,
    action: &str,
    resource: Option<String>,
    outcome: &str,
    detail: Option<serde_json::Value>,
) {
    let event = AuditLog {
        user_id,
        action: action.to_owned(),
        resource,
        outcome: outcome.to_owned(),
        detail,
        ip_addr: ctx.ip_addr.clone(),
        user_agent: ctx.user_agent.clone(),
    };

    if let Err(e) = jobs.enqueue(event).await {
        tracing::error!(action = %action, error = %e, "failed to enqueue audit event");
    }
}
