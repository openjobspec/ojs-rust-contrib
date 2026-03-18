//! # ojs-actix
//!
//! Actix-web middleware and app data integration for OJS (Open Job Spec).
//!
//! Provides [`OjsMiddleware`] to inject an [`ojs::Client`] into Actix-web's
//! application data, and [`OjsClient`] as an extractor for handler functions.
//!
//! ## Quick Start — Middleware
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
//!
//! ## Quick Start — App Data
//!
//! ```rust,no_run
//! use actix_web::{web, App, HttpServer, HttpResponse};
//! use ojs::Client;
//! use ojs_actix::{OjsAppData, configure_ojs};
//!
//! async fn handler(ojs: web::Data<OjsAppData>) -> HttpResponse {
//!     HttpResponse::Ok().finish()
//! }
//!
//! # #[actix_web::main]
//! # async fn main() -> std::io::Result<()> {
//! let client = Client::builder().url("http://localhost:8080").build().unwrap();
//!
//! HttpServer::new(move || {
//!     App::new()
//!         .configure(configure_ojs(client.clone()))
//!         .route("/", web::get().to(handler))
//! })
//! .bind("0.0.0.0:3000")?
//! .run()
//! .await
//! # }
//! ```

mod app_data;
mod extractor;
mod health;
mod middleware;
mod worker;

pub use app_data::{configure_ojs, OjsAppData};
pub use extractor::OjsClient;
pub use health::health_handler;
pub use middleware::OjsMiddleware;
pub use worker::{BoxedHandler, JobContext, OjsWorkerManager, WorkerConfig, WorkerError};

/// Re-export core OJS types for convenience.
pub use ojs::Client;
