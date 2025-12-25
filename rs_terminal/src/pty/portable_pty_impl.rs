use crate::pty::pty_trait::{AsyncPty, PtyConfig, PtyError, PtyFactory};
use async_trait::async_trait;
use portable_pty::{Child, CommandBuilder, PtySize};
use std::pin::Pin;
use std::process::ExitStatus as StdExitStatus;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;
use tracing::{info, error};

/// 基于 portable-pty 库的异步 PTY 实现
/// 使用异步通道处理数据流，避免阻塞
pub struct PortablePty {
    cols: u16,
    rows: u16,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send>>>,
    child_exited: Arc<Mutex<bool>>,
    // 异步通道用于处理 PTY 输出数据
    data_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    data_tx: mpsc::UnboundedSender<Vec<u8>>,
    // 缓冲区用于存储未消费的数据
    buffer: Vec<u8>,
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
        
        // Create writer only (reader will be handled by background task)
        let writer = pair.master.take_writer()?;
        
        // Create async channel for data flow
        let (data_tx, data_rx) = mpsc::unbounded_channel();

        // Clone for background task
        let reader = pair.master.try_clone_reader()?;
        let data_tx_clone = data_tx.clone();
        let child_exited = Arc::new(Mutex::new(false));
        let child_exited_clone = child_exited.clone();

        // Start background task for reading PTY output
        // 使用 spawn_blocking 来运行阻塞的读取操作，避免阻塞 tokio 运行时
        tokio::spawn(async move {
            let reader = reader;
            let data_tx = data_tx_clone;
            
            // 在阻塞上下文中运行读取循环
            let result = spawn_blocking(move || {
                let mut reader = reader;
                let mut buffer = vec![0; 1024];
                
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            // EOF - PTY closed
                            info!("PTY EOF reached, stopping background reader");
                            break Ok(());
                        }
                        Ok(n) => {
                            let data = buffer[..n].to_vec();
                            info!("PTY background reader: read {} bytes", n);
                            
                            // 使用阻塞发送，因为我们在阻塞上下文中
                            if data_tx.send(data).is_err() {
                                // Receiver dropped, stop reading
                                info!("PTY background reader: receiver dropped, stopping");
                                break Ok(());
                            }
                        }
                        Err(e) => {
                            error!("Error reading from PTY: {}", e);
                            break Err(e);
                        }
                    }
                }
            }).await;
            
            match result {
                Ok(Ok(())) => info!("PTY background reader finished successfully"),
                Ok(Err(e)) => error!("PTY background reader failed: {}", e),
                Err(e) => error!("PTY background reader task failed: {}", e),
            }
            
            // Mark child as exited
            *child_exited_clone.lock().unwrap() = true;
        });

        Ok(Self {
            cols: config.cols,
            rows: config.rows,
            master: Arc::new(Mutex::new(pair.master)),
            writer: Arc::new(Mutex::new(writer)),
            child: Arc::new(Mutex::new(child)),
            child_exited,
            data_rx,
            data_tx,
            buffer: Vec::new(),
        })
    }
}

// 实现 AsyncRead for PortablePty
// 优化实现：零拷贝数据流，避免不必要的缓冲
impl AsyncRead for PortablePty {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.as_mut().get_mut();
        
        // 首先检查缓冲区是否有数据
        if !this.buffer.is_empty() {
            let to_copy = std::cmp::min(this.buffer.len(), buf.remaining());
            buf.put_slice(&this.buffer[..to_copy]);
            this.buffer.drain(..to_copy);
            info!("PTY AsyncRead: copied {} bytes from buffer", to_copy);
            return Poll::Ready(Ok(()));
        }
        
        // 从异步通道接收数据
        match this.data_rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                info!("PTY AsyncRead: received {} bytes from channel", data.len());
                
                // 优化：如果数据完全适合输出缓冲区，直接使用而不复制
                if data.len() <= buf.remaining() {
                    // 零拷贝：直接使用接收到的数据
                    buf.put_slice(&data);
                    info!("PTY AsyncRead: zero-copy copied {} bytes to output", data.len());
                } else {
                    // 部分复制：只复制能放入缓冲区的部分
                    let to_copy = buf.remaining();
                    buf.put_slice(&data[..to_copy]);
                    
                    // 剩余数据放入内部缓冲区
                    this.buffer.extend_from_slice(&data[to_copy..]);
                    info!("PTY AsyncRead: copied {} bytes to output, {} bytes to buffer", to_copy, data.len() - to_copy);
                }
                
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => {
                // 通道关闭，PTY 已结束
                info!("PTY AsyncRead: channel closed, PTY ended");
                Poll::Ready(Ok(()))
            }
            Poll::Pending => {
                // 没有数据可用，等待下次唤醒
                Poll::Pending
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
        info!("PTY AsyncWrite: writing {} bytes to PTY", buf.len());
        match writer.write(buf) {
            Ok(n) => {
                info!("PTY AsyncWrite: successfully wrote {} bytes", n);
                Poll::Ready(Ok(n))
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // 资源暂时不可用，返回 Pending
                info!("PTY AsyncWrite: would block, pending");
                Poll::Pending
            },
            Err(e) => {
                error!("PTY AsyncWrite: error writing to PTY: {}", e);
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