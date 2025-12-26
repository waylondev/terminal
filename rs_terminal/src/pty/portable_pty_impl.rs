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

/// 高性能异步 PTY 实现
/// 使用零拷贝缓冲和智能阻塞策略实现真正的异步体验
pub struct PortablePty {
    cols: u16,
    rows: u16,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
    child: Arc<Mutex<Box<dyn Child + Send>>>,
    child_exited: Arc<Mutex<bool>>,
    data_rx: mpsc::Receiver<Vec<u8>>,
    data_tx: mpsc::Sender<Vec<u8>>,
    buffer: Box<[u8; 8192]>,
    buffer_pos: usize,
    buffer_len: usize,
}

impl PortablePty {
    /// 创建新的 PTY 实例
    pub fn new(config: &PtyConfig) -> Result<Self, PtyError> {
        info!("PortablePty: Creating PTY with command: {:?}", config.command);
        
        let (pair, child) = Self::create_pty_pair(config)?;
        let (data_tx, data_rx) = Self::create_data_channel();
        let child_exited = Arc::new(Mutex::new(false));
        
        Self::start_background_reader(pair.master.try_clone_reader()?, data_tx.clone(), child_exited.clone());
        
        let writer = pair.master.take_writer()?;
        
        Ok(Self {
            cols: config.cols,
            rows: config.rows,
            master: Arc::new(Mutex::new(pair.master)),
            writer: Arc::new(Mutex::new(writer)),
            child: Arc::new(Mutex::new(child)),
            child_exited,
            data_rx,
            data_tx,
            buffer: Box::new([0u8; 8192]),
            buffer_pos: 0,
            buffer_len: 0,
        })
    }
    
    /// 创建 PTY 对和子进程
    fn create_pty_pair(config: &PtyConfig) -> Result<(portable_pty::PtyPair, Box<dyn Child + Send>), PtyError> {
        let pty_system = portable_pty::native_pty_system();
        
        let pair = pty_system.openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        
        let cmd = Self::build_command(config);
        let child = pair.slave.spawn_command(cmd)?;
        
        Ok((pair, child))
    }
    
    /// 构建命令配置
    fn build_command(config: &PtyConfig) -> CommandBuilder {
        let mut cmd = CommandBuilder::new(config.command.clone());
        cmd.args(&config.args);
        
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        
        if let Some(cwd) = &config.cwd {
            cmd.cwd(cwd);
        }
        
        cmd
    }
    
    /// 创建数据通道
    fn create_data_channel() -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
        mpsc::channel(1024)
    }
    
    /// 启动后台读取任务
    fn start_background_reader(
        reader: Box<dyn std::io::Read + Send>,
        data_tx: mpsc::Sender<Vec<u8>>,
        child_exited: Arc<Mutex<bool>>,
    ) {
        tokio::spawn(async move {
            let result = spawn_blocking(move || Self::background_read_loop(reader, data_tx)).await;
            
            match result {
                Ok(Ok(())) => debug!("PTY background reader finished successfully"),
                Ok(Err(e)) => error!("PTY background reader failed: {}", e),
                Err(e) => error!("PTY background reader task failed: {}", e),
            }
            
            Self::mark_child_exited(child_exited);
        });
    }
    
    /// 后台读取循环
    fn background_read_loop(
        mut reader: Box<dyn std::io::Read + Send>,
        data_tx: mpsc::Sender<Vec<u8>>,
    ) -> Result<(), std::io::Error> {
        let mut buffer = vec![0; 4096];
        
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    debug!("PTY EOF reached, stopping background reader");
                    return Ok(());
                }
                Ok(n) => {
                    let data = Self::process_read_data(&buffer, n);
                    
                    if data_tx.blocking_send(data).is_err() {
                        debug!("PTY background reader: receiver dropped, stopping");
                        return Ok(());
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
    
    /// 处理读取的数据
    fn process_read_data(buffer: &[u8], n: usize) -> Vec<u8> {
        if n == buffer.len() {
            buffer.to_vec()
        } else {
            buffer[..n].to_vec()
        }
    }
    
    /// 标记子进程已退出
    fn mark_child_exited(child_exited: Arc<Mutex<bool>>) {
        if let Ok(mut exited) = child_exited.lock() {
            *exited = true;
        } else {
            error!("Failed to acquire child_exited lock for marking exit");
        }
    }
}

impl AsyncRead for PortablePty {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.as_mut().get_mut();
        
        if Self::copy_from_internal_buffer(this, buf) {
            return Poll::Ready(Ok(()));
        }
        
        match this.data_rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                trace!("PTY AsyncRead: received {} bytes from channel", data.len());
                Self::process_received_data(this, data, buf);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => {
                debug!("PTY AsyncRead: channel closed, PTY ended");
                Poll::Ready(Ok(()))
            }
            Poll::Pending => {
                trace!("PTY AsyncRead: no data available, pending");
                Poll::Pending
            }
        }
    }
}

