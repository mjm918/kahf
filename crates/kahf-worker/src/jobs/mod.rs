//! Job type definitions for background processing.
//!
//! Each job is a serializable struct representing a unit of work. Jobs are
//! enqueued via `JobProducer` and processed by registered workers. All job
//! types re-exported here for convenience.
//!
//! ## email
//!
//! Email delivery jobs: `SendOtpEmail`, `SendPasswordResetEmail`,
//! `SendInviteEmail`.
//!
//! ## audit
//!
//! `AuditLog` — non-blocking security audit trail event.

pub mod audit;
pub mod email;

pub use audit::AuditLog;
pub use email::{SendInviteEmail, SendOtpEmail, SendPasswordResetEmail};
