/// Error types for the service layer
use thiserror::Error;

/// Service layer error type
#[derive(Error, Debug)]
pub enum ServiceError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// PTY error
    #[error("PTY error: {0}")]
    Pty(#[from] crate::pty::PtyError),
    
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(Box<dyn std::error::Error + Send + 'static>),
    
    /// Message handling error
    #[error("Message handling error: {0}")]
    MessageHandling(String),
    
    /// Session error
    #[error("Session error: {0}")]
    Session(String),
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}

impl From<Box<dyn std::error::Error + Send + 'static>> for ServiceError {
    fn from(e: Box<dyn std::error::Error + Send + 'static>) -> Self {
        ServiceError::Connection(e)
    }
}
