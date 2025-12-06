/// PTY manager for managing PTY instances
use crate::pty::{self, AsyncPty, PtyError};
use crate::config::TerminalConfig;
use tracing::{info, error};

/// PTY manager responsible for managing PTY instances
pub struct PtyManager;

impl PtyManager {
    /// Create a new PTY manager
    pub fn new() -> Self {
        Self
    }

    /// Create a new PTY instance using application configuration
    pub async fn create_pty_from_config(&self, config: &TerminalConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
        match pty::create_pty_from_config(config).await {
            Ok(pty) => {
                info!("Created new PTY instance from configuration");
                Ok(pty)
            },
            Err(e) => {
                error!("Failed to create PTY from configuration: {}", e);
                Err(e)
            }
        }
    }

    /// Kill a PTY instance
    pub async fn kill_pty(&self, pty: &mut Box<dyn AsyncPty>) -> Result<(), PtyError> {
        match pty.kill().await {
            Ok(_) => {
                info!("PTY killed successfully");
                Ok(())
            },
            Err(e) => {
                error!("Failed to kill PTY: {}", e);
                Err(e)
            }
        }
    }

    /// Check if a PTY is alive
    pub fn is_pty_alive(&self, pty: &Box<dyn AsyncPty>) -> bool {
        pty.is_alive()
    }
}
