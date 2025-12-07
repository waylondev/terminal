#[cfg(feature = "portable-pty")]
mod portable_pty_impl;
/// PTY (Pseudo Terminal) handling for Waylon Terminal
/// This module provides a trait abstraction for different PTY implementations
/// with a focus on pure async operations
mod pty_trait;
mod tokio_process_pty_impl;

// Export all public types and traits
#[cfg(feature = "portable-pty")]
pub use portable_pty_impl::PortablePtyFactory;
pub use pty_trait::*;
pub use tokio_process_pty_impl::TokioProcessPtyFactory;

use tracing::info;

/// Get the PTY factory based on configuration
/// This function allows dynamic selection of PTY implementation from configuration
pub fn get_pty_factory(implementation_name: &str) -> Box<dyn PtyFactory + Send + Sync> {
    match implementation_name.to_lowercase().as_str() {
        #[cfg(feature = "portable-pty")]
        "portable_pty" => {
            info!("Using PortablePtyFactory implementation");
            Box::new(PortablePtyFactory)
        }
        "tokio_process" => {
            info!("Using TokioProcessPtyFactory implementation");
            Box::new(TokioProcessPtyFactory)
        }
        _ => {
            info!("Using default PTY implementation (TokioProcessPtyFactory)");
            Box::new(TokioProcessPtyFactory)
        }
    }
}

/// Create a new PTY instance using configuration from the application config
pub async fn create_pty_from_config(
    app_config: &crate::config::TerminalConfig,
) -> Result<Box<dyn AsyncPty>, PtyError> {
    // Get default shell configuration
    let default_shell_type = &app_config.default_shell_type;
    let shell_config = match app_config.shells.get(default_shell_type) {
        Some(config) => config,
        None => {
            // If default shell is not found, try bash
            match app_config.shells.get("bash") {
                Some(config) => config,
                None => {
                    return Err(PtyError::Other(format!(
                        "No shell configuration found for default shell: {}",
                        default_shell_type
                    )));
                }
            }
        }
    };

    // Extract command and arguments from shell config (command is required for each shell)
    let command = shell_config.command[0].clone();
    let args: Vec<String> = shell_config.command.iter().skip(1).cloned().collect();

    // Determine working directory with priority: shell_config.working_directory > default_shell_config.working_directory
    let working_directory = shell_config
        .working_directory
        .clone()
        .or_else(|| app_config.default_shell_config.working_directory.clone());

    // Determine terminal size with priority: shell_config.size > default_shell_config.size
    let terminal_size = shell_config
        .size
        .as_ref()
        .unwrap_or(&app_config.default_shell_config.size)
        .clone();

    // Determine environment variables with priority: shell_config.environment > default_shell_config.environment
    let mut environment = Vec::new();

    // Add default environment variables from default_shell_config
    if let Some(default_env) = &app_config.default_shell_config.environment {
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
        command,
        args,
        cols: terminal_size.columns,
        rows: terminal_size.rows,
        env: environment,
        cwd: working_directory,
    };

    // Get PTY factory based on configuration
    let factory = get_pty_factory(&app_config.pty_implementation);
    let pty = factory.create(&pty_config).await?;
    Ok(pty)
}

/// Create a new PTY instance with custom configuration
/// This function uses the default PTY implementation (tokio_process)
pub async fn create_pty_with_config(config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
    // 使用默认的PTY实现
    let factory = get_pty_factory("tokio_process");
    factory.create(config).await
}

/// Create a new PTY instance using a specific factory
pub async fn create_pty_with_factory(
    factory: &dyn PtyFactory,
    config: &PtyConfig,
) -> Result<Box<dyn AsyncPty>, PtyError> {
    factory.create(config).await
}
