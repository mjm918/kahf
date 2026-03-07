//! Kahf authentication layer.
//!
//! Provides password hashing, JWT token management, email OTP verification,
//! and axum middleware for the Kahf platform. Depends on `kahf-db` for
//! user, session, and OTP storage.
//!
//! ## Modules
//!
//! - `password` — Argon2id password hashing and verification
//! - `jwt` — JWT access/refresh token issuance and verification
//! - `middleware` — Axum `AuthUser` extractor for protected routes
//! - `email` — SMTP email sending and OTP code generation
//! - `service` — High-level auth operations: signup, login, refresh, verify_otp, resend_otp

pub mod password;
pub mod jwt;
pub mod middleware;
pub mod email;
pub mod service;

pub use jwt::{Claims, JwtConfig};
pub use middleware::AuthUser;
pub use email::{EmailSender, SmtpConfig};
