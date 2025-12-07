use thiserror::Error;
use tokio_tungstenite::tungstenite::Error as TungsteniteError;
use tracing_subscriber::filter::ParseError as TracingParseError;
use toml::de::Error as TomlDeError;

/// Result type alias with our custom Error
pub type Result<T> = std::result::Result<T, Error>;

/// Custom error type for the WebSocket client
#[derive(Error, Debug)]
pub enum Error {
    /// WebSocket connection or protocol error
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] TungsteniteError),
    
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Configuration parsing error
    #[error("Config error: {0}")]
    Config(#[from] TomlDeError),
    
    /// Tracing/logging error
    #[error("Logging error: {0}")]
    Logging(#[from] TracingParseError),
    
    /// File not found error
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    /// Invalid URL error
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    
    /// Custom error with message
    #[error("{0}")]
    Custom(String),
}
