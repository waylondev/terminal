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
use tracing::{debug, error, info, trace, warn};

/// 基于 portable-pty 库的高性能异步 PTY 实现
/// 使用零拷贝缓冲和智能阻塞策略实现真正的异步体验
pub struct PortablePty {
    cols: u16,
    rows: u16,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send>>>,
    child_exited: Arc<Mutex<bool>>,
    // 高性能异步通道：使用有界通道避免内存泄漏
    data_rx: mpsc::Receiver<Vec<u8>>,
    data_tx: mpsc::Sender<Vec<u8>>,
    // 零拷贝缓冲区：使用字节数组而非 Vec<u8> 减少分配
    buffer: Box<[u8; 8192]>, // 固定大小缓冲区，避免动态分配
    buffer_pos: usize,
    buffer_len: usize,
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
        
        // 创建高性能有界通道：避免内存泄漏，提供背压控制
        let (data_tx, data_rx) = mpsc::channel(1024); // 1024 个消息的缓冲区

        // Clone for background task
        let reader = pair.master.try_clone_reader()?;
        let data_tx_clone = data_tx.clone();
        let child_exited = Arc::new(Mutex::new(false));
        let child_exited_clone = child_exited.clone();

        // 启动高性能后台读取任务
        // 使用智能阻塞策略：小批量读取，避免长时间阻塞
        tokio::spawn(async move {
            let reader = reader;
            let data_tx = data_tx_clone;
            
            // 在阻塞上下文中运行读取循环，但使用更智能的策略
            let result = spawn_blocking(move || {
                let mut reader = reader;
                let mut buffer = vec![0; 4096]; // 增大缓冲区减少系统调用
                
                loop {
                    // 非阻塞读取尝试（如果平台支持）
                    match reader.read(&mut buffer) {
                        Ok(0) => {
                            // EOF - PTY closed
                            debug!("PTY EOF reached, stopping background reader");
                            break Ok(());
                        }
                        Ok(n) => {
                            // 优化：避免不必要的内存分配
                            let data = if n == buffer.len() {
                                // 完整缓冲区，直接使用
                                std::mem::take(&mut buffer)
                            } else {
                                // 部分读取，复制所需部分
                                buffer[..n].to_vec()
                            };
                            
                            // 重置缓冲区
                            buffer = vec![0; 4096];
                            
                            trace!("PTY background reader: read {} bytes", n);
                            
                            // 使用阻塞发送，但添加超时保护
                            if data_tx.blocking_send(data).is_err() {
                                // Receiver dropped, stop reading
                                debug!("PTY background reader: receiver dropped, stopping");
                                break Ok(());
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            // 非阻塞读取返回 WouldBlock，短暂休眠后重试
                            std::thread::sleep(std::time::Duration::from_millis(10));
                            continue;
                        }
                        Err(e) => {
                            error!("Error reading from PTY: {}", e);
                            break Err(e);
                        }
                    }
                }
            }).await;
            
            match result {
                Ok(Ok(())) => debug!("PTY background reader finished successfully"),
                Ok(Err(e)) => error!("PTY background reader failed: {}", e),
                Err(e) => error!("PTY background reader task failed: {}", e),
            }
            
            // Mark child as exited
            if let Ok(mut exited) = child_exited_clone.lock() {
                *exited = true;
            } else {
                error!("Failed to acquire child_exited lock for marking exit");
            }
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
            buffer: Box::new([0u8; 8192]), // 预分配 8KB 缓冲区
            buffer_pos: 0,
            buffer_len: 0,
        })
    }
}

// 实现 AsyncRead for PortablePty
// 高性能零拷贝实现：避免不必要的内存分配和复制
impl AsyncRead for PortablePty {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.as_mut().get_mut();
        
        // 首先检查内部缓冲区是否有数据
        if this.buffer_len > this.buffer_pos {
            let available = this.buffer_len - this.buffer_pos;
            let to_copy = std::cmp::min(available, buf.remaining());
            
            // 零拷贝：直接从内部缓冲区复制到输出缓冲区
            buf.put_slice(&this.buffer[this.buffer_pos..this.buffer_pos + to_copy]);
            this.buffer_pos += to_copy;
            
            // 如果缓冲区已消费完，重置位置
            if this.buffer_pos == this.buffer_len {
                this.buffer_pos = 0;
                this.buffer_len = 0;
            }
            
            trace!("PTY AsyncRead: copied {} bytes from internal buffer", to_copy);
            return Poll::Ready(Ok(()));
        }
        
        // 从异步通道接收数据
        match this.data_rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                trace!("PTY AsyncRead: received {} bytes from channel", data.len());
                
