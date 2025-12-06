/// Terminal session handler for processing terminal connections
use tokio::select;
use tracing::{debug, error, info};

use crate::pty::Pty;
use crate::{app_state::AppState, protocol::TerminalConnection};

/// Handle a terminal session using the TerminalConnection trait
pub async fn handle_terminal_session(mut connection: impl TerminalConnection, state: AppState) {
    let conn_id = connection.id().to_string();
    let conn_type = connection.connection_type();

    info!(
        "New terminal connection: {} (Type: {:?})",
        conn_id, conn_type
    );

    // Add session to state
    let mut sessions = state.sessions.lock().await;
    sessions.push(conn_id.clone());
    drop(sessions);

    // Create PTY for this session using factory function
    let pty = match crate::pty::create_pty().await {
        Ok(pty) => pty,
        Err(e) => {
            error!("Failed to create PTY for session {}: {}", conn_id, e);
            // Send error message and close connection
            let _ = connection
                .send_text(&format!("Error: Failed to create terminal session: {}", e))
                .await;
            let _ = connection.close().await;
            // Clean up session
            let mut sessions = state.sessions.lock().await;
            sessions.retain(|id| id != &conn_id);
            drop(sessions);
            return;
        }
    };

    info!("PTY created for session {}", conn_id);

    // Wrap PTY in Arc and Mutex for safe sharing between tasks
    let pty = std::sync::Arc::new(tokio::sync::Mutex::new(pty));

    // Create a channel to communicate between PTY read task and main loop
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    // Start a separate async task to continuously read PTY output
    // This ensures the main select! loop won't be blocked by PTY reads
    let pty_clone = pty.clone();
    let conn_id_clone = conn_id.clone();
    let read_task = tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        loop {
            // Get a lock on the PTY and call the async read method
            let mut pty_guard = pty_clone.lock().await;
            let read_result = pty_guard.read(&mut buffer).await;
            
            match read_result {
                Ok(read_bytes) => {
                    if read_bytes > 0 {
                        // Send the read data to the main loop
                        let data = buffer[..read_bytes].to_vec();
                        info!("Read task: Sending {} bytes to channel for session {}", data.len(), conn_id_clone);
                        if let Err(e) = tx.send(data) {
                            error!("Read task: Failed to send PTY output to main loop: {}", e);
                            break;
                        }
                        info!("Read task: Successfully sent data to channel");
                    } else {
                        // No data read, sleep briefly to avoid CPU spin
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                },
                Err(e) => {
                    error!("Read task: Error reading from PTY: {}", e);
                    // Don't break the loop on error, just log it and continue
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                },
            }
        }
    });

    // Main session loop - handle both incoming messages and PTY output
    loop {
        select! {
            // Handle incoming messages from the connection
            msg_result = connection.receive() => {
                match msg_result {
                    Some(Ok(msg)) => {
                        match msg {
                            crate::protocol::TerminalMessage::Text(text) => {
                                debug!("Received text message from session {}: {}", conn_id, text);
                                // Write the text to PTY
                                let mut pty_guard = pty.lock().await;
                                if let Err(e) = pty_guard.write(text.as_bytes()).await {
                                    error!("Failed to write to PTY for session {}: {}", conn_id, e);
                                    break;
                                }
                            }
                            crate::protocol::TerminalMessage::Binary(bin) => {
                                debug!("Received binary message from session {} of length {}", conn_id, bin.len());
                                // Write binary data to PTY
                                let mut pty_guard = pty.lock().await;
                                if let Err(e) = pty_guard.write(&bin).await {
                                    error!("Failed to write binary data to PTY for session {}: {}", conn_id, e);
                                    break;
                                }
                            }
                            crate::protocol::TerminalMessage::Ping(_) => {
                                debug!("Received ping from session {}", conn_id);
                                // Respond with pong
                                if let Err(e) = connection.send_text(&"Pong").await {
                                    error!("Failed to send pong response to session {}: {}", conn_id, e);
                                    break;
                                }
                            }
                            crate::protocol::TerminalMessage::Pong(_) => {
                                debug!("Received pong from session {}", conn_id);
                                // Pong received, do nothing
                            }
                            crate::protocol::TerminalMessage::Close => {
                                info!("Received close message from session {}", conn_id);
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
            // Handle PTY output received from the read task
            // This is non-blocking as we're just receiving from a channel
            Some(data) = rx.recv() => {
                info!("Main loop: Received {} bytes from PTY read task for session {}", data.len(), conn_id);
                // Print the raw data for debugging
                debug!("Main loop: Raw PTY data: {:?}", data);
                
                // Try to convert data to string for text-based protocols
                match String::from_utf8(data.clone()) {
                    Ok(text) => {
                        info!("Main loop: Sending text data to client via connection");
                        debug!("Main loop: Text content to send: {:?}", text);
                        debug!("Main loop: Text length: {}", text.len());
                        
                        if let Err(e) = connection.send_text(&text).await {
                            error!("Main loop: Failed to send PTY text output to session {}: {}", conn_id, e);
                            break;
                        }
                        info!("Main loop: Successfully sent PTY text output to client");
                    },
                    Err(_) => {
                        // If conversion fails, send as binary
                        info!("Main loop: Sending binary data to client via connection");
                        debug!("Main loop: Binary data to send: {:?}", data);
                        debug!("Main loop: Binary length: {}", data.len());
                        
                        if let Err(e) = connection.send_binary(&data).await {
                            error!("Main loop: Failed to send PTY binary output to session {}: {}", conn_id, e);
                            break;
                        }
                        info!("Main loop: Successfully sent PTY binary output to client");
                    }
                }
            },
        }
    }

    // Clean up the read task
    read_task.abort();
    let _ = read_task.await;

    // Clean up resources
    info!("Cleaning up session {}", conn_id);

    // Close the connection
    if let Err(e) = connection.close().await {
        error!("Failed to close connection for session {}: {}", conn_id, e);
    }

    // Kill the PTY process
    if let Err(e) = pty.lock().await.kill().await {
        error!("Failed to kill PTY process for session {}: {}", conn_id, e);
    }

    // Remove session from state
    let mut sessions = state.sessions.lock().await;
    sessions.retain(|id| id != &conn_id);
    drop(sessions);

    info!("Terminal session {} closed", conn_id);
}
