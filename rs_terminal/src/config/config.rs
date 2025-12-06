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
    
    /// Shell configurations including default
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
    /// Command to execute (optional, defaults to shells.default.command)
    pub command: Option<Vec<String>>,
    
    /// Working directory (optional, defaults to shells.default.working_directory)
    pub working_directory: Option<PathBuf>,
    
    /// Terminal size (optional, defaults to shells.default.size)
    pub size: Option<TerminalSize>,
    
    /// Environment variables (optional, defaults to shells.default.environment)
    pub environment: Option<std::collections::HashMap<String, String>>,
}

// 删除了硬编码的默认配置，所有配置必须从配置文件读取
