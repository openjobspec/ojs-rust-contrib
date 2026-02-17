# ojs-axum

Axum state extractor and Tower layer for the [OJS Rust SDK](https://github.com/openjobspec/ojs-rust-sdk).

## Installation

```toml
[dependencies]
ojs-axum = "0.1"
```

## Quick Start

```rust,no_run
use axum::{routing::post, Router};
use ojs::Client;
use ojs_axum::{OjsState, shutdown_signal};
use serde_json::json;

#[tokio::main]
async fn main() {
    let client = Client::builder()
        .url("http://localhost:8080")
        .build()
        .expect("failed to build OJS client");

    let app = Router::new()
        .route("/enqueue", post(enqueue_job))
        .with_state(OjsState::new(client));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn enqueue_job(
    ojs: ojs_axum::OjsClient,
) -> String {
    match ojs.enqueue("email.send", json!({"to": "user@example.com"})).await {
        Ok(job) => format!("Enqueued: {}", job.id),
        Err(e) => format!("Error: {e}"),
    }
}
```

## API

### `OjsState`

A wrapper around `ojs::Client` that implements `Clone` and can be used as Axum router state.

### `OjsClient`

Axum extractor that pulls the `ojs::Client` from the shared state.

### `OjsLayer`

Tower layer that injects the OJS client into request extensions.

### `shutdown_signal()`

Utility function that listens for `SIGINT`/`SIGTERM` and returns a future suitable for `with_graceful_shutdown`.

## Examples

See the [`examples/`](./examples/) directory for a complete runnable project with Docker Compose.

## License

Apache 2.0
