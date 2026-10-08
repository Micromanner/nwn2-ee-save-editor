use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::State;
use tauri_plugin_shell::ShellExt;
use tracing::{error, info};

use crate::services::playerinfo::PlayerInfo;
use crate::services::savegame_handler::SaveGameHandler;
use crate::services::savegame_handler::backup;
use crate::services::savegame_handler::backup_manifest::BackupManifest;
use crate::state::AppState;

/// Return true if the directory looks like a valid NWN2 save in either format.
///
/// EE saves contain `resgff.zip`; original-NWN2 saves have loose `player.bic` and
/// `playerlist.ifo` at the root. Matches `SaveFormat::detect` but doesn't
/// allocate a handler — used as a cheap discovery filter.
pub fn is_save_dir(path: &Path) -> bool {
    if path.join("resgff.zip").exists() {
        return true;
    }
    path.join("player.bic").exists() && path.join("playerlist.ifo").exists()
}

/// Return the display name for a save folder.
///
/// Prefers EE's `savename.txt` (plain text). Original NWN2 stores the user-typed name
/// only in the folder name (`000002 - matt`), so that suffix comes next; `savenfo.txt`
/// holds the area name as `{strref}text` and is the last resort before `fallback`.
pub fn read_save_display_name(save_dir: &Path, fallback: &str) -> String {
    if let Ok(content) = std::fs::read_to_string(save_dir.join("savename.txt")) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(folder) = save_dir.file_name().and_then(|n| n.to_str())
        && let Some(name) = legacy_folder_save_name(folder)
    {
        return name.to_string();
    }
    if let Ok(content) = std::fs::read_to_string(save_dir.join("savenfo.txt")) {
        let trimmed = content.trim();
        let stripped = strip_strref_prefix(trimmed);
        if !stripped.is_empty() {
            return stripped.to_string();
        }
    }
    fallback.to_string()
}

/// Extract the user-typed name from an original-NWN2 save folder (`000002 - matt` -> `matt`).
fn legacy_folder_save_name(folder: &str) -> Option<&str> {
    let (digits, rest) = folder.split_once(" - ")?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = rest.trim();
    (!rest.is_empty()).then_some(rest)
}

fn strip_strref_prefix(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix('{')
        && let Some(end) = rest.find('}')
    {
        return rest[end + 1..].trim();
    }
    s
}

/// Return the path to a thumbnail image inside the save folder, if any exists.
/// EE writes `screen.tga`; original NWN2 writes `screen.jpg`. EE takes precedence when
/// both are present (a converted save).
pub fn find_save_thumbnail(save_dir: &Path) -> Option<PathBuf> {
    let tga = save_dir.join("screen.tga");
    if tga.exists() {
        return Some(tga);
    }
    let jpg = save_dir.join("screen.jpg");
    if jpg.exists() {
        return Some(jpg);
    }
    None
}

fn read_save_character_name(save_path: &Path) -> Option<String> {
    // playerinfo.bin is NWN2's own load-menu metadata file - cheap flat read.
    // Only fall back to parsing player.bic out of the zip when it's missing.
    if let Ok(name) = PlayerInfo::get_player_name(save_path.join("playerinfo.bin"))
        && !name.trim().is_empty()
    {
        return Some(name);
    }

    SaveGameHandler::new(save_path)
        .ok()?
        .read_character_name()
        .ok()
        .flatten()
}

fn unix_secs(time: std::time::SystemTime) -> i64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Modification time in seconds since the Unix epoch.
fn modified_secs(path: &Path) -> Option<i64> {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .map(unix_secs)
}

/// Total size of the files directly inside `dir`.
fn files_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .filter_map(|f| f.metadata().ok())
            .filter(std::fs::Metadata::is_file)
            .map(|m| m.len())
            .sum()
    })
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct SaveFile {
    pub path: String,
    pub name: String,
    pub thumbnail: Option<String>,
    pub modified: Option<i64>,
    pub character_name: Option<String>,
}

