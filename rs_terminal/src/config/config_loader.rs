/// Configuration file loader for rs_terminal
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::info;
use crate::config::TerminalConfig;

/// Configuration loader responsible for loading and parsing configuration files
pub struct ConfigLoader;

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self
    }

    /// Load configuration from a file
    pub fn load_config(&self, config_path: Option<&Path>) -> TerminalConfig {
        // 处理配置文件路径
        let config_file_path = match config_path {
            Some(path) => path.to_path_buf(),
            None => {
                // 使用默认配置文件路径
                match default_config_path() {
                    Some(path) => {
                        info!("Using default configuration file path: {:?}", path);
                        path
                    },
                    None => {
                        panic!("No configuration file path specified and default path not available")
                    }
                }
            }
        };
        
        // 从文件加载配置
        self.load_config_from_file(&config_file_path)
    }

    /// Load configuration from a specific file path
    fn load_config_from_file(&self, path: &Path) -> TerminalConfig {
        info!("Loading configuration from file: {:?}", path);
        
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) => {
                panic!("Failed to open configuration file: {}", e);
            }
        };
        
        let mut contents = String::new();
        if let Err(e) = file.read_to_string(&mut contents) {
            panic!("Failed to read configuration file: {}", e);
        }
        
        self.parse_config(&contents)
    }

    /// Parse configuration from string content
    fn parse_config(&self, content: &str) -> TerminalConfig {
        match toml::from_str::<TerminalConfig>(content) {
            Ok(config) => {
                info!("Configuration parsed successfully");
                config
            },
            Err(e) => {
                panic!("Failed to parse configuration: {}", e);
            }
        }
    }
}

/// Default configuration path
pub fn default_config_path() -> Option<std::path::PathBuf> {
    // 使用当前工作目录作为默认配置文件目录
    Some(std::env::current_dir().unwrap().join("config.toml"))
}
