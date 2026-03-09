//! Job handler implementations.
//!
//! Each handler processes a specific job type. Handlers receive shared
//! dependencies (email sender, DB pool) via apalis's data injection and
//! record audit entries on completion or failure.
//!
//! ## email
//!
//! Handlers for `SendOtpEmail`, `SendPasswordResetEmail`, and
//! `SendInviteEmail` jobs using `kahf_email::EmailSender`.
//!
//! ## audit
//!
//! Handler for `AuditLog` jobs — inserts audit events into the
//! `audit_log` TimescaleDB hypertable.

pub mod audit;
pub mod email;
