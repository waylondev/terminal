/// Service layer for terminal session management
/// This module provides a structured approach to handling terminal sessions
/// with clear separation of concerns following SOLID principles

mod session_handler;
mod session_manager;
mod pty_manager;
mod message_handler;

// Re-export public types and functions
pub use session_handler::handle_terminal_session;
pub use session_manager::SessionManager;
pub use pty_manager::PtyManager;
pub use message_handler::MessageHandler;