/// The saves folder, or its `multiplayer` subfolder for `save_mode` "mp".
fn saves_dir(state: &AppState, save_mode: Option<&str>) -> Result<PathBuf, String> {
    let saves = state
        .paths
        .read()
        .saves()
        .ok_or("Could not determine NWN2 saves path")?;
    Ok(if save_mode == Some("mp") {
        saves.join("multiplayer")
    } else {
        saves
    })
}

pub(crate) fn backups_dir(state: &AppState) -> Result<PathBuf, String> {
    Ok(saves_dir(state, None)?.join(backup::INNER_BACKUPS_DIR))
}

#[tauri::command]
pub async fn find_nwn2_saves(
    state: State<'_, AppState>,
    save_mode: Option<String>,
) -> Result<Vec<SaveFile>, String> {
    let saves_path = saves_dir(&state, save_mode.as_deref())?;
    if !saves_path.is_dir() {
        return Ok(Vec::new());
    }

    let saves = list_saves(&saves_path, 3, 0, None)?
        .files
        .into_iter()
        .map(|save| SaveFile {
            name: save.save_name.unwrap_or(save.name),
            path: save.path,
            thumbnail: save.thumbnail,
            modified: Some(save.modified),
            character_name: save.character_name,
        })
        .collect();
    Ok(saves)
}

#[tauri::command]
pub async fn get_save_thumbnail(thumbnail_path: String) -> Result<String, String> {
    let path = PathBuf::from(&thumbnail_path);

    let image = image::open(&path).map_err(|e| {
        error!("Failed to decode thumbnail at '{}': {}", path.display(), e);
        "Failed to process thumbnail. The file may be corrupt or inaccessible.".to_string()
    })?;

    let encoder = webp::Encoder::from_image(&image).map_err(|e| {
        error!("Failed to create WebP encoder: {e}");
        "Failed to create WebP encoder from image.".to_string()
    })?;
    let webp_data = encoder.encode(85.0);

    Ok(BASE64_STANDARD.encode(&*webp_data))
}

