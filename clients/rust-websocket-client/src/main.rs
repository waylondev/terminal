mod config;
mod error;
mod logger;
mod terminal;
mod websocket;

use clap::Parser;
use config::Config;
use error::Result;
use logger::init_logging;
use websocket::WebSocketClient;

/// Production-ready Rust WebSocket client for terminal applications
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// WebSocket server URL to connect to
    #[arg(short, long, default_value = "ws://localhost:8080/ws")]
    url: String,
    
    /// Enable debug logging
    #[arg(short, long, default_value_t = false)]
    debug: bool,
    
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();
    
    // Initialize logging
    init_logging(cli.debug)?;
    
    // Load configuration
    let config = Config::load(cli.config)?;
    
    // Use command line URL if provided, otherwise use config
    let url = if !cli.url.is_empty() {
        cli.url
    } else {
        config.server.url.clone()
    };
    
    // Create WebSocket client
    let mut client = WebSocketClient::new(&url).await?;
    
    // Run the client
    client.run().await?;
    
    Ok(())
}