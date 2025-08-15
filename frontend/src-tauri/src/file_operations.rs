use tauri_plugin_dialog::DialogExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use base64::prelude::*;
use tauri_plugin_shell::ShellExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveFile {
    pub path: String,
    pub name: String,
    pub thumbnail: Option<String>,
}

#[tauri::command]
pub async fn select_save_file(app: tauri::AppHandle) -> Result<SaveFile, String> {
    log::info!("[Rust] The 'select_save_file' command has been invoked.");
    let mut dialog = app.dialog().file();
    
    let mut initial_dir = None;
    
    // Try to get saves path from FastAPI backend first  
    // Ensure FastAPI sidecar is running (ignore errors for fallback)
    let _ = crate::sidecar_manager::ensure_fastapi_running(app.clone()).await;
    
    if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build() {
        if let Ok(response) = client.get("http://localhost:8000/api/gamedata/paths/").send().await {
            if response.status().is_success() {
                if let Ok(paths_info) = response.json::<serde_json::Value>().await {
                    if let Some(saves_path_str) = paths_info.get("saves").and_then(|p| p.as_str()) {
                        let saves_path = PathBuf::from(saves_path_str);
                        if saves_path.exists() {
                            initial_dir = Some(saves_path);
                            log::info!("[Rust] Using backend-detected saves path: {}", saves_path_str);
                        }
                    }
                }
            }
        }
    }
    if let Some(dir) = initial_dir {
        log::info!("[Rust] Setting initial directory for dialog.");
        dialog = dialog.set_directory(&dir);
    }
    
    // [FIX] Added logging to pinpoint the exact location of the freeze.
    log::info!("[Rust] About to call blocking_pick_folder. If the app freezes, this is the last log you will see.");
    
    let dir_path = dialog
        .blocking_pick_folder()
        .ok_or("No save directory selected or the dialog was cancelled.")?;

    // If the application doesn't freeze, you will see this log message.
    log::info!("[Rust] blocking_pick_folder completed successfully. A folder was selected.");

    let path_str = match &dir_path {
        tauri_plugin_dialog::FilePath::Path(p) => p.to_string_lossy().to_string(),
        _ => return Err("Invalid directory path format".to_string()),
    };
    
    let name = match &dir_path {
        tauri_plugin_dialog::FilePath::Path(p) => {
            // Try to read actual save name from savename.txt
            match std::fs::read_to_string(p.join("savename.txt")) {
                Ok(content) => content.trim().to_string(),
                Err(_) => {
                    // Fallback to folder name
                    p.file_name()
                     .and_then(|n| n.to_str())
                     .unwrap_or("Unknown")
                     .to_string()
                }
            }
        },
        _ => "Unknown".to_string(),
    };

    let save_path = PathBuf::from(&path_str);
    let resgff_path = save_path.join("resgff.zip");
    if !resgff_path.exists() {
        log::error!("[Rust] Validation failed: selected directory is missing resgff.zip");
        return Err("Selected directory doesn't appear to be a valid NWN2 save (missing resgff.zip)".to_string());
    }

    log::info!("[Rust] Save file validated. Returning path to frontend.");
    // Check for thumbnail in selected save
    let thumbnail_path = save_path.join("screen.tga");
    let thumbnail = if thumbnail_path.exists() {
        Some(thumbnail_path.to_string_lossy().to_string())
    } else {
        None
    };
    
    Ok(SaveFile { path: path_str, name, thumbnail })
}

#[tauri::command]
pub async fn select_nwn2_directory(app: tauri::AppHandle) -> Result<String, String> {
    log::info!("[Rust] About to call blocking_pick_folder for NWN2 directory.");
    let dir_path = app.dialog()
        .file()
        .blocking_pick_folder()
        .ok_or("No directory selected or the dialog was cancelled.")?;
    log::info!("[Rust] blocking_pick_folder for NWN2 directory completed.");

    match dir_path {
        tauri_plugin_dialog::FilePath::Path(p) => Ok(p.to_string_lossy().to_string()),
        _ => Err("Invalid directory path format".to_string()),
    }
}

