use tokio::io::AsyncReadExt;
/// Terminal session handler for processing terminal connections
use tokio::select;
use tracing::{error, info};

use super::{MessageHandler, PtyManager};
use crate::{
    app_state::{AppState, ConnectionType, Session, SessionStatus},
    pty::AsyncPty,
    protocol::{TerminalConnection, TerminalMessage},
    service::ServiceError,
};

/// Handle a terminal session using the TerminalConnection trait
pub async fn handle_terminal_session(mut connection: impl TerminalConnection, state: AppState) {
    let conn_id = connection.id().to_string();
    let conn_type = connection.connection_type();

    info!("New terminal connection: {} (Type: {:?})", conn_id, conn_type);

    // Initialize managers
    let pty_manager = PtyManager::new();
    let message_handler = MessageHandler::new();

    // Initialize session
    if let Err(e) = SessionHandlerHelper::initialize_session(&conn_id, conn_type, &state).await {
        SessionHandlerHelper::handle_session_initialization_error(e, connection, &conn_id, &state).await;
        return;
    }

    // Create PTY for this session
    let mut pty = match SessionHandlerHelper::create_session_pty(&pty_manager, &state, &conn_id).await {
        Ok(pty) => pty,
        Err(e) => {
            SessionHandlerHelper::handle_pty_creation_error(e, connection, &conn_id, &state).await;
            return;
        }
    };

    info!("PTY created for session {}", conn_id);

    // Run main session loop
    SessionHandlerHelper::run_session_loop(&mut connection, &mut pty, &message_handler, &conn_id).await;

    // Clean up session resources
    SessionHandlerHelper::cleanup_session_resources(connection, pty, &pty_manager, &conn_id, &state).await;

    info!("Terminal session {} closed", conn_id);
}

/// 会话处理器辅助方法
struct SessionHandlerHelper;

impl SessionHandlerHelper {
    /// 初始化会话
    async fn initialize_session(conn_id: &str, conn_type: crate::protocol::ConnectionType, state: &AppState) -> Result<(), ServiceError> {
        match state.get_session(conn_id).await {
            Some(mut session) => {
                // Update session status to active
                session.set_status(SessionStatus::Active);
                state.update_session(session).await;
            }
            None => {
                // Get default shell command from config
                let shell_type = state.config.default_shell_type.clone();

                // Create a new session if it doesn't exist
                let session = Session::new(
                    conn_id.to_string(),
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
                state.add_session(session).await;
            }
        };

        info!("Session status updated to active: {}", conn_id);
        Ok(())
    }

    /// 创建会话 PTY
    async fn create_session_pty(pty_manager: &PtyManager, state: &AppState, conn_id: &str) -> Result<Box<dyn AsyncPty>, ServiceError> {
        match pty_manager.create_pty_from_config(&state.config).await {
            Ok(pty) => {
                info!("PTY created for session {}", conn_id);
                Ok(pty)
            }
            Err(e) => {
                error!("Failed to create PTY for session {}: {}", conn_id, e);
                Err(ServiceError::Other(format!("Failed to create PTY: {}", e)))
            }
        }
    }

    /// 处理会话初始化错误
    async fn handle_session_initialization_error(e: ServiceError, mut connection: impl TerminalConnection, conn_id: &str, state: &AppState) {
        error!("Failed to initialize session {}: {}", conn_id, e);
        
        let error_msg = format!("Error: Failed to initialize terminal session: {}", e);
        let _ = connection.send_text(&error_msg).await;
        let _ = connection.close().await;
        
        // Clean up session if it was added
        state.remove_session(conn_id).await;
    }

    /// 处理 PTY 创建错误
    async fn handle_pty_creation_error(e: ServiceError, mut connection: impl TerminalConnection, conn_id: &str, state: &AppState) {
        error!("Failed to create PTY for session {}: {}", conn_id, e);
        
        let error_msg = format!("Error: Failed to create terminal session: {}", e);
        let _ = connection.send_text(&error_msg).await;
        let _ = connection.close().await;
        
        // Clean up session if it was added
        state.remove_session(conn_id).await;
    }

    /// 运行会话主循环
    async fn run_session_loop(
        connection: &mut impl TerminalConnection,
        pty: &mut Box<dyn AsyncPty>,
        message_handler: &MessageHandler,
        conn_id: &str,
    ) {
        let mut pty_buffer = [0u8; 4096];

        loop {
            select! {
                // Handle incoming messages from the connection
                msg_result = connection.receive() => {
                    if Self::handle_connection_message(msg_result, connection, pty, message_handler, conn_id).await {
                        break;
                    }
                },
                // Handle PTY output directly (non-blocking async)
                read_result = pty.read(&mut pty_buffer) => {
                    if Self::handle_pty_output(read_result, &pty_buffer, connection, message_handler, conn_id).await {
                        break;
                    }
                },
            }
        }
    }

    /// 处理连接消息
    async fn handle_connection_message(
        msg_result: Option<Result<TerminalMessage, Box<dyn std::error::Error + Send>>>,
        connection: &mut impl TerminalConnection,
        pty: &mut Box<dyn AsyncPty>,
        message_handler: &MessageHandler,
        conn_id: &str,
    ) -> bool {
        match msg_result {
            Some(Ok(msg)) => {
                match message_handler.handle_message(msg, connection, pty, conn_id).await {
                    Ok(close) => close,
                    Err(e) => {
                        error!("Failed to handle message for session {}: {}", conn_id, e);
                        true
                    }
                }
            }
            Some(Err(e)) => {
                error!("Connection error for session {}: {}", conn_id, e);
                true
            }
            None => {
                info!("Connection closed by client for session {}", conn_id);
                true
            }
        }
    }

    /// 处理 PTY 输出
    async fn handle_pty_output(
        read_result: Result<usize, std::io::Error>,
        pty_buffer: &[u8],
        connection: &mut impl TerminalConnection,
        message_handler: &MessageHandler,
        conn_id: &str,
    ) -> bool {
        match read_result {
            Ok(0) => {
                info!("PTY closed for session {}", conn_id);
                true
            }
            Ok(n) => {
                let data = &pty_buffer[..n];
                if let Err(e) = message_handler.handle_pty_output(data, connection, conn_id).await {
                    error!("Failed to handle PTY output for session {}: {}", conn_id, e);
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                error!("Error reading from PTY for session {}: {}", conn_id, e);
                true
            }
        }
    }

    /// 清理会话资源
    async fn cleanup_session_resources(
        mut connection: impl TerminalConnection,
        mut pty: Box<dyn AsyncPty>,
        pty_manager: &PtyManager,
        conn_id: &str,
        state: &AppState,
    ) {
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
        if let Some(mut session) = state.get_session(conn_id).await {
            session.set_status(SessionStatus::Terminated);
            state.update_session(session.clone()).await;
        }

        // Remove session from state after a short delay (allowing time for cleanup)
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        state.remove_session(conn_id).await;
    }
}
