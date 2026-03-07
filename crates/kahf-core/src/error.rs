//! Domain error types for the KahfLane platform.
//!
//! `KahfError` covers the standard failure modes across all crates.
//! It integrates with `eyre::Report` for rich error context via the
//! `.wrap_err()` and `?` operator chains.
//!
//! ## Variants
//!
//! - `NotFound { entity, id }` — requested resource does not exist
//! - `Unauthorized` — caller is not authenticated
//! - `Forbidden(String)` — caller lacks permission for the requested action
//! - `Validation(String)` — input failed validation
//! - `Conflict(String)` — concurrent modification conflict
//! - `Internal(String)` — unexpected internal failure
//!
//! ## Convenience Constructors
//!
//! Each variant has a static method that returns `eyre::Report`:
//! `KahfError::not_found("task", id)`, `KahfError::validation("bad input")`,
//! `KahfError::unauthorized()`, `KahfError::forbidden("reason")`, etc.

use std::fmt;

#[derive(Debug)]
pub enum KahfError {
    NotFound { entity: String, id: String },
    Unauthorized,
    Forbidden(String),
    Validation(String),
    Conflict(String),
    Internal(String),
}

impl fmt::Display for KahfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { entity, id } => write!(f, "{entity} not found: {id}"),
            Self::Unauthorized => write!(f, "unauthorized"),
            Self::Forbidden(msg) => write!(f, "forbidden: {msg}"),
            Self::Validation(msg) => write!(f, "validation error: {msg}"),
            Self::Conflict(msg) => write!(f, "conflict: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for KahfError {}

impl KahfError {
    pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> eyre::Report {
        eyre::Report::new(Self::NotFound {
            entity: entity.into(),
            id: id.into(),
        })
    }

    pub fn validation(msg: impl Into<String>) -> eyre::Report {
        eyre::Report::new(Self::Validation(msg.into()))
    }

    pub fn conflict(msg: impl Into<String>) -> eyre::Report {
        eyre::Report::new(Self::Conflict(msg.into()))
    }

    pub fn internal(msg: impl Into<String>) -> eyre::Report {
        eyre::Report::new(Self::Internal(msg.into()))
    }

    pub fn unauthorized() -> eyre::Report {
        eyre::Report::new(Self::Unauthorized)
    }

    pub fn forbidden(msg: impl Into<String>) -> eyre::Report {
        eyre::Report::new(Self::Forbidden(msg.into()))
    }
}
