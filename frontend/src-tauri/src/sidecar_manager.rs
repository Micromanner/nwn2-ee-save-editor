use tauri::Manager;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use std::sync::Mutex;
use log::{info, error};
use tauri_plugin_shell::process::CommandChild;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use std::path::PathBuf;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct FastAPISidecar {
    pub pid: Mutex<Option<u32>>,
    pub child: Mutex<Option<CommandChild>>,
    pub startup_mutex: Arc<TokioMutex<bool>>, // Prevents concurrent startups
}

impl FastAPISidecar {
    pub fn new() -> Self {
        FastAPISidecar {
            pid: Mutex::new(None),
            child: Mutex::new(None),
            startup_mutex: Arc::new(TokioMutex::new(false)),
        }
    }
}

impl Drop for FastAPISidecar {
    fn drop(&mut self) {
        // Clean up FastAPI process when app exits
        info!("FastAPISidecar Drop called - attempting graceful shutdown");
        
        // Try graceful shutdown first
        let rt = tokio::runtime::Runtime::new();
        if let Ok(runtime) = rt {
            let _ = runtime.block_on(async {
                // Try to send shutdown signal to FastAPI
                match reqwest::Client::new()
                    .post("http://127.0.0.1:8000/api/system/shutdown/")
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    Ok(_) => {
                        info!("Sent graceful shutdown signal to FastAPI");
                        // Wait a bit for graceful shutdown
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    }
                    Err(e) => {
                        info!("Could not send graceful shutdown (server may already be down): {}", e);
                    }
                }
            });
        }
        
        // Force kill if still running
        if let Ok(mut child_lock) = self.child.lock() {
            if let Some(child) = child_lock.take() {
                info!("Force killing FastAPI sidecar on exit");
                let _ = child.kill();
            }
        }
    }
}

