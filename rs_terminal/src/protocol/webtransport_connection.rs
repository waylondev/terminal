/// WebTransport connection implementation for TerminalConnection trait
use std::fmt::Debug;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::protocol::{ConnectionType, TerminalConnection, TerminalMessage};

/// WebTransport connection implementation that implements TerminalConnection trait
/// This follows the same pattern as WebSocketConnection
pub struct WebTransportConnection {
    pub id: String,
    // WebTransport connection wrapped in Arc<Mutex> for thread safety
    connection: Arc<Mutex<Option<wtransport::Connection>>>,
    // Bidirectional stream for communication
    stream: Arc<Mutex<Option<wtransport::stream::BiStream>>>,
}

impl Debug for WebTransportConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebTransportConnection")
            .field("id", &self.id)
            .finish()
    }
}

impl WebTransportConnection {
    /// Create a new WebTransport connection
    pub fn new(id: String) -> Self {
        Self {
            id,
            connection: Arc::new(Mutex::new(None)),
            stream: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the WebTransport connection
    pub async fn set_connection(&self, connection: wtransport::Connection) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn_guard = self.connection.lock().await;
        *conn_guard = Some(connection);
        
        // Create a bidirectional stream
        let conn = conn_guard.as_ref().unwrap();
        let opening_stream = conn.open_bi().await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        let stream = opening_stream.await.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        
        let mut stream_guard = self.stream.lock().await;
        *stream_guard = Some(stream.into());
        
        info!("WebTransport connection established for session: {}", self.id);
        Ok(())
    }
}

#[async_trait::async_trait]
impl TerminalConnection for WebTransportConnection {
    async fn send_text(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error + Send>> {
        let stream_guard = self.stream.lock().await;
        if let Some(ref stream) = *stream_guard {
            // For wtransport 0.6, we need to use a different approach for sending data
            // The bidirectional stream doesn't have a split method in this version
            // We'll need to use the connection directly or find the correct API
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Unsupported, "WebTransport send_text not implemented yet")));
        } else {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotConnected, "WebTransport stream not available")));
        }
    }

    async fn send_binary(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send>> {
        let stream_guard = self.stream.lock().await;
        if let Some(ref stream) = *stream_guard {
            // For wtransport 0.6, we need to use a different approach for sending data
            // The bidirectional stream doesn't have a split method in this version
            // We'll need to use the connection directly or find the correct API
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Unsupported, "WebTransport send_binary not implemented yet")));
        } else {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::NotConnected, "WebTransport stream not available")));
        }
    }

    async fn receive(
        &mut self,
    ) -> Option<Result<TerminalMessage, Box<dyn std::error::Error + Send>>> {
        let stream_guard = self.stream.lock().await;
        if let Some(ref stream) = *stream_guard {
            // For wtransport 0.6, we need to use a different approach for receiving data
            // The bidirectional stream doesn't have a split method in this version
            // We'll need to use the connection directly or find the correct API
            error!("WebTransport receive not implemented yet");
            None
        } else {
            // No stream available, wait a bit before checking again
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            None
        }
    }

    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("Closing WebTransport connection: {}", self.id);
        
        // Close the stream
        let mut stream_guard = self.stream.lock().await;
        if let Some(_stream) = stream_guard.take() {
            // For wtransport 0.6, we need to use a different approach for closing streams
            // The bidirectional stream doesn't have a split method in this version
            debug!("WebTransport stream closed");
        }
        
        // Close the connection
        let mut conn_guard = self.connection.lock().await;
        if let Some(conn) = conn_guard.take() {
            // Use the correct API for closing WebTransport connections
            conn.close(0u32.into(), &[]);
        }
        
        info!("WebTransport connection closed: {}", self.id);
        Ok(())
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::WebTransport
    }
}