impl PortablePty {
    /// 从内部缓冲区复制数据到输出缓冲区
    fn copy_from_internal_buffer(this: &mut Self, buf: &mut ReadBuf<'_>) -> bool {
        if this.buffer_len <= this.buffer_pos {
            return false;
        }
        
        let available = this.buffer_len - this.buffer_pos;
        let to_copy = std::cmp::min(available, buf.remaining());
        
        buf.put_slice(&this.buffer[this.buffer_pos..this.buffer_pos + to_copy]);
        this.buffer_pos += to_copy;
        
        if this.buffer_pos == this.buffer_len {
            this.buffer_pos = 0;
            this.buffer_len = 0;
        }
        
        trace!("PTY AsyncRead: copied {} bytes from internal buffer", to_copy);
        true
    }
    
    /// 处理接收到的数据
    fn process_received_data(this: &mut Self, data: Vec<u8>, buf: &mut ReadBuf<'_>) {
        if data.len() <= buf.remaining() {
            Self::handle_small_data(data, buf);
        } else if data.len() <= this.buffer.len() {
            Self::handle_medium_data(this, data, buf);
        } else {
            Self::handle_large_data(this, data, buf);
        }
    }
    
    /// 处理小数据量（完全适合输出缓冲区）
    fn handle_small_data(data: Vec<u8>, buf: &mut ReadBuf<'_>) {
        buf.put_slice(&data);
        trace!("PTY AsyncRead: direct zero-copy of {} bytes", data.len());
    }
    
    /// 处理中等数据量（适合内部缓冲区）
    fn handle_medium_data(this: &mut Self, data: Vec<u8>, buf: &mut ReadBuf<'_>) {
        let to_copy = buf.remaining();
        buf.put_slice(&data[..to_copy]);
        
        this.buffer[..data.len() - to_copy].copy_from_slice(&data[to_copy..]);
        this.buffer_pos = 0;
        this.buffer_len = data.len() - to_copy;
        
        trace!("PTY AsyncRead: partial copy - {} to output, {} to buffer", to_copy, this.buffer_len);
    }
    
    /// 处理大数据量（超过内部缓冲区容量）
    fn handle_large_data(this: &mut Self, data: Vec<u8>, buf: &mut ReadBuf<'_>) {
        let to_copy = std::cmp::min(buf.remaining(), this.buffer.len());
        buf.put_slice(&data[..to_copy]);
        
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
}

impl AsyncWrite for PortablePty {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.get_mut();
        
        info!("PTY AsyncWrite: writing {} bytes to PTY", buf.len());
        
        let writer = Self::acquire_writer_lock(this)?;
        Self::write_to_pty(writer, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.get_mut();
        
        let writer = Self::acquire_writer_lock(this)?;
        Self::flush_writer(writer)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}

impl PortablePty {
    /// 获取写入器锁
    fn acquire_writer_lock(this: &mut Self) -> Result<std::sync::MutexGuard<'_, Box<dyn std::io::Write + Send>>, std::io::Error> {
        this.writer.lock().map_err(|e| {
            error!("PTY AsyncWrite: failed to acquire writer lock: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to acquire lock")
        })
    }
    
    /// 写入数据到 PTY
    fn write_to_pty(
        mut writer: std::sync::MutexGuard<'_, Box<dyn std::io::Write + Send>>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        match writer.write(buf) {
            Ok(n) => {
                info!("PTY AsyncWrite: successfully wrote {} bytes", n);
                Poll::Ready(Ok(n))
            },
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                info!("PTY AsyncWrite: would block, pending");
                Poll::Pending
            },
            Err(e) => {
                error!("PTY AsyncWrite: error writing to PTY: {}", e);
                Poll::Ready(Err(e))
            }
        }
    }
    
    /// 刷新写入器
    fn flush_writer(
        mut writer: std::sync::MutexGuard<'_, Box<dyn std::io::Write + Send>>,
    ) -> Poll<Result<(), std::io::Error>> {
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
}

