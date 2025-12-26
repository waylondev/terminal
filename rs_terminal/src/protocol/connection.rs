/// Terminal connection trait for abstracting different transport protocols
use std::fmt::Debug;

use thiserror::Error;

/// 连接错误类型
#[derive(Error, Debug)]
pub enum ConnectionError {
    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// WebSocket 错误
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// WebTransport 错误
    #[error("WebTransport error: {0}")]
    WebTransport(String),

    /// 连接已关闭
    #[error("Connection closed")]
    ConnectionClosed,

    /// 消息序列化错误
    #[error("Message serialization error: {0}")]
    Serialization(String),

    /// 消息反序列化错误
    #[error("Message deserialization error: {0}")]
    Deserialization(String),

    /// 超时错误
    #[error("Operation timeout")]
    Timeout,

    /// 其他错误
    #[error("Connection error: {0}")]
    Other(String),
}

/// 连接结果类型
pub type ConnectionResult<T> = Result<T, ConnectionError>;

/// Terminal connection trait that defines common capabilities for all transport protocols
#[async_trait::async_trait]
pub trait TerminalConnection: Send + Debug {
    /// Send a text message over the connection
    async fn send_text(&mut self, message: &str) -> ConnectionResult<()>;

    /// Send a binary message over the connection
    async fn send_binary(&mut self, data: &[u8]) -> ConnectionResult<()>;

    /// Receive a message from the connection
    /// Returns None when the connection is closed
    async fn receive(&mut self) -> Option<ConnectionResult<TerminalMessage>>;

    /// Close the connection
    async fn close(&mut self) -> ConnectionResult<()>;

    /// Get the connection ID
    fn id(&self) -> &str;

    /// Get the connection type
    fn connection_type(&self) -> ConnectionType;

    /// Check if the connection is still alive
    fn is_alive(&self) -> bool;
}

/// Terminal message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TerminalMessage {
    /// Text message
    Text(String),
    /// Binary message
    Binary(Vec<u8>),
    /// Ping message
    Ping(Vec<u8>),
    /// Pong message
    Pong(()),
    /// Close message
    Close,
}

/// Connection types
#[derive(Debug, Clone, Copy)]
pub enum ConnectionType {
    /// WebSocket connection
    WebSocket,
    /// WebTransport connection
    WebTransport,
}
