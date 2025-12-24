use anyhow::Result;
use axum::{response::IntoResponse, routing::post, Extension, Router};
use clap::Parser;
use std::fs::read_to_string;
use std::sync::Arc;

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
    axum::Json(request): axum::Json<FileRequest>
) -> impl IntoResponse {
    // Use file path from request body if provided, otherwise use default
    let file_path = request.file_path.as_ref().unwrap_or(&state.file_path);
    
    let result = read_to_string(file_path);
    result.unwrap_or_else(|_| format!("Failed to read file: {}", file_path))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let config = ServerConfig::parse();
    
    // Build socket address and print info before moving config.file_path
    let addr = format!("{}:{}", config.host, config.port);
    println!("Server listening on http://{}", addr);
    println!("Serving file: {}", config.file_path);
    
    // Create app state after printing
    let state = Arc::new(AppState {
        file_path: config.file_path,
    });
    
    // Create router with handlers
    let app = Router::new()
        .route("/file", post(get_file_content))
        .layer(Extension(state));
    
    // Use tokio::net::TcpListener to bind and serve
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
