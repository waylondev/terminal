/// Configuration data structures for rs_terminal
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Terminal configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TerminalConfig {
    /// Default shell type
    pub default_shell_type: String,
    
    /// Default terminal dimensions
    pub default_size: TerminalSize,
    
    /// Default working directory
    pub default_working_directory: PathBuf,
    
    /// Session timeout in milliseconds (default: 30 minutes)
    pub session_timeout: u64,
    
    /// Shell configurations
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

/// Shell configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ShellConfig {
    /// Command to execute
    pub command: Vec<String>,
    
    /// Working directory (optional)
    pub working_directory: Option<PathBuf>,
    
    /// Terminal size (optional)
    pub size: Option<TerminalSize>,
    
    /// Environment variables
    pub environment: std::collections::HashMap<String, String>,
}

// 删除了硬编码的默认配置，所有配置必须从配置文件读取
