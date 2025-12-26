/// Main entry point for Waylon Terminal Rust backend
// Import modules
mod api;
mod app_state;
mod config;
mod handlers;
mod protocol;
mod pty;
mod server;
mod service;

// Use public API from modules
use app_state::AppState;
use config::{ConfigLoader, init_logging};
use server::{build_router, run_server_with_graceful_shutdown, start_webtransport_service};

#[tokio::main]
async fn main() {
    // Initialize logging
    init_logging();

    // Load configuration
    let config_loader = ConfigLoader::new();
    let config = match config_loader.load_config(None) {
        // Use None for default path
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Create application state with configuration
    let app_state = AppState::new(config.clone());

    // Start WebTransport service
    start_webtransport_service(app_state.clone());

    // Build router and run server with graceful shutdown
    let app = build_router(app_state);
    if let Err(e) = run_server_with_graceful_shutdown(app, &config).await {
        eprintln!("Failed to run server: {}", e);
        std::process::exit(1);
    }
}
