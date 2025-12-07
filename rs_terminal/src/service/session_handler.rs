use tokio::io::AsyncReadExt;
/// Terminal session handler for processing terminal connections
use tokio::select;
use tracing::{error, info};

use super::{MessageHandler, PtyManager};
use crate::{
    app_state::{AppState, ConnectionType, Session, SessionStatus},
    protocol::TerminalConnection,
};

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
    let pty_manager = PtyManager::new();
    let message_handler = MessageHandler::new();

    // Check if session already exists in state (created via REST API)
    let session = match state.get_session(&conn_id).await {
        Some(mut session) => {
            // Update session status to active
            session.set_status(SessionStatus::Active);
            state.update_session(session.clone()).await;
            session
        }
        None => {
            // Get default shell command from config
            let shell_type = state.config.default_shell_type.clone();

            // Create a new session if it doesn't exist
            let session = Session::new(
                conn_id.clone(),
                "anonymous".to_string(), // Default to anonymous user
                None,
                None,
                shell_type,
                state.config.default_shell_config.size.columns,
                state.config.default_shell_config.size.rows,
                match conn_type {
                    crate::protocol::ConnectionType::WebSocket => ConnectionType::WebSocket,
                    crate::protocol::ConnectionType::WebTransport => ConnectionType::WebTransport,
                },
            );
            state.add_session(session.clone()).await;
            session
        }
    };

    info!("Session status updated to active: {}", conn_id);

    // Create PTY for this session using application configuration
    let mut pty = match pty_manager.create_pty_from_config(&state.config).await {
        Ok(pty) => pty,
        Err(e) => {
            // Send error message and close connection
            let error_msg = format!("Error: Failed to create terminal session: {}", e);
            let _ = connection.send_text(&error_msg).await;
            let _ = connection.close().await;
            // Clean up session if it was added
            state.remove_session(&conn_id).await;
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

    // Update session status to terminated
    if let Some(mut session) = state.get_session(&conn_id).await {
        session.set_status(SessionStatus::Terminated);
        state.update_session(session.clone()).await;
    }

    // Remove session from state after a short delay (allowing time for cleanup)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    state.remove_session(&conn_id).await;

    info!("Terminal session {} closed", conn_id);
}