#[tauri::command]
pub async fn start_fastapi_sidecar(app: tauri::AppHandle) -> Result<String, String> {
    if let Some(sidecar) = app.try_state::<FastAPISidecar>() {
        // Prevent concurrent startups
        let _guard = sidecar.startup_mutex.lock().await;
        
        // Check if FastAPI is already running
        if let Ok(response) = reqwest::get("http://127.0.0.1:8000/api/health/").await {
            if response.status().is_success() {
                info!("FastAPI already running, force killing it for fresh start");
                let _ = force_kill_fastapi_sidecar(app.clone()).await;
            }
        }
        
        info!("Starting FastAPI sidecar");
        
        // Create lock file to indicate startup in progress
        if let Err(e) = create_fastapi_lock() {
            error!("Failed to create FastAPI lock: {}", e);
        }
        
        // Only kill if we're really starting fresh - use force kill during startup
        let _ = force_kill_fastapi_sidecar(app.clone()).await;
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    } else {
        return Err("FastAPI sidecar state not initialized".to_string());
    }
    
    // Check if port 8000 is actually in use before trying to kill anything
    if cfg!(debug_assertions) {
        // Quick check if port is already free
        let port_in_use = match reqwest::Client::new()
            .get("http://127.0.0.1:8000/api/health/")
            .timeout(std::time::Duration::from_millis(500))
            .send()
            .await 
        {
            Ok(_) => true,
            Err(_) => false,
        };
        
        if !port_in_use {
            info!("Port 8000 is free, skipping kill process - proceeding directly to startup");
        } else {
            info!("Port 8000 is in use, attempting to kill existing process");
            
            // Use native OS commands to find and kill process on port 8000
        let kill_result = if cfg!(target_os = "windows") {
            // Windows: Use netstat and taskkill
            // First find the PID
            let netstat_output = app.shell()
                .command("cmd")
                .args(["/C", "netstat -ano | findstr :8000"])
                .output()
                .await;
                
            if let Ok(output) = netstat_output {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Parse PID from netstat output (last column)
                if let Some(line) = output_str.lines().find(|l| l.contains("LISTENING")) {
                    if let Some(pid) = line.split_whitespace().last() {
                        info!("Found process {} on port 8000, killing it", pid);
                        app.shell()
                            .command("cmd")
                            .args(["/C", &format!("taskkill /F /PID {}", pid)])
                            .output()
                            .await
                    } else {
                        Ok(output)
                    }
                } else {
                    Ok(output)
                }
            } else {
                netstat_output
            }
        } else {
            // Unix: Use lsof or ss and kill
            app.shell()
                .command("sh")
                .args(["-c", "lsof -ti:8000 | xargs kill -9 2>/dev/null || true"])
                .output()
                .await
        };
        
        match kill_result {
            Ok(output) => {
                if !output.stdout.is_empty() {
                    info!("Kill port output: {}", String::from_utf8_lossy(&output.stdout));
                }
            }
            Err(e) => error!("Failed to kill process on port: {}", e),
        }
        
        // Double-check that port is free with retries
        let mut port_check_attempts = 0;
        while port_check_attempts < 5 {
            match reqwest::get("http://127.0.0.1:8000/api/health/").await {
                Ok(_) => {
                    if port_check_attempts == 0 {
                        error!("FastAPI server is still running after kill attempt!");
                        // Try a more aggressive kill using shell command
                        let _ = app.shell()
                            .command("sh")
                            .args(["-c", "lsof -ti :8000 | xargs -r kill -9"])
                            .output()
                            .await;
                    }
                    port_check_attempts += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                }
                Err(_) => {
                    info!("Port 8000 is free, proceeding with startup");
                    break;
                }
            }
        }
        
            if port_check_attempts >= 5 {
                return Err("Failed to free port 8000 after multiple attempts".to_string());
            }
        } // End of port_in_use check
    }
    
    let sidecar_command = if cfg!(debug_assertions) {
        // In development, run FastAPI directly with Python from venv
        if cfg!(target_os = "windows") {
            info!("Starting FastAPI on Windows with python.exe from venv");
            // Use absolute path to backend directory
            let backend_dir = std::env::current_dir()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("backend");
            let python_exe = backend_dir.join("venv").join("Scripts").join("python.exe");
            
            info!("Python path: {:?}", python_exe);
            info!("Backend dir: {:?}", backend_dir);
            
            app.shell()
                .command(python_exe.to_string_lossy().to_string())
                .args(["fastapi_server.py"])
                .current_dir(backend_dir)
        } else {
            info!("Starting FastAPI on Unix with venv/bin/python3");
            app.shell()
                .command("venv/bin/python3")
                .args(["fastapi_server.py"])
                .current_dir("../../backend") // Relative to src-tauri
        }
    } else {
        // In production, run the bundled FastAPI executable
        app.shell()
            .sidecar("fastapi-server")
            .map_err(|e| format!("Failed to create sidecar command: {}", e))?
            .args(["--port", "8000"])
    };

    let (mut rx, child) = sidecar_command
        .spawn()
        .map_err(|e| format!("Failed to spawn FastAPI sidecar: {}", e))?;

    // Store the PID and child process
    if let Some(sidecar) = app.try_state::<FastAPISidecar>() {
        let mut pid = sidecar.pid.lock().unwrap();
        *pid = Some(child.pid());
        info!("FastAPI sidecar started with PID: {}", child.pid());
        
        let mut child_lock = sidecar.child.lock().unwrap();
        *child_lock = Some(child);
    }

    // Monitor sidecar output
    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    info!("FastAPI: {}", line_str);
                }
                CommandEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    // FastAPI logs INFO messages to stderr, so check the content
                    if line_str.contains("ERROR") || line_str.contains("CRITICAL") || line_str.contains("FATAL") {
                        error!("FastAPI Error: {}", line_str);
                    } else if line_str.contains("WARNING") || line_str.contains("WARN") {
                        log::warn!("FastAPI Warning: {}", line_str);
                    } else {
                        // Treat other stderr output as info (FastAPI logs INFO to stderr)
                        info!("FastAPI: {}", line_str);
                    }
                }
                CommandEvent::Error(error) => error!("FastAPI Process Error: {}", error),
                CommandEvent::Terminated(payload) => {
                    if let Some(code) = payload.code {
                        if code != 0 {
                            error!("FastAPI terminated with error code: {}", code);
                        } else {
                            info!("FastAPI terminated successfully");
                        }
                    } else {
                        info!("FastAPI process terminated");
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for FastAPI to be ready and trigger background loading
    match wait_for_fastapi_and_trigger_background_loading().await {
        Ok(message) => {
            info!("FastAPI startup and background loading: {}", message);
            // Remove lock file after successful startup
            remove_fastapi_lock();
            Ok("FastAPI sidecar started with background loading".to_string())
        }
        Err(e) => {
            error!("FastAPI startup error: {}", e);
            // Remove lock file even on error
            remove_fastapi_lock();
            Err(format!("FastAPI sidecar started but background loading failed: {}", e))
        }
    }
}

async fn wait_for_fastapi_and_trigger_background_loading() -> Result<String, String> {
    // Wait for FastAPI health check to pass (max 30 seconds)
    let mut attempts = 0;
    let max_attempts = 30;
    
    info!("Waiting for FastAPI health check to pass...");
    
    while attempts < max_attempts {
        match reqwest::get("http://127.0.0.1:8000/api/health/").await {
            Ok(response) if response.status().is_success() => {
                info!("FastAPI health check passed after {} attempts", attempts + 1);
                break;
            }
            Ok(response) => {
                info!("FastAPI health check failed with status: {}", response.status());
            }
            Err(_) => {
                if attempts == 0 {
                    info!("Waiting for FastAPI to start...");
                } else if attempts % 5 == 0 {
                    info!("Still waiting for FastAPI... (attempt {}/{})", attempts + 1, max_attempts);
                }
            }
        }
        
        attempts += 1;
        if attempts >= max_attempts {
            return Err("FastAPI health check timeout after 30 seconds".to_string());
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    // Health check passed, now trigger background loading
    info!("Triggering background loading of complete game data...");
    
    match reqwest::Client::new()
        .post("http://127.0.0.1:8000/api/system/background-loading/trigger/")
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            info!("Background loading started successfully");
            Ok("FastAPI ready and background loading started".to_string())
        }
        Ok(response) => {
            let error_msg = format!("Background loading failed with status: {}", response.status());
            log::warn!("{}", error_msg);
            // Don't fail the entire startup for this
            Ok("FastAPI ready but background loading failed".to_string())
        }
        Err(e) => {
            let error_msg = format!("Background loading request failed: {}", e);
            log::warn!("{}", error_msg);
            // Don't fail the entire startup for this
            Ok("FastAPI ready but background loading request failed".to_string())
        }
    }
}

#[tauri::command]
pub async fn stop_fastapi_sidecar(app: tauri::AppHandle) -> Result<String, String> {
    info!("Attempting graceful FastAPI shutdown...");
    
    // Try graceful shutdown first
    match reqwest::Client::new()
        .post("http://127.0.0.1:8000/api/system/shutdown/")
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            info!("Graceful shutdown signal sent successfully");
            // Wait for graceful shutdown
            tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
            
            // Check if server actually stopped
            match reqwest::get("http://127.0.0.1:8000/api/health/").await {
                Ok(_) => {
                    info!("Server still responding, will force kill");
                }
                Err(_) => {
                    info!("Server stopped gracefully");
                    // Clear stored PID/child
                    if let Some(sidecar) = app.try_state::<FastAPISidecar>() {
                        let mut child_lock = sidecar.child.lock().unwrap();
                        *child_lock = None;
                        let mut pid = sidecar.pid.lock().unwrap();
                        *pid = None;
                    }
                    return Ok("FastAPI sidecar stopped gracefully".to_string());
                }
            }
        }
        Ok(response) => {
            info!("Graceful shutdown failed with status: {}", response.status());
        }
        Err(e) => {
            info!("Could not send graceful shutdown: {}", e);
        }
    }
    
    // Force kill if graceful shutdown failed
    if let Some(sidecar) = app.try_state::<FastAPISidecar>() {
        let mut child_lock = sidecar.child.lock().unwrap();
        if let Some(child) = child_lock.take() {
            info!("Force stopping FastAPI sidecar with PID: {}", child.pid());
            if let Err(e) = child.kill() {
                error!("Failed to kill FastAPI sidecar: {}", e);
                return Err(format!("Failed to stop FastAPI sidecar: {}", e));
            }
        }
        
        let mut pid = sidecar.pid.lock().unwrap();
        *pid = None;
    }
    
    Ok("FastAPI sidecar stopped (forced)".to_string())
}

