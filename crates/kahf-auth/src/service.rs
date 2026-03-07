//! High-level authentication service operations.
//!
//! ## signup
//!
//! Creates a new user account. Hashes the password with Argon2id,
//! inserts the user into the database, issues access + refresh tokens.
//! Returns `AuthResponse` with both tokens and user info.
//!
//! ## login
//!
//! Authenticates an existing user by email and password. Looks up the
//! user, verifies the password hash, issues new tokens. Returns
//! `AuthResponse` on success, `Unauthorized` on bad credentials.
//!
//! ## refresh
//!
//! Exchanges a valid refresh token for a new access token. Verifies
//! the refresh token, checks that the user still exists, issues a
//! fresh access token. Does not rotate the refresh token.
//!
//! ## AuthResponse
//!
//! Response payload containing `access_token`, `refresh_token`, and
//! basic user info (`user_id`, `email`, `name`).

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::jwt::{JwtConfig, issue_access_token, issue_refresh_token, verify_token};
use crate::password::{hash_password, verify_password};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
}

pub async fn signup(
    pool: &PgPool,
    config: &JwtConfig,
    email: &str,
    password: &str,
    name: &str,
) -> eyre::Result<AuthResponse> {
    let password_hash = hash_password(password)?;
    let user = kahf_db::user_repo::create_user(pool, email, &password_hash, name).await?;

    let access_token = issue_access_token(config, user.id, None, None)?;
    let refresh_token = issue_refresh_token(config, user.id)?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        user_id: user.id,
        email: user.email,
        name: user.name,
    })
}

pub async fn login(
    pool: &PgPool,
    config: &JwtConfig,
    email: &str,
    password: &str,
) -> eyre::Result<AuthResponse> {
    let user = kahf_db::user_repo::get_user_by_email(pool, email)
        .await?
        .ok_or_else(|| kahf_core::KahfError::unauthorized())?;

    verify_password(password, &user.password)?;

    let access_token = issue_access_token(config, user.id, None, None)?;
    let refresh_token = issue_refresh_token(config, user.id)?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        user_id: user.id,
        email: user.email,
        name: user.name,
    })
}

pub async fn refresh(
    pool: &PgPool,
    config: &JwtConfig,
    refresh_token_str: &str,
) -> eyre::Result<String> {
    let claims = verify_token(config, refresh_token_str)?;

    if claims.token_type != "refresh" {
        return Err(kahf_core::KahfError::unauthorized());
    }

    kahf_db::user_repo::get_user_by_id(pool, claims.sub)
        .await?
        .ok_or_else(|| kahf_core::KahfError::unauthorized())?;

    let access_token = issue_access_token(config, claims.sub, None, None)?;
    Ok(access_token)
}
