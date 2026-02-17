use std::task::{Context, Poll};
use tower::{Layer, Service};

/// Tower [`Layer`] that injects an [`ojs::Client`] into request extensions.
///
/// This is an alternative to using [`OjsState`](crate::OjsState) when you
/// prefer extension-based access over typed state.
///
/// # Example
///
/// ```rust,ignore
/// use axum::Router;
/// use ojs::Client;
/// use ojs_axum::OjsLayer;
///
/// let client = Client::builder()
///     .url("http://localhost:8080")
///     .build()
///     .unwrap();
///
/// let app = Router::new().layer(OjsLayer::new(client));
/// ```
#[derive(Clone)]
pub struct OjsLayer {
    client: ojs::Client,
}

impl OjsLayer {
    /// Create a new layer with the given OJS client.
    pub fn new(client: ojs::Client) -> Self {
        Self { client }
    }
}

impl<S> Layer<S> for OjsLayer {
    type Service = OjsService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OjsService {
            inner,
            client: self.client.clone(),
        }
    }
}

/// The Tower service produced by [`OjsLayer`].
#[derive(Clone)]
pub struct OjsService<S> {
    inner: S,
    client: ojs::Client,
}

impl<S, ReqBody> Service<axum::http::Request<ReqBody>> for OjsService<S>
where
    S: Service<axum::http::Request<ReqBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: axum::http::Request<ReqBody>) -> Self::Future {
        req.extensions_mut().insert(self.client.clone());
        self.inner.call(req)
    }
}
