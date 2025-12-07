/// Configuration data structures for rs_terminal
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Terminal configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TerminalConfig {
    /// Default shell type
    pub default_shell_type: String,

    /// Session timeout in milliseconds (default: 30 minutes)
    pub session_timeout: u64,

    /// Default shell configuration (used as fallback for all shells)
    pub default_shell_config: DefaultShellConfig,

    /// Shell configurations (specific shell types)
    pub shells: std::collections::HashMap<String, ShellConfig>,
}

/// Terminal size configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TerminalSize {
    /// Number of columns
    pub columns: u16,

    /// Number of rows
    pub rows: u16,
}

/// Default shell configuration (used as fallback template)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DefaultShellConfig {
    /// Terminal size (required in default config)
    pub size: TerminalSize,

    /// Working directory (optional)
    pub working_directory: Option<PathBuf>,

    /// Environment variables (optional)
    pub environment: Option<std::collections::HashMap<String, String>>,
}

/// Shell configuration for specific shell types
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShellConfig {
    /// Command to execute (required for each shell type)
    pub command: Vec<String>,

    /// Working directory (optional, defaults to default_shell_config.working_directory)
    pub working_directory: Option<PathBuf>,

    /// Terminal size (optional, defaults to default_shell_config.size)
    pub size: Option<TerminalSize>,

    /// Environment variables (optional, defaults to default_shell_config.environment)
    pub environment: Option<std::collections::HashMap<String, String>>,
}

impl TerminalConfig {
    /// Get the complete shell configuration for a given shell type
    /// Priority: shell-specific config > default config
    pub fn get_shell_config(&self, shell_type: &str) -> ResolvedShellConfig {
        // Get the shell-specific configuration if it exists
        let shell_config = self.shells.get(shell_type);

        // Resolve terminal size
        let size = shell_config
            .and_then(|sc| sc.size.clone())
            .unwrap_or(self.default_shell_config.size.clone());

        // Resolve working directory
        let working_directory = shell_config
            .and_then(|sc| sc.working_directory.clone())
            .or_else(|| self.default_shell_config.working_directory.clone());

        // Resolve environment variables
        let environment = shell_config
            .and_then(|sc| sc.environment.clone())
            .or_else(|| self.default_shell_config.environment.clone());

        // Get the command for this shell type (required)
        let command = shell_config
            .map(|sc| sc.command.clone())
            // If no command is found for this shell type, return an empty vector
            .unwrap_or(Vec::new());

        ResolvedShellConfig {
            shell_type: shell_type.to_string(),
            command,
            size,
            working_directory,
            environment,
        }
    }
}

/// Resolved shell configuration with all fields populated
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedShellConfig {
    /// Shell type
    pub shell_type: String,

    /// Command to execute
    pub command: Vec<String>,

    /// Terminal size
    pub size: TerminalSize,

    /// Working directory
    pub working_directory: Option<PathBuf>,

    /// Environment variables
    pub environment: Option<std::collections::HashMap<String, String>>,
}
