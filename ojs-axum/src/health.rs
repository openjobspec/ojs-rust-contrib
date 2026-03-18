//! Health check handler for Axum applications using OJS.
//!
//! Provides a ready-made health endpoint that checks OJS backend connectivity
//! and returns structured JSON responses.
//!
//! ## Example
//!
//! ```rust,no_run
//! use axum::Router;
//! use ojs::Client;
//! use ojs_axum::{OjsState, health::health_router};
//!
//! let client = Client::builder()
//!     .url("http://localhost:8080")
//!     .build()
//!     .unwrap();
//!
//! let app: Router = Router::new()
//!     .merge(health_router())
//!     .with_state(OjsState::new(client));
//! ```

use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;

use crate::{OjsClient, OjsState};

/// Top-level health response.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Overall status: `"healthy"` or `"degraded"`.
    pub status: String,
    /// OJS backend connectivity status.
    pub ojs: OjsHealth,
}

/// OJS backend health detail.
#[derive(Debug, Clone, Serialize)]
pub struct OjsHealth {
    /// Backend status: `"ok"` or `"error"`.
    pub status: String,
    /// Error message when the backend is unreachable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Axum handler that checks OJS backend connectivity and returns health JSON.
///
/// Returns HTTP 200 with `{ "status": "healthy", "ojs": { "status": "ok" } }`
/// when the backend is reachable, or HTTP 503 with a degraded response when it
/// is not.
///
/// ## Example
///
/// ```rust,no_run
/// use axum::{routing::get, Router};
/// use ojs_axum::{OjsState, health::health_handler};
///
/// let app: Router = Router::new()
///     .route("/health", get(health_handler))
///     .with_state(OjsState::new(
///         ojs::Client::builder().url("http://localhost:8080").build().unwrap(),
///     ));
/// ```
pub async fn health_handler(ojs: OjsClient) -> impl IntoResponse {
    match ojs.health().await {
        Ok(_) => {
            let body = HealthResponse {
                status: "healthy".into(),
                ojs: OjsHealth {
                    status: "ok".into(),
                    error: None,
                },
            };
            (StatusCode::OK, Json(body))
        }
        Err(e) => {
            tracing::warn!(error = %e, "OJS health check failed");
            let body = HealthResponse {
                status: "degraded".into(),
                ojs: OjsHealth {
                    status: "error".into(),
                    error: Some(e.to_string()),
                },
            };
            (StatusCode::SERVICE_UNAVAILABLE, Json(body))
        }
    }
}

/// Returns a [`Router<OjsState>`] with a `GET /health` endpoint.
///
/// Merge this into your application router:
///
/// ```rust,no_run
/// use axum::Router;
/// use ojs_axum::{OjsState, health::health_router};
///
/// let app: Router = Router::new()
///     .merge(health_router())
///     .with_state(OjsState::new(
///         ojs::Client::builder().url("http://localhost:8080").build().unwrap(),
///     ));
/// ```
pub fn health_router() -> Router<OjsState> {
    Router::new().route("/health", get(health_handler))
}
