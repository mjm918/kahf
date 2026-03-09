//! Unified notification system for the KahfLane platform.
//!
//! Multi-channel notification delivery: email (SMTP), Telegram (Bot API),
//! web push (VAPID), and in-app (WebSocket + DB). Replaces the standalone
//! `kahf-email` crate. Each channel is independently configured via env
//! vars — missing vars simply disable that channel.
//!
//! ## Modules
//!
//! - `channel` — `NotificationChannel` enum (Email, Telegram, WebPush, InApp)
//! - `dispatch` — Preference-aware channel resolution and dispatch actions
//! - `email` — SMTP email delivery with Tera HTML templates
//! - `otp` — OTP generation and TTL constants
//! - `telegram` — Telegram Bot API message sender via reqwest
//! - `web_push` — VAPID-based browser push via the `web-push` crate

pub mod channel;
pub mod dispatch;
pub mod email;
pub mod otp;
pub mod telegram;
pub mod telegram_bot;
pub mod web_push;

pub use channel::NotificationChannel;
pub use dispatch::{ChannelPreference, DispatchAction, Notification, NotificationKind, resolve_channels};
pub use email::{EmailSender, SmtpConfig, SmtpEmailSender, SmtpEncryption};
pub use otp::{INVITE_TTL_DAYS, OTP_TTL_MINUTES, generate_otp};
pub use telegram::{TelegramConfig, TelegramSender};
pub use telegram_bot::{
    BotCommand, TelegramUpdate, LINK_CODE_TTL_MINUTES,
    generate_link_code, parse_command, register_webhook, set_bot_commands,
};
pub use web_push::{PushPayload, VapidConfig, WebPushSender};