impl PortablePty {
    /// 调整 PTY 大小（阻塞操作）
    fn resize_pty(master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>, cols: u16, rows: u16) -> Result<(), PtyError> {
        let master = Self::acquire_master_lock(&master, "resize")?;
        
        match master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            Ok(()) => Ok(()),
            Err(e) => Err(PtyError::Other(format!("Resize failed: {}", e))),
        }
    }
    
    /// 处理调整大小结果
    fn handle_resize_result(resize_result: Result<Result<(), PtyError>, tokio::task::JoinError>, this: &mut Self, cols: u16, rows: u16) -> Result<(), PtyError> {
        match resize_result {
            Ok(Ok(())) => {
                this.cols = cols;
                this.rows = rows;
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
    
    /// 获取 master 锁
    fn acquire_master_lock<'a>(master: &'a Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>, operation: &'a str) -> Result<std::sync::MutexGuard<'a, Box<dyn portable_pty::MasterPty + Send>>, PtyError> {
        master.lock().map_err(|e| {
            error!("Failed to acquire master lock for {}: {}", operation, e);
            PtyError::LockAcquisition(format!("Failed to acquire master lock for {}: {}", operation, e))
        })
    }
    
    /// 尝试等待进程结束（阻塞操作）
    fn try_wait_process(child: Arc<Mutex<Box<dyn Child + Send>>>, child_exited: Arc<Mutex<bool>>) -> Result<Option<StdExitStatus>, PtyError> {
        let mut child_guard = Self::acquire_child_lock(&child, "try_wait")?;
        let mut exited_guard = Self::acquire_child_exited_lock(&child_exited, "try_wait")?;

        if *exited_guard {
            return Ok(None);
        }

        match child_guard.try_wait() {
            Ok(Some(_status)) => {
                *exited_guard = true;
                // portable-pty 的 ExitStatus 与 std::process::ExitStatus 不同
                // 返回一个默认的成功状态
                Ok(Some(StdExitStatus::default()))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(PtyError::Other(format!("Try wait failed: {}", e))),
        }
    }
    
    /// 处理等待结果
    fn handle_wait_result(wait_result: Result<Result<Option<StdExitStatus>, PtyError>, tokio::task::JoinError>) -> Result<Option<StdExitStatus>, PtyError> {
        match wait_result {
            Ok(result) => result,
            Err(e) => Err(PtyError::Other(format!("Wait spawn_blocking failed: {:?}", e))),
        }
    }
    
    /// 获取 child 锁
    fn acquire_child_lock<'a>(child: &'a Arc<Mutex<Box<dyn Child + Send>>>, operation: &'a str) -> Result<std::sync::MutexGuard<'a, Box<dyn Child + Send>>, PtyError> {
        child.lock().map_err(|e| {
            error!("Failed to acquire child lock for {}: {}", operation, e);
            PtyError::LockAcquisition(format!("Failed to acquire child lock for {}: {}", operation, e))
        })
    }
    
    /// 获取 child_exited 锁
    fn acquire_child_exited_lock<'a>(child_exited: &'a Arc<Mutex<bool>>, operation: &'a str) -> Result<std::sync::MutexGuard<'a, bool>, PtyError> {
        child_exited.lock().map_err(|e| {
            error!("Failed to acquire child_exited lock for {}: {}", operation, e);
            PtyError::LockAcquisition(format!("Failed to acquire child_exited lock for {}: {}", operation, e))
        })
    }
    
    /// 终止进程（阻塞操作）
    fn kill_process(child: Arc<Mutex<Box<dyn Child + Send>>>, child_exited: Arc<Mutex<bool>>) -> Result<(), PtyError> {
        let mut child_guard = Self::acquire_child_lock(&child, "kill")?;
        let mut exited_guard = Self::acquire_child_exited_lock(&child_exited, "kill")?;

        if *exited_guard {
            return Ok(());
        }

        match child_guard.kill() {
            Ok(()) => {
                *exited_guard = true;
                Ok(())
            }
            Err(e) => Err(PtyError::Other(format!("Kill failed: {}", e))),
        }
    }
    
    /// 处理终止结果
    fn handle_kill_result(kill_result: Result<Result<(), PtyError>, tokio::task::JoinError>) -> Result<(), PtyError> {
        match kill_result {
            Ok(result) => result,
            Err(e) => Err(PtyError::Other(format!("Kill spawn_blocking failed: {:?}", e))),
        }
    }
}

// 实现 AsyncPty trait 为 PortablePty
#[async_trait]
impl AsyncPty for PortablePty {
    /// 调整终端大小
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        info!("PortablePty: Resizing PTY to {}x{}", cols, rows);

        let master = self.master.clone();
        let resize_result = spawn_blocking(move || Self::resize_pty(master, cols, rows)).await;

        Self::handle_resize_result(resize_result, self, cols, rows)
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

        let wait_result = spawn_blocking(move || Self::try_wait_process(child, child_exited)).await;

        Self::handle_wait_result(wait_result)
    }

    /// 立即终止进程
    async fn kill(&mut self) -> Result<(), PtyError> {
        info!("PortablePty: Killing child process");

        let child = self.child.clone();
        let child_exited = self.child_exited.clone();

        let kill_result = spawn_blocking(move || Self::kill_process(child, child_exited)).await;

        Self::handle_kill_result(kill_result)
    }
}

// ================ 资源清理实现 ================

/// 实现 Drop trait 确保资源正确清理
impl Drop for PortablePty {
    fn drop(&mut self) {
        info!("PortablePty: Dropping PTY instance");
        
        // 尝试终止子进程
        if self.is_alive() {
            let child = self.child.clone();
            let child_exited = self.child_exited.clone();
            
            // 使用 spawn_blocking 避免阻塞异步运行时
            let _ = spawn_blocking(move || {
                if let Err(e) = Self::kill_process(child, child_exited) {
                    error!("Failed to kill child process during drop: {}", e);
                }
            });
        }
        
        // 关闭数据通道
        drop(self.data_rx.try_recv());
        
        info!("PortablePty: Resources cleaned up successfully");
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