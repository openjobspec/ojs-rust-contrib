//! Cron scheduling bridge for Axum applications using OJS.
//!
//! Provides [`OjsCronBridge`] for registering, listing, and removing cron
//! schedules via the OJS backend, plus a [`cron_router`] with CRUD endpoints.
//!
//! ## Example
//!
//! ```rust,no_run
//! use ojs_axum::cron::{CronConfig, OjsCronBridge};
//!
//! # async fn example() {
//! let client = ojs::Client::builder()
//!     .url("http://localhost:8080")
//!     .build()
//!     .unwrap();
//!
//! let bridge = OjsCronBridge::new(client);
//!
//! let config = CronConfig {
//!     name: "hourly-cleanup".into(),
//!     schedule: "0 * * * *".into(),
//!     job_type: "cleanup".into(),
//!     ..Default::default()
//! };
//!
//! bridge.register(config).await.unwrap();
//! let jobs = bridge.list().await.unwrap();
//! bridge.remove("hourly-cleanup").await.unwrap();
//! # }
//! ```

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{OjsClient, OjsState};
use crate::error::OjsAxumError;

/// Configuration for a single cron schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    /// Unique name for this cron schedule.
    pub name: String,
    /// Cron expression (5-6 field or `@hourly` / `@daily` shortcuts).
    pub schedule: String,
    /// Job type to enqueue on each tick.
    pub job_type: String,
    /// Optional queue name. Defaults to `"default"`.
    #[serde(default = "default_queue")]
    pub queue: String,
    /// Job arguments passed on each invocation.
    #[serde(default)]
    pub args: serde_json::Value,
    /// IANA timezone (e.g. `"UTC"`, `"America/New_York"`).
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_queue() -> String {
    "default".into()
}

fn default_timezone() -> String {
    "UTC".into()
}

impl Default for CronConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            schedule: String::new(),
            job_type: String::new(),
            queue: default_queue(),
            args: serde_json::Value::Null,
            timezone: default_timezone(),
        }
    }
}

impl CronConfig {
    /// Validate that required fields are non-empty.
    pub fn validate(&self) -> Result<(), OjsAxumError> {
        if self.name.is_empty() {
            return Err(OjsAxumError::Validation("cron name is required".into()));
        }
        if self.schedule.is_empty() {
            return Err(OjsAxumError::Validation("schedule expression is required".into()));
        }
        if self.job_type.is_empty() {
            return Err(OjsAxumError::Validation("job_type is required".into()));
        }
        Ok(())
    }
}

/// Summary returned for each registered cron schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronEntry {
    /// Schedule name.
    pub name: String,
    /// Cron expression.
    pub schedule: String,
    /// Job type.
    pub job_type: String,
    /// Queue.
    pub queue: String,
    /// Timezone.
    pub timezone: String,
    /// Whether the schedule is currently enabled.
    pub enabled: bool,
}

/// Bridge between Axum handlers and the OJS cron API.
///
/// Wraps an [`ojs::Client`] and exposes register / list / remove helpers.
pub struct OjsCronBridge {
    client: ojs::Client,
}

impl OjsCronBridge {
    /// Create a new cron bridge with the given OJS client.
    pub fn new(client: ojs::Client) -> Self {
        Self { client }
    }

    /// Register a new cron schedule with the OJS backend.
    pub async fn register(&self, config: CronConfig) -> Result<CronEntry, OjsAxumError> {
        config.validate()?;

        let req = ojs::CronJobRequest {
            name: config.name.clone(),
            cron: config.schedule.clone(),
            timezone: config.timezone.clone(),
            job_type: config.job_type.clone(),
            args: Some(config.args.clone()),
            meta: None,
            options: None,
            overlap_policy: ojs::OverlapPolicy::Skip,
            enabled: true,
            description: None,
        };

        let cron_job = self.client.register_cron_job(req).await.map_err(OjsAxumError::Ojs)?;

        Ok(CronEntry {
            name: cron_job.name,
            schedule: cron_job.cron,
            job_type: cron_job.job_type,
            queue: config.queue,
            timezone: cron_job.timezone,
            enabled: cron_job.enabled,
        })
    }

    /// List all registered cron schedules.
    pub async fn list(&self) -> Result<Vec<CronEntry>, OjsAxumError> {
        let jobs = self.client.list_cron_jobs().await.map_err(OjsAxumError::Ojs)?;

        Ok(jobs
            .into_iter()
            .map(|j| CronEntry {
                name: j.name,
                schedule: j.cron,
                job_type: j.job_type,
                queue: "default".into(),
                timezone: j.timezone,
                enabled: j.enabled,
            })
            .collect())
    }

    /// Remove a cron schedule by name.
    pub async fn remove(&self, name: &str) -> Result<(), OjsAxumError> {
        self.client
            .unregister_cron_job(name)
            .await
            .map_err(OjsAxumError::Ojs)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Axum handlers
// ---------------------------------------------------------------------------

/// `POST /cron` – register a new cron schedule.
async fn create_cron(ojs: OjsClient, Json(config): Json<CronConfig>) -> impl IntoResponse {
    let bridge = OjsCronBridge::new(ojs.into_inner());
    match bridge.register(config).await {
        Ok(entry) => (StatusCode::CREATED, Json(serde_json::to_value(entry).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `GET /cron` – list all cron schedules.
async fn list_cron(ojs: OjsClient) -> impl IntoResponse {
    let bridge = OjsCronBridge::new(ojs.into_inner());
    match bridge.list().await {
        Ok(entries) => (StatusCode::OK, Json(serde_json::to_value(entries).unwrap())).into_response(),
        Err(e) => e.into_response(),
    }
}

/// `DELETE /cron/:name` – remove a cron schedule.
async fn delete_cron(ojs: OjsClient, Path(name): Path<String>) -> impl IntoResponse {
    let bridge = OjsCronBridge::new(ojs.into_inner());
    match bridge.remove(&name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

/// Returns a [`Router<OjsState>`] with cron CRUD endpoints.
///
/// | Method   | Path          | Description           |
/// |----------|---------------|-----------------------|
/// | `POST`   | `/cron`       | Register a schedule   |
/// | `GET`    | `/cron`       | List all schedules    |
/// | `DELETE` | `/cron/:name` | Remove a schedule     |
///
/// ```rust,no_run
/// use axum::Router;
/// use ojs_axum::{OjsState, cron::cron_router};
///
/// let app: Router = Router::new()
///     .merge(cron_router())
///     .with_state(OjsState::new(
///         ojs::Client::builder().url("http://localhost:8080").build().unwrap(),
///     ));
/// ```
pub fn cron_router() -> Router<OjsState> {
    Router::new()
        .route("/cron", post(create_cron).get(list_cron))
        .route("/cron/{name}", delete(delete_cron))
}
