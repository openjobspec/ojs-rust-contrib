use actix_web::{web, App, HttpResponse, HttpServer};
use ojs::Client;
use ojs_actix::{OjsClient, OjsMiddleware};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct EnqueueRequest {
    job_type: String,
    to: String,
}

async fn enqueue_job(ojs: OjsClient, body: web::Json<EnqueueRequest>) -> HttpResponse {
    match ojs.enqueue(&body.job_type, json!({"to": &body.to})).await {
        Ok(job) => HttpResponse::Ok().json(json!({"job_id": job.id.to_string()})),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::builder()
        .url("http://localhost:8080")
        .build()
        .expect("failed to build OJS client");

    println!("Starting server on http://0.0.0.0:3000");

    HttpServer::new(move || {
        App::new()
            .wrap(OjsMiddleware::new(client.clone()))
            .route("/health", web::get().to(health))
            .route("/enqueue", web::post().to(enqueue_job))
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}
