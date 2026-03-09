//! Telegram Bot API client for sending notification messages.
//!
//! Uses the Telegram Bot API directly via `reqwest` rather than pulling in a
//! full bot framework. Only the `sendMessage` endpoint is needed for
//! notification delivery.
//!
//! ## TelegramConfig
//!
//! Holds the bot token loaded from the `TELEGRAM_BOT_TOKEN` env var.
//!
//! ## TelegramSender
//!
//! Async Telegram message sender. Constructed with a `TelegramConfig` and
//! a shared `reqwest::Client`. The `send_message` method posts a text message
//! to a specific chat ID using MarkdownV2 parse mode.

use eyre::WrapErr;

#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
}

impl TelegramConfig {
    pub fn from_env() -> eyre::Result<Self> {
        let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .wrap_err("TELEGRAM_BOT_TOKEN must be set")?;
        Ok(Self { bot_token })
    }

    pub fn from_env_optional() -> Option<Self> {
        std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|t| !t.is_empty())
            .map(|bot_token| Self { bot_token })
    }
}

pub struct TelegramSender {
    client: reqwest::Client,
    bot_token: String,
}

impl TelegramSender {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            bot_token: config.bot_token,
        }
    }

    pub async fn send_message(&self, chat_id: i64, text: &str) -> eyre::Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML"
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .wrap_err("failed to send Telegram message")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(eyre::eyre!(
                "Telegram API returned {}: {}",
                status,
                body_text
            ));
        }

        Ok(())
    }
}
