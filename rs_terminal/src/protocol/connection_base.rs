/// Base connection implementation for TerminalConnection trait
use std::fmt::Debug;
use tracing::info;

use super::{ConnectionType, TerminalConnection, TerminalMessage};

/// Base connection struct that implements common functionality for all connection types
#[derive(Debug)]
pub struct ConnectionBase<T: Debug + Send> {
    /// Connection ID
    pub id: String,
    /// Connection type
    pub conn_type: ConnectionType,
    /// Inner connection implementation
    pub inner: T,
}

impl<T: Debug + Send> ConnectionBase<T> {
    /// Create a new connection base
    pub fn new(id: String, conn_type: ConnectionType, inner: T) -> Self {
        Self {
            id,
            conn_type,
            inner,
        }
    }
    
    /// Get the inner connection
    pub fn inner(&self) -> &T {
        &self.inner
    }
    
    /// Get mutable reference to inner connection
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

#[async_trait::async_trait]
impl<T: Debug + Send> TerminalConnection for ConnectionBase<T>
where
    Self: Send + Debug,
{
    async fn send_text(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("ConnectionBase send_text called: {}", message);
        info!("Connection type: {:?}", self.conn_type);
        // This method should be implemented by the specific connection type
        // For now, we'll just return an error indicating this method is not implemented
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "send_text not implemented for this connection type",
        )))
    }

    async fn send_binary(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("ConnectionBase send_binary called with {} bytes", data.len());
        info!("Connection type: {:?}", self.conn_type);
        // This method should be implemented by the specific connection type
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "send_binary not implemented for this connection type",
        )))
    }

    async fn receive(&mut self) -> Option<Result<TerminalMessage, Box<dyn std::error::Error + Send>>> {
        info!("ConnectionBase receive called");
        info!("Connection type: {:?}", self.conn_type);
        // This method should be implemented by the specific connection type
        None
    }

    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("ConnectionBase close called");
        info!("Connection type: {:?}", self.conn_type);
        // This method should be implemented by the specific connection type
        Ok(())
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn connection_type(&self) -> ConnectionType {
        self.conn_type
    }
}