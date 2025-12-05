/// WebSocket connection implementation for TerminalConnection trait
use std::fmt::Debug;

use axum::extract::ws::WebSocket;
use axum::extract::ws::Message::{Text, Binary, Ping, Pong, Close};
use futures_util::StreamExt;

use crate::protocol::{TerminalConnection, TerminalMessage, ConnectionType};

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
    async fn send_text(&mut self, message: &str) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.socket.send(Text(message.to_string())).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
        Ok(())
    }
    
    async fn send_binary(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.socket.send(Binary(data.to_vec())).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
        Ok(())
    }
    
    async fn receive(&mut self) -> Option<Result<TerminalMessage, Box<dyn std::error::Error + Send>>> {
        match self.socket.next().await {
            Some(Ok(Text(text))) => {
                Some(Ok(TerminalMessage::Text(text)))
            },
            Some(Ok(Binary(bin))) => {
                Some(Ok(TerminalMessage::Binary(bin)))
            },
            Some(Ok(Ping(ping))) => {
                Some(Ok(TerminalMessage::Ping(ping)))
            },
            Some(Ok(Pong(_pong))) => {
                Some(Ok(TerminalMessage::Pong(())))
            },
            Some(Ok(Close(_))) => {
                Some(Ok(TerminalMessage::Close))
            },
            Some(Err(e)) => {
                Some(Err(Box::new(e) as Box<dyn std::error::Error + Send>))
            },
            None => {
                None
            },
        }
    }
    
    async fn close(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        self.socket.send(Close(None)).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
        Ok(())
    }
    
    fn id(&self) -> &str {
        &self.id
    }
    
    fn connection_type(&self) -> ConnectionType {
        ConnectionType::WebSocket
    }
}
