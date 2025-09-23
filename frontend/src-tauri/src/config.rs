use std::env;
use std::path::PathBuf;

/// Configuration for the NWN2 Save Editor Tauri app
/// Reads from the same .env file as the Python backend

pub struct Config {
    pub fastapi_port: u16,
    pub fastapi_host: String,
}

impl Config {
    pub fn new() -> Self {
        // Load .env file from backend directory
        let backend_dir = Self::get_backend_dir();
        let env_path = backend_dir.join(".env");
        
        if env_path.exists() {
            if let Err(e) = dotenvy::from_path(&env_path) {
                log::warn!("Failed to load .env from {:?}: {}", env_path, e);
            }
        }
        
        let port = env::var("PORT")
            .unwrap_or_else(|_| "8001".to_string())
            .parse::<u16>()
            .unwrap_or(8001);
            
        let host = env::var("HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        
        Config {
            fastapi_port: port,
            fastapi_host: host,
        }
    }
    
    fn get_backend_dir() -> PathBuf {
        // Get path to backend directory relative to Tauri app
        let exe_dir = env::current_exe()
            .map(|p| p.parent().unwrap_or_else(|| std::path::Path::new(".")).to_path_buf())
            .unwrap_or_else(|_| PathBuf::from("."));
            
        // In development: frontend/src-tauri/target/debug/
        // In production: different structure
        if cfg!(debug_assertions) {
            exe_dir.join("../../../../backend")
        } else {
            exe_dir.join("../../backend")
        }
    }
    
    pub fn get_base_url(&self) -> String {
        format!("http://{}:{}", self.fastapi_host, self.fastapi_port)
    }
    
    pub fn get_api_base_url(&self) -> String {
        format!("http://{}:{}/api", self.fastapi_host, self.fastapi_port)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}