use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio::net::TcpStream;

use crate::error::{Result, Error};
use crate::terminal::{read_line, display_message};

/// WebSocket client for terminal applications
pub struct WebSocketClient {
    /// WebSocket server URL
    url: String,
    /// WebSocket stream
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl WebSocketClient {
    /// Create a new WebSocket client
    pub async fn new(url: &str) -> Result<Self> {
        tracing::info!("Creating WebSocket client for URL: {}", url);
        
        Ok(Self {
            url: url.to_string(),
            stream: None,
        })
    }
    
    /// Connect to the WebSocket server
    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!("Connecting to WebSocket server at: {}", self.url);
        
        // Parse the URL and create a client request
        let request = self.url.clone().into_client_request()
            .map_err(|e| Error::InvalidUrl(e.to_string()))?;
        
        // Connect to the server
        let (stream, response) = connect_async(request).await?;
        
        tracing::info!("Connected to server! Response status: {:?}", response.status());
        tracing::debug!("Response headers: {:?}", response.headers());
        
        self.stream = Some(stream);
        Ok(())
    }
    
    /// Disconnect from the WebSocket server
    #[allow(dead_code)]
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            tracing::info!("Disconnecting from WebSocket server...");
            stream.close(None).await?;
        }
        Ok(())
    }
    
    /// Run the WebSocket client main loop
    pub async fn run(&mut self) -> Result<()> {
        // Connect to the server
        self.connect().await?;
        
        // Get the stream
        let stream = self.stream.take().ok_or_else(|| {
            Error::Custom("WebSocket stream not available".to_string())
        })?;
        
        // Split the stream into read and write halves
        let (mut write, mut read) = stream.split();
        
        // Spawn a task to read messages from the server
        let read_task = tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        tracing::info!("Received from server: {}", text);
                        display_message(&text);
                    },
                    Ok(Message::Binary(bin)) => {
                        tracing::debug!("Received binary message, length: {}", bin.len());
                        display_message(&format!("Received binary data: {} bytes", bin.len()));
                    },
                    Ok(Message::Ping(_ping)) => {
                        tracing::debug!("Received ping from server");
                    },
                    Ok(Message::Pong(_)) => {
                        tracing::debug!("Received pong from server");
                    },
                    Ok(Message::Close(frame)) => {
                        if let Some(frame) = frame {
                            tracing::info!("Received close frame: code={}, reason={}", frame.code, frame.reason);
                        } else {
                            tracing::info!("Received close frame");
                        }
                        break;
                    },
                    Ok(Message::Frame(frame)) => {
                        tracing::debug!("Received raw frame: {:?}", frame);
                    },
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    },
                }
            }
        });
        
        // Main write loop
        let write_task = tokio::spawn(async move {
            loop {
                // Read input from stdin
                let input = match read_line("Enter message (or /quit to exit): ") {
                    Ok(input) => input,
                    Err(e) => {
                        tracing::error!("IO error: {}", e);
                        continue;
                    },
                };
                
                // Check for quit command
                if input == "/quit" {
                    tracing::info!("Closing connection...");
                    if let Err(e) = write.send(Message::Close(None)).await {
                        tracing::error!("Failed to send close message: {}", e);
                    }
                    break;
                }
                
                // Check for empty input
                if input.is_empty() {
                    continue;
                }
                
                // Send the message to the server
                if let Err(e) = write.send(Message::Text(input.clone())).await {
                    tracing::error!("Failed to send message: {}", e);
                    break;
                }
                
                tracing::info!("Sent message: {}", input);
            }
        });
        
        // Wait for either task to complete
        tokio::select! {
            _ = read_task => tracing::info!("Read task completed"),
            _ = write_task => tracing::info!("Write task completed"),
        }
        
        Ok(())
    }
}

impl Drop for WebSocketClient {
    /// Ensure the connection is closed when the client is dropped
    fn drop(&mut self) {
        // Note: We can't use async in drop, so we just log a message
        if self.stream.is_some() {
            tracing::info!("WebSocket client dropped, connection will be closed");
        }
    }
}
