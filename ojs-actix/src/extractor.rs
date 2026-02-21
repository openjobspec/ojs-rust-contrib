use actix_web::web::Data;
use actix_web::{FromRequest, HttpMessage, HttpRequest};
use std::future::{ready, Ready};
use std::ops::Deref;

/// Extractor that retrieves the [`ojs::Client`] from Actix-web request extensions.
///
/// Use this as a handler parameter after applying [`OjsMiddleware`](crate::OjsMiddleware).
///
/// # Example
///
/// ```rust,no_run
/// use actix_web::HttpResponse;
/// use ojs_actix::OjsClient;
/// use serde_json::json;
///
/// async fn enqueue(ojs: OjsClient) -> HttpResponse {
///     match ojs.enqueue("email.send", json!({"to": "a@b.com"})).await {
///         Ok(j) => HttpResponse::Ok().json(serde_json::json!({"id": j.id.to_string()})),
///         Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
///     }
/// }
/// ```
pub struct OjsClient(Data<ojs::Client>);

impl OjsClient {
    /// Get a reference to the inner [`ojs::Client`].
    pub fn into_inner(self) -> Data<ojs::Client> {
        self.0
    }
}

impl Deref for OjsClient {
    type Target = ojs::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for OjsClient {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        match req.extensions().get::<Data<ojs::Client>>() {
            Some(client) => ready(Ok(OjsClient(client.clone()))),
            None => ready(Err(actix_web::error::ErrorInternalServerError(
                "OJS client not configured. Did you forget to add OjsMiddleware?",
            ))),
        }
    }
}

