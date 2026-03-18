use ojs_axum::OjsState;

fn assert_from_ref() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let state = OjsState::new(client);
    let extracted: ojs::Client = axum::extract::FromRef::from_ref(&state);
    drop(extracted);
}

#[tokio::test]
async fn test_ojs_state_from_ref() {
    assert_from_ref();
}

#[tokio::test]
async fn test_ojs_state_client_accessor() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let state = OjsState::new(client);
    let _ = state.client();
}

#[tokio::test]
async fn test_router_round_trip() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::OjsClient;
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    async fn handler(ojs: OjsClient) -> String {
        // Verify we can access the client through Deref
        let _ = ojs.inner();
        "ok".to_string()
    }

    let app = Router::new()
        .route("/health", get(handler))
        .with_state(OjsState::new(client));

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_ojs_layer_injects_client() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::OjsLayer;
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    async fn handler(req: axum::http::Request<Body>) -> String {
        let has_client = req.extensions().get::<ojs::Client>().is_some();
        format!("{has_client}")
    }

    let app = Router::new()
        .route("/check", get(handler))
        .layer(OjsLayer::new(client));

    let req = Request::builder()
        .uri("/check")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[test]
fn test_re_export_client() {
    // Verify ojs::Client is re-exported from ojs_axum
    let _client = ojs_axum::Client::builder()
        .url("http://localhost:9999")
        .build()
        .unwrap();
}

#[tokio::test]
async fn test_layer_ordering_with_state() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::{OjsClient, OjsLayer, OjsState};
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    async fn handler(ojs: OjsClient) -> String {
        let _ = ojs.inner();
        "layer-ok".to_string()
    }

    // Layer applied after state — both should work without conflict
    let app = Router::new()
        .route("/check", get(handler))
        .with_state(OjsState::new(client.clone()))
        .layer(OjsLayer::new(client));

    let req = Request::builder()
        .uri("/check")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_state_extraction_with_multiple_extractors() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::{OjsClient, OjsState};
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    async fn handler(
        uri: axum::http::Uri,
        ojs: OjsClient,
    ) -> String {
        let _ = ojs.inner();
        format!("path={}", uri.path())
    }

    let app = Router::new()
        .route("/multi", get(handler))
        .with_state(OjsState::new(client));

    let req = Request::builder()
        .uri("/multi")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"path=/multi");
}

#[tokio::test]
async fn test_error_response_formatting() {
    use axum::{body::Body, http::Request, http::StatusCode, response::IntoResponse, routing::get, Router};
    use ojs_axum::{OjsClient, OjsState};
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    async fn handler(_ojs: OjsClient) -> impl IntoResponse {
        (StatusCode::UNPROCESSABLE_ENTITY, "validation failed")
    }

    let app = Router::new()
        .route("/err", get(handler))
        .with_state(OjsState::new(client));

    let req = Request::builder()
        .uri("/err")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&body[..], b"validation failed");
}

#[tokio::test]
async fn test_graceful_shutdown_signal_is_pending() {
    // shutdown_signal should remain pending when no signal is sent
    let signal = ojs_axum::shutdown_signal();
    let result = tokio::time::timeout(std::time::Duration::from_millis(50), signal).await;
    assert!(result.is_err(), "shutdown_signal should not resolve without a signal");
}

// ---------------------------------------------------------------------------
// Health module tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_handler_returns_json_structure() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::{OjsState, health::health_handler};
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .unwrap();

    let app = Router::new()
        .route("/health", get(health_handler))
        .with_state(OjsState::new(client));

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Backend is unreachable so we expect 503
    assert_eq!(resp.status(), 503);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "degraded");
    assert_eq!(json["ojs"]["status"], "error");
    assert!(json["ojs"]["error"].is_string());
}

#[tokio::test]
async fn test_health_router_mounts_at_health() {
    use axum::{body::Body, http::Request, Router};
    use ojs_axum::{OjsState, health::health_router};
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .unwrap();

    let app = Router::new()
        .merge(health_router())
        .with_state(OjsState::new(client));

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    // Should respond (even if degraded) instead of 404
    assert_ne!(resp.status(), 404);
}

