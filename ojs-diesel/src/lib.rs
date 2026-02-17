//! # ojs-diesel
//!
//! Transactional job enqueue via Diesel connection callbacks for OJS.
//!
//! Implements the transactional outbox pattern: jobs are written to an outbox
//! table within the same database transaction as your domain data, then
//! asynchronously published to the OJS backend by [`OutboxPublisher`].
//!
//! ## Example
//!
//! ```rust,ignore
//! use diesel::prelude::*;
//! use ojs_diesel::{enqueue_to_outbox, OutboxPublisher};
//! use serde_json::json;
//!
//! fn create_order(conn: &mut PgConnection, order_id: i64) -> QueryResult<()> {
//!     conn.transaction(|conn| {
//!         // ... insert order ...
//!         enqueue_to_outbox(conn, "order.process", json!({"order_id": order_id}))?;
//!         Ok(())
//!     })
//! }
//! ```

mod enqueue;
mod outbox;
mod publisher;

pub use enqueue::{enqueue_to_outbox, enqueue_to_outbox_with_options, EnqueueOptions};
pub use outbox::{NewOutboxEntry, OjsOutbox, OutboxEntry};
pub use publisher::OutboxPublisher;

/// Re-export core OJS types for convenience.
pub use ojs::Client;
