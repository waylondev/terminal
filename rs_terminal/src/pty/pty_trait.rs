use async_trait::async_trait;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

// ================ 配置与错误类型 ================

#[derive(Debug, Clone)]
pub struct PtyConfig {
    pub command: String,
    pub args: Vec<String>,
    pub cols: u16,
    pub rows: u16,
    pub env: Vec<(String, String)>,
    pub cwd: Option<std::path::PathBuf>,
}

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Process spawn failed: {0}")]
    SpawnFailed(String),
    #[error("PTY not available")]
    NotAvailable,
    #[error("Process already terminated")]
    ProcessTerminated,
    #[error("Resize failed: {0}")]
    ResizeFailed(String),
    #[error("Lock acquisition error: {0}")]
    LockAcquisition(String),
    #[error("Resource cleanup error: {0}")]
    ResourceCleanup(String),
    #[error("Background task error: {0}")]
    BackgroundTask(String),
    #[error("Buffer overflow error")]
    BufferOverflow,
    #[error("Channel communication error: {0}")]
    ChannelCommunication(String),
    #[error("Other error: {0}")]
    Other(String),
}

// 添加From<anyhow::Error>实现
impl From<anyhow::Error> for PtyError {
    fn from(error: anyhow::Error) -> Self {
        PtyError::Other(error.to_string())
    }
}

// ================ 核心Trait定义 ================

/// 异步PTY Trait - 专为异步终端设计
#[async_trait]
pub trait AsyncPty: AsyncRead + AsyncWrite + Send + Sync + Unpin {
    /// 调整终端大小
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError>;

    /// 获取进程ID（如果可用）
    fn pid(&self) -> Option<u32>;

    /// 检查进程是否存活
    fn is_alive(&self) -> bool;

    /// 等待进程结束（非阻塞检查）
    async fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, PtyError>;

    /// 立即终止进程
    async fn kill(&mut self) -> Result<(), PtyError>;
}

/// PTY工厂Trait
#[async_trait]
pub trait PtyFactory: Send + Sync {
    /// 创建新的PTY实例
    async fn create(&self, config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError>;

    /// 工厂名称
    fn name(&self) -> &'static str;
}