#[test]
fn test_health_response_serialization() {
    use ojs_axum::health::{HealthResponse, OjsHealth};

    let resp = HealthResponse {
        status: "healthy".into(),
        ojs: OjsHealth {
            status: "ok".into(),
            error: None,
        },
    };
    let json = serde_json::to_value(&resp).unwrap();
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["ojs"]["status"], "ok");
    assert!(json["ojs"].get("error").is_none());
}

// ---------------------------------------------------------------------------
// Events module tests
// ---------------------------------------------------------------------------

#[test]
fn test_event_config_default() {
    use ojs_axum::events::EventConfig;

    let cfg = EventConfig::default();
    assert_eq!(cfg.url, "http://localhost:8080");
    assert!(cfg.event_types.is_empty());
    assert_eq!(cfg.poll_interval_ms, 1000);
}

#[test]
fn test_event_type_as_event_str() {
    use ojs_axum::events::OjsEventType;

    assert_eq!(OjsEventType::JobCompleted.as_event_str(), "job.completed");
    assert_eq!(OjsEventType::JobFailed.as_event_str(), "job.failed");
    assert_eq!(OjsEventType::JobRetrying.as_event_str(), "job.retrying");
    assert_eq!(OjsEventType::JobCancelled.as_event_str(), "job.cancelled");
    assert_eq!(OjsEventType::WorkflowCompleted.as_event_str(), "workflow.completed");
}

#[test]
fn test_event_type_serialization_roundtrip() {
    use ojs_axum::events::OjsEventType;

    let types = vec![
        OjsEventType::JobCompleted,
        OjsEventType::JobFailed,
        OjsEventType::JobRetrying,
        OjsEventType::JobCancelled,
        OjsEventType::WorkflowCompleted,
    ];

    let json = serde_json::to_string(&types).unwrap();
    let deserialized: Vec<OjsEventType> = serde_json::from_str(&json).unwrap();
    assert_eq!(types, deserialized);
}

#[test]
fn test_event_subscriber_config() {
    use ojs_axum::events::{EventConfig, OjsEventSubscriber, OjsEventType};

    let config = EventConfig {
        url: "http://localhost:9999".into(),
        event_types: vec![OjsEventType::JobCompleted, OjsEventType::JobFailed],
        poll_interval_ms: 500,
    };

    let subscriber = OjsEventSubscriber::new(config);
    assert_eq!(subscriber.config().url, "http://localhost:9999");
    assert_eq!(subscriber.config().event_types.len(), 2);
    assert_eq!(subscriber.config().poll_interval_ms, 500);
}

#[test]
fn test_ojs_event_serialization() {
    use ojs_axum::events::OjsEvent;

    let event = OjsEvent {
        id: "evt-1".into(),
        event_type: "job.completed".into(),
        timestamp: "2025-01-01T00:00:00Z".into(),
        subject: Some("job-abc".into()),
        data: Some(serde_json::json!({"result": "ok"})),
    };

    let json = serde_json::to_value(&event).unwrap();
    assert_eq!(json["id"], "evt-1");
    assert_eq!(json["event_type"], "job.completed");
    assert_eq!(json["subject"], "job-abc");
}

// ---------------------------------------------------------------------------
// Cron module tests
// ---------------------------------------------------------------------------

#[test]
fn test_cron_config_default() {
    use ojs_axum::cron::CronConfig;

    let cfg = CronConfig::default();
    assert_eq!(cfg.queue, "default");
    assert_eq!(cfg.timezone, "UTC");
    assert!(cfg.name.is_empty());
}

#[test]
fn test_cron_config_validate_success() {
    use ojs_axum::cron::CronConfig;

    let cfg = CronConfig {
        name: "test-job".into(),
        schedule: "0 * * * *".into(),
        job_type: "cleanup".into(),
        ..Default::default()
    };

    assert!(cfg.validate().is_ok());
}

