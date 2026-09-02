use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{watch, Mutex, Notify, RwLock};
use tokio::task::JoinHandle;

/// A boxed, sendable async job handler.
pub type BoxedHandler = Arc<
    dyn Fn(
            JobContext,
        )
            -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>
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

/// Mutable lifecycle state for the current worker generation.
struct LifecycleState {
    next_generation: u64,
    phase: LifecyclePhase,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self {
            next_generation: 1,
            phase: LifecyclePhase::Stopped,
        }
    }
}

/// A task handle exists only in `Running` and has exactly one owner.
enum LifecyclePhase {
    Stopped,
    Running {
        generation: u64,
        shutdown_tx: watch::Sender<bool>,
        task: JoinHandle<()>,
    },
    Stopping {
        generation: u64,
    },
}

/// Cloneable handle to the shared worker [`LifecycleState`].
#[derive(Clone)]
struct WorkerLifecycle {
    inner: Arc<Mutex<LifecycleState>>,
    changed: Arc<Notify>,
}

impl Default for WorkerLifecycle {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LifecycleState::default())),
            changed: Arc::new(Notify::new()),
        }
    }
}

impl WorkerLifecycle {
    /// Spawn a new generation while atomically installing its task handle.
    ///
    /// A naturally-finished prior generation is joined before the new one is
    /// installed. A generation being stopped remains exclusive until its
    /// handle has been joined and [`finish_join`](Self::finish_join) commits
    /// the `Stopping -> Stopped` transition.
    async fn spawn<F, Fut>(&self, task_factory: F) -> Result<u64, WorkerError>
    where
        F: FnOnce(watch::Receiver<bool>) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut task_factory = Some(task_factory);

        loop {
            let prior_task = {
                let mut state = self.inner.lock().await;
                match &state.phase {
                    LifecyclePhase::Stopped => {
                        let generation = state.next_generation;
                        state.next_generation = state.next_generation.wrapping_add(1);

                        let (shutdown_tx, shutdown_rx) = watch::channel(false);
                        let task = tokio::spawn(task_factory
                            .take()
                            .expect("worker task factory used once")(
                            shutdown_rx
                        ));
                        state.phase = LifecyclePhase::Running {
                            generation,
                            shutdown_tx,
                            task,
                        };
                        return Ok(generation);
                    }
                    LifecyclePhase::Running {
                        generation, task, ..
                    } if task.is_finished() => {
                        let generation = *generation;
                        let phase = std::mem::replace(
                            &mut state.phase,
                            LifecyclePhase::Stopping { generation },
                        );
                        let LifecyclePhase::Running { task, .. } = phase else {
                            unreachable!("running phase changed while locked");
                        };
                        Some(WorkerJoin::new(self.clone(), generation, None, task))
                    }
                    LifecyclePhase::Running { .. } | LifecyclePhase::Stopping { .. } => {
                        return Err(WorkerError::AlreadyRunning);
                    }
                }
            };

            prior_task
                .expect("finished running task must be joinable")
                .join()
                .await;
        }
    }

    /// Transfer the running generation's handles to one stop caller.
    async fn begin_stop(&self) -> Option<WorkerJoin> {
        let mut state = self.inner.lock().await;
        let generation = match &state.phase {
            LifecyclePhase::Running { generation, .. } => *generation,
            LifecyclePhase::Stopped | LifecyclePhase::Stopping { .. } => return None,
        };
        let phase = std::mem::replace(&mut state.phase, LifecyclePhase::Stopping { generation });
        let LifecyclePhase::Running {
            shutdown_tx, task, ..
        } = phase
        else {
            unreachable!("running phase changed while locked");
        };

        Some(WorkerJoin::new(
            self.clone(),
            generation,
            Some(shutdown_tx),
            task,
        ))
    }

    /// Whether the worker is currently running.
    async fn is_running(&self) -> bool {
        let state = self.inner.lock().await;
        matches!(
            &state.phase,
            LifecyclePhase::Running { task, .. } if !task.is_finished()
        )
    }

    /// Complete a join only if it still belongs to the stopping generation.
    async fn finish_join(&self, generation: u64) -> bool {
        let mut state = self.inner.lock().await;
        if !matches!(
            state.phase,
            LifecyclePhase::Stopping {
                generation: current
            } if current == generation
        ) {
            return false;
        }

        state.phase = LifecyclePhase::Stopped;
        drop(state);
        self.changed.notify_waiters();
        true
    }

    #[cfg(test)]
    async fn current_generation(&self) -> Option<u64> {
        let state = self.inner.lock().await;
        match state.phase {
            LifecyclePhase::Stopped => None,
            LifecyclePhase::Running { generation, .. }
            | LifecyclePhase::Stopping { generation } => Some(generation),
        }
    }

    #[cfg(test)]
    async fn wait_until_stopped(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if matches!(self.inner.lock().await.phase, LifecyclePhase::Stopped) {
                return;
            }
            changed.await;
        }
    }
}

