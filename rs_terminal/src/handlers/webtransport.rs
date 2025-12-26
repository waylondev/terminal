use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{debug, error, info};

use crate::app_state::AppState;
use crate::protocol::WebTransportConnection;
use crate::service::handle_terminal_session;

/// WebTransport server implementation
pub async fn start_webtransport_server(addr: SocketAddr, state: AppState) {
    info!("Starting WebTransport server on {}", addr);

    // Create a shutdown signal channel for graceful shutdown
    let (shutdown_tx, mut shutdown_rx) = broadcast::channel(1);
    let shutdown_tx = Arc::new(shutdown_tx);

    // Clone state for use in the server task
    let state_clone = state.clone();
    let shutdown_tx_clone = Arc::clone(&shutdown_tx);

    // Start the WebTransport server in a separate task
    let server_task = tokio::spawn(async move {
        if let Err(e) = run_webtransport_server(addr, state_clone, shutdown_tx_clone).await {
            error!("WebTransport server error: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = shutdown_rx.recv() => {
            info!("Received shutdown signal for WebTransport server");
        }
        result = server_task => {
            match result {
                Ok(()) => info!("WebTransport server task completed normally"),
                Err(e) => error!("WebTransport server task failed: {}", e),
            }
        }
    }

    info!("WebTransport server shutdown complete");
}

/// Run the actual WebTransport server
async fn run_webtransport_server(
    addr: SocketAddr,
    state: AppState,
    shutdown_tx: Arc<broadcast::Sender<()>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Configuring WebTransport server on {}", addr);

    // Generate server certificate for WebTransport (HTTPS required)
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".to_string()])?;
    let private_key = certificate.serialize_private_key_der();
    let certificate_der = certificate.serialize_der()?;

    // Configure WebTransport endpoint using the correct API
    // For wtransport 0.6, we need to use a different certificate configuration approach
    let identity = wtransport::Identity::self_signed(vec!["localhost"])?;
    let config = wtransport::ServerConfig::builder()
        .with_bind_address(addr)
        .with_identity(identity)
        .build();

    let endpoint = wtransport::Endpoint::server(config)?;

    info!("WebTransport server listening on {}", addr);

    // Handle incoming connections
    loop {
        tokio::select! {
            biased;
            
            // Handle shutdown signal
            _ = async {
                let mut rx = shutdown_tx.subscribe();
                rx.recv().await.ok();
            } => {
                info!("WebTransport server received shutdown signal");
                break;
            }
            
            // Accept incoming connections
            incoming_session = endpoint.accept() => {
                match incoming_session.await {
                    Ok(session) => {
                        info!("New WebTransport session accepted");
                        
                        // Accept the session to get the connection
                        match session.accept().await {
                            Ok(connection) => {
                                info!("WebTransport connection established");
                                
                                // Handle the connection in a separate task
                                let state_clone = state.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = handle_webtransport_connection(connection, state_clone).await {
                                        error!("WebTransport connection error: {}", e);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("Error accepting WebTransport session: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error accepting WebTransport session: {}", e);
                    }
                }
            }
        }
    }

    info!("WebTransport server shutting down");
    Ok(())
}

/// Handle individual WebTransport connection
async fn handle_webtransport_connection(
    connection: wtransport::Connection,
    state: AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection_id = uuid::Uuid::new_v4().to_string();
    info!("Handling WebTransport connection: {}", connection_id);

    // Extract session ID from the connection path
    let session_id = extract_session_id_from_connection(&connection)?;
    
    info!("WebTransport session ID: {}", session_id);

    // Create WebTransport connection wrapper and set the actual connection
    let webtransport_conn = WebTransportConnection::new(connection_id.clone());
    
    // Set the actual WebTransport connection
    if let Err(e) = webtransport_conn.set_connection(connection).await {
        error!("Failed to set WebTransport connection: {}", e);
        return Err(e);
    }

    // Use the shared session handler to handle this connection
    handle_terminal_session(webtransport_conn, state).await;

    info!("WebTransport connection closed: {}", connection_id);
    Ok(())
}

/// Extract session ID from WebTransport connection
fn extract_session_id_from_connection(
    _connection: &wtransport::Connection,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // WebTransport connections typically include the path in the URL
    // For now, we'll generate a session ID based on the connection
    // In a real implementation, we'd extract this from the connection metadata
    Ok(uuid::Uuid::new_v4().to_string())
}