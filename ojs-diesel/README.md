# ojs-diesel

Transactional job enqueue via Diesel connection callbacks for the [OJS Rust SDK](https://github.com/openjobspec/ojs-rust-sdk).

Implements the **transactional outbox pattern**: jobs are written to an outbox table within the same database transaction as your domain data, then asynchronously published to the OJS backend.

## Installation

```toml
[dependencies]
ojs-diesel = "0.1"
```

## Quick Start

```rust,ignore
use diesel::prelude::*;
use ojs::Client;
use ojs_diesel::{OutboxEntry, enqueue_to_outbox, OutboxPublisher};
use serde_json::json;

// Within a Diesel transaction, insert a job into the outbox
fn create_order(conn: &mut PgConnection, order_id: i64) -> QueryResult<()> {
    conn.transaction(|conn| {
        // ... insert your order ...

        enqueue_to_outbox(conn, "order.process", json!({"order_id": order_id}))?;
        Ok(())
    })
}

// Start the background publisher
#[tokio::main]
async fn main() {
    let client = Client::builder()
        .url("http://localhost:8080")
        .build()
        .unwrap();

    let publisher = OutboxPublisher::new(client, /* db_url */ "postgres://...");
    publisher.start().await;
}
```

## How It Works

1. **`enqueue_to_outbox()`** inserts a row into the `ojs_outbox` table within your existing Diesel transaction.
2. **`OutboxPublisher`** runs as a background tokio task, polling the outbox table for pending entries.
3. Published entries are marked with `status = 'published'`. Failed entries are retried up to a configurable maximum.

This guarantees that job enqueue and your domain write succeed or fail atomically.

## Database Setup

Create the outbox table in your PostgreSQL database:

```sql
CREATE TABLE ojs_outbox (
    id UUID PRIMARY KEY,
    job_type TEXT NOT NULL,
    args JSONB NOT NULL,
    queue TEXT,
    priority INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    published_at TIMESTAMPTZ,
    error_message TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error_at TIMESTAMPTZ
);

CREATE INDEX idx_ojs_outbox_pending ON ojs_outbox (status, created_at) WHERE status = 'pending';
CREATE INDEX idx_ojs_outbox_cleanup ON ojs_outbox (published_at) WHERE published_at IS NOT NULL;
```

## Examples

See the [`examples/`](./examples/) directory for a complete runnable project with Docker Compose.

## License

Apache 2.0

