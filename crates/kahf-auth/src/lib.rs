//! Kahf authentication layer.
//!
//! Provides password hashing, JWT token management, and axum middleware
//! for the Kahf platform. Depends on `kahf-db` for user and session
//! storage.
//!
//! ## Modules
//!
//! - `password` — Argon2id password hashing and verification
//! - `jwt` — JWT access/refresh token issuance and verification
//! - `middleware` — Axum `AuthUser` extractor for protected routes
//! - `service` — High-level auth operations: signup, login, refresh, logout

pub mod password;
pub mod jwt;
pub mod middleware;
pub mod service;

pub use jwt::{Claims, JwtConfig};
pub use middleware::AuthUser;
