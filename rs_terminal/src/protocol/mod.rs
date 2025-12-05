/// Protocol abstraction for Waylon Terminal Rust backend
mod connection;
mod websocket_connection;
mod webtransport_connection;

pub use connection::{TerminalConnection, TerminalMessage, ConnectionType};
pub use websocket_connection::WebSocketConnection;
// pub use webtransport_connection::WebTransportConnection; // Not used yet
