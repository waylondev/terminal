use crate::app_state::Session;
use crate::config::TerminalConfig;
use std::collections::HashMap;
/// Application state implementation for Waylon Terminal Rust backend
use std::sync::Arc;
use tokio::sync::Mutex;

/// Application state containing shared data across handlers
#[derive(Clone)]
pub struct AppState {
    /// Map of active sessions by session ID
    pub sessions: Arc<Mutex<HashMap<String, Session>>>,
    /// Application configuration
    pub config: Arc<TerminalConfig>,
}

impl AppState {
    /// Create a new instance of AppState with configuration
    pub fn new(config: TerminalConfig) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(config),
        }
    }

    /// Add a new session to the state
    pub async fn add_session(&self, session: Session) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session.id.clone(), session);
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.lock().await;
        sessions.get(session_id).cloned()
    }

    /// Remove a session by ID
    pub async fn remove_session(&self, session_id: &str) -> Option<Session> {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id)
    }

    /// Update an existing session
    pub async fn update_session(&self, session: Session) -> bool {
        let mut sessions = self.sessions.lock().await;
        if sessions.contains_key(&session.id) {
            sessions.insert(session.id.clone(), session);
            true
        } else {
            false
        }
    }

    /// Get all sessions
    pub async fn get_all_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.lock().await;
        sessions.values().cloned().collect()
    }

    /// Get the number of active sessions
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.lock().await;
        sessions.len()
    }

    /// Clean up all sessions and return the number of sessions cleaned
    pub async fn cleanup_all_sessions(&self) -> usize {
        let mut sessions = self.sessions.lock().await;
        let count = sessions.len();
        sessions.clear();
        count
    }
}
