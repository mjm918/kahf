//! Telegram bot command handling and webhook management.
//!
//! Processes incoming Telegram webhook updates, handles bot commands,
//! and manages the bot lifecycle (webhook registration, command menu).
//!
//! ## BotCommand
//!
//! Parsed command from a Telegram message. Variants: `Start`, `Link`,
//! `Unlink`, `Status`, `Mute`, `Unmute`, `Help`, `Unknown`.
//!
//! ## parse_command
//!
//! Extracts a `BotCommand` from a raw message text string.
//!
//! ## register_webhook
//!
//! Calls the Telegram `setWebhook` API to register the server's webhook
//! URL with a secret token for request verification.
//!
//! ## set_bot_commands
//!
//! Registers the bot's command menu with Telegram via `setMyCommands`
//! so users see autocomplete suggestions in the chat.
//!
//! ## generate_link_code
//!
//! Creates a 6-character alphanumeric link code for the bot-linking flow.
//!
//! ## LINK_CODE_TTL_MINUTES
//!
//! Link code expiration time in minutes (10).
//!
//! ## TelegramUpdate
//!
//! Deserialized Telegram webhook update payload.
//!
//! ## TelegramMessage
//!
//! Deserialized message within an update.
//!
//! ## TelegramChat
//!
//! Chat metadata from a Telegram message.
//!
//! ## TelegramUser
//!
//! User metadata from a Telegram message.
//!
//! ## BOT_RESPONSES
//!
//! Static response templates for each bot command.

use rand::Rng;
use serde::Deserialize;

pub const LINK_CODE_TTL_MINUTES: i64 = 10;

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    #[serde(default)]
    pub message: Option<TelegramMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub from: Option<TelegramUser>,
    pub chat: TelegramChat,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub date: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BotCommand {
    Start,
    Link { code: String },
    Unlink,
    Status,
    Mute { minutes: Option<i64> },
    Unmute,
    Help,
    Unknown { text: String },
}

pub fn parse_command(text: &str) -> BotCommand {
    let trimmed = text.trim();

    if trimmed == "/start" || trimmed.starts_with("/start ") {
        return BotCommand::Start;
    }

    if trimmed == "/help" || trimmed.starts_with("/help ") {
        return BotCommand::Help;
    }

    if trimmed == "/unlink" || trimmed.starts_with("/unlink ") {
        return BotCommand::Unlink;
    }

    if trimmed == "/status" || trimmed.starts_with("/status ") {
        return BotCommand::Status;
    }

    if trimmed == "/unmute" || trimmed.starts_with("/unmute ") {
        return BotCommand::Unmute;
    }

    if trimmed.starts_with("/link ") {
        let code = trimmed.strip_prefix("/link ").unwrap_or("").trim().to_owned();
        if code.is_empty() {
            return BotCommand::Link { code: String::new() };
        }
        return BotCommand::Link { code };
    }
    if trimmed == "/link" {
        return BotCommand::Link { code: String::new() };
    }

    if trimmed.starts_with("/mute") {
        let rest = trimmed.strip_prefix("/mute").unwrap_or("").trim();
        if rest.is_empty() {
            return BotCommand::Mute { minutes: None };
        }
        let minutes = rest.parse::<i64>().ok();
        return BotCommand::Mute { minutes };
    }

    BotCommand::Unknown {
        text: trimmed.to_owned(),
    }
}

pub fn generate_link_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn register_webhook(
    client: &reqwest::Client,
    bot_token: &str,
    webhook_url: &str,
    secret_token: &str,
) -> eyre::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/setWebhook", bot_token);

    let body = serde_json::json!({
        "url": webhook_url,
        "secret_token": secret_token,
        "allowed_updates": ["message"],
        "drop_pending_updates": false,
        "max_connections": 40
    });

    let resp = client.post(&url).json(&body).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(eyre::eyre!("setWebhook failed ({}): {}", status, text));
    }

    let result: serde_json::Value = resp.json().await?;
    if result.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        return Err(eyre::eyre!("setWebhook returned not ok: {}", result));
    }

    tracing::info!(webhook_url = webhook_url, "Telegram webhook registered");
    Ok(())
}

pub async fn delete_webhook(
    client: &reqwest::Client,
    bot_token: &str,
) -> eyre::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/deleteWebhook", bot_token);
    client.post(&url).send().await?;
    tracing::info!("Telegram webhook deleted");
    Ok(())
}

pub async fn set_bot_commands(
    client: &reqwest::Client,
    bot_token: &str,
) -> eyre::Result<()> {
    let url = format!("https://api.telegram.org/bot{}/setMyCommands", bot_token);

    let body = serde_json::json!({
        "commands": [
            {"command": "start", "description": "Welcome message and getting started"},
            {"command": "link", "description": "Link your Telegram to KahfLane — usage: /link CODE"},
            {"command": "unlink", "description": "Unlink your Telegram from KahfLane"},
            {"command": "status", "description": "Check your link and notification status"},
            {"command": "mute", "description": "Mute notifications — usage: /mute or /mute 60"},
            {"command": "unmute", "description": "Unmute notifications"},
            {"command": "help", "description": "Show available commands"}
        ]
    });

    let resp = client.post(&url).json(&body).send().await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::warn!(status = %status, "setMyCommands failed: {}", text);
    } else {
        tracing::info!("Telegram bot commands registered");
    }

    Ok(())
}

