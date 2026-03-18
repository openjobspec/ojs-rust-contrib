use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

/// A boxed, sendable async job handler.
pub type BoxedHandler = Arc<
    dyn Fn(JobContext) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Context passed to job handler functions registered with [`OjsWorkerManager`].
///
/// Contains the job type, raw JSON arguments, and attempt metadata.
#[derive(Debug, Clone)]
pub struct JobContext {
    /// The job type identifier (e.g. `"email.send"`).
    pub job_type: String,
    /// Raw JSON arguments from the job envelope.
    pub args: serde_json::Value,
    /// Current attempt number (1-indexed).
    pub attempt: u32,
}

/// Configuration for the OJS worker.
///
/// # Defaults
///
/// | Field | Default |
/// |-------|---------|
/// | `url` | `"http://localhost:8080"` |
/// | `queues` | `["default"]` |
/// | `concurrency` | `10` |
/// | `poll_interval_ms` | `1000` |
/// | `shutdown_timeout_secs` | `25` |
///
/// # Example
///
/// ```rust
/// use ojs_actix::WorkerConfig;
///
/// let config = WorkerConfig {
///     url: "http://ojs-server:8080".to_string(),
///     queues: vec!["critical".to_string(), "default".to_string()],
///     concurrency: 20,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// OJS server URL.
    pub url: String,
    /// Queues to poll, in priority order (left = highest).
    pub queues: Vec<String>,
    /// Maximum number of concurrent jobs.
    pub concurrency: usize,
    /// Polling interval in milliseconds.
    pub poll_interval_ms: u64,
    /// Graceful shutdown timeout in seconds.
    pub shutdown_timeout_secs: u64,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".to_string(),
            queues: vec!["default".to_string()],
            concurrency: 10,
            poll_interval_ms: 1000,
            shutdown_timeout_secs: 25,
        }
    }
}

/// Error types for worker operations.
#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    /// The worker is already running.
    #[error("worker is already running")]
    AlreadyRunning,
    /// The worker is not running.
    #[error("worker is not running")]
    NotRunning,
    /// An OJS SDK error occurred.
    #[error("ojs error: {0}")]
    Ojs(#[from] ojs::OjsError),
}

/// Manages an OJS worker lifecycle within an actix-web application.
///
/// Register job handlers, then call [`start()`](Self::start) to begin
/// processing in a background tokio task. The worker drains gracefully
/// on [`stop()`](Self::stop).
///
/// Store this as actix-web app data so handlers and middleware can inspect
/// worker state.
///
/// # Example
///
/// ```rust,no_run
/// use ojs_actix::{OjsWorkerManager, WorkerConfig, JobContext};
/// use std::sync::Arc;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let manager = OjsWorkerManager::new(WorkerConfig::default());
///
/// manager.register("email.send", |ctx: JobContext| {
///     Box::pin(async move {
///         println!("processing {}", ctx.job_type);
///         Ok(())
///     })
/// }).await;
///
/// manager.start().await?;
/// // ... server runs ...
/// manager.stop().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct OjsWorkerManager {
    config: WorkerConfig,
    handlers: Arc<RwLock<HashMap<String, BoxedHandler>>>,
    running: Arc<RwLock<bool>>,
    shutdown_tx: Arc<RwLock<Option<tokio::sync::watch::Sender<bool>>>>,
}

impl OjsWorkerManager {
    /// Create a new worker manager with the given configuration.
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Register a handler for a job type.
    ///
    /// The handler receives a [`JobContext`] and returns a future that
    /// resolves to `Result<(), Box<dyn Error + Send + Sync>>`.
    pub async fn register<F>(&self, job_type: impl Into<String>, handler: F)
    where
        F: Fn(JobContext) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>
            + Send
            + Sync
            + 'static,
    {
        let mut handlers = self.handlers.write().await;
        handlers.insert(job_type.into(), Arc::new(handler));
    }

    /// Return a sorted list of registered job type names.
    pub async fn registered_types(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        let mut types: Vec<String> = handlers.keys().cloned().collect();
        types.sort();
        types
    }

    /// Start the worker in a background tokio task.
    ///
    /// Builds an [`ojs::Worker`], registers all handlers, and spawns the
    /// polling loop. Returns [`WorkerError::AlreadyRunning`] if the worker
    /// is already active.
    pub async fn start(&self) -> Result<(), WorkerError> {
        {
            let running = self.running.read().await;
            if *running {
                return Err(WorkerError::AlreadyRunning);
            }
        }

        let worker = ojs::Worker::builder()
            .url(&self.config.url)
            .queues(self.config.queues.clone())
            .concurrency(self.config.concurrency)
            .poll_interval(Duration::from_millis(self.config.poll_interval_ms))
            .grace_period(Duration::from_secs(self.config.shutdown_timeout_secs))
            .build()?;

        // Register all handlers with the ojs::Worker.
        let handlers = self.handlers.read().await;
        for (job_type, handler) in handlers.iter() {
            let handler = handler.clone();
            worker
                .register(job_type.clone(), move |ctx: ojs::JobContext| {
                    let handler = handler.clone();
                    async move {
                        let local_ctx = JobContext {
                            job_type: ctx.job.job_type.clone(),
                            args: ctx.job.args.clone(),
                            attempt: ctx.attempt,
                        };
                        handler(local_ctx)
                            .await
                            .map_err(|e| ojs::OjsError::Handler(e.to_string()))?;
                        Ok(serde_json::Value::Null)
                    }
                })
                .await;
        }

        let (tx, mut rx) = tokio::sync::watch::channel(false);
        {
            let mut shutdown_tx = self.shutdown_tx.write().await;
            *shutdown_tx = Some(tx);
        }

        let running = self.running.clone();
        {
            let mut r = running.write().await;
            *r = true;
        }

        let running_flag = running.clone();
        tokio::spawn(async move {
            tracing::info!("OJS worker started");
            tokio::select! {
                res = worker.start() => {
                    if let Err(e) = res {
                        tracing::error!(error = %e, "OJS worker exited with error");
                    }
                }
                _ = rx.changed() => {
                    tracing::info!("OJS worker received shutdown signal");
                }
            }
            let mut r = running_flag.write().await;
            *r = false;
            tracing::info!("OJS worker stopped");
        });

        Ok(())
    }

    /// Stop the background worker gracefully.
    ///
    /// Sends a shutdown signal and waits briefly for the background task to
    /// acknowledge. Returns [`WorkerError::NotRunning`] if the worker is
    /// not active.
    pub async fn stop(&self) -> Result<(), WorkerError> {
        let running = self.running.read().await;
        if !*running {
            return Err(WorkerError::NotRunning);
        }
        drop(running);

        let mut tx_guard = self.shutdown_tx.write().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(true);
        }

        // Give the background task a moment to clean up.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut r = self.running.write().await;
        *r = false;

        Ok(())
    }

    /// Check whether the worker is currently running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get a reference to the worker configuration.
    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }
}
