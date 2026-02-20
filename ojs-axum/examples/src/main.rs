use axum::{routing::post, Json, Router};
use ojs::Client;
use ojs_axum::{shutdown_signal, OjsClient, OjsState};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct EnqueueRequest {
    job_type: String,
    to: String,
}

async fn enqueue_job(ojs: OjsClient, Json(body): Json<EnqueueRequest>) -> String {
    match ojs.enqueue(&body.job_type, json!({"to": &body.to})).await {
        Ok(job) => format!("Enqueued job: {}", job.id),
        Err(e) => format!("Error: {e}"),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let client = Client::builder()
        .url("http://localhost:8080")
        .build()
        .expect("failed to build OJS client");

    let app = Router::new()
        .route("/enqueue", post(enqueue_job))
        .with_state(OjsState::new(client));

    println!("Starting server on http://0.0.0.0:3000");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

