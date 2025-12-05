/// Terminal connection trait for abstracting different transport protocols
use std::fmt::Debug;

/// Terminal connection trait that defines common capabilities for all transport protocols
#[async_trait::async_trait]
pub trait TerminalConnection: Send + Debug {
    /// Send a text message over the connection
    async fn send_text(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error + Send>>;
    
    /// Send a binary message over the connection
    async fn send_binary(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send>>;
    
    /// Receive a message from the connection
    /// Returns None when the connection is closed
    async fn receive(&mut self) -> Option<Result<TerminalMessage, Box<dyn std::error::Error + Send>>>;
    
    /// Close the connection
    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error + Send>>;
    
    /// Get the connection ID
    fn id(&self) -> &str;
    
    /// Get the connection type
    fn connection_type(&self) -> ConnectionType;
}

/// Terminal message types
#[derive(Debug, Clone)]
pub enum TerminalMessage {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(()),
    /// Close message
    Close,
}

/// Connection types
#[derive(Debug, Clone, Copy)]
pub enum ConnectionType {
    /// WebSocket connection
    WebSocket,
    /// WebTransport connection
    WebTransport,
}
