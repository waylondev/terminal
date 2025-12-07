use serde::{Deserialize, Serialize};
/// Terminal session implementation
use std::sync::Arc;
use std::time::SystemTime;

/// Terminal session state
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum SessionStatus {
    /// Session has been created but not yet connected
    Created,
    /// Session is active and connected
    Active,
    /// Session has been disconnected but not yet terminated
    Disconnected,
    /// Session has been terminated
    Terminated,
}

/// Terminal session connection type
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum ConnectionType {
    /// WebSocket connection
    WebSocket,
    /// WebTransport connection
    WebTransport,
}

/// Terminal session structure
#[derive(Debug, Clone, Serialize)]
pub struct Session {
    /// Unique session ID
    pub session_id: String,

    /// User ID associated with this session
    pub user_id: String,

    /// Optional title for the session
    pub title: Option<String>,

    /// Session status
    pub status: SessionStatus,

    /// Terminal columns
    pub columns: u16,

    /// Terminal rows
    pub rows: u16,

    /// Working directory
    pub working_directory: Option<String>,

    /// Shell type
    pub shell_type: String,

    /// Connection type
    pub connection_type: ConnectionType,

    /// Session creation timestamp (UNIX epoch in seconds)
    pub created_at: u64,

    /// Session last updated timestamp (UNIX epoch in seconds)
    pub updated_at: u64,
}

impl Session {
    /// Create a new session with the given parameters
    pub fn new(
        session_id: String,
        user_id: String,
        title: Option<String>,
        working_directory: Option<String>,
        shell_type: String,
        columns: u16,
        rows: u16,
        connection_type: ConnectionType,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            session_id,
            user_id,
            title,
            status: SessionStatus::Created,
            columns,
            rows,
            working_directory,
            shell_type,
            connection_type,
            created_at: now,
            updated_at: now,
        }
    }

    /// Update the terminal size
    pub fn resize(&mut self, columns: u16, rows: u16) {
        self.columns = columns;
        self.rows = rows;
        self.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Update the session status
    pub fn set_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}
