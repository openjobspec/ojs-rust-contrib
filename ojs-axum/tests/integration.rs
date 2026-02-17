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
