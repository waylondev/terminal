use axum::{
    extract::Path,
    extract::State,
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};

use crate::{app_state::AppState, protocol::WebSocketConnection, service::handle_terminal_session};

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    ws.on_upgrade(|socket| handle_socket(socket, state_clone))
}

pub async fn websocket_handler_with_id(
    ws: WebSocketUpgrade,
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let state_clone = state.clone();
    ws.on_upgrade(|socket| handle_socket_with_id(socket, session_id, state_clone))
}

pub async fn handle_socket(socket: WebSocket, state: AppState) {
    // Generate session ID if none is provided
    let sessions = state.sessions.lock().await;
    let session_id = format!("session-{}", sessions.len());
    drop(sessions);

    handle_socket_with_id(socket, session_id, state).await;
}

pub async fn handle_socket_with_id(socket: WebSocket, session_id: String, state: AppState) {
    // Create WebSocket connection that implements TerminalConnection trait
    let ws_connection = WebSocketConnection {
        socket,
        id: session_id.clone(),
    };

    // Use the shared session handler to handle this connection
    handle_terminal_session(ws_connection, state).await;
}
