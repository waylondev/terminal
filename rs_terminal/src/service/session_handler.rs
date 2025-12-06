/// Terminal session handler for processing terminal connections
use tokio::select;
use tokio::io::AsyncReadExt;
use tracing::{error, info};

use crate::{app_state::AppState, protocol::TerminalConnection};
use super::{SessionManager, PtyManager, MessageHandler};

/// Handle a terminal session using the TerminalConnection trait
pub async fn handle_terminal_session(mut connection: impl TerminalConnection, state: AppState) {
    // Clone conn_id immediately to avoid borrowing issues
    let conn_id = connection.id().to_string();
    let conn_type = connection.connection_type();

    info!(
        "New terminal connection: {} (Type: {:?})",
        conn_id, conn_type
    );

    // Initialize managers
    let session_manager = SessionManager::new(state.clone());
    let pty_manager = PtyManager::new();
    let message_handler = MessageHandler::new();

    // Add session to state
    session_manager.add_session(&conn_id).await;

    // Create PTY for this session using application configuration
    let mut pty = match pty_manager.create_pty_from_config(&state.config).await {
        Ok(pty) => pty,
        Err(e) => {
            // Send error message and close connection
            let error_msg = format!("Error: Failed to create terminal session: {}", e);
            let _ = connection.send_text(&error_msg).await;
            let _ = connection.close().await;
            // Clean up session
            session_manager.remove_session(&conn_id).await;
            return;
        }
    };

    info!("PTY created for session {}", conn_id);

    // Main session loop - handle both incoming messages and PTY output directly
    let mut pty_buffer = [0u8; 4096];
    let mut should_close = false;

    loop {
        select! {
            // Handle incoming messages from the connection
            msg_result = connection.receive() => {
                match msg_result {
                    Some(Ok(msg)) => {
                        // Use message handler to process the message
                        match message_handler.handle_message(msg, &mut connection, &mut pty, &conn_id).await {
                            Ok(close) => {
                                if close {
                                    should_close = true;
                                    break;
                                }
                            },
                            Err(e) => {
                                error!("Failed to handle message for session {}: {}", conn_id, e);
                                break;
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("Connection error for session {}: {}", conn_id, e);
                        break;
                    }
                    None => {
                        // Connection closed
                        info!("Connection closed by client for session {}", conn_id);
                        break;
                    }
                }
            },
            // Handle PTY output directly (non-blocking async)
            read_result = pty.read(&mut pty_buffer) => {
                match read_result {
                    Ok(0) => {
                        // EOF - PTY has closed
                        info!("PTY closed for session {}", conn_id);
                        break;
                    }
                    Ok(n) => {
                        // Use message handler to process PTY output
                        let data = &pty_buffer[..n];
                        if let Err(e) = message_handler.handle_pty_output(data, &mut connection, &conn_id).await {
                            error!("Failed to handle PTY output for session {}: {}", conn_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Error reading from PTY for session {}: {}", conn_id, e);
                        break;
                    }
                }
            },
        }
    }

    // Clean up resources
    info!("Cleaning up session {}", conn_id);

    // Close the connection
    if let Err(e) = connection.close().await {
        error!("Failed to close connection for session {}: {}", conn_id, e);
    }

    // Kill the PTY process
    if let Err(e) = pty_manager.kill_pty(&mut pty).await {
        error!("Failed to kill PTY process for session {}: {}", conn_id, e);
    }

    // Remove session from state
    session_manager.remove_session(&conn_id).await;

    info!("Terminal session {} closed", conn_id);
}
