//! Error types for the ojs-axum crate.

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

/// Errors produced by ojs-axum operations.
#[derive(Debug, thiserror::Error)]
pub enum OjsAxumError {
    /// An error from the underlying OJS client.
    #[error("ojs error: {0}")]
    Ojs(#[from] ojs::OjsError),

    /// A request validation error.
    #[error("validation error: {0}")]
    Validation(String),
}

impl IntoResponse for OjsAxumError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match &self {
            Self::Ojs(e) => (StatusCode::BAD_GATEWAY, e.to_string()),
            Self::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
        };

        let body = json!({ "error": message });
        (status, Json(body)).into_response()
    }
}
