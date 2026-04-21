use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_shell::ShellExt;

use crate::services::playerinfo::PlayerInfo;
use crate::services::savegame_handler::SaveGameHandler;
use crate::state::AppState;

fn read_save_character_name(save_path: &std::path::Path) -> Option<String> {
    if let Ok(handler) = SaveGameHandler::new(save_path, false, false)
        && let Ok(Some(summary)) = handler.read_character_summary()
    {
        let name = [summary.first_name, summary.last_name]
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !name.trim().is_empty() {
            return Some(name);
        }
    }

    PlayerInfo::get_player_name(save_path.join("playerinfo.bin")).ok()
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct SaveFile {
    pub path: String,
    pub name: String,
    pub thumbnail: Option<String>,
    pub modified: Option<i64>,
    pub character_name: Option<String>,
}

#[tauri::command]
pub async fn select_save_file(app: tauri::AppHandle) -> Result<SaveFile, String> {
    log::info!("[Rust] The 'select_save_file' command has been invoked.");
    let dialog = app.dialog().file();

    // [FIX] Added logging to pinpoint the exact location of the freeze.
    log::info!(
        "[Rust] About to call blocking_pick_folder. If the app freezes, this is the last log you will see."
    );

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
        }
        _ => "Unknown".to_string(),
    };

    let save_path = PathBuf::from(&path_str);
    let resgff_path = save_path.join("resgff.zip");
    if !resgff_path.exists() {
        log::error!("[Rust] Validation failed: selected directory is missing resgff.zip");
        return Err(
            "Selected directory doesn't appear to be a valid NWN2 save (missing resgff.zip)"
                .to_string(),
        );
    }

    log::info!("[Rust] Save file validated. Returning path to frontend.");
    // Check for thumbnail in selected save
    let thumbnail_path = save_path.join("screen.tga");
    let thumbnail = if thumbnail_path.exists() {
        Some(thumbnail_path.to_string_lossy().to_string())
    } else {
        None
    };

    let modified = save_path
        .metadata()
        .and_then(|m| m.modified())
        .ok()
        .map(|time| {
            time.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64
        });

    let character_name = read_save_character_name(&save_path);

    Ok(SaveFile {
        path: path_str,
        name,
        thumbnail,
        modified,
        character_name,
    })
}

