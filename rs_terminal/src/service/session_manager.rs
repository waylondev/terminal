/// Session manager for managing terminal sessions
use crate::app_state::AppState;
use tracing::{info, error};

/// Session manager responsible for managing terminal sessions
pub struct SessionManager {
    app_state: AppState,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(app_state: AppState) -> Self {
        Self {
            app_state,
        }
    }

    /// Add a new session to the state
    pub async fn add_session(&self, session_id: &str) {
        let mut sessions = self.app_state.sessions.lock().await;
        sessions.push(session_id.into());
        info!("Added session: {}", session_id);
    }

    /// Remove a session from the state
    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.app_state.sessions.lock().await;
        let initial_len = sessions.len();
        sessions.retain(|id| id.as_ref() != session_id);
        
        if sessions.len() < initial_len {
            info!("Removed session: {}", session_id);
        } else {
            error!("Session not found for removal: {}", session_id);
        }
    }

    /// Get the current number of sessions
    pub async fn session_count(&self) -> usize {
        let sessions = self.app_state.sessions.lock().await;
        sessions.len()
    }
}
