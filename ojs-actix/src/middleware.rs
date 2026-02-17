use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::web::Data;
use actix_web::{Error, HttpMessage};
use std::future::{ready, Ready};
use std::pin::Pin;

/// Actix-web middleware that injects an [`ojs::Client`] into application data.
///
/// Wrap your `App` with this middleware to make the OJS client available to all
/// handlers via the [`OjsClient`](crate::OjsClient) extractor.
///
/// # Example
///
/// ```rust,no_run
/// use actix_web::{App, HttpServer};
/// use ojs::Client;
/// use ojs_actix::OjsMiddleware;
///
/// # #[actix_web::main]
/// # async fn main() -> std::io::Result<()> {
/// let client = Client::builder()
///     .url("http://localhost:8080")
///     .build()
///     .unwrap();
///
/// HttpServer::new(move || {
///     App::new().wrap(OjsMiddleware::new(client.clone()))
/// })
/// .bind("0.0.0.0:3000")?
/// .run()
/// .await
/// # }
/// ```
#[derive(Clone)]
pub struct OjsMiddleware {
    client: ojs::Client,
}

impl OjsMiddleware {
    /// Create a new middleware with the given OJS client.
    pub fn new(client: ojs::Client) -> Self {
        Self { client }
    }
}

impl<S, B> Transform<S, ServiceRequest> for OjsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = OjsMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(OjsMiddlewareService {
            service,
            client: self.client.clone(),
        }))
    }
}

/// The actual middleware service created by [`OjsMiddleware`].
pub struct OjsMiddlewareService<S> {
    service: S,
    client: ojs::Client,
}

impl<S, B> Service<ServiceRequest> for OjsMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        req.extensions_mut().insert(Data::new(self.client.clone()));
        let fut = self.service.call(req);
        Box::pin(fut)
    }
}
