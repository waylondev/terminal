/// Application state implementation for Waylon Terminal Rust backend
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::TerminalConfig;

/// Application state containing shared data across handlers
#[derive(Clone)]
pub struct AppState {
    /// List of active session IDs
    pub sessions: Arc<Mutex<Vec<Arc<str>>>>,
    /// Application configuration
    pub config: Arc<TerminalConfig>,
}

impl AppState {
    /// Create a new instance of AppState with configuration
    pub fn new(config: TerminalConfig) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(Vec::new())),
            config: Arc::new(config),
        }
    }
}
