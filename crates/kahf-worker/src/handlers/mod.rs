//! Job handler implementations.
//!
//! Each handler processes a specific job type. Handlers receive shared
//! dependencies (email sender, DB pool, Telegram/push senders) via apalis's
//! data injection and record audit entries on completion or failure.
//!
//! ## email
//!
//! Handlers for `SendOtpEmail`, `SendPasswordResetEmail`, and
//! `SendInviteEmail` jobs using `kahf_notify::EmailSender`.
//!
//! ## telegram
//!
//! Handler for `SendTelegramMessage` using `kahf_notify::TelegramSender`.
//!
//! ## web_push
//!
//! Handler for `SendWebPush` using `kahf_notify::WebPushSender`.
//!
//! ## notification
//!
//! Handler for `CreateInAppNotification` — persists to database.
//!
//! ## audit
//!
//! Handler for `AuditLog` jobs — inserts audit events into the
//! `audit_log` TimescaleDB hypertable.

pub mod audit;
pub mod email;
pub mod notification;
pub mod telegram;
pub mod web_push;