#[tauri::command]
pub async fn select_nwn2_directory(app: tauri::AppHandle) -> Result<String, String> {
    log::info!("[Rust] About to call blocking_pick_folder for NWN2 directory.");
    let dir_path = app
        .dialog()
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
pub async fn find_nwn2_saves(
    state: State<'_, AppState>,
    save_mode: Option<String>,
) -> Result<Vec<SaveFile>, String> {
    use std::time::Instant;
    let start_time = Instant::now();
    log::info!("[Rust] Finding available NWN2 saves.");

    let base_saves_path = state
        .paths
        .read()
        .saves()
        .ok_or("Could not determine NWN2 saves path")?;
    let saves_path = if save_mode.as_deref() == Some("mp") {
        base_saves_path.join("multiplayer")
    } else {
        base_saves_path
    };
    let mut saves = Vec::new();

    let scan_start = Instant::now();
    if saves_path.is_dir()
        && let Ok(entries) = std::fs::read_dir(&saves_path)
    {
        // Collect save directory entries for sorting
        let mut save_entries: Vec<_> = entries
            .flatten()
            .filter(|entry| entry.path().is_dir() && entry.path().join("resgff.zip").exists())
            .collect();

        // Sort by directory modification time (newest first)
        save_entries.sort_by(|a, b| {
            let time_a = a
                .path()
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            let time_b = b
                .path()
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            time_b.cmp(&time_a) // Reverse order for newest first
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

            let modified = entry
                .path()
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .map(|time| {
                    time.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64
                });

            let character_name = read_save_character_name(&entry.path());

            saves.push(SaveFile {
                name: save_name,
                path: save_path,
                thumbnail,
                modified,
                character_name,
            });

            // Limit to 3 saves
            if saves.len() >= 3 {
                log::info!("[Rust] Limited scan to first 3 saves (newest by modification time)");
                break;
            }
        }
    }
    log::info!("[Rust] Directory scan took: {:?}", scan_start.elapsed());

    log::info!("[Rust] Total function took: {:?}", start_time.elapsed());
    log::info!(
        "[Rust] Found {} potential save(s) in {}",
        saves.len(),
        saves_path.display()
    );
    Ok(saves)
}

#[tauri::command]
pub async fn get_steam_workshop_path() -> Result<Option<String>, String> {
    // This function is unchanged
    let mut steam_paths = vec![PathBuf::from(
        "C:/Program Files (x86)/Steam/steamapps/workshop/content/2760",
    )];
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
    log::info!("[Rust] Starting thumbnail conversion process for: {thumbnail_path}");

    let path = PathBuf::from(&thumbnail_path);

    // Open and decode TGA file
    log::debug!(
        "[Rust] Attempting to decode TGA file at: {}",
        path.display()
    );

    let dynamic_image = image::open(&path).map_err(|e| {
        log::error!(
            "Failed to open/decode TGA file at '{}': {}",
            path.display(),
            e
        );
        "Failed to process thumbnail. The file may be corrupt or inaccessible.".to_string()
    })?;

    log::debug!(
        "[Rust] TGA decoded successfully: {}x{}",
        dynamic_image.width(),
        dynamic_image.height()
    );

    // Convert to WebP with quality control using webp crate
    let encoder = webp::Encoder::from_image(&dynamic_image).map_err(|e| {
        log::error!("Failed to create WebP encoder: {e}");
        "Failed to create WebP encoder from image.".to_string()
    })?;

    // Encode with quality setting of 85.0 (out of 100) for good balance of quality and size
    let webp_memory = encoder.encode(85.0);
    let webp_data = webp_memory.to_vec();

    log::debug!(
        "[Rust] Successfully converted TGA to WebP ({} bytes)",
        webp_data.len()
    );

    // Debug: Save a test file to verify WebP is valid (debug builds only)
    #[cfg(debug_assertions)]
    {
        if let Some(parent) = path.parent() {
            let test_path = parent.join("debug_thumbnail.webp");
            if let Err(e) = std::fs::write(&test_path, &webp_data) {
                log::warn!("[Rust] Could not save debug WebP: {e}");
            } else {
                log::debug!("[Rust] Saved debug WebP to: {}", test_path.display());
            }
        }
    }

    // Encode as base64 for safe transfer
    let base64_data = base64::prelude::BASE64_STANDARD.encode(&webp_data);
    log::debug!(
        "[Rust] Base64 encoding complete ({} chars), WebP size: {} bytes",
        base64_data.len(),
        webp_data.len()
    );

    Ok(base64_data)
}

#[tauri::command]
pub async fn detect_nwn2_installation(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    log::info!("[Rust] Detecting NWN2:EE installation");

    let paths = state.paths.read();
    if let Some(game_folder) = paths.game_folder()
        && game_folder.exists()
    {
        let path_str = game_folder.to_string_lossy().to_string();
        log::info!("[Rust] Found NWN2 installation: {path_str}");
        return Ok(Some(path_str));
    }

    log::info!("[Rust] No NWN2 installation found");
    Ok(None)
}

#[tauri::command]
pub async fn open_folder_in_explorer(
    app: tauri::AppHandle,
    folder_path: String,
) -> Result<(), String> {
    log::info!("[Rust] Opening folder in file explorer: {folder_path}");

    let path = PathBuf::from(&folder_path);

    // Check if path exists
    if !path.exists() {
        return Err(format!("Folder does not exist: {folder_path}"));
    }

    let shell = app.shell();

    if cfg!(windows) {
        // On Windows, use explorer.exe
        shell
            .command("explorer")
            .args([&folder_path])
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    } else if cfg!(target_os = "macos") {
        // On macOS, use open command
        shell
            .command("open")
            .args([&folder_path])
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    } else {
        // On Linux, try xdg-open
        shell
            .command("xdg-open")
            .args([&folder_path])
            .spawn()
            .map_err(|e| format!("Failed to open folder: {e}"))?;
    }

    log::info!("[Rust] Successfully opened folder in file explorer");
    Ok(())
}

#[tauri::command]
pub async fn launch_nwn2_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    game_path: Option<String>,
) -> Result<(), String> {
    log::info!("[Rust] Launching NWN2:EE game");

    let installation_path = match game_path {
        Some(path) => path,
        None => match state
            .paths
            .read()
            .game_folder()
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
        {
            Some(path) => path,
            None => {
                return Err(
                    "NWN2:EE installation not found. Please set the game path in settings."
                        .to_string(),
                );
            }
        },
    };

    let base_path = PathBuf::from(&installation_path);

    // Determine which executable to use (prefer NWN2Player.exe if available)
    let exe_path = if base_path.join("NWN2Player.exe").exists() {
        base_path.join("NWN2Player.exe")
    } else if base_path.join("NWN2.exe").exists() {
        base_path.join("NWN2.exe")
    } else {
        return Err(format!("No NWN2 executable found in: {installation_path}"));
    };

    log::info!("[Rust] Launching game executable: {}", exe_path.display());

    // Launch the game
    let shell = app.shell();

    if cfg!(windows) {
        // On Windows, launch directly
        shell
            .command(&exe_path)
            .spawn()
            .map_err(|e| format!("Failed to launch NWN2: {e}"))?;
    } else {
        // On Linux/WSL, might need to use wine or different approach
        // For now, try direct execution
        shell.command(&exe_path).spawn().map_err(|e| {
            format!("Failed to launch NWN2: {e}. You may need to configure Wine or use Windows.")
        })?;
    }

    log::info!("[Rust] NWN2 game launched successfully");
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseSaveEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: f64,
    pub is_directory: bool,
    pub save_name: Option<String>,
    pub character_name: Option<String>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseSavesResponse {
    pub files: Vec<BrowseSaveEntry>,
    pub total_count: usize,
    pub path: String,
    pub current_path: String,
}

#[tauri::command]
pub async fn browse_saves(
    state: State<'_, AppState>,
    path: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<BrowseSavesResponse, String> {
    let target_path = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => state
            .paths
            .read()
            .saves()
            .ok_or("Could not determine NWN2 saves path")?,
    };

    if !target_path.exists() {
        return Err(format!("Directory not found: {}", target_path.display()));
    }

    if !target_path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let entries =
        std::fs::read_dir(&target_path).map_err(|e| format!("Failed to read directory: {e}"))?;

    let mut files_list: Vec<BrowseSaveEntry> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let name_lower = name.to_lowercase();

        if name.starts_with('.')
            || name_lower == "backups"
            || name_lower == "steamcloud"
            || name_lower.ends_with(".cam")
        {
            continue;
        }

        let entry_path = entry.path();
        let is_directory = entry_path.is_dir();

        if !is_directory {
            continue;
        }

        if !entry_path.join("resgff.zip").exists() {
            continue;
        }

        let metadata = entry_path.metadata().ok();
        let modified = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
            })
            .unwrap_or(0.0);

        let mut size: u64 = 0;
        if let Ok(dir_entries) = std::fs::read_dir(&entry_path) {
            for f in dir_entries.flatten() {
                if let Ok(meta) = f.metadata()
                    && meta.is_file()
                {
                    size += meta.len();
                }
            }
        }

        let save_name = std::fs::read_to_string(entry_path.join("savename.txt"))
            .ok()
            .map(|s| s.trim().to_string());

        let character_name = read_save_character_name(&entry_path);

        let thumbnail_path = entry_path.join("screen.tga");
        let thumbnail = if thumbnail_path.exists() {
            Some(thumbnail_path.to_string_lossy().to_string())
        } else {
            None
        };

        files_list.push(BrowseSaveEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            size,
            modified,
            is_directory,
            save_name,
            character_name,
            thumbnail,
        });
    }

    files_list.sort_by(|a, b| {
        b.modified
            .partial_cmp(&a.modified)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_count = files_list.len();
    let offset = offset.unwrap_or(0);
    let limit = limit.unwrap_or(500);
    let paginated: Vec<BrowseSaveEntry> = files_list.into_iter().skip(offset).take(limit).collect();

    log::info!(
        "[Rust] browse_saves: path={}, count={}, total={}",
        target_path.display(),
        paginated.len(),
        total_count
    );

    Ok(BrowseSavesResponse {
        files: paginated,
        total_count,
        path: target_path.to_string_lossy().to_string(),
        current_path: target_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn get_default_saves_path(
    state: State<'_, AppState>,
    save_mode: Option<String>,
) -> Result<String, String> {
    let base_saves_path = state
        .paths
        .read()
        .saves()
        .ok_or("Could not determine NWN2 saves path")?;
    let saves_path = if save_mode.as_deref() == Some("mp") {
        base_saves_path.join("multiplayer")
    } else {
        base_saves_path
    };
    Ok(saves_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_default_backups_path(state: State<'_, AppState>) -> Result<String, String> {
    let saves_path = state
        .paths
        .read()
        .saves()
        .ok_or("Could not determine NWN2 saves path")?;
    let backups_path = saves_path.join("backups");
    Ok(backups_path.to_string_lossy().to_string())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrowseBackupEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub timestamp: String,
    pub created_at: i64,
    pub character_name: Option<String>,
    pub save_name: Option<String>,
}

fn find_newest_inner_backup(save_folder: &std::path::Path) -> Option<std::path::PathBuf> {
    use std::time::SystemTime;

    let entries = std::fs::read_dir(save_folder).ok()?;
    let mut newest: Option<(SystemTime, std::path::PathBuf)> = None;

    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("backup_") && !name.starts_with("pre_restore_") {
            continue;
        }

        let path = entry.path();
        let ts = std::fs::metadata(&path)
            .and_then(|m| m.created().or_else(|_| m.modified()))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        match &newest {
            Some((best_ts, _)) if ts <= *best_ts => {}
            _ => newest = Some((ts, path)),
        }
    }

    newest.map(|(_, p)| p)
}

#[tauri::command]
pub async fn browse_backups(path: String) -> Result<Vec<BrowseBackupEntry>, String> {
    use std::time::SystemTime;

    let backup_path = PathBuf::from(&path);

    if !backup_path.exists() {
        return Ok(Vec::new());
    }

    if !backup_path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let is_root_backups = path.replace('\\', "/").ends_with("/backups")
        || path.replace('\\', "/").ends_with("/backups/");

    let mut backups = Vec::new();

    let entries = std::fs::read_dir(&backup_path)
        .map_err(|e| format!("Failed to read backup directory: {e}"))?;

    for entry in entries.flatten() {
        let entry_path = entry.path();

        if !entry_path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') {
            continue;
        }

        if !is_root_backups && !name.starts_with("backup_") && !name.starts_with("pre_restore_") {
            continue;
        }

        let timestamp = if name.starts_with("backup_") || name.starts_with("pre_restore_") {
            name.strip_prefix("backup_")
                .or_else(|| name.strip_prefix("pre_restore_"))
                .unwrap_or(&name)
                .to_string()
        } else {
            String::new()
        };

        let metadata = std::fs::metadata(&entry_path).ok();
        let created_at = metadata
            .as_ref()
            .and_then(|m| m.created().or_else(|_| m.modified()).ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let source_path: Option<PathBuf> = if is_root_backups {
            find_newest_inner_backup(&entry_path)
        } else {
            Some(entry_path.clone())
        };

        let (size, character_name, save_name) = match &source_path {
            Some(src) => {
                let mut size: u64 = 0;
                if let Ok(dir_entries) = std::fs::read_dir(src) {
                    for f in dir_entries.flatten() {
                        if f.file_type().is_ok_and(|ft| ft.is_file())
                            && let Ok(meta) = f.metadata()
                        {
                            size += meta.len();
                        }
                    }
                }
                let character_name = PlayerInfo::get_player_name(src.join("playerinfo.bin")).ok();
                let save_name = std::fs::read_to_string(src.join("savename.txt"))
                    .ok()
                    .map(|s| s.trim().to_string());
                (size, character_name, save_name)
            }
            None => (0u64, None, None),
        };

        backups.push(BrowseBackupEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            size,
            timestamp,
            created_at,
            character_name,
            save_name,
        });
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));

    log::info!(
        "[Rust] browse_backups: path={}, is_root={}, count={}",
        path,
        is_root_backups,
        backups.len()
    );

    Ok(backups)
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseVaultEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseVaultResponse {
    pub files: Vec<BrowseVaultEntry>,
    pub total_count: usize,
    pub path: String,
}

#[tauri::command]
pub async fn browse_localvault(state: State<'_, AppState>) -> Result<BrowseVaultResponse, String> {
    let vault_path = state
        .paths
        .read()
        .localvault()
        .ok_or("Could not determine NWN2 localvault path")?;

    if !vault_path.exists() {
        return Ok(BrowseVaultResponse {
            files: Vec::new(),
            total_count: 0,
            path: vault_path.to_string_lossy().to_string(),
        });
    }

    if !vault_path.is_dir() {
        return Err("LocalVault path is not a directory".to_string());
    }

    let entries = std::fs::read_dir(&vault_path)
        .map_err(|e| format!("Failed to read localvault directory: {e}"))?;

    let mut files_list: Vec<BrowseVaultEntry> = Vec::new();

    for entry in entries.flatten() {
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();

        if !name.to_lowercase().ends_with(".bic") {
            continue;
        }

        let display_name = name
            .strip_suffix(".bic")
            .or_else(|| name.strip_suffix(".BIC"))
            .unwrap_or(&name)
            .to_string();

        let metadata = entry_path.metadata().ok();
        let modified = metadata
            .as_ref()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64()
            })
            .unwrap_or(0.0);

        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        files_list.push(BrowseVaultEntry {
            name: display_name,
            path: entry_path.to_string_lossy().to_string(),
            size,
            modified,
        });
    }

    files_list.sort_by(|a, b| {
        b.modified
            .partial_cmp(&a.modified)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let total_count = files_list.len();

    log::info!(
        "[Rust] browse_localvault: path={}, count={}",
        vault_path.display(),
        total_count
    );

    Ok(BrowseVaultResponse {
        files: files_list,
        total_count,
        path: vault_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn get_default_localvault_path(state: State<'_, AppState>) -> Result<String, String> {
    let vault_path = state
        .paths
        .read()
        .localvault()
        .ok_or("Could not determine NWN2 localvault path")?;
    Ok(vault_path.to_string_lossy().to_string())
}
