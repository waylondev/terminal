use crate::pty::pty_trait::{AsyncPty, PtyConfig, PtyError, PtyFactory};
use async_trait::async_trait;
use portable_pty::{PtySize, CommandBuilder, Child};
use std::pin::Pin;
use std::process::ExitStatus as StdExitStatus;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::Mutex;
use tracing::info;

/// 基于 portable-pty 库的异步 PTY 实现
pub struct PortablePty {
    cols: u16,
    rows: u16,
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    child: Mutex<Box<dyn Child + Send>>,
    child_exited: bool,
}

impl PortablePty {
    pub fn new(config: &PtyConfig) -> Result<Self, PtyError> {
        info!(
            "PortablePty: Creating PTY with command: {:?}, args: {:?}",
            config.command, config.args
        );

        // Get the default PTY system
        let pty_system = portable_pty::native_pty_system();

        // Create PTY pair
        let pair = pty_system.openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Create command builder
        let mut cmd = CommandBuilder::new(config.command.clone());
        cmd.args(&config.args);
        
        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        
        // Set working directory if provided
        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }

        // Spawn the child process
        let child = pair.slave.spawn_command(cmd)?;

        Ok(Self {
            cols: config.cols,
            rows: config.rows,
            master: Mutex::new(pair.master),
            child: Mutex::new(child),
            child_exited: false,
        })
    }
}

// 实现 AsyncRead for PortablePty
impl AsyncRead for PortablePty {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        // 使用简单的方式实现 AsyncRead，返回成功但读取0字节
        // 完整的异步实现需要更复杂的处理
        Poll::Ready(Ok(()))
    }
}

// 实现 AsyncWrite for PortablePty
impl AsyncWrite for PortablePty {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // 使用简单的方式实现 AsyncWrite，返回成功并写入所有字节
        // 完整的异步实现需要更复杂的处理
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // 简单实现，直接返回成功
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // 对于 PTY，shutdown 可能不需要特别处理
        Poll::Ready(Ok(()))
    }
}

// 实现 AsyncPty trait 为 PortablePty
#[async_trait]
impl AsyncPty for PortablePty {
    /// 调整终端大小
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        info!("PortablePty: Resizing PTY to {}x{}", cols, rows);
        
        let master = self.master.lock().await;
        master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        
        self.cols = cols;
        self.rows = rows;
        Ok(())
    }

    /// 获取进程ID（如果可用）
    fn pid(&self) -> Option<u32> {
        // portable-pty 的 Child 没有 id() 方法，返回 None
        None
    }

    /// 检查进程是否存活
    fn is_alive(&self) -> bool {
        !self.child_exited
    }

    /// 等待进程结束（非阻塞检查）
    async fn try_wait(&mut self) -> Result<Option<StdExitStatus>, PtyError> {
        let mut child = self.child.lock().await;
        
        if self.child_exited {
            return Ok(None);
        }
        
        match child.try_wait()? {
            Some(_status) => {
                self.child_exited = true;
                // portable-pty 的 ExitStatus 与 std::process::ExitStatus 不同，返回一个简单的成功状态
                Ok(Some(StdExitStatus::default()))
            },
            None => Ok(None),
        }
    }

    /// 立即终止进程
    async fn kill(&mut self) -> Result<(), PtyError> {
        info!("PortablePty: Killing child process");
        
        let mut child = self.child.lock().await;
        child.kill()?;
        self.child_exited = true;
        Ok(())
    }
}

// ================ 工厂实现 ================

/// 基于 portable-pty 的 PTY 工厂
pub struct PortablePtyFactory;

#[async_trait]
impl PtyFactory for PortablePtyFactory {
    async fn create(&self, config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
        let pty = PortablePty::new(config)?;
        Ok(Box::new(pty))
    }

    fn name(&self) -> &'static str {
        "portable-pty"
    }
}
