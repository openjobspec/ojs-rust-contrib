use actix_web::HttpResponse;
use serde::Serialize;

use crate::OjsClient;

/// Health status for the overall service.
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    ojs: OjsHealthDetail,
}

/// OJS backend health detail.
#[derive(Debug, Serialize)]
struct OjsHealthDetail {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Actix-web handler that checks OJS backend connectivity.
///
/// Returns JSON indicating the health of the OJS backend. Mount this on any
/// route to provide liveness/readiness probes for load balancers or Kubernetes.
///
/// # Responses
///
/// * **200 OK** – Backend is reachable.
///   ```json
///   { "status": "healthy", "ojs": { "status": "ok" } }
///   ```
/// * **503 Service Unavailable** – Backend is unreachable.
///   ```json
///   { "status": "unhealthy", "ojs": { "status": "error", "error": "..." } }
///   ```
///
/// # Example
///
/// ```rust,no_run
/// use actix_web::{web, App, HttpServer};
/// use ojs::Client;
/// use ojs_actix::{OjsMiddleware, health_handler};
///
/// # #[actix_web::main]
/// # async fn main() -> std::io::Result<()> {
/// let client = Client::builder()
///     .url("http://localhost:8080")
///     .build()
///     .unwrap();
///
/// HttpServer::new(move || {
///     App::new()
///         .wrap(OjsMiddleware::new(client.clone()))
///         .route("/health", web::get().to(health_handler))
/// })
/// .bind("0.0.0.0:3000")?
/// .run()
/// .await
/// # }
/// ```
pub async fn health_handler(ojs: OjsClient) -> HttpResponse {
    match ojs.health().await {
        Ok(_) => {
            tracing::debug!("OJS health check passed");
            HttpResponse::Ok().json(HealthResponse {
                status: "healthy",
                ojs: OjsHealthDetail {
                    status: "ok",
                    error: None,
                },
            })
        }
        Err(e) => {
            tracing::warn!(error = %e, "OJS health check failed");
            HttpResponse::ServiceUnavailable().json(HealthResponse {
                status: "unhealthy",
                ojs: OjsHealthDetail {
                    status: "error",
                    error: Some(e.to_string()),
                },
            })
        }
    }
}
