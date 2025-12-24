use anyhow::Result;
use arboard::Clipboard;
use clap::Parser;
use reqwest::Client;
use std::time::Duration;
use tokio::time::interval;

// Client configuration
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct ClientConfig {
    #[clap(short, long, required = true)]
    http_address: String,

    #[clap(short, long, default_value = "/file")]
    endpoint: String,

    #[clap(short, long, default_value = "5")]
    interval: u64,

    #[clap(short = 'f', long, default_value = "content.txt")]
    file_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let config = ClientConfig::parse();
    
    // Build full URL
    let url = format!("{}{}", config.http_address, config.endpoint);
    
    // Create HTTP client
    let client = Client::new();
    
    // Create clipboard
    let mut clipboard = Clipboard::new()?;
    
    // Create interval timer
    let mut interval = interval(Duration::from_secs(config.interval));
    
    println!("Client starting with configuration:");
    println!("  HTTP Address: {}", config.http_address);
    println!("  Endpoint: {}", config.endpoint);
    println!("  Update Interval: {} seconds", config.interval);
    println!();
    println!("Press Ctrl+C to exit.");
    println!();
    
    // Main loop
    loop {
        // Wait for next interval
        interval.tick().await;
        
        // Prepare request body with file_path from config
        let request_body = serde_json::json!({ "file_path": &config.file_path });
        
        // Fetch file content using POST
        match client.post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(content) => {
                            // Copy to clipboard
                            if let Err(e) = clipboard.set_text(content.clone()) {
                                eprintln!("Failed to copy to clipboard: {}", e);
                                continue;
                            }
                            
                            println!("✓ Updated clipboard at {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
                        }
                        Err(e) => {
                            eprintln!("Failed to read response text: {}", e);
                        }
                    }
                } else {
                    eprintln!("Server returned error: {}", response.status());
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch file content: {}", e);
            }
        }
    }
}
