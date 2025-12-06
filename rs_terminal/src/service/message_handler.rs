/// Message handler for processing terminal messages
use crate::{protocol::{TerminalConnection, TerminalMessage}, pty::AsyncPty};
use tracing::{debug, error, info};
use tokio::io::AsyncWriteExt;

/// Message handler responsible for processing terminal messages
pub struct MessageHandler;

impl MessageHandler {
    /// Create a new message handler
    pub fn new() -> Self {
        Self
    }

    /// Handle a terminal message
    pub async fn handle_message(
        &self,
        message: TerminalMessage,
        connection: &mut impl TerminalConnection,
        pty: &mut Box<dyn AsyncPty>,
        session_id: &str
    ) -> Result<bool, std::io::Error> {
        match message {
            TerminalMessage::Text(text) => {
                self.handle_text_message(text, connection, pty, session_id).await
            },
            TerminalMessage::Binary(bin) => {
                self.handle_binary_message(bin, connection, pty, session_id).await
            },
            TerminalMessage::Ping(_) => {
                self.handle_ping_message(connection, session_id).await
            },
            TerminalMessage::Pong(_) => {
                self.handle_pong_message(session_id).await
            },
            TerminalMessage::Close => {
                self.handle_close_message(connection, session_id).await
            }
        }
    }

    /// Handle a text message
    async fn handle_text_message(
        &self,
        text: String,
        _connection: &mut impl TerminalConnection,
        pty: &mut Box<dyn AsyncPty>,
        session_id: &str
    ) -> Result<bool, std::io::Error> {
        debug!("Received text message from session {}: {}", session_id, text);
        
        // Write the text to PTY directly (non-blocking async)
        match pty.write(text.as_bytes()).await {
            Ok(_) => Ok(false),
            Err(e) => {
                error!("Failed to write text to PTY for session {}: {}", session_id, e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            }
        }
    }

    /// Handle a binary message
    async fn handle_binary_message(
        &self,
        bin: Vec<u8>,
        _connection: &mut impl TerminalConnection,
        pty: &mut Box<dyn AsyncPty>,
        session_id: &str
    ) -> Result<bool, std::io::Error> {
        debug!("Received binary message from session {} of length {}", session_id, bin.len());
        
        // Write binary data to PTY directly (non-blocking async)
        match pty.write(&bin).await {
            Ok(_) => Ok(false),
            Err(e) => {
                error!("Failed to write binary data to PTY for session {}: {}", session_id, e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            }
        }
    }

    /// Handle a ping message
    async fn handle_ping_message(
        &self,
        connection: &mut impl TerminalConnection,
        session_id: &str
    ) -> Result<bool, std::io::Error> {
        debug!("Received ping from session {}", session_id);
        
        // Respond with pong
        match connection.send_text(&"Pong").await {
            Ok(_) => Ok(false),
            Err(e) => {
                error!("Failed to send pong response to session {}: {}", session_id, e);
                Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            }
        }
    }

    /// Handle a pong message
    async fn handle_pong_message(
        &self,
        session_id: &str
    ) -> Result<bool, std::io::Error> {
        debug!("Received pong from session {}", session_id);
        // Pong received, do nothing
        Ok(false)
    }

    /// Handle a close message
    async fn handle_close_message(
        &self,
        _connection: &mut impl TerminalConnection,
        session_id: &str
    ) -> Result<bool, std::io::Error> {
        info!("Received close message from session {}", session_id);
        // Return true to indicate that the session should be closed
        Ok(true)
    }

    /// Handle PTY output
    pub async fn handle_pty_output(
        &self,
        data: &[u8],
        connection: &mut impl TerminalConnection,
        session_id: &str
    ) -> Result<(), std::io::Error> {
        debug!("Received PTY data for session {}: {:?}", session_id, String::from_utf8_lossy(data));
        
        // Try to convert data to string for text-based protocols
        match String::from_utf8(data.to_vec()) {
            Ok(text) => {
                // Send text to client
                if let Err(e) = connection.send_text(&text).await {
                    error!("Failed to send PTY text output to session {}: {}", session_id, e);
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
                }
            },
            Err(_) => {
                // Send as binary if conversion fails
                if let Err(e) = connection.send_binary(data).await {
                    error!("Failed to send PTY binary output to session {}: {}", session_id, e);
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
                }
            }
        }
        
        Ok(())
    }
}
