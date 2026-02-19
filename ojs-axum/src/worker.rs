//! Worker lifecycle management for Axum applications.
//!
//! Provides [`OjsWorkerManager`] for registering job handlers and running
//! an OJS worker alongside an Axum server.
//!
//! ## Example
//!
//! ```rust,no_run
//! use ojs_axum::worker::{OjsWorkerManager, WorkerConfig};
//!
//! let mut manager = OjsWorkerManager::new(WorkerConfig {
//!     url: "http://localhost:8080".into(),
//!     queues: vec!["default".into(), "emails".into()],
//!     concurrency: 10,
//!     ..Default::default()
//! });
//!
//! manager.register("email.send", |ctx| Box::pin(async move {
//!     println!("Sending email: {:?}", ctx.args);
//!     Ok(())
//! }));
//!
//! // Start worker in background alongside Axum server
//! tokio::spawn(async move { manager.start().await });
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Configuration for the OJS worker.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// URL of the OJS server.
    pub url: String,
    /// Queues to process.
    pub queues: Vec<String>,
    /// Number of concurrent job processors.
    pub concurrency: usize,
    /// Poll interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Graceful shutdown timeout in seconds.
    pub shutdown_timeout_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".into(),
            queues: vec!["default".into()],
            concurrency: 10,
            poll_interval_ms: 1000,
            shutdown_timeout_secs: 30,
        }
    }
}

/// Context provided to job handlers.
#[derive(Debug, Clone)]
pub struct JobContext {
    /// Unique job ID.
    pub id: String,
    /// Job type.
    pub job_type: String,
    /// Job arguments.
    pub args: serde_json::Value,
    /// Current attempt number.
    pub attempt: u32,
    /// Queue this job was dequeued from.
    pub queue: String,
}

/// Type alias for boxed async job handler functions.
pub type BoxedHandler = Arc<
    dyn Fn(JobContext) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Manages OJS worker lifecycle with handler registration.
pub struct OjsWorkerManager {
    config: WorkerConfig,
    handlers: HashMap<String, BoxedHandler>,
}

impl OjsWorkerManager {
    /// Create a new worker manager with the given configuration.
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            handlers: HashMap::new(),
        }
    }

    /// Register a handler function for a specific job type.
    pub fn register<F, Fut>(&mut self, job_type: &str, handler: F)
    where
        F: Fn(JobContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'static,
    {
        let handler = Arc::new(move |ctx: JobContext| {
            let fut = handler(ctx);
            Box::pin(fut) as Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>>
        });
        self.handlers.insert(job_type.to_string(), handler);
    }

    /// Returns the list of registered job types.
    pub fn registered_types(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }

    /// Start the worker. This is a blocking async call that runs until
    /// the cancellation token is triggered or an error occurs.
    ///
    /// In practice, this would integrate with `ojs::Worker` to poll for
    /// jobs and dispatch to registered handlers.
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This would integrate with the OJS Rust SDK worker.
        // For now, we provide the registration framework.
        tracing::info!(
            queues = ?self.config.queues,
            concurrency = self.config.concurrency,
            handlers = ?self.registered_types(),
            "OJS worker starting"
        );

        // Worker polling loop would go here, delegating to ojs::Worker
        tokio::signal::ctrl_c().await?;
        tracing::info!("OJS worker shutting down");
        Ok(())
    }

    /// Stop the worker gracefully.
    pub async fn stop(&self) {
        tracing::info!("OJS worker stop requested");
    }
}
