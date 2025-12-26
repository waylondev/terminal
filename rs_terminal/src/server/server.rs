/// Server implementation for Waylon Terminal Rust backend
use std::net::SocketAddr;

use axum::{
    Router,
    http::Method,
    routing::{delete, get, post},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::{app_state::AppState, handlers};
use tokio::signal;
use std::time::Duration;

/// Start WebTransport server in a separate task
pub fn start_webtransport_service(state: AppState) {
    let webtransport_addr = SocketAddr::from(([0, 0, 0, 0], state.config.webtransport_port));
    let webtransport_state = state.clone();
    tokio::spawn(async move {
        crate::handlers::webtransport::start_webtransport_server(
            webtransport_addr,
            webtransport_state,
        )
        .await;
    });
}

/// Build the application router with routes
pub fn build_router(state: AppState) -> Router {
    // Create CORS layer to allow cross-origin requests
    let cors = CorsLayer::new()
        // Allow all origins
        .allow_origin(Any)
        // Allow specific HTTP methods instead of wildcard
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
            // WebSocket upgrade method
            // 使用更安全的错误处理，避免panic
            Method::from_bytes(b"UPGRADE").unwrap_or_else(|_| Method::OPTIONS),
        ])
        // Allow all headers (now allowed since we're not using credentials)
        .allow_headers(Any);
    // Removed allow_credentials(true) to comply with CORS spec
    // When allow_credentials is true, you can't use wildcard for origin or headers

    Router::new()
        // Health check endpoint
        .route("/", get(|| async { "Waylon Terminal - Rust Backend" }))
        .route("/health", get(handlers::rest::health_check))
        // WebSocket endpoints for terminal communication
        // Support both /ws and /ws/:session_id formats
        .route("/ws", get(handlers::websocket::websocket_handler))
        .route(
            "/ws/:session_id",
            get(handlers::websocket::websocket_handler_with_id),
        )
        // REST API endpoints for session management
        .nest("/api", api_routes())
        // Add CORS middleware layer
        .layer(cors)
        .with_state(state)
}

/// Build API routes for session management
fn api_routes() -> Router<AppState> {
    Router::new()
        // Session management endpoints
        .route("/sessions", post(handlers::rest::create_session))
        .route("/sessions", get(handlers::rest::get_all_sessions))
        .route("/sessions/:session_id", get(handlers::rest::get_session))
        .route(
            "/sessions/:session_id/resize",
            post(handlers::rest::resize_session),
        )
        .route(
            "/sessions/:session_id",
            delete(handlers::rest::terminate_session),
        )
}

/// Run the HTTP server
pub async fn run_server(
    router: Router,
    config: &crate::config::TerminalConfig,
) -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    let webtransport_addr = SocketAddr::from(([0, 0, 0, 0], config.webtransport_port));

    let listener = TcpListener::bind(addr).await?;

    info!("Server running on http://{}", addr);
    info!("WebSocket server available at ws://{}/ws", addr);
    info!(
        "WebTransport server available at https://{}",
        webtransport_addr
    );

    axum::serve(listener, router).await?;
    Ok(())
}

/// Run the HTTP server with graceful shutdown support
pub async fn run_server_with_graceful_shutdown(
    router: Router,
    config: &crate::config::TerminalConfig,
) -> Result<(), std::io::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], config.http_port));
    let webtransport_addr = SocketAddr::from(([0, 0, 0, 0], config.webtransport_port));

    let listener = TcpListener::bind(addr).await?;

    info!("Server running on http://{}", addr);
    info!("WebSocket server available at ws://{}/ws", addr);
    info!(
        "WebTransport server available at https://{}",
        webtransport_addr
    );

    // Create graceful shutdown signal
    let graceful_shutdown = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            info!("Received Ctrl+C signal, initiating graceful shutdown...");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
            info!("Received SIGTERM signal, initiating graceful shutdown...");
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    };

    // Run server with graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(graceful_shutdown)
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