#[tauri::command]
pub async fn find_nwn2_saves(app: tauri::AppHandle) -> Result<Vec<SaveFile>, String> {
    use std::time::Instant;
    let start_time = Instant::now();
    log::info!("[Rust] Finding available NWN2 saves via FastAPI backend.");
    
    // Only ensure FastAPI is running if health check fails
    let sidecar_start = Instant::now();
    let health_check = crate::sidecar_manager::check_fastapi_health().await;
    match health_check {
        Ok(true) => {
            log::info!("[Rust] FastAPI already running and healthy, skipping startup");
        }
        _ => {
            log::info!("[Rust] FastAPI not healthy, ensuring startup");
            crate::sidecar_manager::ensure_fastapi_running(app).await
                .map_err(|e| format!("Failed to start FastAPI sidecar: {}", e))?;
        }
    }
    log::info!("[Rust] FastAPI sidecar check/startup took: {:?}", sidecar_start.elapsed());
    
    // Call FastAPI backend to get the proper save path using nwn2_settings.py
    let backend_start = Instant::now();
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8000/api/gamedata/paths/")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to FastAPI backend: {}", e))?;
    log::info!("[Rust] Backend API call took: {:?}", backend_start.elapsed());
    
    if !response.status().is_success() {
        return Err("Failed to get NWN2 paths from backend".to_string());
    }
    
    let paths_info: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse paths response: {}", e))?;
    
    // Extract saves path from backend response
    let saves_path_str = paths_info
        .get("saves")
        .and_then(|p| p.as_str())
        .ok_or("Backend did not return saves path")?;
    
    let saves_path = PathBuf::from(saves_path_str);
    let mut saves = Vec::new();
    
    let scan_start = Instant::now();
    if saves_path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&saves_path) {
            // Collect entries and sort by modification time (newest first)
            let mut save_entries: Vec<_> = entries.flatten()
                .filter(|entry| entry.path().is_dir() && entry.path().join("resgff.zip").exists())
                .collect();
            
            // Sort by folder name (which contains timestamp) in descending order
            save_entries.sort_by(|a, b| {
                let name_a = a.file_name().to_string_lossy().to_string();
                let name_b = b.file_name().to_string_lossy().to_string();
                name_b.cmp(&name_a) // Reverse order for newest first
            });
            
            for entry in save_entries {
                let folder_name = entry.file_name().to_string_lossy().to_string();
                let save_path = entry.path().to_string_lossy().to_string();
                
                // Try to read actual save name from savename.txt
                let save_name = match std::fs::read_to_string(entry.path().join("savename.txt")) {
                    Ok(content) => content.trim().to_string(),
                    Err(_) => folder_name, // Fallback to folder name if savename.txt doesn't exist
                };
                
                // Check for thumbnail
                let thumbnail_path = entry.path().join("screen.tga");
                let thumbnail = if thumbnail_path.exists() {
                    Some(thumbnail_path.to_string_lossy().to_string())
                } else {
                    None
                };
                
                saves.push(SaveFile {
                    name: save_name,
                    path: save_path,
                    thumbnail,
                });
                
                // Limit to 3 saves
                if saves.len() >= 3 {
                    log::info!("[Rust] Limited scan to first 3 saves (newest first)");
                    break;
                }
            }
        }
    }
    log::info!("[Rust] Directory scan took: {:?}", scan_start.elapsed());
    
    log::info!("[Rust] Total function took: {:?}", start_time.elapsed());
    log::info!("[Rust] Found {} potential save(s) in {}", saves.len(), saves_path_str);
    Ok(saves)
}

