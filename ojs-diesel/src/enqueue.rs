use crate::outbox::{ojs_outbox, NewOutboxEntry};
use diesel::prelude::*;
use uuid::Uuid;

/// Options for enqueuing a job to the outbox.
#[derive(Debug, Clone, Default)]
pub struct EnqueueOptions {
    /// Target queue name (uses server default if `None`).
    pub queue: Option<String>,
    /// Job priority (default: 0).
    pub priority: i32,
}

/// Insert a job into the outbox table within the current transaction.
///
/// Call this inside a `conn.transaction(|conn| { ... })` block to ensure
/// the job is committed atomically with your domain data.
///
/// # Arguments
///
/// * `conn` - A mutable reference to a Diesel `PgConnection`
/// * `job_type` - The OJS job type (e.g., `"email.send"`)
/// * `args` - Job arguments as a JSON value
///
/// # Example
///
/// ```rust,ignore
/// use diesel::prelude::*;
/// use ojs_diesel::enqueue_to_outbox;
/// use serde_json::json;
///
/// fn do_work(conn: &mut PgConnection) -> QueryResult<()> {
///     conn.transaction(|conn| {
///         // ... your domain writes ...
///         enqueue_to_outbox(conn, "report.generate", json!({"id": 42}))?;
///         Ok(())
///     })
/// }
/// ```
pub fn enqueue_to_outbox(
    conn: &mut PgConnection,
    job_type: &str,
    args: serde_json::Value,
) -> QueryResult<()> {
    enqueue_to_outbox_with_options(conn, job_type, args, EnqueueOptions::default())
}

/// Insert a job into the outbox table with additional options.
///
/// # Example
///
/// ```rust,ignore
/// use diesel::prelude::*;
/// use ojs_diesel::{enqueue_to_outbox_with_options, EnqueueOptions};
/// use serde_json::json;
///
/// fn do_work(conn: &mut PgConnection) -> QueryResult<()> {
///     conn.transaction(|conn| {
///         enqueue_to_outbox_with_options(
///             conn,
///             "report.generate",
///             json!({"id": 42}),
///             EnqueueOptions { queue: Some("reports".into()), priority: 10 },
///         )?;
///         Ok(())
///     })
/// }
/// ```
pub fn enqueue_to_outbox_with_options(
    conn: &mut PgConnection,
    job_type: &str,
    args: serde_json::Value,
    options: EnqueueOptions,
) -> QueryResult<()> {
    let entry = NewOutboxEntry {
        id: Uuid::now_v7(),
        job_type: job_type.to_string(),
        args,
        queue: options.queue,
        priority: options.priority,
        status: "pending".to_string(),
        created_at: chrono::Utc::now(),
    };

    diesel::insert_into(ojs_outbox::table)
        .values(&entry)
        .execute(conn)?;

    Ok(())
}
