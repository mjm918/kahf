//! Kahf authentication layer.
//!
//! Provides password hashing, JWT token management, email OTP verification,
//! password reset, tenant-level user invitations, and axum middleware for
//! the Kahf platform. Depends on `kahf-db` for user, session, OTP, and
//! invitation storage.
//!
//! ## Modules
//!
//! - `password` — Argon2id password hashing and verification
//! - `jwt` — JWT access/refresh token issuance and verification
//! - `middleware` — Axum `AuthUser` extractor for protected routes
//! - `email` — SMTP email sending, OTP generation, invitation emails
//! - `service` — High-level auth operations: signup, login, refresh, verify_otp, resend_otp, forgot_password, reset_password, invite_user, validate_invite

pub mod password;
pub mod jwt;
pub mod middleware;
pub mod email;
pub mod service;

pub use jwt::{Claims, JwtConfig};
pub use middleware::AuthUser;
pub use email::{EmailSender, SmtpConfig};