#[tauri::command]
pub async fn get_steam_workshop_path() -> Result<Option<String>, String> {
    // This function is unchanged
    let mut steam_paths = vec![
        PathBuf::from("C:/Program Files (x86)/Steam/steamapps/workshop/content/2760"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        steam_paths.push(PathBuf::from(&home).join(".steam/steam/steamapps/workshop/content/2760"));
    }
    for path in steam_paths {
        if path.exists() {
            return Ok(Some(path.to_string_lossy().to_string()));
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn validate_nwn2_installation(path: String) -> Result<bool, String> {
    // This function is unchanged
    let base_path = PathBuf::from(path);
    let required_items = vec!["Data", "dialog.tlk"];
    for item in required_items {
        if !base_path.join(item).exists() {
            return Ok(false);
        }
    }
    Ok(true)
}

#[tauri::command]
pub async fn get_save_thumbnail(thumbnail_path: String) -> Result<String, String> {
    log::info!("[Rust] Starting thumbnail conversion process for: {}", thumbnail_path);
    
    let path = PathBuf::from(&thumbnail_path);
    
    // Open and decode TGA file
    log::debug!("[Rust] Attempting to decode TGA file at: {}", path.display());
    
    let dynamic_image = image::open(&path).map_err(|e| {
        log::error!("Failed to open/decode TGA file at '{}': {}", path.display(), e);
        "Failed to process thumbnail. The file may be corrupt or inaccessible.".to_string()
    })?;
    
    log::debug!("[Rust] TGA decoded successfully: {}x{}", dynamic_image.width(), dynamic_image.height());
    
    // Convert to WebP with quality control using webp crate
    let encoder = webp::Encoder::from_image(&dynamic_image)
        .map_err(|e| {
            log::error!("Failed to create WebP encoder: {}", e);
            "Failed to create WebP encoder from image.".to_string()
        })?;
    
    // Encode with quality setting of 85.0 (out of 100) for good balance of quality and size
    let webp_memory = encoder.encode(85.0);
    let webp_data = webp_memory.to_vec();
    
    log::debug!("[Rust] Successfully converted TGA to WebP ({} bytes)", webp_data.len());
    
    // Debug: Save a test file to verify WebP is valid (debug builds only)
    #[cfg(debug_assertions)]
    {
        if let Some(parent) = path.parent() {
            let test_path = parent.join("debug_thumbnail.webp");
            if let Err(e) = std::fs::write(&test_path, &webp_data) {
                log::warn!("[Rust] Could not save debug WebP: {}", e);
            } else {
                log::debug!("[Rust] Saved debug WebP to: {}", test_path.display());
            }
        }
    }
    
    // Encode as base64 for safe transfer
    let base64_data = base64::prelude::BASE64_STANDARD.encode(&webp_data);
    log::debug!("[Rust] Base64 encoding complete ({} chars), WebP size: {} bytes", base64_data.len(), webp_data.len());
    
    Ok(base64_data)
}

#[tauri::command]
pub async fn detect_nwn2_installation(app: tauri::AppHandle) -> Result<Option<String>, String> {
    log::info!("[Rust] Detecting NWN2:EE installation using FastAPI backend");
    
    // Ensure FastAPI sidecar is running
    let _ = crate::sidecar_manager::ensure_fastapi_running(app).await;
    
    // Use FastAPI backend's Rust-powered path discovery
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8000/api/gamedata/paths/")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to FastAPI backend: {}", e))?;
    
    if !response.status().is_success() {
        return Err("Failed to get NWN2 paths from backend".to_string());
    }
    
    let paths_info: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse paths response: {}", e))?;
    
    // Extract installation path from backend response
    // The response structure is: { "paths": { "game_folder": { "path": "...", "exists": true } } }
    if let Some(paths) = paths_info.get("paths") {
        if let Some(game_folder) = paths.get("game_folder") {
            if let Some(installation_path) = game_folder.get("path").and_then(|p| p.as_str()) {
                log::info!("[Rust] Found NWN2 installation via FastAPI backend: {}", installation_path);
                return Ok(Some(installation_path.to_string()));
            }
        }
    }
    
    log::info!("[Rust] No NWN2 installation found via FastAPI backend");
    Ok(None)
}

#[tauri::command]
pub async fn launch_nwn2_game(app: tauri::AppHandle, game_path: Option<String>) -> Result<(), String> {
    log::info!("[Rust] Launching NWN2:EE game");
    
    let installation_path = match game_path {
        Some(path) => path,
        None => {
            match detect_nwn2_installation(app.clone()).await? {
                Some(path) => path,
                None => return Err("NWN2:EE installation not found. Please set the game path in settings.".to_string()),
            }
        }
    };
    
    let base_path = PathBuf::from(&installation_path);
    
    // Determine which executable to use (prefer NWN2Player.exe if available)
    let exe_path = if base_path.join("NWN2Player.exe").exists() {
        base_path.join("NWN2Player.exe")
    } else if base_path.join("NWN2.exe").exists() {
        base_path.join("NWN2.exe") 
    } else {
        return Err(format!("No NWN2 executable found in: {}", installation_path));
    };
    
    log::info!("[Rust] Launching game executable: {}", exe_path.display());
    
    // Launch the game
    let shell = app.shell();
    
    if cfg!(windows) {
        // On Windows, launch directly
        shell.command(&exe_path)
            .spawn()
            .map_err(|e| format!("Failed to launch NWN2: {}", e))?;
    } else {
        // On Linux/WSL, might need to use wine or different approach
        // For now, try direct execution
        shell.command(&exe_path)
            .spawn()
            .map_err(|e| format!("Failed to launch NWN2: {}. You may need to configure Wine or use Windows.", e))?;
    }
    
    log::info!("[Rust] NWN2 game launched successfully");
    Ok(())
}
