use crate::pty::pty_trait::{PtyConfig, PtyError, AsyncPty, PtyFactory};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{info, error};

/// 基于 tokio-process 的 PTY 实现
/// 使用标准的进程 I/O，不依赖 Unix 特定的 PTY API，跨平台兼容
pub struct TokioProcessPty {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    child_exited: bool,
}

impl TokioProcessPty {
    pub fn new(config: &PtyConfig) -> Result<Self, PtyError> {
        info!("TokioProcessPty: Creating PTY with command: {:?}, args: {:?}", config.command, config.args);
        
        // 构建命令
        let mut cmd = tokio::process::Command::new(&config.command);
        
        // 添加参数
        for arg in &config.args {
            cmd.arg(arg);
        }
        
        // 设置工作目录
        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
            info!("TokioProcessPty: Setting cwd to: {:?}", cwd);
        }
        
        // 设置环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
            if key == "PATH" || key == "TERM" {
                info!("TokioProcessPty: Setting env {}={:?}", key, value);
            }
        }
        
        // 设置标准输入输出
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        
        // 生成子进程
        let mut child = cmd.spawn().map_err(|e| {
            error!("TokioProcessPty: Failed to spawn process: {}", e);
            PtyError::Other(e.to_string())
        })?;
        
        // 获取标准输入输出
        let stdin = child.stdin.take().ok_or_else(|| {
            error!("TokioProcessPty: Failed to get stdin");
            PtyError::Other("Failed to get stdin".to_string())
        })?;
        
        let stdout = child.stdout.take().ok_or_else(|| {
            error!("TokioProcessPty: Failed to get stdout");
            PtyError::Other("Failed to get stdout".to_string())
        })?;
        
        let stderr = child.stderr.take().ok_or_else(|| {
            error!("TokioProcessPty: Failed to get stderr");
            PtyError::Other("Failed to get stderr".to_string())
        })?;
        
        info!("TokioProcessPty: Successfully created process");
        
        Ok(Self {
            child,
            stdin,
            stdout,
            stderr,
            child_exited: false,
        })
    }
}

// 实现 AsyncRead
impl AsyncRead for TokioProcessPty {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        Pin::new(&mut self_mut.stdout).poll_read(cx, buf)
    }
}

// 实现 AsyncWrite
impl AsyncWrite for TokioProcessPty {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let self_mut = self.get_mut();
        Pin::new(&mut self_mut.stdin).poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        Pin::new(&mut self_mut.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        Pin::new(&mut self_mut.stdin).poll_shutdown(cx)
    }
}

// 实现 AsyncPty trait
#[async_trait]
impl AsyncPty for TokioProcessPty {
    async fn resize(&mut self, _cols: u16, _rows: u16) -> Result<(), PtyError> {
        info!("TokioProcessPty: Resize not supported in this implementation");
        // 不支持调整大小，返回 Ok
        Ok(())
    }
    
    fn pid(&self) -> Option<u32> {
        // 获取进程 ID
        self.child.id()
    }
    
    fn is_alive(&self) -> bool {
        !self.child_exited
    }
    
    async fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, PtyError> {
        // 尝试等待进程结束（非阻塞）
        if self.child_exited {
            return Ok(None);
        }
        
        match self.child.try_wait() {
            Ok(Some(status)) => {
                info!("TokioProcessPty: Child process exited with status: {:?}", status);
                self.child_exited = true;
                Ok(Some(status))
            },
            Ok(None) => Ok(None),
            Err(e) => {
                error!("TokioProcessPty: Failed to check child status: {}", e);
                Err(PtyError::Other(e.to_string()))
            },
        }
    }
    
    async fn kill(&mut self) -> Result<(), PtyError> {
        // 杀死进程
        info!("TokioProcessPty: Killing child process");
        
        self.child.kill().await
            .map_err(|e| {
                error!("TokioProcessPty: Failed to kill child process: {}", e);
                PtyError::Other(e.to_string())
            })?;
        
        self.child_exited = true;
        Ok(())
    }
}

// ================ 工厂实现 ================

/// 基于 tokio-process 的 PTY 工厂
pub struct TokioProcessPtyFactory;

impl Default for TokioProcessPtyFactory {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait]
impl PtyFactory for TokioProcessPtyFactory {
    async fn create(&self, config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
        let pty = TokioProcessPty::new(config)?;
        Ok(Box::new(pty))
    }
    
    fn name(&self) -> &'static str {
        "tokio-process-pty"
    }
}