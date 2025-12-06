/// Mock PTY implementation for testing purposes
/// This implementation simulates PTY operations with fixed responses
use std::sync::{Arc, Mutex};
use tracing::{info, error, debug};

use crate::pty::Pty;

/// Mock PTY implementation that returns fixed responses
pub struct MockPty {
    /// The shared buffer for communication between writer and reader
    buffer: Arc<Mutex<Vec<u8>>>,
    /// The input handler that processes commands and generates responses
    input_handler: Arc<Mutex<Box<dyn FnMut(&[u8]) -> Vec<u8> + Send>>>,
    /// Flag to track if the PTY is alive
    alive: Arc<Mutex<bool>>,
}

impl MockPty {
    /// Create a new MockPty instance
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send>> {
        info!("Creating new MockPty instance");
        
        // Create a shared buffer for communication
        let buffer = Arc::new(Mutex::new(Vec::new()));
        
        // Create a mock input handler
        let mut alive = Arc::new(Mutex::new(true));
        let alive_clone = alive.clone();
        
        // Create a closure to handle input and generate responses
        let input_handler: Box<dyn FnMut(&[u8]) -> Vec<u8> + Send> = Box::new(move |data| {
            let data_str = String::from_utf8_lossy(data);
            info!("Mock PTY input: {:?}", data_str);
            
            // Simulate processing the input and generating a fixed response
            let response = match data_str.trim() {
                "dir" => String::from("Directory listing:\n  file1.txt\n  file2.txt\n  folder/\n"),
                "echo hello" => String::from("hello\n"),
                "ls" => String::from("file1.txt  file2.txt  folder/\n"),
                "pwd" => String::from("/home/user\n"),
                "whoami" => String::from("user\n"),
                "exit" => {
                    // Update alive status
                    *alive_clone.lock().unwrap() = false;
                    String::from("Exiting...\n")
                },
                _ => format!("Unknown command: {}\n", data_str),
            };
            
            info!("Mock PTY generated response: {:?}", response);
            response.into_bytes()
        });
        
        Ok(Self {
            buffer,
            input_handler: Arc::new(Mutex::new(input_handler)),
            alive,
        })
    }
}

#[async_trait::async_trait]
impl Pty for MockPty {
    async fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("Mock PTY writing {} bytes", data.len());
        
        // Process the input and generate a response
        let response = { 
            let mut handler = self.input_handler.lock().unwrap();
            handler(data)
        };
        
        // Add the response to the shared buffer
        let mut buffer = self.buffer.lock().unwrap();
        buffer.extend_from_slice(&response);
        
        info!("Mock PTY write completed, generated {} bytes response", response.len());
        Ok(())
    }
    
    async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Box<dyn std::error::Error + Send>> {
        info!("Mock PTY read operation started, buffer size: {}", buffer.len());
        
        let mut internal_buffer = self.buffer.lock().unwrap();
        
        if internal_buffer.is_empty() {
            // No data available, return 0 to simulate non-blocking behavior
            debug!("Mock PTY buffer empty, returning 0 bytes");
            return Ok(0);
        }
        
        // Determine how much data to read
        let read_len = std::cmp::min(internal_buffer.len(), buffer.len());
        
        // Copy data to the provided buffer
        buffer[..read_len].copy_from_slice(&internal_buffer[..read_len]);
        
        // Remove the read data from the internal buffer
        internal_buffer.drain(0..read_len);
        
        let read_str = String::from_utf8_lossy(&buffer[..read_len]);
        info!("Mock PTY read {} bytes: {:?}", read_len, read_str);
        
        Ok(read_len)
    }
    
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("Mock PTY resized to {} cols x {} rows", cols, rows);
        Ok(())
    }
    
    async fn kill(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        info!("Mock PTY killed");
        *self.alive.lock().unwrap() = false;
        Ok(())
    }
    
    async fn is_alive(&self) -> Result<bool, Box<dyn std::error::Error + Send>> {
        let alive = *self.alive.lock().unwrap();
        debug!("Mock PTY alive status: {}", alive);
        Ok(alive)
    }
}