/// Force kill FastAPI without graceful shutdown - used during startup
pub async fn force_kill_fastapi_sidecar(app: tauri::AppHandle) -> Result<String, String> {
    info!("Force killing FastAPI sidecar (startup cleanup)");
    
    // Skip graceful shutdown, go straight to force kill
    if let Some(sidecar) = app.try_state::<FastAPISidecar>() {
        let mut child_lock = sidecar.child.lock().unwrap();
        if let Some(child) = child_lock.take() {
            info!("Force killing FastAPI sidecar with PID: {}", child.pid());
            if let Err(e) = child.kill() {
                error!("Failed to kill FastAPI sidecar: {}", e);
            }
        }
        
        let mut pid = sidecar.pid.lock().unwrap();
        *pid = None;
    }
    
    // Also kill any process on port 8000 directly
    if cfg!(target_os = "windows") {
        let _ = tokio::process::Command::new("cmd")
            .args(["/C", "for /f \"tokens=5\" %a in ('netstat -aon ^| findstr :8000 ^| findstr LISTENING') do taskkill /F /PID %a 2>nul"])
            .output()
            .await;
    } else {
        let _ = tokio::process::Command::new("sh")
            .args(["-c", "lsof -ti:8000 | xargs -r kill -9 2>/dev/null || true"])
            .output()
            .await;
    }
    
    Ok("FastAPI sidecar force killed".to_string())
}

