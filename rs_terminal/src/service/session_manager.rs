/// Session manager for managing terminal sessions
use crate::app_state::AppState;
use tracing::{error, info};

/// Session manager responsible for managing terminal sessions
pub struct SessionManager {
    app_state: AppState,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    /// Get the current number of sessions
    pub async fn session_count(&self) -> usize {
        self.app_state.session_count().await
    }
}
