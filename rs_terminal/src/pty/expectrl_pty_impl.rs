use std::process::ExitStatus;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::pty::{AsyncPty, PtyConfig, PtyError, PtyFactory};

/// 基于expectrl库的PTY实现
pub struct ExpectrlPty {
    #[cfg(unix)]
    // 使用动态类型避免泛型参数问题
    session: Mutex<Box<dyn std::any::Any + Send + Sync>>,
    pid: Option<u32>,
    child_exited: bool,
}

#[async_trait]
impl AsyncPty for ExpectrlPty {
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtyError> {
        #[cfg(unix)]
        {
            let mut session_box = self.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .ok_or_else(|| PtyError::Other("Failed to downcast session".to_string()))?;
            
            session.resize(cols, rows).await.map_err(|e| {
                PtyError::ResizeFailed(e.to_string())
            })
        }
        
        #[cfg(not(unix))]
        {
            Err(PtyError::NotAvailable)
        }
    }

    fn pid(&self) -> Option<u32> {
        self.pid
    }

    fn is_alive(&self) -> bool {
        !self.child_exited
    }

    async fn try_wait(&mut self) -> Result<Option<ExitStatus>, PtyError> {
        #[cfg(unix)]
        {
            let mut session_box = self.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .ok_or_else(|| PtyError::Other("Failed to downcast session".to_string()))?;
            
            // 使用try_wait方法尝试获取进程状态
            let status = session.try_wait().await.map_err(|e| {
                PtyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
            })?;
            
            if status.is_some() {
                self.child_exited = true;
            }
            
            Ok(status)
        }
        
        #[cfg(not(unix))]
        {
            Err(PtyError::NotAvailable)
        }
    }

    async fn kill(&mut self) -> Result<(), PtyError> {
        #[cfg(unix)]
        {
            let mut session_box = self.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .ok_or_else(|| PtyError::Other("Failed to downcast session".to_string()))?;
            
            session.kill().await.map_err(|e| {
                PtyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
            })
        }
        
        #[cfg(not(unix))]
        {
            Err(PtyError::NotAvailable)
        }
    }
}

// 实现AsyncRead和AsyncWrite，转发给expectrl的Session
impl tokio::io::AsyncRead for ExpectrlPty {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        #[cfg(unix)]
        {
            let this = self.get_mut();
            let mut session_box = this.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .expect("Failed to downcast session");
            
            tokio::io::AsyncRead::poll_read(std::pin::Pin::new(session), cx, buf)
        }
        
        #[cfg(not(unix))]
        {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Not available on non-Unix platforms"
            )))
        }
    }
}

impl tokio::io::AsyncWrite for ExpectrlPty {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        #[cfg(unix)]
        {
            let this = self.get_mut();
            let mut session_box = this.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .expect("Failed to downcast session");
            
            tokio::io::AsyncWrite::poll_write(std::pin::Pin::new(session), cx, buf)
        }
        
        #[cfg(not(unix))]
        {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Not available on non-Unix platforms"
            )))
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        #[cfg(unix)]
        {
            let this = self.get_mut();
            let mut session_box = this.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .expect("Failed to downcast session");
            
            tokio::io::AsyncWrite::poll_flush(std::pin::Pin::new(session), cx)
        }
        
        #[cfg(not(unix))]
        {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Not available on non-Unix platforms"
            )))
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        #[cfg(unix)]
        {
            let this = self.get_mut();
            let mut session_box = this.session.lock().unwrap();
            // 尝试将动态类型转换为实际类型
            let session = session_box.downcast_mut::<expectrl::Session<_, _>>()
                .expect("Failed to downcast session");
            
            tokio::io::AsyncWrite::poll_shutdown(std::pin::Pin::new(session), cx)
        }
        
        #[cfg(not(unix))]
        {
            std::task::Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Not available on non-Unix platforms"
            )))
        }
    }
}

/// Expectrl PTY工厂
pub struct ExpectrlPtyFactory;

#[async_trait]
impl PtyFactory for ExpectrlPtyFactory {
    async fn create(&self, config: &PtyConfig) -> Result<Box<dyn AsyncPty>, PtyError> {
        #[cfg(unix)]
        {
            // 构建命令行字符串
            let mut cmd_str = config.command.clone();
            for arg in &config.args {
                cmd_str.push_str(" ");
                cmd_str.push_str(arg);
            }
            
            // 生成并启动会话
            let session = expectrl::spawn(cmd_str).map_err(|e| {
                PtyError::SpawnFailed(e.to_string())
            })?;
            
            // 使用动态类型来存储会话
            let session_box: Box<dyn std::any::Any + Send + Sync> = Box::new(session);
            
            // PID初始化为None，因为expectrl库可能不直接提供PID访问
            let pid = None;
            
            Ok(Box::new(ExpectrlPty {
                session: Mutex::new(session_box),
                pid,
                child_exited: false,
            }))
        }
        
        #[cfg(not(unix))]
        {
            Err(PtyError::NotAvailable)
        }
    }

    fn name(&self) -> &'static str {
        "expectrl-pty"
    }
}