pub mod responses {
    //! Static response templates for bot commands.
    //!
    //! ## WELCOME
    //!
    //! Sent on `/start`. Introduces the bot and lists available commands.
    //!
    //! ## HELP
    //!
    //! Sent on `/help`. Lists all commands with usage examples.
    //!
    //! ## LINK_MISSING_CODE
    //!
    //! Sent when `/link` is used without a code.
    //!
    //! ## LINK_SUCCESS
    //!
    //! Sent after successful account linking.
    //!
    //! ## LINK_INVALID
    //!
    //! Sent when the provided link code is invalid or expired.
    //!
    //! ## LINK_ALREADY_LINKED
    //!
    //! Sent when the Telegram account is already linked.
    //!
    //! ## UNLINK_SUCCESS
    //!
    //! Sent after successful unlinking.
    //!
    //! ## UNLINK_NOT_LINKED
    //!
    //! Sent when trying to unlink an account that isn't linked.
    //!
    //! ## STATUS_LINKED
    //!
    //! Template for linked status (includes user name).
    //!
    //! ## STATUS_NOT_LINKED
    //!
    //! Sent when checking status of an unlinked account.
    //!
    //! ## MUTE_SUCCESS
    //!
    //! Template for mute confirmation (includes duration).
    //!
    //! ## MUTE_INDEFINITE
    //!
    //! Sent when muting without a duration.
    //!
    //! ## UNMUTE_SUCCESS
    //!
    //! Sent after unmuting.
    //!
    //! ## UNKNOWN_COMMAND
    //!
    //! Sent for unrecognized messages.

    pub const WELCOME: &str = "\
<b>Welcome to KahfLane Bot!</b> 🏗️

I deliver notifications from your KahfLane workspace directly to Telegram.

<b>To get started:</b>
1. Go to KahfLane → Settings → Notifications
2. Click \"Link Telegram\"
3. Send me the code: <code>/link YOUR_CODE</code>

Type /help to see all commands.";

    pub const HELP: &str = "\
<b>KahfLane Bot Commands</b>

/link CODE — Link your Telegram to KahfLane
/unlink — Unlink your account
/status — Check link & notification status
/mute — Mute all notifications
/mute 60 — Mute for 60 minutes
/unmute — Resume notifications
/help — Show this message";

    pub const LINK_MISSING_CODE: &str = "\
Please provide a link code.

<b>Usage:</b> <code>/link YOUR_CODE</code>

Get your code from KahfLane → Settings → Notifications → Link Telegram.";

    pub const LINK_SUCCESS: &str = "\
<b>Account linked successfully!</b> ✅

You'll now receive KahfLane notifications here. Manage your preferences in KahfLane → Settings → Notifications.";

    pub const LINK_INVALID: &str = "\
<b>Invalid or expired code.</b> ❌

Link codes expire after 10 minutes. Generate a new one from KahfLane → Settings → Notifications → Link Telegram.";

    pub const LINK_ALREADY_LINKED: &str = "\
Your Telegram is already linked to a KahfLane account.

Use /unlink first if you want to link a different account.";

    pub const UNLINK_SUCCESS: &str = "\
<b>Account unlinked.</b>

You will no longer receive KahfLane notifications here. Link again anytime with /link.";

    pub const UNLINK_NOT_LINKED: &str = "\
Your Telegram is not linked to any KahfLane account.

Use /link CODE to connect your account.";

    pub fn status_linked(first_name: &str, last_name: &str, email: &str) -> String {
        format!(
            "<b>Status: Linked</b> ✅\n\n\
            Account: {} {} ({})\n\
            Telegram notifications are active.",
            first_name, last_name, email
        )
    }

    pub const STATUS_NOT_LINKED: &str = "\
<b>Status: Not linked</b>

Your Telegram is not connected to a KahfLane account.
Use /link CODE to connect.";

    pub fn mute_success(minutes: i64) -> String {
        if minutes >= 60 {
            let hours = minutes / 60;
            let remaining = minutes % 60;
            if remaining == 0 {
                format!("<b>Notifications muted for {} hour(s).</b>\n\nUse /unmute to resume.", hours)
            } else {
                format!("<b>Notifications muted for {}h {}m.</b>\n\nUse /unmute to resume.", hours, remaining)
            }
        } else {
            format!("<b>Notifications muted for {} minutes.</b>\n\nUse /unmute to resume.", minutes)
        }
    }

    pub const MUTE_INDEFINITE: &str = "\
<b>Notifications muted.</b>

You won't receive notifications until you /unmute.
To mute for a specific time: <code>/mute 60</code> (minutes)";

    pub const UNMUTE_SUCCESS: &str = "\
<b>Notifications resumed.</b> 🔔

You'll receive KahfLane notifications again.";

    pub const UNKNOWN_COMMAND: &str = "\
I don't understand that. Type /help to see available commands.";
}