#[test]
fn test_cron_config_validate_missing_name() {
    use ojs_axum::cron::CronConfig;

    let cfg = CronConfig {
        schedule: "0 * * * *".into(),
        job_type: "cleanup".into(),
        ..Default::default()
    };

    let err = cfg.validate().unwrap_err();
    assert!(err.to_string().contains("name"));
}

#[test]
fn test_cron_config_validate_missing_schedule() {
    use ojs_axum::cron::CronConfig;

    let cfg = CronConfig {
        name: "test".into(),
        job_type: "cleanup".into(),
        ..Default::default()
    };

    let err = cfg.validate().unwrap_err();
    assert!(err.to_string().contains("schedule"));
}

#[test]
fn test_cron_config_validate_missing_job_type() {
    use ojs_axum::cron::CronConfig;

    let cfg = CronConfig {
        name: "test".into(),
        schedule: "0 * * * *".into(),
        ..Default::default()
    };

    let err = cfg.validate().unwrap_err();
    assert!(err.to_string().contains("job_type"));
}

#[test]
fn test_cron_config_serialization() {
    use ojs_axum::cron::CronConfig;

    let cfg = CronConfig {
        name: "hourly-ping".into(),
        schedule: "0 * * * *".into(),
        job_type: "ping".into(),
        queue: "critical".into(),
        args: serde_json::json!({"target": "db"}),
        timezone: "America/New_York".into(),
    };

    let json = serde_json::to_value(&cfg).unwrap();
    assert_eq!(json["name"], "hourly-ping");
    assert_eq!(json["schedule"], "0 * * * *");
    assert_eq!(json["timezone"], "America/New_York");

    // Round-trip
    let back: CronConfig = serde_json::from_value(json).unwrap();
    assert_eq!(back.name, "hourly-ping");
}

#[test]
fn test_cron_bridge_creation() {
    use ojs_axum::cron::OjsCronBridge;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .unwrap();

    // Just verifying construction doesn't panic
    let _bridge = OjsCronBridge::new(client);
}

// ---------------------------------------------------------------------------
// Trace layer tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_trace_layer_passes_through() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::OjsTraceLayer;
    use tower::ServiceExt;

    let app = Router::new()
        .route("/traced", get(|| async { "traced-ok" }))
        .layer(OjsTraceLayer::new());

    let req = Request::builder()
        .uri("/traced")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"traced-ok");
}

#[tokio::test]
async fn test_trace_layer_with_ojs_headers() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use ojs_axum::OjsTraceLayer;
    use tower::ServiceExt;

    let app = Router::new()
        .route("/traced", get(|| async { "ok" }))
        .layer(OjsTraceLayer::new());

    let req = Request::builder()
        .uri("/traced")
        .header("x-ojs-job-id", "job-123")
        .header("x-ojs-queue", "critical")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), 200);
}

// ---------------------------------------------------------------------------
// Module integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_new_re_exports_accessible() {
    // Verify top-level re-exports compile
    let _ = std::any::type_name::<ojs_axum::OjsTraceLayer>();
    let _ = std::any::type_name::<ojs_axum::CronConfig>();
    let _ = std::any::type_name::<ojs_axum::OjsCronBridge>();
    let _ = std::any::type_name::<ojs_axum::EventConfig>();
    let _ = std::any::type_name::<ojs_axum::OjsEventSubscriber>();
    let _ = std::any::type_name::<ojs_axum::OjsEventType>();
    let _ = std::any::type_name::<ojs_axum::HealthResponse>();
    let _ = std::any::type_name::<ojs_axum::OjsAxumError>();
}

#[tokio::test]
async fn test_combined_router_health_and_cron() {
    use axum::{body::Body, http::Request, Router};
    use ojs_axum::{OjsState, health::health_router, cron::cron_router};
    use tower::ServiceExt;

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .unwrap();

    let app = Router::new()
        .merge(health_router())
        .merge(cron_router())
        .with_state(OjsState::new(client));

    // Health endpoint responds
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_ne!(resp.status(), 404);

    // Cron list endpoint responds (will fail against backend, but not 404)
    let req = Request::builder()
        .uri("/cron")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_ne!(resp.status(), 404);
}
