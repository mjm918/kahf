//! Axum authentication middleware extractor.
//!
//! ## AuthUser
//!
//! Extractor that reads the `Authorization: Bearer <token>` header,
//! verifies the JWT, and provides the authenticated user's claims
//! to handler functions. Returns 401 Unauthorized on missing/invalid
//! tokens.
//!
//! ## Usage
//!
//! Add `AuthUser` as a parameter to any axum handler:
//! ```ignore
//! async fn my_handler(auth: AuthUser) -> impl IntoResponse {
//!     let user_id = auth.claims.sub;
//! }
//! ```
//!
//! The `JwtConfig` must be present in axum's state via `Extension`.

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};

use crate::jwt::{Claims, JwtConfig, verify_token};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub claims: Claims,
}

pub struct AuthError;

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let config = parts
            .extensions
            .get::<JwtConfig>()
            .ok_or(AuthError)?;

        let header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AuthError)?;

        let token = header.strip_prefix("Bearer ").ok_or(AuthError)?;

        let claims = verify_token(config, token).map_err(|_| AuthError)?;

        if claims.token_type != "access" {
            return Err(AuthError);
        }

        Ok(AuthUser { claims })
    }
}
