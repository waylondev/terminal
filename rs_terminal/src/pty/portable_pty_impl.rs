use crate::pty::pty_trait::{AsyncPty, PtyConfig, PtyError, PtyFactory};
use async_trait::async_trait;
use portable_pty::{Child, CommandBuilder, PtySize};
use std::pin::Pin;
use std::process::ExitStatus as StdExitStatus;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::task::spawn_blocking;
use tracing::info;

/// 基于 portable-pty 库的异步 PTY 实现
/// 按照 tokio 异步编程最佳实践设计
pub struct PortablePty {
    cols: u16,
    rows: u16,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    reader: Arc<Mutex<Box<dyn std::io::Read + Send>>>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send>>>,
    child_exited: Arc<Mutex<bool>>,
}

impl PortablePty {
    pub fn new(config: &PtyConfig) -> Result<Self, PtyError> {
        info!(
            "PortablePty: Creating PTY with command: {:?}, args: {:?}",
            config.command, config.args
        );

        // Get the default PTY system
        let pty_system = portable_pty::native_pty_system();

        // Create PTY pair - 这是阻塞操作，但只在初始化时执行一次
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

        // Spawn the child process - 这是阻塞操作，但只在初始化时执行一次
        let child = pair.slave.spawn_command(cmd)?;
        
        // Create reader and writer
        let reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;

        Ok(Self {
            cols: config.cols,
            rows: config.rows,
            master: Arc::new(Mutex::new(pair.master)),
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
            child: Arc::new(Mutex::new(child)),
            child_exited: Arc::new(Mutex::new(false)),
        })
    }
}

// 实现 AsyncRead for PortablePty
// 简化实现：直接使用同步读取，在异步上下文中
impl AsyncRead for PortablePty {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.get_mut();
        let mut reader = this.reader.lock().unwrap();
        
        // 使用标准的同步读取，在 tokio 的异步上下文中
        // 注意：这是一个简化实现，实际生产环境中应该使用更高效的方式
        let mut local_buf = vec![0; buf.remaining()];
        
        match reader.read(&mut local_buf) {
            Ok(n) => {
                buf.put_slice(&local_buf[..n]);
                Poll::Ready(Ok(()))
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 资源暂时不可用，返回 Pending
                Poll::Pending
            },
            Err(e) => {
                Poll::Ready(Err(e))
            }
        }
    }
}

// 实现 AsyncWrite for PortablePty
// 简化实现：直接使用同步写入，在异步上下文中
impl AsyncWrite for PortablePty {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        let mut writer = this.writer.lock().unwrap();
        
        // 使用标准的同步写入，在 tokio 的异步上下文中
        match writer.write(buf) {
            Ok(n) => {
                Poll::Ready(Ok(n))
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 资源暂时不可用，返回 Pending
                Poll::Pending
            },
            Err(e) => {
                Poll::Ready(Err(e))
            }
        }
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.get_mut();
        let mut writer = this.writer.lock().unwrap();
        
        match writer.flush() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Poll::Pending
            },
            Err(e) => {
                Poll::Ready(Err(e))
            }
        }
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        // PTY 不需要特殊的关闭处理，flush 就足够了
        self.poll_flush(cx)
    }
}

// 实现 AsyncPty trait 为 PortablePty
#[async_trait]
impl AsyncPty for PortablePty {
    /// 调整终端大小
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        info!("PortablePty: Resizing PTY to {}x{}", cols, rows);

        let master = self.master.clone();
        let resize_result = spawn_blocking(move || {
            let mut master = master.lock().unwrap();
            master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
        }).await;

        match resize_result {
            Ok(result) => {
                result?;
                self.cols = cols;
                self.rows = rows;
                Ok(())
            },
            Err(e) => {
                Err(PtyError::Other(format!("Resize operation failed: {:?}", e)))
            }
        }
    }

    /// 获取进程ID（如果可用）
    fn pid(&self) -> Option<u32> {
        // portable-pty 的 Child 没有 id() 方法，返回 None
        None
    }

    /// 检查进程是否存活
    fn is_alive(&self) -> bool {
        !*self.child_exited.lock().unwrap()
    }

    /// 等待进程结束（非阻塞检查）
    async fn try_wait(&mut self) -> Result<Option<StdExitStatus>, PtyError> {
        let child = self.child.clone();
        let child_exited = self.child_exited.clone();

        let wait_result = spawn_blocking(move || {
            let mut child = child.lock().unwrap();
            let mut exited = child_exited.lock().unwrap();

            if *exited {
                return Ok(None);
            }

            match child.try_wait()? {
                Some(_status) => {
                    *exited = true;
                    // portable-pty 的 ExitStatus 与 std::process::ExitStatus 不同
                    // 返回一个默认的成功状态
                    Ok(Some(StdExitStatus::default()))
                },
                None => Ok(None),
            }
        }).await;

        wait_result.map_err(|e| PtyError::Other(format!("Wait operation failed: {:?}", e)))?
    }

    /// 立即终止进程
    async fn kill(&mut self) -> Result<(), PtyError> {
        info!("PortablePty: Killing child process");

        let child = self.child.clone();
        let child_exited = self.child_exited.clone();

        let kill_result = spawn_blocking(move || {
            let mut child = child.lock().unwrap();
            let mut exited = child_exited.lock().unwrap();

            child.kill()?;
            *exited = true;
            Ok(())
        }).await;

        kill_result.map_err(|e| PtyError::Other(format!("Kill operation failed: {:?}", e)))?
    }
}

// ================ 工厂实现 ================

/// 基于 portable-pty 的 PTY 工厂
pub struct PortablePtyFactory;

#[async_trait]
impl PtyFactory for PortablePtyFactory {
    async fn create(&self, config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
        // 创建 PTY 实例 - 这是阻塞操作，但只在初始化时执行一次
        // 使用 spawn_blocking 确保它不会阻塞异步运行时
        let config_clone = config.clone();
        let pty_result = spawn_blocking(move || PortablePty::new(&config_clone)).await;

        match pty_result {
            Ok(pty) => Ok(Box::new(pty?)),
            Err(e) => Err(PtyError::Other(format!("Failed to create PTY: {:?}", e))),
        }
    }

    fn name(&self) -> &'static str {
        "portable-pty"
    }
}