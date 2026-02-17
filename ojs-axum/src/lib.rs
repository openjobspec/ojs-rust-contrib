//! # ojs-axum
//!
//! Axum state extractor and Tower layer for OJS (Open Job Spec).
//!
//! Provides [`OjsState`] for Axum router state, [`OjsClient`] as an extractor,
//! [`OjsLayer`] as a Tower layer, and [`shutdown_signal`] for graceful shutdown.
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

mod extract;
mod layer;
mod shutdown;

pub use extract::{OjsClient, OjsState};
pub use layer::OjsLayer;
pub use shutdown::shutdown_signal;

/// Re-export core OJS types for convenience.
pub use ojs::Client;
