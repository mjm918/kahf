//! Maps `eyre::Report` errors into axum HTTP responses.
//!
//! ## AppError
//!
//! Wrapper around `eyre::Report` that implements `IntoResponse`.
//! Inspects the error chain for `KahfError` variants and returns the
//! appropriate HTTP status code. Falls back to 500 Internal Server
//! Error for unrecognized errors. Response body is always JSON with
//! an `error` field containing the display message.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use kahf_core::KahfError;

pub struct AppError(pub eyre::Report);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0.downcast_ref::<KahfError>() {
            Some(KahfError::NotFound { entity, id }) => {
                (StatusCode::NOT_FOUND, format!("{entity} not found: {id}"))
            }
            Some(KahfError::Unauthorized) => {
                (StatusCode::UNAUTHORIZED, "unauthorized".into())
            }
            Some(KahfError::Forbidden(msg)) => {
                (StatusCode::FORBIDDEN, msg.clone())
            }
            Some(KahfError::Validation(msg)) => {
                (StatusCode::BAD_REQUEST, msg.clone())
            }
            Some(KahfError::Conflict(msg)) => {
                (StatusCode::CONFLICT, msg.clone())
            }
            Some(KahfError::Internal(msg)) => {
                tracing::error!("internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
            None => {
                tracing::error!("unhandled error: {:?}", self.0);
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };

        let body = serde_json::json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<eyre::Report>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
