/// Application state management for Waylon Terminal Rust backend
mod app_state;
mod session;

pub use app_state::AppState;
pub use session::{ConnectionType, Session, SessionStatus};
