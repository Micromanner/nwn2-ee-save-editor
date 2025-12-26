use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Configuration for the NWN2 Save Editor Tauri app
/// Reads from the same .env file as the Python backend

pub struct Config {
    pub fastapi_port: u16,
    pub fastapi_host: String,
}

// Global state for dynamic port assignment
lazy_static::lazy_static! {
    static ref DYNAMIC_PORT: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
}

impl Config {
    /// Set the dynamically assigned port (called by sidecar manager)
    pub fn set_dynamic_port(port: u16) {
        let mut dynamic_port = DYNAMIC_PORT.lock().unwrap();
        *dynamic_port = Some(port);
    }
    
    /// Get the current effective port (dynamic if available, otherwise configured)
    /// If the configured port is 0 and no dynamic port is assigned, it finds a free one.
    pub fn get_effective_port(&self) -> u16 {
        let mut dynamic_port = DYNAMIC_PORT.lock().unwrap();
        
        // Return already assigned dynamic port (if valid)
        if let Some(p) = *dynamic_port {
            if p != 0 {
                return p;
            }
        }

        // If a specific port is configured (non-zero), use it
        if self.fastapi_port != 0 {
            return self.fastapi_port;
        }

        // Port is 0 (dynamic) but not yet assigned - find a free one now
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap_or_else(|_| {
                log::warn!("Failed to bind to 127.0.0.1:0, trying 0.0.0.0:0");
                std::net::TcpListener::bind("0.0.0.0:0")
                    .expect("Critical: Could not find any free port on the system")
            });
            
        let port = listener.local_addr().expect("Failed to get local addr").port();
        log::info!("Pre-assigning dynamic port: {}", port);
        
        *dynamic_port = Some(port);
        port
    }
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
            .unwrap_or_else(|_| "0".to_string())
            .parse::<u16>()
            .unwrap_or(0);
            
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
        let effective_port = self.get_effective_port();
        format!("http://{}:{}", self.fastapi_host, effective_port)
    }
    
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}