/// Exclusive ownership of a generation's task while it is being joined.
struct WorkerJoin {
    lifecycle: WorkerLifecycle,
    generation: u64,
    shutdown_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
}

impl WorkerJoin {
    fn new(
        lifecycle: WorkerLifecycle,
        generation: u64,
        shutdown_tx: Option<watch::Sender<bool>>,
        task: JoinHandle<()>,
    ) -> Self {
        Self {
            lifecycle,
            generation,
            shutdown_tx,
            task: Some(task),
        }
    }

    async fn join(mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(true);
        }
        if let Some(task) = self.task.as_mut() {
            let _ = task.await;
        }
        self.lifecycle.finish_join(self.generation).await;
        self.task = None;
    }
}

impl Drop for WorkerJoin {
    fn drop(&mut self) {
        let Some(task) = self.task.take() else {
            return;
        };

        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(true);
        }
        task.abort();

        let lifecycle = self.lifecycle.clone();
        let generation = self.generation;
        tokio::spawn(async move {
            let _ = task.await;
            lifecycle.finish_join(generation).await;
        });
    }
}

async fn run_worker_until_shutdown(
    worker: Arc<ojs::Worker>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    tracing::info!("OJS worker started");
    let start = worker.start();
    tokio::pin!(start);

    let result = tokio::select! {
        result = &mut start => result,
        _ = shutdown_rx.changed() => {
            tracing::info!("OJS worker received shutdown signal");
            worker.shutdown();
            start.await
        }
    };

    if let Err(error) = result {
        tracing::error!(%error, "OJS worker exited with error");
    }
    tracing::info!("OJS worker stopped");
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
    lifecycle: WorkerLifecycle,
}

