// src/portable/windows.rs
use super::pty_trait::{AsyncPty, PtyConfig, PtyError, PtyFactory};
use async_trait::async_trait;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, Child, PtySystem};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use std::task::{Context, Poll, Waker};
use std::thread;
use std::time::Duration;
use tracing::{error, info};

/// Windows 专用的异步 PTY 包装
pub struct WindowsPty {
    // 用于接收 PTY 输出的通道
    read_rx: Arc<Mutex<Receiver<Vec<u8>>>>,
    // 用于发送输入到 PTY 的通道
    write_tx: Sender<Vec<u8>>,
    // 读取缓冲区
    read_buffer: Vec<u8>,
    // 缓冲区读取位置
    read_pos: usize,
    // 子进程引用
    child: Arc<Mutex<Option<Box<dyn Child>>>>,
    // 进程是否已退出
    child_exited: bool,
    // 读取唤醒器
    waker: Arc<Mutex<Option<Waker>>>,
}

impl WindowsPty {
    pub fn new(config: &PtyConfig) -> Result<Self, PtyError> {
        let pty_system = NativePtySystem::default();

        // 打开 PTY 并设置大小
        let pair = pty_system.openpty(PtySize {
            rows: config.rows,
            cols: config.cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // 打印调试信息
        info!("WindowsPty: Creating command with:");
        info!("  command: {:?}", config.command);
        info!("  args: {:?}", config.args);
        info!("  cwd: {:?}", config.cwd);
        info!("  env count: {}", config.env.len());
        // 打印几个关键的环境变量
        if let Some((_, path)) = config.env.iter().find(|(k, _)| k == "PATH") {
            info!("  PATH: {:?}", path);
        }
        if let Some((_, term)) = config.env.iter().find(|(k, _)| k == "TERM") {
            info!("  TERM: {:?}", term);
        }

        // 设置命令
        let mut cmd = CommandBuilder::new(&config.command);
        for arg in &config.args {
            cmd.arg(arg);
        }

        // 首先应用配置文件中的环境变量
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // 设置工作目录
        if let Some(cwd) = &config.cwd {
            info!("  Setting cwd to: {:?}", cwd);
            cmd.cwd(cwd);
        }

        // 启动子进程
        let child = pair.slave.spawn_command(cmd)?;

        // 放弃 slave 所有权，否则子进程会立即退出
        drop(pair.slave);

        // 创建通信通道
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>();
        let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>();
        let waker = Arc::new(Mutex::new(None::<Waker>));
        let waker_clone = waker.clone();

        // 复制 master 读取器
        let mut reader = pair.master.try_clone_reader()?;
        let mut writer = pair.master.take_writer()?;

        // 读取线程：从 PTY 读取数据并发送到通道
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => {
                        // EOF，退出循环
                        break;
                    }
                    Ok(n) => {
                        let data = buffer[..n].to_vec();
                        if read_tx.send(data).is_err() {
                            // 通道已关闭，退出循环
                            break;
                        }
                        // 唤醒异步读取器
                        if let Some(w) = waker_clone.lock().unwrap().take() {
                            w.wake();
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // 无数据，短暂休眠后重试
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => {
                        // 其他错误，退出循环
                        break;
                    }
                }
            }
        });

        // 写入线程：从通道读取数据并写入 PTY
        thread::spawn(move || {
            loop {
                match write_rx.recv() {
                    Ok(data) => {
                        if writer.write_all(&data).is_err() {
                            // 写入错误，退出循环
                            break;
                        }
                        if writer.flush().is_err() {
                            // 刷新错误，退出循环
                            break;
                        }
                    }
                    Err(_) => {
                        // 通道已关闭，退出循环
                        break;
                    }
                }
            }
        });

        Ok(Self {
            read_rx: Arc::new(Mutex::new(read_rx)),
            write_tx,
            read_buffer: Vec::new(),
            read_pos: 0,
            child: Arc::new(Mutex::new(Some(child))),
            child_exited: false,
            waker,
        })
    }
}

