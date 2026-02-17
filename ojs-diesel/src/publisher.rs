use crate::outbox::{ojs_outbox, OutboxEntry};
use diesel::prelude::*;
use std::time::Duration;

/// Maximum number of retries before marking an entry as failed.
const DEFAULT_MAX_RETRIES: i32 = 5;

/// Background publisher that polls the outbox table and forwards jobs to OJS.
///
/// Runs as a tokio task, periodically checking for unpublished outbox entries,
/// sending them to the OJS backend, and marking them as published.
///
/// # Example
///
/// ```rust,ignore
/// use ojs::Client;
/// use ojs_diesel::OutboxPublisher;
///
/// let client = Client::builder()
///     .url("http://localhost:8080")
///     .build()
///     .unwrap();
///
/// let publisher = OutboxPublisher::new(client, "postgres://localhost/mydb");
/// publisher.start().await;
/// ```
pub struct OutboxPublisher {
    client: ojs::Client,
    database_url: String,
    poll_interval: Duration,
    batch_size: i64,
    max_retries: i32,
}

impl OutboxPublisher {
    /// Create a new publisher.
    pub fn new(client: ojs::Client, database_url: impl Into<String>) -> Self {
        Self {
            client,
            database_url: database_url.into(),
            poll_interval: Duration::from_secs(1),
            batch_size: 100,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }

    /// Set the polling interval (default: 1 second).
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set the batch size per poll (default: 100).
    pub fn batch_size(mut self, size: i64) -> Self {
        self.batch_size = size;
        self
    }

    /// Set the maximum number of publish retries before marking as failed (default: 5).
    pub fn max_retries(mut self, max: i32) -> Self {
        self.max_retries = max;
        self
    }

    /// Start the publisher as a background tokio task.
    ///
    /// Returns a `JoinHandle` that can be used to await or abort the task.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    async fn run(&self) {
        let mut conn = match diesel::PgConnection::establish(&self.database_url) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to connect to database: {e}");
                return;
            }
        };

        loop {
            match self.poll_and_publish(&mut conn).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::debug!("Published {count} outbox entries");
                    }
                }
                Err(e) => {
                    tracing::error!("Outbox publisher error: {e}");
                }
            }

            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn poll_and_publish(
        &self,
        conn: &mut PgConnection,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let entries: Vec<OutboxEntry> = ojs_outbox::table
            .filter(ojs_outbox::status.eq("pending"))
            .order(ojs_outbox::created_at.asc())
            .limit(self.batch_size)
            .load(conn)?;

        let count = entries.len();

        for entry in entries {
            match self
                .client
                .enqueue(&entry.job_type, entry.args.clone())
                .await
            {
                Ok(_) => {
                    let now = chrono::Utc::now();
                    diesel::update(ojs_outbox::table.find(entry.id))
                        .set((
                            ojs_outbox::status.eq("published"),
                            ojs_outbox::published_at.eq(Some(now)),
                        ))
                        .execute(conn)?;
                }
                Err(e) => {
                    let now = chrono::Utc::now();
                    let new_retry_count = entry.retry_count + 1;
                    let new_status = if new_retry_count >= self.max_retries {
                        "failed"
                    } else {
                        "pending"
                    };

                    diesel::update(ojs_outbox::table.find(entry.id))
                        .set((
                            ojs_outbox::status.eq(new_status),
                            ojs_outbox::error_message.eq(e.to_string()),
                            ojs_outbox::retry_count.eq(new_retry_count),
                            ojs_outbox::last_error_at.eq(Some(now)),
                        ))
                        .execute(conn)?;

                    tracing::warn!(
                        "Failed to publish outbox entry {} (attempt {}/{}): {e}",
                        entry.id,
                        new_retry_count,
                        self.max_retries,
                    );
                }
            }
        }

        Ok(count)
    }
}