impl OjsWorkerManager {
    /// Create a new worker manager with the given configuration.
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            lifecycle: WorkerLifecycle::default(),
        }
    }

    /// Register a handler for a job type.
    ///
    /// The handler receives a [`JobContext`] and returns a future that
    /// resolves to `Result<(), Box<dyn Error + Send + Sync>>`.
    pub async fn register<F>(&self, job_type: impl Into<String>, handler: F)
    where
        F: Fn(
                JobContext,
            )
                -> Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send>>
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
    ///
    /// The `stopped -> running` transition is atomic: two concurrent `start`
    /// calls will never both spawn a worker.
    pub async fn start(&self) -> Result<(), WorkerError> {
        let worker = Arc::new(self.build_worker()?);
        self.register_handlers(&worker).await;

        self.lifecycle
            .spawn(move |shutdown_rx| run_worker_until_shutdown(worker, shutdown_rx))
            .await?;

        Ok(())
    }

    /// Build an [`ojs::Worker`] from the current configuration.
    fn build_worker(&self) -> Result<ojs::Worker, WorkerError> {
        Ok(ojs::Worker::builder()
            .url(&self.config.url)
            .queues(self.config.queues.clone())
            .concurrency(self.config.concurrency)
            .poll_interval(Duration::from_millis(self.config.poll_interval_ms))
            .grace_period(Duration::from_secs(self.config.shutdown_timeout_secs))
            .build()?)
    }

    /// Register every locally-registered handler with the [`ojs::Worker`].
    ///
    /// The registry snapshot is cloned under a short-lived read lock, which is
    /// released before the `await`-heavy registration loop so that concurrent
    /// [`register`](Self::register) calls are not blocked. Handlers are
    /// `Arc`-backed, so the snapshot is cheap.
    async fn register_handlers(&self, worker: &ojs::Worker) {
        let handlers: Vec<(String, BoxedHandler)> = {
            let guard = self.handlers.read().await;
            guard
                .iter()
                .map(|(job_type, handler)| (job_type.clone(), handler.clone()))
                .collect()
        };

        for (job_type, handler) in handlers {
            worker
                .register(job_type, move |ctx: ojs::JobContext| {
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
    }

    /// Stop the background worker gracefully.
    ///
    /// Signals the polling loop to shut down and awaits the background task so
    /// that the worker has fully drained before this method returns. Returns
    /// [`WorkerError::NotRunning`] if the worker is not active.
    pub async fn stop(&self) -> Result<(), WorkerError> {
        let task = self
            .lifecycle
            .begin_stop()
            .await
            .ok_or(WorkerError::NotRunning)?;
        task.join().await;
        Ok(())
    }

    /// Check whether the worker is currently running.
    pub async fn is_running(&self) -> bool {
        self.lifecycle.is_running().await
    }

    /// Get a reference to the worker configuration.
    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use ojs::transport::{Method, Transport};
    use serde_json::json;
    use std::future::pending;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::sync::{oneshot, Barrier};

    #[tokio::test]
    async fn concurrent_start_has_one_task_owner() {
        let lifecycle = WorkerLifecycle::default();
        let barrier = Arc::new(Barrier::new(3));

        let start = |lifecycle: WorkerLifecycle, barrier: Arc<Barrier>| {
            tokio::spawn(async move {
                barrier.wait().await;
                lifecycle
                    .spawn(|mut shutdown| async move {
                        let _ = shutdown.changed().await;
                    })
                    .await
            })
        };

        let first = start(lifecycle.clone(), barrier.clone());
        let second = start(lifecycle.clone(), barrier.clone());
        barrier.wait().await;

        let results = [first.await.unwrap(), second.await.unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(WorkerError::AlreadyRunning)))
                .count(),
            1
        );

        lifecycle.begin_stop().await.unwrap().join().await;
    }

    #[tokio::test]
    async fn start_during_stop_is_rejected_until_prior_join() {
        let lifecycle = WorkerLifecycle::default();
        let (shutdown_seen_tx, shutdown_seen_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();

        let first_generation = lifecycle
            .spawn(move |mut shutdown| async move {
                let _ = shutdown.changed().await;
                let _ = shutdown_seen_tx.send(());
                let _ = release_rx.await;
            })
            .await
            .unwrap();

        let stop_task = tokio::spawn(lifecycle.begin_stop().await.unwrap().join());
        shutdown_seen_rx.await.unwrap();
        assert!(!lifecycle.is_running().await);
        assert!(matches!(
            lifecycle.spawn(|_| async {}).await,
            Err(WorkerError::AlreadyRunning)
        ));

        release_tx.send(()).unwrap();
        stop_task.await.unwrap();

        let second_generation = lifecycle.spawn(|_| async {}).await.unwrap();
        assert!(second_generation > first_generation);
        lifecycle.begin_stop().await.unwrap().join().await;
    }

    #[tokio::test]
    async fn stale_cleanup_cannot_clear_a_new_generation() {
        let lifecycle = WorkerLifecycle::default();
        let first_generation = lifecycle
            .spawn(|mut shutdown| async move {
                let _ = shutdown.changed().await;
            })
            .await
            .unwrap();
        lifecycle.begin_stop().await.unwrap().join().await;

        let (release_tx, release_rx) = oneshot::channel();
        let second_generation = lifecycle
            .spawn(move |mut shutdown| async move {
                let _ = shutdown.changed().await;
                let _ = release_rx.await;
            })
            .await
            .unwrap();
        assert!(second_generation > first_generation);

        assert!(!lifecycle.finish_join(first_generation).await);
        assert_eq!(
            lifecycle.current_generation().await,
            Some(second_generation)
        );
        assert!(lifecycle.is_running().await);

        let second_join = lifecycle.begin_stop().await.unwrap();
        assert!(!lifecycle.finish_join(first_generation).await);
        assert_eq!(
            lifecycle.current_generation().await,
            Some(second_generation)
        );
        release_tx.send(()).unwrap();
        second_join.join().await;
    }

    #[tokio::test]
    async fn repeated_stop_does_not_duplicate_handle_ownership() {
        let lifecycle = WorkerLifecycle::default();
        let (release_tx, release_rx) = oneshot::channel();

        lifecycle
            .spawn(move |mut shutdown| async move {
                let _ = shutdown.changed().await;
                let _ = release_rx.await;
            })
            .await
            .unwrap();

        let first_join = lifecycle.begin_stop().await.unwrap();
        assert!(lifecycle.begin_stop().await.is_none());
        release_tx.send(()).unwrap();
        first_join.join().await;
        assert!(lifecycle.begin_stop().await.is_none());
    }

    struct DropSignal(Option<oneshot::Sender<()>>);

    impl Drop for DropSignal {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _ = sender.send(());
            }
        }
    }

    #[tokio::test]
    async fn cancelled_stop_retains_and_reaps_the_task_handle() {
        let lifecycle = WorkerLifecycle::default();
        let (shutdown_seen_tx, shutdown_seen_rx) = oneshot::channel();
        let (worker_dropped_tx, worker_dropped_rx) = oneshot::channel();

        lifecycle
            .spawn(move |mut shutdown| async move {
                let _drop_signal = DropSignal(Some(worker_dropped_tx));
                let _ = shutdown.changed().await;
                let _ = shutdown_seen_tx.send(());
                pending::<()>().await;
            })
            .await
            .unwrap();

        let stop_task = tokio::spawn(lifecycle.begin_stop().await.unwrap().join());
        shutdown_seen_rx.await.unwrap();
        stop_task.abort();
        assert!(stop_task.await.unwrap_err().is_cancelled());

        worker_dropped_rx.await.unwrap();
        lifecycle.wait_until_stopped().await;
        assert!(!lifecycle.is_running().await);

        lifecycle.spawn(|_| async {}).await.unwrap();
        lifecycle.begin_stop().await.unwrap().join().await;
    }

    #[derive(Debug)]
    struct DrainTransport {
        fetched: AtomicBool,
        ack_count: AtomicUsize,
        nack_count: AtomicUsize,
    }

    impl DrainTransport {
        fn new() -> Self {
            Self {
                fetched: AtomicBool::new(false),
                ack_count: AtomicUsize::new(0),
                nack_count: AtomicUsize::new(0),
            }
        }
    }

    impl Transport for DrainTransport {
        fn request(
            &self,
            _method: Method,
            path: &str,
            _body: Option<serde_json::Value>,
            _raw_path: bool,
        ) -> Pin<Box<dyn Future<Output = ojs::Result<Option<serde_json::Value>>> + Send + '_>>
        {
            let response = match path {
                "/workers/fetch" if !self.fetched.swap(true, Ordering::SeqCst) => Some(json!({
                    "jobs": [{
                        "specversion": "1.0",
                        "id": "019b7e37-7a50-7000-8000-000000000001",
                        "type": "slow.job",
                        "queue": "default",
                        "args": [],
                        "state": "active",
                        "attempt": 1,
                        "priority": 0,
                        "tags": []
                    }]
                })),
                "/workers/fetch" => Some(json!({ "jobs": [] })),
                "/workers/heartbeat" => Some(json!({ "state": "running" })),
                "/workers/ack" => {
                    self.ack_count.fetch_add(1, Ordering::SeqCst);
                    None
                }
                "/workers/nack" => {
                    self.nack_count.fetch_add(1, Ordering::SeqCst);
                    None
                }
                other => panic!("unexpected worker path: {other}"),
            };
            Box::pin(async move { Ok(response) })
        }
    }

    #[tokio::test]
    async fn stop_waits_for_active_job_to_drain_without_aborting_it() {
        let transport = Arc::new(DrainTransport::new());
        let worker = Arc::new(
            ojs::Worker::builder()
                .transport(transport.clone())
                .poll_interval(Duration::from_millis(1))
                .heartbeat_interval(Duration::from_secs(60))
                .grace_period(Duration::from_secs(2))
                .build()
                .unwrap(),
        );
        let handler_started = Arc::new(Barrier::new(2));
        let release_handler = Arc::new(Barrier::new(2));
        let handler_completed = Arc::new(AtomicBool::new(false));

        worker
            .register("slow.job", {
                let handler_started = handler_started.clone();
                let release_handler = release_handler.clone();
                let handler_completed = handler_completed.clone();
                move |_| {
                    let handler_started = handler_started.clone();
                    let release_handler = release_handler.clone();
                    let handler_completed = handler_completed.clone();
                    async move {
                        handler_started.wait().await;
                        release_handler.wait().await;
                        handler_completed.store(true, Ordering::SeqCst);
                        Ok(serde_json::Value::Null)
                    }
                }
            })
            .await;

        let lifecycle = WorkerLifecycle::default();
        lifecycle
            .spawn({
                let worker = worker.clone();
                move |shutdown_rx| run_worker_until_shutdown(worker, shutdown_rx)
            })
            .await
            .unwrap();
        handler_started.wait().await;

        let stop = tokio::spawn(lifecycle.begin_stop().await.unwrap().join());
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(
            !stop.is_finished(),
            "stop returned before the handler drained"
        );
        assert!(!handler_completed.load(Ordering::SeqCst));
        assert_eq!(transport.ack_count.load(Ordering::SeqCst), 0);
        assert_eq!(transport.nack_count.load(Ordering::SeqCst), 0);

        release_handler.wait().await;
        tokio::time::timeout(Duration::from_secs(1), stop)
            .await
            .expect("stop must finish after the active handler drains")
            .unwrap();

        assert!(handler_completed.load(Ordering::SeqCst));
        assert_eq!(transport.ack_count.load(Ordering::SeqCst), 1);
        assert_eq!(transport.nack_count.load(Ordering::SeqCst), 0);
    }
}
