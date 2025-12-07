/// Service layer for terminal session management
/// This module provides a structured approach to handling terminal sessions
/// with clear separation of concerns following SOLID principles
mod error;
mod message_handler;
mod pty_manager;
mod session_handler;
mod session_manager;

// Re-export public types and functions
pub use error::ServiceError;
pub use message_handler::MessageHandler;
pub use pty_manager::PtyManager;
pub use session_handler::handle_terminal_session;
pub use session_manager::SessionManager;
