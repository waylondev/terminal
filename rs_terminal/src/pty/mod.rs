/// PTY (Pseudo Terminal) handling for Waylon Terminal
/// This module provides a trait abstraction for different PTY implementations
/// with a focus on pure async operations

mod pty_trait;
#[cfg(unix)]
mod unix_pty_impl;
#[cfg(windows)]
mod windows_pty_impl;
mod memory_pty;

// Export all public types and traits
pub use pty_trait::*;
#[cfg(unix)]
pub use unix_pty_impl::{UnixPty, UnixPtyFactory};
#[cfg(windows)]
pub use windows_pty_impl::{WindowsPty, WindowsPtyFactory};
pub use memory_pty::{MemoryPty, MemoryPtyFactory};

/// Create a new PTY instance with default configuration
/// This function returns a pure async PTY implementation
pub async fn create_pty() -> Result<Box<dyn AsyncPty>, PtyError> {
    // 创建配置
    let config = PtyConfig {
        command: "bash".to_string(),
        args: vec![],
        cols: 80,
        rows: 24,
        env: vec![
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("COLORTERM".to_string(), "truecolor".to_string()),
        ],
        cwd: None,
    };

    // 根据平台选择不同的PTY实现
    #[cfg(unix)]
    {
        let factory = UnixPtyFactory::default();
        let pty = factory.create(&config).await?;
        Ok(pty)
    }
    #[cfg(windows)]
    {
        // 首先尝试WindowsPty，如果不可用则回退到MemoryPty
        let factory = WindowsPtyFactory::default();
        match factory.create(&config).await {
            Ok(pty) => Ok(pty),
            Err(_) => {
                let factory = MemoryPtyFactory::default();
                let pty = factory.create(&config).await?;
                Ok(pty)
            }
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let factory = MemoryPtyFactory::default();
        let pty = factory.create(&config).await?;
        Ok(pty)
    }
}

/// Create a new PTY instance using configuration from the application config
pub async fn create_pty_from_config(app_config: &crate::config::TerminalConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
    // Get default shell configuration
    let default_shell_type = &app_config.default_shell_type;
    let shell_config = match app_config.shells.get(default_shell_type) {
        Some(config) => config,
        None => {
            // If default shell is not found, try bash
            match app_config.shells.get("bash") {
                Some(config) => config,
                None => {
                    return Err(PtyError::Other(format!("No shell configuration found for default shell: {}", default_shell_type)));
                }
            }
        }
    };
    
    // Get default shell configuration as fallback
    let default_shell_config = match app_config.shells.get("default") {
        Some(config) => config,
        None => {
            return Err(PtyError::Other("No default shell configuration found in shells.default".to_string()));
        }
    };
    
    // Extract command and arguments from shell config (command is required for each shell)
    let command = shell_config.command[0].clone();
    let args: Vec<String> = shell_config.command.iter().skip(1).cloned().collect();
    
    // Determine working directory with priority: shell_config.working_directory > default_shell_config.working_directory
    let working_directory = shell_config.working_directory.clone()
        .or_else(|| default_shell_config.working_directory.clone());
    
    // Determine terminal size with priority: shell_config.size > default_shell_config.size
    let terminal_size = shell_config.size.as_ref().unwrap_or_else(|| {
        default_shell_config.size.as_ref().expect("No terminal size configured in shells.default")
    }).clone();
    
    // Determine environment variables with priority: shell_config.environment > default_shell_config.environment
    let mut environment = Vec::new();
    
    // Add default environment variables from shells.default
    if let Some(default_env) = &default_shell_config.environment {
        environment.reserve(default_env.len());
        for (key, value) in default_env {
            environment.push((key.clone(), value.clone()));
        }
    }
    
    // Add explicit environment variables from shell config, overwriting defaults
    if let Some(shell_env) = &shell_config.environment {
        environment.reserve(environment.len() + shell_env.len());
        for (key, value) in shell_env {
            // Check if the key already exists, if so, replace it
            if let Some(index) = environment.iter().position(|(k, _)| k == key) {
                environment[index] = (key.clone(), value.clone());
            } else {
                environment.push((key.clone(), value.clone()));
            }
        }
    }
    
    // Create PTY config
    let pty_config = PtyConfig {
        command: command,
        args: args,
        cols: terminal_size.columns,
        rows: terminal_size.rows,
        env: environment,
        cwd: working_directory,
    };
    
    // 根据平台选择不同的PTY实现
    #[cfg(unix)]
    {
        let factory = UnixPtyFactory::default();
        let pty = factory.create(&pty_config).await?;
        Ok(pty)
    }
    #[cfg(windows)]
    {
        // 首先尝试WindowsPty，如果不可用则回退到MemoryPty
        let factory = WindowsPtyFactory::default();
        match factory.create(&pty_config).await {
            Ok(pty) => Ok(pty),
            Err(e) => {
                tracing::info!("Falling back to MemoryPty because WindowsPty failed: {}", e);
                let factory = MemoryPtyFactory::default();
                let pty = factory.create(&pty_config).await?;
                Ok(pty)
            }
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let factory = MemoryPtyFactory::default();
        let pty = factory.create(&pty_config).await?;
        Ok(pty)
    }
}

/// Create a new PTY instance with custom configuration
pub async fn create_pty_with_config(config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
    #[cfg(unix)]
    return UnixPtyFactory::default().create(config).await;
    
    #[cfg(windows)]
    {
        // 首先尝试WindowsPty，如果不可用则回退到MemoryPty
        let factory = WindowsPtyFactory::default();
        match factory.create(config).await {
            Ok(pty) => Ok(pty),
            Err(_) => MemoryPtyFactory::default().create(config).await,
        }
    }
    
    #[cfg(not(any(unix, windows)))]
    return MemoryPtyFactory::default().create(config).await;
}

/// Create a new PTY instance using a specific factory
pub async fn create_pty_with_factory(
    factory: &dyn PtyFactory,
    config: &PtyConfig
) -> Result<Box<dyn AsyncPty>, PtyError> {
    factory.create(config).await
}
