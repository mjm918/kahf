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
//! ## telegram
//!
//! `SendTelegramMessage` — delivers a text message via the Telegram Bot API.
//!
//! ## web_push
//!
//! `SendWebPush` — delivers a VAPID-encrypted browser push notification.
//!
//! ## notification
//!
//! `CreateInAppNotification` — persists and broadcasts an in-app notification.
//!
//! ## audit
//!
//! `AuditLog` — non-blocking security audit trail event.

pub mod audit;
pub mod email;
pub mod notification;
pub mod telegram;
pub mod web_push;

pub use audit::AuditLog;
pub use email::{SendInviteEmail, SendOtpEmail, SendPasswordResetEmail};
pub use notification::CreateInAppNotification;
pub use telegram::SendTelegramMessage;
pub use web_push::SendWebPush;