#[tauri::command]
pub async fn check_fastapi_health() -> Result<bool, String> {
    match reqwest::get("http://127.0.0.1:8000/api/health/").await {
        Ok(response) => Ok(response.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn check_background_loading_status() -> Result<serde_json::Value, String> {
    match reqwest::get("http://127.0.0.1:8000/api/system/background-loading/status/").await {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("Failed to parse background loading status: {}", e)),
                }
            } else {
                Err(format!("Background loading status check failed: {}", response.status()))
            }
        }
        Err(e) => Err(format!("Failed to check background loading status: {}", e)),
    }
}

fn get_fastapi_lock_file() -> PathBuf {
    let mut lock_path = std::env::temp_dir();
    lock_path.push("nwn2_editor_fastapi.lock");
    lock_path
}

fn is_fastapi_startup_in_progress() -> bool {
    let lock_file = get_fastapi_lock_file();
    if let Ok(metadata) = fs::metadata(&lock_file) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = SystemTime::now().duration_since(modified) {
                // Consider startup in progress if lock file is less than 30 seconds old
                return duration.as_secs() < 30;
            }
        }
    }
    false
}

fn create_fastapi_lock() -> Result<(), String> {
    let lock_file = get_fastapi_lock_file();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();
    
    fs::write(&lock_file, timestamp.to_string())
        .map_err(|e| format!("Failed to create lock file: {}", e))?;
    
    Ok(())
}

fn remove_fastapi_lock() {
    let lock_file = get_fastapi_lock_file();
    let _ = fs::remove_file(&lock_file);
}

/// Ensures FastAPI is running, but doesn't restart if already healthy
pub async fn ensure_fastapi_running(app: tauri::AppHandle) -> Result<(), String> {
    // Quick health check first
    match reqwest::Client::new()
        .get("http://127.0.0.1:8000/api/health/")
        .timeout(std::time::Duration::from_secs(1))
        .send()
        .await 
    {
        Ok(response) if response.status().is_success() => {
            info!("FastAPI already running and healthy");
            return Ok(());
        }
        Ok(response) => {
            info!("FastAPI health check failed with status: {}", response.status());
        }
        Err(_) => {
            // Check if startup is in progress
            if is_fastapi_startup_in_progress() {
                info!("FastAPI startup in progress, waiting for completion...");
                
                // Wait for startup to complete with extended timeout
                for attempt in 0..20 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                    
                    match reqwest::Client::new()
                        .get("http://127.0.0.1:8000/api/health/")
                        .timeout(std::time::Duration::from_secs(2))
                        .send()
                        .await 
                    {
                        Ok(response) if response.status().is_success() => {
                            info!("FastAPI startup completed successfully");
                            return Ok(());
                        }
                        _ => {
                            if attempt % 5 == 0 {
                                info!("Still waiting for FastAPI startup... (attempt {}/20)", attempt + 1);
                            }
                        }
                    }
                }
                
                info!("FastAPI startup timeout, will attempt to start new instance");
            }
        }
    }
    
    // FastAPI isn't running, start it
    info!("Starting new FastAPI instance");
    start_fastapi_sidecar(app).await?;
    Ok(())
}

#[tauri::command]
pub async fn graceful_shutdown_on_exit(app: tauri::AppHandle) -> Result<String, String> {
    // Called when Tauri app is about to exit - ensures FastAPI shuts down properly
    info!("App exit detected - ensuring FastAPI shutdown");
    
    // Try graceful shutdown with shorter timeout since we're exiting
    match reqwest::Client::new()
        .post("http://127.0.0.1:8000/api/system/shutdown/")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(_) => {
            info!("Graceful shutdown signal sent on app exit");
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Err(_) => {
            info!("Could not send graceful shutdown on exit (server may already be down)");
        }
    }
    
    // Also kill process directly
    let _ = stop_fastapi_sidecar(app).await;
    
    // Final fallback: kill any remaining Python processes on port 8000
    if cfg!(target_os = "windows") {
        let _ = tokio::process::Command::new("cmd")
            .args(["/C", "for /f \"tokens=5\" %a in ('netstat -aon ^| findstr :8000 ^| findstr LISTENING') do taskkill /F /PID %a 2>nul"])
            .output()
            .await;
    } else {
        let _ = tokio::process::Command::new("sh")
            .args(["-c", "lsof -ti:8000 | xargs -r kill -9 2>/dev/null || true"])
            .output()
            .await;
    }
    
    Ok("Shutdown complete".to_string())
}