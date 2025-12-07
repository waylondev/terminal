use serde::Deserialize;
use std::fs;
use std::path::Path;

use crate::error::{Result, Error};

/// Server configuration
#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    /// WebSocket server URL
    pub url: String,
}

/// Main configuration structure
#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                url: "ws://localhost:8080/ws".to_string(),
            },
        }
    }
}

impl Config {
    /// Load configuration from file or use default values
    pub fn load(config_path: Option<String>) -> Result<Self> {
        match config_path {
            Some(path) => {
                // Load from specified file
                Self::from_file(&path)
            },
            None => {
                // Try to load from default location, otherwise use defaults
                let default_path = "client.toml";
                if Path::new(default_path).exists() {
                    Self::from_file(default_path)
                } else {
                    Ok(Self::default())
                }
            },
        }
    }
    
    /// Load configuration from a specific file
    pub fn from_file(path: &str) -> Result<Self> {
        tracing::info!("Loading configuration from file: {}", path);
        
        // Read the file
        let content = fs::read_to_string(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Error::FileNotFound(path.to_string()),
                _ => Error::Io(e),
            })?;
        
        // Parse the TOML content
        let config = toml::from_str(&content)?;
        
        tracing::debug!("Loaded configuration: {:?}", config);
        Ok(config)
    }
}
