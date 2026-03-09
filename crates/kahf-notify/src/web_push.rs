//! VAPID-based web push notification sender.
//!
//! Uses the `web-push` crate to send encrypted push notifications directly
//! to browser push endpoints without FCM or APNS. Requires a VAPID key pair
//! generated via `openssl ec -genkey -name prime256v1`.
//!
//! ## VapidConfig
//!
//! Holds the VAPID private key (base64url-encoded raw 32-byte scalar),
//! public key (base64url-encoded), and subject (usually a `mailto:` URI).
//! Loaded from `VAPID_PRIVATE_KEY`, `VAPID_PUBLIC_KEY`, and `VAPID_SUBJECT`
//! env vars.
//!
//! ## WebPushSender
//!
//! Sends push notifications to a browser subscription endpoint. Constructs
//! a `PartialVapidSignatureBuilder` once, clones it per message, binds to
//! the subscription info, encrypts the payload, and delivers via HTTPS POST.
//!
//! ## PushPayload
//!
//! JSON payload sent to the browser's service worker. Contains `title`,
//! `body`, `icon`, `url`, and `tag` fields.

use eyre::WrapErr;
use web_push::{
    ContentEncoding, IsahcWebPushClient, PartialVapidSignatureBuilder, SubscriptionInfo,
    VapidSignatureBuilder, WebPushClient, WebPushMessageBuilder,
};

#[derive(Debug, Clone)]
pub struct VapidConfig {
    pub private_key: String,
    pub public_key: String,
    pub subject: String,
}

impl VapidConfig {
    pub fn from_env() -> eyre::Result<Self> {
        Ok(Self {
            private_key: std::env::var("VAPID_PRIVATE_KEY")
                .wrap_err("VAPID_PRIVATE_KEY must be set")?,
            public_key: std::env::var("VAPID_PUBLIC_KEY")
                .wrap_err("VAPID_PUBLIC_KEY must be set")?,
            subject: std::env::var("VAPID_SUBJECT")
                .unwrap_or_else(|_| "mailto:admin@kahflane.com".into()),
        })
    }

    pub fn from_env_optional() -> Option<Self> {
        let private_key = std::env::var("VAPID_PRIVATE_KEY").ok()?;
        let public_key = std::env::var("VAPID_PUBLIC_KEY").ok()?;
        if private_key.is_empty() || public_key.is_empty() {
            return None;
        }
        Some(Self {
            private_key,
            public_key,
            subject: std::env::var("VAPID_SUBJECT")
                .unwrap_or_else(|_| "mailto:admin@kahflane.com".into()),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PushPayload {
    pub title: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

pub struct WebPushSender {
    partial_builder: PartialVapidSignatureBuilder,
    client: IsahcWebPushClient,
    subject: String,
}

impl WebPushSender {
    pub fn new(config: &VapidConfig) -> eyre::Result<Self> {
        let partial_builder =
            VapidSignatureBuilder::from_base64_no_sub(&config.private_key)
                .map_err(|e| eyre::eyre!("failed to parse VAPID private key: {:?}", e))?;

        let client = IsahcWebPushClient::new()
            .map_err(|e| eyre::eyre!("failed to create web push client: {:?}", e))?;

        Ok(Self {
            partial_builder,
            client,
            subject: config.subject.clone(),
        })
    }

    pub async fn send(
        &self,
        endpoint: &str,
        p256dh: &str,
        auth: &str,
        payload: &PushPayload,
    ) -> eyre::Result<()> {
        let subscription_info = SubscriptionInfo::new(endpoint, p256dh, auth);

        let content =
            serde_json::to_vec(payload).wrap_err("failed to serialize push payload")?;

        let mut sig_builder = self.partial_builder.clone().add_sub_info(&subscription_info);
        sig_builder.add_claim("sub", self.subject.as_str());
        let signature = sig_builder
            .build()
            .map_err(|e| eyre::eyre!("failed to build VAPID signature: {:?}", e))?;

        let mut msg_builder = WebPushMessageBuilder::new(&subscription_info);
        msg_builder.set_payload(ContentEncoding::Aes128Gcm, &content);
        msg_builder.set_vapid_signature(signature);

        let message = msg_builder
            .build()
            .map_err(|e| eyre::eyre!("failed to build web push message: {:?}", e))?;

        self.client
            .send(message)
            .await
            .map_err(|e| eyre::eyre!("failed to send web push notification: {:?}", e))?;

        Ok(())
    }
}
