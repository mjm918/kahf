//! JWT access and refresh token issuance and verification.
//!
//! ## Claims
//!
//! JWT payload: `sub` (user ID), `workspace_id` (optional, scoped to
//! workspace), `role` (user role within workspace), `token_type`
//! (either `"access"` or `"refresh"`), `exp` (expiration timestamp),
//! `iat` (issued-at timestamp).
//!
//! ## JwtConfig
//!
//! Holds the HS256 secret, access token TTL (default 15 minutes),
//! and refresh token TTL (default 7 days).
//!
//! ## issue_access_token
//!
//! Creates a short-lived access token (15min default) for API requests.
//!
//! ## issue_refresh_token
//!
//! Creates a long-lived refresh token (7 days default) for obtaining
//! new access tokens without re-authentication.
//!
//! ## verify_token
//!
//! Decodes and validates a JWT string. Returns the claims on success.
//! Checks signature, expiration, and required fields.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub workspace_id: Option<Uuid>,
    pub role: Option<String>,
    pub token_type: String,
    pub exp: u64,
    pub iat: u64,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

impl JwtConfig {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            access_ttl: Duration::minutes(15),
            refresh_ttl: Duration::days(7),
        }
    }
}

pub fn issue_access_token(
    config: &JwtConfig,
    user_id: Uuid,
    workspace_id: Option<Uuid>,
    role: Option<String>,
) -> eyre::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        workspace_id,
        role,
        token_type: "access".to_string(),
        iat: now.timestamp() as u64,
        exp: (now + config.access_ttl).timestamp() as u64,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| eyre::eyre!("failed to encode access token: {e}"))?;

    Ok(token)
}

pub fn issue_refresh_token(
    config: &JwtConfig,
    user_id: Uuid,
) -> eyre::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        workspace_id: None,
        role: None,
        token_type: "refresh".to_string(),
        iat: now.timestamp() as u64,
        exp: (now + config.refresh_ttl).timestamp() as u64,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| eyre::eyre!("failed to encode refresh token: {e}"))?;

    Ok(token)
}

pub fn verify_token(config: &JwtConfig, token: &str) -> eyre::Result<Claims> {
    let validation = Validation::new(Algorithm::HS256);

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &validation,
    )
    .map_err(|e| match *e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => kahf_core::KahfError::unauthorized(),
        jsonwebtoken::errors::ErrorKind::InvalidToken => kahf_core::KahfError::unauthorized(),
        jsonwebtoken::errors::ErrorKind::InvalidSignature => kahf_core::KahfError::unauthorized(),
        _ => eyre::eyre!("JWT verification failed: {e}"),
    })?;

    Ok(token_data.claims)
}
