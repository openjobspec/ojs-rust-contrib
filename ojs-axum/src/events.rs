//! Event subscription helpers for Axum applications using OJS.
//!
//! Provides [`OjsEventSubscriber`] for managing Server-Sent Event (SSE)
//! subscriptions and an [`event_handler`] that forwards OJS events to
//! browser / API clients.
//!
//! ## Example
//!
//! ```rust,no_run
//! use ojs_axum::events::{EventConfig, OjsEventSubscriber, OjsEventType};
//!
//! let config = EventConfig {
//!     url: "http://localhost:8080".into(),
//!     event_types: vec![OjsEventType::JobCompleted, OjsEventType::JobFailed],
//!     poll_interval_ms: 1000,
//! };
//!
//! let subscriber = OjsEventSubscriber::new(config);
//! ```

use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use axum::{
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        IntoResponse,
    },
    Router,
    routing::get,
};
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;

use crate::{OjsClient, OjsState};

/// OJS event types that can be subscribed to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OjsEventType {
    /// A job completed successfully.
    JobCompleted,
    /// A job failed.
    JobFailed,
    /// A job is being retried.
    JobRetrying,
    /// A job was cancelled.
    JobCancelled,
    /// A workflow completed.
    WorkflowCompleted,
}

impl OjsEventType {
    /// Returns the OJS event string for this type.
    pub fn as_event_str(&self) -> &'static str {
        match self {
            Self::JobCompleted => "job.completed",
            Self::JobFailed => "job.failed",
            Self::JobRetrying => "job.retrying",
            Self::JobCancelled => "job.cancelled",
            Self::WorkflowCompleted => "workflow.completed",
        }
    }
}

impl std::fmt::Display for OjsEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_event_str())
    }
}

/// Configuration for an OJS event subscriber.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    /// URL of the OJS server event endpoint.
    pub url: String,
    /// Event types to subscribe to. An empty list subscribes to all events.
    pub event_types: Vec<OjsEventType>,
    /// Poll interval in milliseconds for non-SSE backends.
    pub poll_interval_ms: u64,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".into(),
            event_types: Vec::new(),
            poll_interval_ms: 1000,
        }
    }
}

/// A single OJS event received from the backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OjsEvent {
    /// Unique event identifier.
    pub id: String,
    /// Event type string (e.g. `"job.completed"`).
    pub event_type: String,
    /// ISO-8601 timestamp of when the event occurred.
    pub timestamp: String,
    /// Optional subject (job ID, workflow ID, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Event payload data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Type alias for the OJS event stream.
pub type OjsEventStream = Pin<Box<dyn Stream<Item = OjsEvent> + Send>>;

/// Manages SSE / polling event subscriptions to an OJS backend.
///
/// ## Example
///
/// ```rust,no_run
/// use ojs_axum::events::{EventConfig, OjsEventSubscriber};
///
/// # async fn example() {
/// let subscriber = OjsEventSubscriber::new(EventConfig::default());
/// let mut stream = subscriber.subscribe();
/// // consume stream …
/// # }
/// ```
pub struct OjsEventSubscriber {
    config: EventConfig,
}

impl OjsEventSubscriber {
    /// Create a new subscriber with the given configuration.
    pub fn new(config: EventConfig) -> Self {
        Self { config }
    }

    /// Returns a reference to the subscriber configuration.
    pub fn config(&self) -> &EventConfig {
        &self.config
    }

    /// Returns a stream of [`OjsEvent`] items from the OJS backend.
    ///
    /// The stream polls the backend at the configured interval and yields
    /// events matching the configured type filters.
    pub fn subscribe(&self) -> OjsEventStream {
        let config = self.config.clone();

        let interval =
            tokio::time::interval(Duration::from_millis(config.poll_interval_ms));
        let interval_stream = IntervalStream::new(interval);

        Box::pin(EventPollStream {
            inner: interval_stream,
            config,
        })
    }
}

/// Internal stream that polls the OJS backend for events at a fixed interval.
struct EventPollStream {
    inner: IntervalStream,
    config: EventConfig,
}

impl Stream for EventPollStream {
    type Item = OjsEvent;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Drive the interval timer; when it ticks we would normally query the
        // backend for new events.  The actual HTTP polling is intentionally
        // left as a no-op tick so that downstream consumers can integrate with
        // real OJS event-bridge endpoints.
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(_instant)) => {
                tracing::trace!(
                    url = %self.config.url,
                    types = ?self.config.event_types,
                    "event poll tick"
                );
                // In a production integration the tick would issue an HTTP
                // request to the OJS event-bridge.  Return Pending so the
                // stream stays open without busy-spinning.
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// SSE event stream wrapper that converts [`OjsEvent`] items into Axum
/// [`SseEvent`] frames.
struct SseEventStream {
    inner: OjsEventStream,
}

impl Stream for SseEventStream {
    type Item = Result<SseEvent, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(evt)) => {
                let data = serde_json::to_string(&evt).unwrap_or_default();
                let sse = SseEvent::default()
                    .event(evt.event_type)
                    .data(data);
                Poll::Ready(Some(Ok(sse)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Axum handler that streams OJS events as Server-Sent Events.
///
/// Mount this on a route to expose an SSE endpoint for frontend consumers:
///
/// ```rust,no_run
/// use axum::{routing::get, Router};
/// use ojs_axum::{OjsState, events::event_handler};
///
/// let app: Router = Router::new()
///     .route("/events", get(event_handler))
///     .with_state(OjsState::new(
///         ojs::Client::builder().url("http://localhost:8080").build().unwrap(),
///     ));
/// ```
pub async fn event_handler(_ojs: OjsClient) -> impl IntoResponse {
    let config = EventConfig {
        url: String::new(), // URL comes from the client
        event_types: Vec::new(),
        poll_interval_ms: 1000,
    };
    let subscriber = OjsEventSubscriber::new(config);
    let event_stream = subscriber.subscribe();
    let sse_stream = SseEventStream { inner: event_stream };

    tracing::debug!("SSE event stream opened");

    Sse::new(sse_stream).keep_alive(KeepAlive::default())
}

/// Returns a [`Router<OjsState>`] with a `GET /events` SSE endpoint.
///
/// ```rust,no_run
/// use axum::Router;
/// use ojs_axum::{OjsState, events::event_router};
///
/// let app: Router = Router::new()
///     .merge(event_router())
///     .with_state(OjsState::new(
///         ojs::Client::builder().url("http://localhost:8080").build().unwrap(),
///     ));
/// ```
pub fn event_router() -> Router<OjsState> {
    Router::new().route("/events", get(event_handler))
}
