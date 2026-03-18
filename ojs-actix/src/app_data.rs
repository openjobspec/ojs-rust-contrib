use actix_web::web;
use std::ops::Deref;

/// Wrapper around [`ojs::Client`] for use as actix-web app data.
///
/// This provides an alternative to [`OjsMiddleware`](crate::OjsMiddleware):
/// instead of injecting the client via middleware on every request, you
/// register it once as shared app data with [`actix_web::App::app_data`].
///
/// # Example
///
/// ```rust,no_run
/// use actix_web::{web, App, HttpServer, HttpResponse};
/// use ojs::Client;
/// use ojs_actix::OjsAppData;
///
/// async fn handler(ojs: web::Data<OjsAppData>) -> HttpResponse {
///     let _ = ojs.enqueue("ping", serde_json::json!([])).await;
///     HttpResponse::Ok().finish()
/// }
///
/// # #[actix_web::main]
/// # async fn main() -> std::io::Result<()> {
/// let client = Client::builder()
///     .url("http://localhost:8080")
///     .build()
///     .unwrap();
///
/// HttpServer::new(move || {
///     App::new()
///         .app_data(web::Data::new(OjsAppData::new(client.clone())))
///         .route("/enqueue", web::post().to(handler))
/// })
/// .bind("0.0.0.0:3000")?
/// .run()
/// .await
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct OjsAppData {
    client: ojs::Client,
}

impl OjsAppData {
    /// Create a new app data wrapper around the given OJS client.
    pub fn new(client: ojs::Client) -> Self {
        Self { client }
    }

    /// Consume the wrapper and return the inner client.
    pub fn into_inner(self) -> ojs::Client {
        self.client
    }

    /// Get a reference to the inner client.
    pub fn client(&self) -> &ojs::Client {
        &self.client
    }
}

impl Deref for OjsAppData {
    type Target = ojs::Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// Returns a closure suitable for [`actix_web::App::configure`] that registers
/// the OJS client as shared app data.
///
/// This is a convenience helper for applications that prefer the `configure()`
/// pattern over manually calling `.app_data()`.
///
/// # Example
///
/// ```rust,no_run
/// use actix_web::{App, HttpServer};
/// use ojs::Client;
/// use ojs_actix::configure_ojs;
///
/// # #[actix_web::main]
/// # async fn main() -> std::io::Result<()> {
/// let client = Client::builder()
///     .url("http://localhost:8080")
///     .build()
///     .unwrap();
///
/// HttpServer::new(move || {
///     App::new().configure(configure_ojs(client.clone()))
/// })
/// .bind("0.0.0.0:3000")?
/// .run()
/// .await
/// # }
/// ```
pub fn configure_ojs(
    client: ojs::Client,
) -> impl FnOnce(&mut web::ServiceConfig) + Clone {
    move |cfg: &mut web::ServiceConfig| {
        cfg.app_data(web::Data::new(OjsAppData::new(client)));
    }
}
