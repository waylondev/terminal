/// WebSocket connection implementation for TerminalConnection trait
use std::fmt::Debug;
use tracing::{debug, error, info};

use axum::extract::ws::Message::{Binary, Close, Ping, Pong, Text};
use axum::extract::ws::WebSocket;
use futures_util::StreamExt;

use crate::protocol::{ConnectionError, ConnectionResult, ConnectionType, TerminalConnection, TerminalMessage};

/// WebSocket connection implementation that implements TerminalConnection trait
pub struct WebSocketConnection {
    pub socket: WebSocket,
    pub id: String,
}

impl Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketConnection")
            .field("id", &self.id)
            .finish()
    }
}

#[async_trait::async_trait]
impl TerminalConnection for WebSocketConnection {
    async fn send_text(&mut self, message: &str) -> ConnectionResult<()> {
        self.socket
            .send(Text(message.to_string()))
            .await
            .map_err(|e| ConnectionError::WebSocket(e.to_string()))?;
        Ok(())
    }

    async fn send_binary(&mut self, data: &[u8]) -> ConnectionResult<()> {
        info!("Sending binary data to client, size: {}", data.len());
        let result = self.socket.send(Binary(data.to_vec())).await;
        match result {
            Ok(_) => {
                info!("Successfully sent binary data to client");
                Ok(())
            }
            Err(e) => {
                error!("Failed to send binary data to client: {}", e);
                Err(ConnectionError::WebSocket(e.to_string()))
            }
        }
    }

    async fn receive(&mut self) -> Option<ConnectionResult<TerminalMessage>> {
        match self.socket.next().await {
            Some(Ok(Text(text))) => {
                debug!("WebSocket received text message: {:?}", text);
                Some(Ok(TerminalMessage::Text(text)))
            }
            Some(Ok(Binary(bin))) => {
                debug!("WebSocket received binary message, length: {}", bin.len());
                Some(Ok(TerminalMessage::Binary(bin)))
            }
            Some(Ok(Ping(ping))) => {
                debug!("WebSocket received ping message");
                Some(Ok(TerminalMessage::Ping(ping)))
            }
            Some(Ok(Pong(_pong))) => {
                debug!("WebSocket received pong message");
                Some(Ok(TerminalMessage::Pong(())))
            }
            Some(Ok(Close(_))) => {
                debug!("WebSocket received close message");
                Some(Ok(TerminalMessage::Close))
            }
            Some(Err(e)) => {
                error!("WebSocket receive error: {}", e);
                Some(Err(ConnectionError::WebSocket(e.to_string())))
            }
            None => {
                debug!("WebSocket connection closed");
                None
            }
        }
    }

    async fn close(&mut self) -> ConnectionResult<()> {
        self.socket
            .send(Close(None))
            .await
            .map_err(|e| ConnectionError::WebSocket(e.to_string()))?;
        Ok(())
    }

    fn is_alive(&self) -> bool {
        // WebSocket 连接状态检查
        // 这里可以添加更精确的连接状态检查逻辑
        true
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn connection_type(&self) -> ConnectionType {
        ConnectionType::WebSocket
    }
}
