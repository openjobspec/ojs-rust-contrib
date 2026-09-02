# ojs-actix

Actix-web middleware and app data integration for the [OJS Rust SDK](https://github.com/openjobspec/ojs-rust-sdk).

## Security-blocked release

`ojs-actix` 0.5.0 is intentionally not published to crates.io. A normal Actix
application commonly enables Actix-web's default HTTP/2 feature, which selects
unpatched `h2` 0.3 and triggers RUSTSEC-2026-0258. Disabling HTTP/2 only in this
integration does not protect consumers whose application dependency enables
Actix defaults through Cargo feature unification.

No released Actix dependency path using patched `h2` 0.4 is currently
available. Source and tests remain maintained, but publication is blocked
until [actix/actix-web#4199](https://github.com/actix/actix-web/issues/4199)
is resolved. Version 0.5.0 requires Rust 1.88 and Actix-web 4.15 or newer.

## Quick Start

```rust,no_run
use actix_web::{web, App, HttpServer, HttpResponse};
use ojs::Client;
use ojs_actix::{OjsMiddleware, OjsClient};
use serde_json::json;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client = Client::builder()
        .url("http://localhost:8080")
        .build()
        .expect("failed to build OJS client");

    HttpServer::new(move || {
        App::new()
            .wrap(OjsMiddleware::new(client.clone()))
            .route("/enqueue", web::post().to(enqueue_job))
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}

async fn enqueue_job(ojs: OjsClient) -> HttpResponse {
    let job = ojs.enqueue("email.send", json!({"to": "user@example.com"})).await;
    match job {
        Ok(job) => HttpResponse::Ok().json(serde_json::json!({"job_id": job.id.to_string()})),
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}
```

## API

### `OjsMiddleware`

Actix-web middleware that injects an `ojs::Client` into application data.

### `OjsClient`

Extractor that retrieves the `ojs::Client` from the request's app data. Use it as a handler parameter.

## Examples

See the [`examples/`](./examples/) directory for a complete runnable project with Docker Compose.

## License

Apache 2.0
