//! # ojs-actix
//!
//! Actix-web middleware and app data integration for OJS (Open Job Spec).
//!
//! Provides [`OjsMiddleware`] to inject an [`ojs::Client`] into Actix-web's
//! application data, and [`OjsClient`] as an extractor for handler functions.
//!
//! ## Example
//!
//! ```rust,no_run
//! use actix_web::{web, App, HttpServer, HttpResponse};
//! use ojs::Client;
//! use ojs_actix::{OjsMiddleware, OjsClient};
//! use serde_json::json;
//!
//! async fn enqueue_job(ojs: OjsClient) -> HttpResponse {
//!     let job = ojs.enqueue("email.send", json!({"to": "user@example.com"})).await;
//!     match job {
//!         Ok(j) => HttpResponse::Ok().json(serde_json::json!({"job_id": j.id.to_string()})),
//!         Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
//!     }
//! }
//! ```

mod extractor;
mod middleware;

pub use extractor::OjsClient;
pub use middleware::OjsMiddleware;

/// Re-export core OJS types for convenience.
pub use ojs::Client;
