use crate::pty::pty_trait::{PtyConfig, PtyError, AsyncPty, PtyFactory};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing::{info, error, debug};

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
        
        // 构建命令 - 完全遵循配置文件，不添加任何硬编码参数
        let mut cmd = tokio::process::Command::new(&config.command);
        
        // 添加配置文件中指定的参数
        for arg in &config.args {
            cmd.arg(arg);
        }
        
        // 设置工作目录
        if let Some(cwd) = &config.cwd {
            cmd.current_dir(cwd);
            info!("TokioProcessPty: Setting cwd to: {:?}", cwd);
        }
        
        // 设置环境变量 - 完全遵循配置文件，不添加任何硬编码环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
            if key == "PATH" || key == "TERM" {
                info!("TokioProcessPty: Setting env {}={:?}", key, value);
            }
        }
        
        // 设置标准输入输出
        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        
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
        
        // 检查进程是否已退出
        if let Ok(Some(status)) = self_mut.child.try_wait() {
            debug!("TokioProcessPty: Child process exited with status: {:?}", status);
            self_mut.child_exited = true;
            return Poll::Ready(Ok(()));
        }
        
        // 首先尝试从 stdout 读取数据
        let stdout_result = Pin::new(&mut self_mut.stdout).poll_read(cx, buf);
        
        match stdout_result {
            Poll::Ready(Ok(())) => {
                // 从 stdout 读取到数据，返回结果
                return Poll::Ready(Ok(()));
            }
            Poll::Ready(Err(e)) => {
                // stdout 出错，尝试从 stderr 读取
                error!("TokioProcessPty: Error reading from stdout: {}", e);
            }
            Poll::Pending => {
                // stdout 没有数据，尝试从 stderr 读取
            }
        }
        
        // 从 stderr 读取数据
        let stderr_result = Pin::new(&mut self_mut.stderr).poll_read(cx, buf);
        
        match stderr_result {
            Poll::Ready(Ok(())) => {
                // 从 stderr 读取到数据，返回结果
                return Poll::Ready(Ok(()));
            }
            Poll::Ready(Err(e)) => {
                // stderr 出错，返回错误
                error!("TokioProcessPty: Error reading from stderr: {}", e);
                return Poll::Ready(Err(e));
            }
            Poll::Pending => {
                // 两个流都没有数据，返回 Pending
                return Poll::Pending;
            }
        }
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
        
        // 添加调试日志
        debug!("TokioProcessPty: Polling for write, buffer length: {}", buf.len());
        debug!("TokioProcessPty: Writing data: {:?}", String::from_utf8_lossy(buf));
        
        // 委托给 stdin 的 poll_write
        let result = Pin::new(&mut self_mut.stdin).poll_write(cx, buf);
        
        match &result {
            Poll::Ready(Ok(n)) => {
                debug!("TokioProcessPty: Wrote {} bytes to stdin", n);
            }
            Poll::Ready(Err(e)) => {
                error!("TokioProcessPty: Error writing to stdin: {}", e);
            }
            Poll::Pending => {
                debug!("TokioProcessPty: Write pending, unable to write");
            }
        }
        
        result
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        
        // 添加调试日志
        debug!("TokioProcessPty: Polling for flush");
        
        // 委托给 stdin 的 poll_flush
        let result = Pin::new(&mut self_mut.stdin).poll_flush(cx);
        
        if let Poll::Ready(Err(e)) = &result {
            error!("TokioProcessPty: Error flushing stdin: {}", e);
        }
        
        result
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        let self_mut = self.get_mut();
        
        // 添加调试日志
        debug!("TokioProcessPty: Polling for shutdown");
        
        // 委托给 stdin 的 poll_shutdown
        let result = Pin::new(&mut self_mut.stdin).poll_shutdown(cx);
        
        if let Poll::Ready(Err(e)) = &result {
            error!("TokioProcessPty: Error shutting down stdin: {}", e);
        }
        
        result
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
            Ok(None) => {
                debug!("TokioProcessPty: Child process still running");
                Ok(None)
            },
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