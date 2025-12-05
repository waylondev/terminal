use axum::{extract::State, response::IntoResponse, extract::ws::{WebSocket, WebSocketUpgrade}};

use crate::{app_state::AppState, protocol::WebSocketConnection, service::handle_terminal_session};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    ws.on_upgrade(|socket| handle_socket(socket, state_clone))
}

pub async fn handle_socket(
    socket: WebSocket,
    state: AppState,
) {
    // Generate session ID
    let sessions = state.sessions.lock().await;
    let session_id = format!("session-{}", sessions.len());
    drop(sessions);
    
    // Create WebSocket connection that implements TerminalConnection trait
    let ws_connection = WebSocketConnection {
        socket,
        id: session_id.clone(),
    };
    
    // Use the shared session handler to handle this connection
    handle_terminal_session(ws_connection, state).await;
}
