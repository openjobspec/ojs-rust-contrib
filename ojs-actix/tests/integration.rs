use actix_web::{test, web, App, HttpResponse};
use ojs_actix::{
    configure_ojs, health_handler, JobContext, OjsAppData, OjsClient, OjsMiddleware,
    OjsWorkerManager, WorkerConfig,
};

async fn simple_handler(_ojs: OjsClient) -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

// ---------------------------------------------------------------------------
// Existing middleware / extractor tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_middleware_injects_client() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .wrap(OjsMiddleware::new(client))
            .route("/health", web::get().to(simple_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_extractor_without_middleware_fails() {
    let app =
        test::init_service(App::new().route("/health", web::get().to(simple_handler))).await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 500);
}

#[actix_web::test]
async fn test_middleware_error_propagation() {
    async fn failing_handler(_ojs: OjsClient) -> HttpResponse {
        HttpResponse::BadRequest().body("bad request")
    }

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .wrap(OjsMiddleware::new(client))
            .route("/fail", web::get().to(failing_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/fail").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn test_concurrent_request_handling() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .wrap(OjsMiddleware::new(client))
            .route("/health", web::get().to(simple_handler)),
    )
    .await;

    let mut handles = Vec::new();
    for _ in 0..10 {
        let req = test::TestRequest::get().uri("/health").to_request();
        handles.push(test::call_service(&app, req));
    }

    for handle in handles {
        let resp = handle.await;
        assert_eq!(resp.status(), 200);
    }
}

#[actix_web::test]
async fn test_custom_error_handler_missing_middleware() {
    let app =
        test::init_service(App::new().route("/health", web::get().to(simple_handler))).await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 500);

    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("OJS client not configured"));
}

#[actix_web::test]
async fn test_app_data_extraction_in_nested_scopes() {
    async fn scoped_handler(ojs: OjsClient) -> HttpResponse {
        let _ = ojs.into_inner();
        HttpResponse::Ok().body("nested ok")
    }

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .wrap(OjsMiddleware::new(client))
            .service(
                web::scope("/api").service(
                    web::scope("/v1").route("/jobs", web::get().to(scoped_handler)),
                ),
            ),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/v1/jobs").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body = test::read_body(resp).await;
    assert_eq!(&body[..], b"nested ok");
}

// ---------------------------------------------------------------------------
// Health handler tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_health_handler_returns_json() {
    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .wrap(OjsMiddleware::new(client))
            .route("/health", web::get().to(health_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    // The backend is not reachable, so we expect 503 with valid JSON.
    assert_eq!(resp.status(), 503);

    let body = test::read_body(resp).await;
    let json: serde_json::Value = serde_json::from_slice(&body).expect("response should be JSON");
    assert_eq!(json["status"], "unhealthy");
    assert_eq!(json["ojs"]["status"], "error");
    assert!(json["ojs"]["error"].is_string());
}

// ---------------------------------------------------------------------------
// Worker manager tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_worker_manager_creation() {
    let config = WorkerConfig::default();
    let manager = OjsWorkerManager::new(config);

    assert!(!manager.is_running().await);
    assert_eq!(manager.config().url, "http://localhost:8080");
}

#[actix_web::test]
async fn test_worker_manager_handler_registration() {
    let manager = OjsWorkerManager::new(WorkerConfig::default());

    manager
        .register("email.send", |_ctx: JobContext| {
            Box::pin(async move { Ok(()) })
        })
        .await;

    manager
        .register("report.generate", |_ctx: JobContext| {
            Box::pin(async move { Ok(()) })
        })
        .await;

    let types = manager.registered_types().await;
    assert_eq!(types, vec!["email.send", "report.generate"]);
}

#[actix_web::test]
async fn test_worker_manager_registered_types_sorted() {
    let manager = OjsWorkerManager::new(WorkerConfig::default());

    manager
        .register("z.last", |_ctx: JobContext| {
            Box::pin(async move { Ok(()) })
        })
        .await;

    manager
        .register("a.first", |_ctx: JobContext| {
            Box::pin(async move { Ok(()) })
        })
        .await;

    let types = manager.registered_types().await;
    assert_eq!(types, vec!["a.first", "z.last"]);
}

// ---------------------------------------------------------------------------
// App data tests
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_ojs_app_data() {
    async fn handler(ojs: web::Data<OjsAppData>) -> HttpResponse {
        let _ = ojs.client();
        HttpResponse::Ok().body("app_data ok")
    }

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(OjsAppData::new(client)))
            .route("/test", web::get().to(handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_configure_ojs() {
    async fn handler(ojs: web::Data<OjsAppData>) -> HttpResponse {
        let _ = ojs.client();
        HttpResponse::Ok().body("configured ok")
    }

    let client = ojs::Client::builder()
        .url("http://localhost:9999")
        .build()
        .expect("failed to build client");

    let app = test::init_service(
        App::new()
            .configure(configure_ojs(client))
            .route("/test", web::get().to(handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/test").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body = test::read_body(resp).await;
    assert_eq!(&body[..], b"configured ok");
}

// ---------------------------------------------------------------------------
// Worker config defaults
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn test_worker_config_defaults() {
    let config = WorkerConfig::default();

    assert_eq!(config.url, "http://localhost:8080");
    assert_eq!(config.queues, vec!["default"]);
    assert_eq!(config.concurrency, 10);
    assert_eq!(config.poll_interval_ms, 1000);
    assert_eq!(config.shutdown_timeout_secs, 25);
}
