/// Data Transfer Objects (DTOs) for REST API endpoints
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request DTO for creating a new terminal session
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    /// User ID associated with this session
    pub user_id: String,

    /// Optional title for the session
    pub title: Option<String>,

    /// Optional working directory for the terminal
    pub working_directory: Option<String>,

    /// Optional shell type to use
    pub shell_type: Option<String>,

    /// Optional terminal columns
    pub columns: Option<u16>,

    /// Optional terminal rows
    pub rows: Option<u16>,
}

/// Request DTO for resizing a terminal session
#[derive(Debug, Deserialize, Serialize)]
pub struct ResizeTerminalRequest {
    /// New terminal columns
    pub columns: u16,

    /// New terminal rows
    pub rows: u16,
}

/// Response DTO for a terminal session
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    /// Unique session ID (renamed to 'id' to match frontend expectations)
    pub id: String,

    /// User ID associated with this session
    pub user_id: String,

    /// Session title
    pub title: Option<String>,

    /// Session status
    pub status: String,

    /// Terminal columns
    pub columns: u16,

    /// Terminal rows
    pub rows: u16,

    /// Working directory (use empty string instead of null if not set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,

    /// Shell type
    pub shell_type: String,

    /// Connection type (WebSocket/WebTransport)
    pub connection_type: String,

    /// Session creation timestamp
    pub created_at: u64,
}

/// Response DTO for terminal resize operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResizeResponse {
    /// Session ID
    pub session_id: String,

    /// New terminal columns
    pub columns: u16,

    /// New terminal rows
    pub rows: u16,

    /// Success flag
    pub success: bool,
}

/// Response DTO for terminal termination operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTerminateResponse {
    /// Session ID
    pub session_id: String,

    /// Success flag
    pub success: bool,

    /// Termination reason
    pub reason: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    /// Success flag
    pub success: bool,

    /// Response message
    pub message: String,
}

/// Generic error response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    /// Error flag
    pub error: bool,

    /// Error message
    pub message: String,

    /// Optional error code
    pub code: Option<u16>,
}
