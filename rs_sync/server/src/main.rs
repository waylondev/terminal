use anyhow::Result;
use axum::{Extension, Router, response::IntoResponse, routing::post};
use chrono::Local;
use clap::Parser;
use std::fs::read_to_string;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::oneshot;

// Server configuration
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct ServerConfig {
    #[clap(short = 's', long, default_value = "127.0.0.1")]
    host: String,

    #[clap(short, long, default_value = "3000")]
    port: u16,

    #[clap(short, long, default_value = "content.txt")]
    file_path: String,
}

// App state containing the file content
#[derive(Clone)]
struct AppState {
    file_path: String,
}

// Request body structure for file path
#[derive(serde::Deserialize)]
struct FileRequest {
    file_path: Option<String>,
}

// Handler for the file content endpoint
async fn get_file_content(
    Extension(state): Extension<Arc<AppState>>,
    axum::Json(request): axum::Json<FileRequest>,
) -> impl IntoResponse {
    // Use file path from request body if provided, otherwise use default
    let file_path = request.file_path.as_ref().unwrap_or(&state.file_path);

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!(
        "[SERVER] {} - Received file request for: {}",
        timestamp, file_path
    );

    let result = read_to_string(file_path);
    match &result {
        Ok(content) => {
            println!(
                "[SERVER] {} - ✓ Successfully served file: {} ({} bytes)",
                timestamp,
                file_path,
                content.len()
            );
        }
        Err(err) => {
            eprintln!(
                "[SERVER] {} - ❌ Error reading file {}: {}",
                timestamp, file_path, err
            );
        }
    }
    println!(); // Add empty line to separate requests

    result.unwrap_or_else(|err| format!("Failed to read file: {} - {}", file_path, err))
}

/// Create and configure the Axum router
fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/file", post(get_file_content))
        .layer(Extension(state))
}

/// Parse the socket address from configuration
fn parse_socket_addr(config: &ServerConfig) -> Result<SocketAddr> {
    let addr_str = format!("{}:{}", config.host, config.port);
    addr_str
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid socket address: {}", e))
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn wait_for_shutdown() -> Result<()> {
    // Wait for either Ctrl+C or SIGTERM signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            println!("\n[SERVER] {} - Received Ctrl+C, shutting down...", timestamp);
        },
        _ = async {
            // Only listen for SIGTERM on Unix systems
            #[cfg(unix)]
            {
                if let Ok(mut sigterm) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
                    sigterm.recv().await;
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    println!("\n[SERVER] {} - Received SIGTERM, shutting down...", timestamp);
                }
            }
            // For Windows, just wait indefinitely
            #[cfg(windows)]
            {
                std::future::pending::<()>().await;
            }
        } => {},
    };
    Ok(())
}

/// Start the server and handle graceful shutdown
async fn run_server(config: ServerConfig) -> Result<()> {
    let addr = parse_socket_addr(&config)?;

    println!("[SERVER] Server listening on http://{}", addr);
    println!("[SERVER] Serving file: {}", config.file_path);
    println!("[SERVER] Press Ctrl+C to gracefully shutdown the server...");
    println!(
        "[SERVER] Server started at {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    // Create app state
    let state = Arc::new(AppState {
        file_path: config.file_path,
    });

    // Create router
    let app = create_router(state);

    // Bind TCP listener
    let listener = TcpListener::bind(addr).await?;

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    // Spawn server task
    let server_handle = tokio::spawn(async move {
        let serve_future = axum::serve(listener, app);

        // Wait for either server completion or shutdown signal
        tokio::select! {
            result = serve_future => {
                if let Err(err) = result {
                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    eprintln!("[SERVER] {} - ❌ Server error: {}", timestamp, err);
                }
            },
            _ = shutdown_rx => {
                let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                println!("[SERVER] {} - Shutting down server...", timestamp);
            }
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown().await?;

    // Send shutdown signal to server task
    let _ = shutdown_tx.send(());

    // Wait for server task to complete
    server_handle.await?;

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("[SERVER] {} - Server gracefully shutdown.", timestamp);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let config = ServerConfig::parse();

    // Run the server
    run_server(config).await
}
