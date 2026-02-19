use actix_web::{test, web, App, HttpResponse};
use ojs_actix::{OjsClient, OjsMiddleware};

async fn health_handler(_ojs: OjsClient) -> HttpResponse {
    HttpResponse::Ok().body("ok")
}

#[actix_web::test]
async fn test_middleware_injects_client() {
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
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_extractor_without_middleware_fails() {
    let app = test::init_service(App::new().route("/health", web::get().to(health_handler))).await;

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
            .route("/health", web::get().to(health_handler)),
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
    let app = test::init_service(App::new().route("/health", web::get().to(health_handler))).await;

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
                web::scope("/api")
                    .service(
                        web::scope("/v1")
                            .route("/jobs", web::get().to(scoped_handler)),
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
