use anyhow::Result;
use arboard::Clipboard;
use clap::Parser;
use reqwest::Client;
use std::time::Duration;
use tokio::signal;
use tokio::sync::oneshot;
use tokio::time::interval;

// Client configuration
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ClientConfig {
    #[clap(short = 'a', long, default_value = "http://localhost:3000")]
    pub http_address: String,

    #[clap(short, long, default_value = "/file")]
    pub endpoint: String,

    #[clap(short, long, default_value = "5")]
    pub interval: u64,

    #[clap(short = 'f', long, default_value = "content.txt")]
    pub file_path: String,
}

/// Build full URL from base address and endpoint
fn build_url(config: &ClientConfig) -> String {
    format!("{}{}", config.http_address, config.endpoint)
}

/// Print client configuration
fn print_config(config: &ClientConfig) {
    println!("Client starting with configuration:");
    println!("  HTTP Address: {}", config.http_address);
    println!("  Endpoint: {}", config.endpoint);
    println!("  Update Interval: {} seconds", config.interval);
    println!("  File Path: {}", config.file_path);
    println!();
    println!("Press Ctrl+C to gracefully exit.");
    println!();
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn wait_for_shutdown() -> Result<()> {
    // Wait for either Ctrl+C or SIGTERM signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("\nReceived Ctrl+C, shutting down...");
        },
        _ = async {
            // Only listen for SIGTERM on Unix systems
            #[cfg(unix)]
            {
                if let Ok(mut sigterm) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
                    sigterm.recv().await;
                    println!("\nReceived SIGTERM, shutting down...");
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

/// Run the main client loop with interval updates
async fn run_client_loop(
    config: &ClientConfig,
    client: &Client,
    url: &str,
    mut clipboard: Clipboard,
    shutdown_rx: &mut oneshot::Receiver<()>,
) -> Result<()> {
    let mut interval = interval(Duration::from_secs(config.interval));

    loop {
        tokio::select! {
            // Wait for next interval
            _ = interval.tick() => {
                println!("[CLIENT] Fetching content from: {} (file: {})", url, config.file_path);

                // Prepare request body with file_path from config
                let request_body = serde_json::json!({ "file_path": &config.file_path });

                // Fetch file content using POST
                match client.post(url)
                    .header("Content-Type", "application/json")
                    .json(&request_body)
                    .send().await {
                    Ok(response) => {
                        let status = response.status();
                        println!("[CLIENT] Received response: {}", status);

                        if status.is_success() {
                            match response.text().await {
                                Ok(content) => {
                                    println!("[CLIENT] Content received: {} bytes", content.len());

                                    // Copy to clipboard
                                    if let Err(e) = clipboard.set_text(content.clone()) {
                                        eprintln!("[CLIENT] ❌ Failed to copy to clipboard: {}", e);
                                        continue;
                                    }

                                    println!("[CLIENT] ✓ Clipboard updated at {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
                                }
                                Err(e) => {
                                    eprintln!("[CLIENT] ❌ Failed to read response text: {}", e);
                                }
                            }
                        } else {
                            eprintln!("[CLIENT] ❌ Server returned error: {}", status);
                        }
                    }
                    Err(e) => {
                        eprintln!("[CLIENT] ❌ Failed to connect to server: {}", e);
                        eprintln!("[CLIENT] Make sure the server is running at: {}", url);
                    }
                }

                println!("[CLIENT] Next update in {} seconds...\n", config.interval);
            },
            // Wait for shutdown signal
            _ = &mut *shutdown_rx => {
                println!("\n[CLIENT] Received shutdown signal...");
                println!("[CLIENT] Shutting down client...");
                break;
            }
        }
    }

    Ok(())
}

/// Main client run function
async fn run_client(config: ClientConfig) -> Result<()> {
    // Build URL and print config
    let url = build_url(&config);
    print_config(&config);

    // Create HTTP client and clipboard
    let client = Client::new();
    let clipboard = Clipboard::new()?;

    // Create shutdown channel
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

    // Spawn task to wait for shutdown signal
    tokio::spawn(async move {
        let _ = wait_for_shutdown().await;
        let _ = shutdown_tx.send(());
    });

    // Run main client loop
    run_client_loop(&config, &client, &url, clipboard, &mut shutdown_rx).await?;

    println!("Client gracefully exited.");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let config = ClientConfig::parse();

    // Run the client
    run_client(config).await
}