// 实现 AsyncRead
impl AsyncRead for WindowsPty {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        // 先检查缓冲区是否有数据
        if self.read_pos < self.read_buffer.len() {
            let remaining = &self.read_buffer[self.read_pos..];
            let to_copy = std::cmp::min(remaining.len(), buf.remaining());
            
            buf.put_slice(&remaining[..to_copy]);
            self.read_pos += to_copy;
            
            // 如果缓冲区已读完，清空它
            if self.read_pos >= self.read_buffer.len() {
                self.read_buffer.clear();
                self.read_pos = 0;
            }
            
            return Poll::Ready(Ok(()));
        }

        // 尝试从通道获取新数据
        let try_recv_result;
        {
            // 在单独的作用域中获取锁，确保锁能及时释放
            let mut read_rx_lock = self.read_rx.lock().unwrap();
            try_recv_result = read_rx_lock.try_recv();
        }

        match try_recv_result {
            Ok(data) => {
                // 将数据存入缓冲区，然后递归调用自身
                self.read_buffer = data;
                self.read_pos = 0;
                self.poll_read(cx, buf)
            }
            Err(TryRecvError::Empty) => {
                // 无数据可用，保存唤醒器并返回Pending
                *self.waker.lock().unwrap() = Some(cx.waker().clone());
                Poll::Pending
            }
            Err(TryRecvError::Disconnected) => {
                // 通道已关闭，返回EOF
                self.child_exited = true;
                Poll::Ready(Ok(()))
            }
        }
    }
}

// 实现 AsyncWrite
impl AsyncWrite for WindowsPty {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let self_mut = self.get_mut();
        
        if self_mut.child_exited {
            return Poll::Ready(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "PTY process has terminated",
            )));
        }
        
        // 尝试发送数据到写入通道
        match self_mut.write_tx.send(buf.to_vec()) {
            Ok(_) => Poll::Ready(Ok(buf.len())),
            Err(_) => {
                // 通道已关闭
                self_mut.child_exited = true;
                Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "PTY write channel closed",
                )))
            }
        }
    }
    
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // 写入操作是同步的，不需要显式刷新
        Poll::Ready(Ok(()))
    }
    
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        // 关闭写入通道
        drop(self.get_mut().write_tx.clone());
        Poll::Ready(Ok(()))
    }
}

// 实现 AsyncPty trait
#[async_trait::async_trait]
impl AsyncPty for WindowsPty {
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        // 简化实现，暂时不支持调整大小
        Ok(())
    }
    
    fn pid(&self) -> Option<u32> {
        // portable-pty 不直接暴露 PID
        None
    }
    
    fn is_alive(&self) -> bool {
        !self.child_exited
    }
    
    async fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, PtyError> {
        if self.child_exited {
            return Ok(None);
        }
        
        // 检查子进程状态
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            if let Some(_status) = child.try_wait()? {
                self.child_exited = true;
                // 转换为标准退出状态
                Ok(Some(std::process::ExitStatus::default()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
    
    async fn kill(&mut self) -> Result<(), PtyError> {
        if self.child_exited {
            return Ok(());
        }
        
        // 杀死子进程
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            child.kill()?;
            self.child_exited = true;
        }
        Ok(())
    }
}

// ================ 工厂实现 ================

/// Windows PTY factory for creating WindowsPty instances
pub struct WindowsPtyFactory;

impl Default for WindowsPtyFactory {
    fn default() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl PtyFactory for WindowsPtyFactory {
    async fn create(&self, config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
        info!("WindowsPtyFactory: Creating Windows PTY with command: {:?}, args: {:?}", config.command, config.args);
        
        match WindowsPty::new(config) {
            Ok(pty) => {
                info!("WindowsPtyFactory: Successfully created Windows PTY");
                Ok(Box::new(pty))
            },
            Err(e) => {
                error!("WindowsPtyFactory: Failed to create Windows PTY: {}", e);
                Err(e)
            }
        }
    }
    
    fn name(&self) -> &'static str {
        "windows-pty"
    }
}