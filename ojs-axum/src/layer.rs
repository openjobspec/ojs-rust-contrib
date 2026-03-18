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

// ---------------------------------------------------------------------------
// OjsTraceLayer – tracing middleware
// ---------------------------------------------------------------------------

/// Tower [`Layer`] that emits [`tracing`] spans with request metadata.
///
/// Each request gets a span that records the HTTP method, path, and any
/// OJS-related headers (those prefixed with `x-ojs-`).
///
/// # Example
///
/// ```rust,ignore
/// use axum::Router;
/// use ojs_axum::OjsTraceLayer;
///
/// let app = Router::new()
///     .layer(OjsTraceLayer::new());
/// ```
#[derive(Clone, Debug, Default)]
pub struct OjsTraceLayer {
    _priv: (),
}

impl OjsTraceLayer {
    /// Create a new tracing layer.
    pub fn new() -> Self {
        Self { _priv: () }
    }
}

impl<S> Layer<S> for OjsTraceLayer {
    type Service = OjsTraceService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OjsTraceService { inner }
    }
}

/// The Tower service produced by [`OjsTraceLayer`].
#[derive(Clone, Debug)]
pub struct OjsTraceService<S> {
    inner: S,
}

impl<S, ReqBody> Service<axum::http::Request<ReqBody>> for OjsTraceService<S>
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

    fn call(&mut self, req: axum::http::Request<ReqBody>) -> Self::Future {
        let method = req.method().clone();
        let path = req.uri().path().to_owned();

        // Collect OJS-specific headers (x-ojs-*).
        let ojs_headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .filter(|(name, _)| name.as_str().starts_with("x-ojs-"))
            .map(|(name, value)| {
                (
                    name.to_string(),
                    value.to_str().unwrap_or("<non-utf8>").to_owned(),
                )
            })
            .collect();

        tracing::info!(
            http.method = %method,
            http.path = %path,
            ojs.headers = ?ojs_headers,
            "ojs request"
        );

        self.inner.call(req)
    }
}
