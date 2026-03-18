//! # ojs-axum
//!
//! Axum state extractor and Tower layer for OJS (Open Job Spec).
//!
//! Provides [`OjsState`] for Axum router state, [`OjsClient`] as an extractor,
//! [`OjsLayer`] / [`OjsTraceLayer`] as Tower layers, [`shutdown_signal`] for
//! graceful shutdown, a health check endpoint, event subscription helpers, and
//! a cron scheduling bridge.
//!
//! ## Example
//!
//! ```rust,no_run
//! use axum::{routing::post, Router};
//! use ojs::Client;
//! use ojs_axum::{OjsState, OjsClient};
//! use serde_json::json;
//!
//! async fn enqueue(ojs: OjsClient) -> String {
//!     match ojs.enqueue("email.send", json!({"to": "a@b.com"})).await {
//!         Ok(j) => format!("Enqueued: {}", j.id),
//!         Err(e) => format!("Error: {e}"),
//!     }
//! }
//! ```

pub mod cron;
pub mod error;
pub mod events;
mod extract;
pub mod health;
mod layer;
mod shutdown;
pub mod worker;

pub use extract::{OjsClient, OjsState};
pub use layer::{OjsLayer, OjsTraceLayer};
pub use shutdown::shutdown_signal;
pub use worker::{OjsWorkerManager, WorkerConfig, JobContext};

// Re-export commonly-used types from sub-modules for convenience.
pub use cron::{CronConfig, OjsCronBridge};
pub use error::OjsAxumError;
pub use events::{EventConfig, OjsEventSubscriber, OjsEventType};
pub use health::{HealthResponse, health_handler, health_router};

/// Re-export core OJS types for convenience.
pub use ojs::Client;
