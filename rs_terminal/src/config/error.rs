use std::io::Error as IoError;
/// Error types for configuration module
use thiserror::Error;
use toml::de::Error as TomlDeError;

/// Configuration error type
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Failed to open configuration file
    #[error("Failed to open configuration file: {0}")]
    FileOpenError(#[from] IoError),

    /// Failed to parse configuration file
    #[error("Failed to parse configuration file: {0}")]
    ParseError(#[from] TomlDeError),

    /// Configuration file not found
    #[error("Configuration file not found at: {0}")]
    FileNotFound(String),

    /// Invalid configuration structure
    #[error("Invalid configuration structure: {0}")]
    InvalidStructure(String),

    /// Default shell configuration not found
    #[error("Default shell configuration not found in shells.default")]
    DefaultShellConfigNotFound,

    /// Shell configuration not found
    #[error("Shell configuration not found for: {0}")]
    ShellConfigNotFound(String),
}