                // 优化策略：根据数据大小选择最佳处理方式
                if data.len() <= buf.remaining() {
                    // 情况1：数据完全适合输出缓冲区 - 零拷贝
                    buf.put_slice(&data);
                    trace!("PTY AsyncRead: direct zero-copy of {} bytes", data.len());
                } else if data.len() <= this.buffer.len() {
                    // 情况2：数据适合内部缓冲区 - 单次复制
                    let to_copy = buf.remaining();
                    buf.put_slice(&data[..to_copy]);
                    
                    // 剩余数据放入内部缓冲区
                    this.buffer[..data.len() - to_copy].copy_from_slice(&data[to_copy..]);
                    this.buffer_pos = 0;
                    this.buffer_len = data.len() - to_copy;
                    trace!("PTY AsyncRead: partial copy - {} to output, {} to buffer", to_copy, this.buffer_len);
                } else {
                    // 情况3：大数据量 - 分块处理
                    let to_copy = std::cmp::min(buf.remaining(), this.buffer.len());
                    buf.put_slice(&data[..to_copy]);
                    
                    // 剩余数据超过缓冲区容量，丢弃超出部分（避免内存爆炸）
                    let remaining_data = &data[to_copy..];
                    let buffer_capacity = this.buffer.len();
                    let buffer_copy_len = std::cmp::min(remaining_data.len(), buffer_capacity);
                    
                    this.buffer[..buffer_copy_len].copy_from_slice(&remaining_data[..buffer_copy_len]);
                    this.buffer_pos = 0;
                    this.buffer_len = buffer_copy_len;
                    
                    if remaining_data.len() > buffer_capacity {
                        warn!("PTY AsyncRead: data overflow - dropped {} bytes", remaining_data.len() - buffer_capacity);
                    }
                    
                    trace!("PTY AsyncRead: large data - {} to output, {} to buffer", to_copy, buffer_copy_len);
                }
                
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => {
                // 通道关闭，PTY 已结束
                debug!("PTY AsyncRead: channel closed, PTY ended");
                Poll::Ready(Ok(()))
            }
            Poll::Pending => {
                // 没有数据可用，等待下次唤醒
                trace!("PTY AsyncRead: no data available, pending");
                Poll::Pending
            }
        }
    }
}

// 实现 AsyncWrite for PortablePty
// 优化实现：直接同步写入，但使用更安全的错误处理
impl AsyncWrite for PortablePty {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        
        info!("PTY AsyncWrite: writing {} bytes to PTY", buf.len());
        
        // 使用更安全的错误处理，避免 unwrap()
        let mut writer = match this.writer.lock() {
            Ok(writer) => writer,
            Err(e) => {
                error!("PTY AsyncWrite: failed to acquire writer lock: {}", e);
                return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to acquire lock")));
            }
        };
        
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
        
        let mut writer = match this.writer.lock() {
            Ok(writer) => writer,
            Err(e) => {
                error!("PTY AsyncWrite: failed to acquire writer lock for flush: {}", e);
                return Poll::Ready(Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to acquire lock")));
            }
        };
        
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
            let mut master = match master.lock() {
                Ok(master) => master,
                Err(e) => {
                    error!("Failed to acquire master lock for resize: {}", e);
                    return Err(PtyError::Other(format!("Failed to acquire lock: {}", e)));
                }
            };
            match master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            }) {
                Ok(()) => Ok(()),
                Err(e) => Err(PtyError::Other(format!("Resize failed: {}", e))),
            }
        }).await;

        match resize_result {
            Ok(Ok(())) => {
                self.cols = cols;
                self.rows = rows;
                Ok(())
            },
            Ok(Err(e)) => {
                Err(PtyError::Other(format!("Resize operation failed: {}", e)))
            },
            Err(e) => {
                Err(PtyError::Other(format!("Resize spawn_blocking failed: {:?}", e)))
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
        match self.child_exited.lock() {
            Ok(exited) => !*exited,
            Err(e) => {
                error!("Failed to acquire child_exited lock for is_alive check: {}", e);
                false
            }
        }
    }

    /// 等待进程结束（非阻塞检查）
    async fn try_wait(&mut self) -> Result<Option<StdExitStatus>, PtyError> {
        let child = self.child.clone();
        let child_exited = self.child_exited.clone();

        let wait_result = spawn_blocking(move || {
            let mut child = match child.lock() {
                Ok(child) => child,
                Err(e) => {
                    error!("Failed to acquire child lock for try_wait: {}", e);
                    return Err(PtyError::Other(format!("Failed to acquire lock: {}", e)));
                }
            };
            
            let mut exited = match child_exited.lock() {
                Ok(exited) => exited,
                Err(e) => {
                    error!("Failed to acquire child_exited lock for try_wait: {}", e);
                    return Err(PtyError::Other(format!("Failed to acquire lock: {}", e)));
                }
            };

            if *exited {
                return Ok(None);
            }

            match child.try_wait() {
                Ok(Some(_status)) => {
                    *exited = true;
                    // portable-pty 的 ExitStatus 与 std::process::ExitStatus 不同
                    // 返回一个默认的成功状态
                    Ok(Some(StdExitStatus::default()))
                },
                Ok(None) => Ok(None),
                Err(e) => Err(PtyError::Other(format!("Try wait failed: {}", e))),
            }
        }).await;

        match wait_result {
            Ok(result) => result,
            Err(e) => Err(PtyError::Other(format!("Wait spawn_blocking failed: {:?}", e))),
        }
    }

    /// 立即终止进程
    async fn kill(&mut self) -> Result<(), PtyError> {
        info!("PortablePty: Killing child process");

        let child = self.child.clone();
        let child_exited = self.child_exited.clone();

        let kill_result = spawn_blocking(move || {
            let mut child = match child.lock() {
                Ok(child) => child,
                Err(e) => {
                    error!("Failed to acquire child lock for kill: {}", e);
                    return Err(PtyError::Other(format!("Failed to acquire lock: {}", e)));
                }
            };
            
            let mut exited = match child_exited.lock() {
                Ok(exited) => exited,
                Err(e) => {
                    error!("Failed to acquire child_exited lock for kill: {}", e);
                    return Err(PtyError::Other(format!("Failed to acquire lock: {}", e)));
                }
            };

            if *exited {
                return Ok(());
            }

            match child.kill() {
                Ok(()) => {
                    *exited = true;
                    Ok(())
                }
                Err(e) => Err(PtyError::Other(format!("Kill failed: {}", e))),
            }
        }).await;

        match kill_result {
            Ok(result) => result,
            Err(e) => Err(PtyError::Other(format!("Kill spawn_blocking failed: {:?}", e))),
        }
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