#[tauri::command]
pub async fn open_folder_in_explorer(
    app: tauri::AppHandle,
    folder_path: String,
) -> Result<(), String> {
    if !Path::new(&folder_path).exists() {
        return Err(format!("Folder does not exist: {folder_path}"));
    }

    let opener = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    app.shell()
        .command(opener)
        .args([&folder_path])
        .spawn()
        .map_err(|e| format!("Failed to open folder: {e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn launch_nwn2_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    game_path: Option<String>,
) -> Result<(), String> {
    let base_path = match game_path {
        Some(path) => PathBuf::from(path),
        None => state
            .paths
            .read()
            .game_folder()
            .filter(|p| p.exists())
            .cloned()
            .ok_or("NWN2:EE installation not found. Please set the game path in settings.")?,
    };

    let exe_path = base_path.join("NWN2.exe");
    if !exe_path.exists() {
        return Err(format!(
            "No NWN2 executable found in: {}",
            base_path.display()
        ));
    }

    info!("Launching {}", exe_path.display());
    app.shell()
        .command(&exe_path)
        .current_dir(&base_path)
        .spawn()
        .map_err(|e| format!("Failed to launch NWN2: {e}"))?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseSaveEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub save_name: Option<String>,
    pub character_name: Option<String>,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseSavesResponse {
    pub files: Vec<BrowseSaveEntry>,
    pub total_count: usize,
    pub current_path: String,
}

#[tauri::command]
pub async fn browse_saves(
    state: State<'_, AppState>,
    path: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    search: Option<String>,
) -> Result<BrowseSavesResponse, String> {
    let target_path = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => saves_dir(&state, None)?,
    };
    list_saves(
        &target_path,
        limit.unwrap_or(500),
        offset.unwrap_or(0),
        search.as_deref(),
    )
}

/// One page of the save folders in `dir`, newest first, filtered by display
/// name. Size, character name and thumbnail are read only for that page.
pub fn list_saves(
    dir: &Path,
    limit: usize,
    offset: usize,
    search: Option<&str>,
) -> Result<BrowseSavesResponse, String> {
    let search = search
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    if !dir.exists() {
        return Err(format!("Directory not found: {}", dir.display()));
    }
    if !dir.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let entries = std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {e}"))?;

    let mut saves: Vec<(PathBuf, String, Option<String>, i64)> = Vec::new();
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
        if !entry_path.is_dir() || !is_save_dir(&entry_path) {
            continue;
        }

        let save_name = Some(read_save_display_name(&entry_path, "")).filter(|n| !n.is_empty());
        if let Some(query) = &search
            && !save_name
                .as_deref()
                .unwrap_or(&name)
                .to_lowercase()
                .contains(query.as_str())
        {
            continue;
        }

        let modified = modified_secs(&entry_path).unwrap_or(0);
        saves.push((entry_path, name, save_name, modified));
    }
    saves.sort_by_key(|&(.., modified)| std::cmp::Reverse(modified));

    let total_count = saves.len();
    let files: Vec<BrowseSaveEntry> = saves
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(|(save_path, name, save_name, modified)| BrowseSaveEntry {
            size: files_size(&save_path),
            character_name: read_save_character_name(&save_path),
            thumbnail: find_save_thumbnail(&save_path).map(|p| p.to_string_lossy().to_string()),
            path: save_path.to_string_lossy().to_string(),
            name,
            modified,
            save_name,
        })
        .collect();

    info!(
        "browse_saves: path={}, count={}, total={}",
        dir.display(),
        files.len(),
        total_count
    );

    Ok(BrowseSavesResponse {
        files,
        total_count,
        current_path: dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn get_default_saves_path(
    state: State<'_, AppState>,
    save_mode: Option<String>,
) -> Result<String, String> {
    Ok(saves_dir(&state, save_mode.as_deref())?
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub async fn get_default_backups_path(state: State<'_, AppState>) -> Result<String, String> {
    Ok(backups_dir(&state)?.to_string_lossy().to_string())
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
    /// Editor change record; `None` at the root level and for backups that
    /// predate manifests.
    pub changes: Option<BackupManifest>,
}

fn find_newest_inner_backup(save_folder: &Path) -> Option<PathBuf> {
    use std::time::SystemTime;

    let entries = std::fs::read_dir(save_folder).ok()?;
    let mut newest: Option<(SystemTime, PathBuf)> = None;

    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if backup::backup_timestamp(&name).is_none() {
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
pub async fn browse_backups(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<BrowseBackupEntry>, String> {
    let backup_path = PathBuf::from(&path);
    let is_root = backups_dir(&state).is_ok_and(|root| root == backup_path);
    list_backups(&backup_path, is_root)
}

/// The backups in `dir`: at the backups root one entry per save folder
/// (described by its newest backup), inside a save folder one per backup.
pub fn list_backups(dir: &Path, is_root: bool) -> Result<Vec<BrowseBackupEntry>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("Failed to read backup directory: {e}"))?;

    let mut backups = Vec::new();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let timestamp = backup::backup_timestamp(&name);
        if !is_root && timestamp.is_none() {
            continue;
        }
        let timestamp = timestamp.unwrap_or_default().to_string();

        let created_at = std::fs::metadata(&entry_path)
            .map(|m| backup::created_at_secs(&m))
            .unwrap_or(0);

        let source_path = if is_root {
            find_newest_inner_backup(&entry_path)
        } else {
            Some(entry_path.clone())
        };

        let (character_name, save_name) = match &source_path {
            Some(src) => (
                PlayerInfo::get_player_name(src.join("playerinfo.bin")).ok(),
                Some(read_save_display_name(src, "")).filter(|n| !n.is_empty()),
            ),
            None => (None, None),
        };

        let (size, changes) = if is_root {
            (backup::dir_size(&entry_path), None)
        } else {
            (
                files_size(&entry_path),
                crate::services::savegame_handler::backup_manifest::read(&entry_path),
            )
        };

        backups.push(BrowseBackupEntry {
            name,
            path: entry_path.to_string_lossy().to_string(),
            size,
            timestamp,
            created_at,
            character_name,
            save_name,
            changes,
        });
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));

    info!(
        "browse_backups: path={}, is_root={}, count={}",
        dir.display(),
        is_root,
        backups.len()
    );

    Ok(backups)
}

#[derive(Debug, Serialize, Deserialize, Clone, specta::Type)]
pub struct BrowseVaultEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: i64,
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
        if !entry_path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("bic"))
        {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }

        files_list.push(BrowseVaultEntry {
            name: entry_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            size: metadata.len(),
            modified: metadata.modified().map_or(0, unix_secs),
            path: entry_path.to_string_lossy().to_string(),
        });
    }

    files_list.sort_by_key(|f| std::cmp::Reverse(f.modified));

    let total_count = files_list.len();
    info!(
        "browse_localvault: path={}, count={}",
        vault_path.display(),
        total_count
    );

    Ok(BrowseVaultResponse {
        files: files_list,
        total_count,
        path: vault_path.to_string_lossy().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_save_dir_accepts_ee_layout() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("resgff.zip"), b"dummy").unwrap();
        assert!(is_save_dir(tmp.path()));
    }

    #[test]
    fn test_is_save_dir_accepts_original_layout() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        assert!(is_save_dir(tmp.path()));
    }

    #[test]
    fn test_is_save_dir_rejects_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!is_save_dir(tmp.path()));
    }

    #[test]
    fn test_is_save_dir_rejects_partial_original_layout() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        assert!(!is_save_dir(tmp.path()));
    }

    #[test]
    fn test_read_save_display_name_prefers_savename_txt() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("savename.txt"), "EE Save Name\n").unwrap();
        std::fs::write(tmp.path().join("savenfo.txt"), "{1234}Old Name\n").unwrap();
        assert_eq!(
            read_save_display_name(tmp.path(), "fallback"),
            "EE Save Name"
        );
    }

    #[test]
    fn test_read_save_display_name_falls_back_to_savenfo_and_strips_strref() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("savenfo.txt"), "{1111}Farlong House, L2\n").unwrap();
        assert_eq!(
            read_save_display_name(tmp.path(), "fallback"),
            "Farlong House, L2"
        );
    }

    #[test]
    fn test_read_save_display_name_prefers_legacy_folder_suffix_over_savenfo() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("000002 - matt");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(
            dir.join("savenfo.txt"),
            "{1013}Merchant Quarter
",
        )
        .unwrap();
        assert_eq!(read_save_display_name(&dir, "fallback"), "matt");
    }

    #[test]
    fn test_legacy_folder_save_name_requires_numeric_prefix() {
        assert_eq!(legacy_folder_save_name("000002 - matt"), Some("matt"));
        assert_eq!(legacy_folder_save_name("Classic - foo"), None);
        assert_eq!(legacy_folder_save_name("000002 - "), None);
    }

    #[test]
    fn test_read_save_display_name_handles_savenfo_without_strref_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("savenfo.txt"), "Just A Name\n").unwrap();
        assert_eq!(
            read_save_display_name(tmp.path(), "fallback"),
            "Just A Name"
        );
    }

    #[test]
    fn test_read_save_display_name_fallback_when_no_files() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            read_save_display_name(tmp.path(), "folder_name"),
            "folder_name"
        );
    }

    #[test]
    fn test_find_save_thumbnail_prefers_tga_then_jpg() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_save_thumbnail(tmp.path()).is_none());

        std::fs::write(tmp.path().join("screen.jpg"), b"jpg").unwrap();
        assert_eq!(
            find_save_thumbnail(tmp.path()).unwrap(),
            tmp.path().join("screen.jpg")
        );

        std::fs::write(tmp.path().join("screen.tga"), b"tga").unwrap();
        assert_eq!(
            find_save_thumbnail(tmp.path()).unwrap(),
            tmp.path().join("screen.tga")
        );
    }
}
