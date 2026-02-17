use axum::extract::{FromRef, FromRequestParts};
use std::convert::Infallible;

/// Shared state wrapping an [`ojs::Client`] for use with Axum's `State` extractor.
///
/// Pass this as router state via `Router::with_state(OjsState::new(client))`.
#[derive(Clone, Debug)]
pub struct OjsState {
    client: ojs::Client,
}

impl OjsState {
    /// Create a new state wrapper.
    pub fn new(client: ojs::Client) -> Self {
        Self { client }
    }

    /// Get a reference to the inner client.
    pub fn client(&self) -> &ojs::Client {
        &self.client
    }
}

impl FromRef<OjsState> for ojs::Client {
    fn from_ref(state: &OjsState) -> Self {
        state.client.clone()
    }
}

/// Axum extractor that retrieves an [`ojs::Client`] from the application state.
///
/// # Example
///
/// ```rust,no_run
/// use ojs_axum::OjsClient;
/// use serde_json::json;
///
/// async fn handler(ojs: OjsClient) -> String {
///     match ojs.enqueue("ping", json!({})).await {
///         Ok(j) => j.id.to_string(),
///         Err(e) => e.to_string(),
///     }
/// }
/// ```
pub struct OjsClient(ojs::Client);

impl OjsClient {
    /// Get a reference to the inner client.
    pub fn inner(&self) -> &ojs::Client {
        &self.0
    }

    /// Consume this wrapper and return the inner client.
    pub fn into_inner(self) -> ojs::Client {
        self.0
    }
}

impl std::ops::Deref for OjsClient {
    type Target = ojs::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for OjsClient
where
    ojs::Client: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(OjsClient(ojs::Client::from_ref(state)))
    }
}
