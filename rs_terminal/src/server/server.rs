/// Server implementation for Waylon Terminal Rust backend
use std::net::SocketAddr;

use axum::{
    Router,
    http::{self, Method},
    routing::{delete, get, post},
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::{app_state::AppState, handlers};

/// Start WebTransport server in a separate task
pub fn start_webtransport_service(state: AppState) {
    let webtransport_addr = SocketAddr::from(([0, 0, 0, 0], 8082));
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
            Method::from_bytes(b"UPGRADE").unwrap(),
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
pub async fn run_server(router: Router) {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let webtransport_addr = SocketAddr::from(([0, 0, 0, 0], 8082));

    let listener = TcpListener::bind(addr).await.unwrap();

    info!("Server running on http://{}", addr);
    info!("WebSocket server available at ws://{}/ws", addr);
    info!(
        "WebTransport server available at https://{}",
        webtransport_addr
    );

    axum::serve(listener, router).await.unwrap();
}
