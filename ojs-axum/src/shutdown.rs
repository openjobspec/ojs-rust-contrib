/// Returns a future that completes when a shutdown signal is received.
///
/// Listens for `SIGINT` (Ctrl+C) and `SIGTERM` on Unix platforms. Suitable
/// for passing to [`axum::serve::Serve::with_graceful_shutdown`].
///
/// # Example
///
/// ```rust,no_run
/// use ojs_axum::shutdown_signal;
///
/// # async fn example() {
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
/// let app = axum::Router::new();
/// axum::serve(listener, app)
///     .with_graceful_shutdown(shutdown_signal())
///     .await
///     .unwrap();
/// # }
/// ```
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, starting graceful shutdown");
}
