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
    Connection(#[from] crate::protocol::ConnectionError),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Session already exists
    #[error("Session already exists: {0}")]
    SessionAlreadyExists(String),

    /// Session initialization error
    #[error("Session initialization error: {0}")]
    SessionInitialization(String),

    /// Message handling error
    #[error("Message handling error: {0}")]
    MessageHandling(String),

    /// PTY creation error
    #[error("PTY creation error: {0}")]
    PtyCreation(String),

    /// Resource cleanup error
    #[error("Resource cleanup error: {0}")]
    ResourceCleanup(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    /// Other error
    #[error("Other error: {0}")]
    Other(String),
}
