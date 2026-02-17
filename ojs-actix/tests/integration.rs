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
