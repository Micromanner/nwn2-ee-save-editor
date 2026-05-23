use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::SystemTime;

use chrono::Local;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use super::error::{SaveGameError, SaveGameResult};

static BACKUP_CREATED_FOR_SAVES: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

const BACKUP_TIMESTAMP_FORMAT: &str = "%Y%m%d_%H%M%S";
const DEFAULT_KEEP_COUNT: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub path: PathBuf,
    pub timestamp: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub pre_restore_backup: Option<PathBuf>,
    pub files_restored: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupResult {
    pub removed_count: usize,
    pub remaining_count: usize,
    pub freed_bytes: u64,
}

pub fn get_backups_dir(save_dir: &Path) -> PathBuf {
    save_dir
        .parent()
        .map_or_else(|| save_dir.join("backups"), |p| p.join("backups"))
}

pub fn get_backup_dir_for_save(save_dir: &Path) -> PathBuf {
    let backups_root = get_backups_dir(save_dir);
    let save_name = save_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    backups_root.join(save_name)
}

pub fn has_backup_been_created(save_path: &Path) -> bool {
    BACKUP_CREATED_FOR_SAVES
        .lock()
        .map(|set| set.contains(save_path))
        .unwrap_or(false)
}

pub fn mark_backup_created(save_path: &Path) {
    if let Ok(mut set) = BACKUP_CREATED_FOR_SAVES.lock() {
        set.insert(save_path.to_path_buf());
    }
}

pub fn clear_backup_tracking() {
    if let Ok(mut set) = BACKUP_CREATED_FOR_SAVES.lock() {
        set.clear();
    }
}

pub fn create_backup(save_dir: &Path) -> SaveGameResult<PathBuf> {
    if !save_dir.exists() {
        return Err(SaveGameError::NotFound {
            path: save_dir.to_path_buf(),
        });
    }

    let backup_dir = get_backup_dir_for_save(save_dir);
    fs::create_dir_all(&backup_dir)?;

    let timestamp = Local::now().format(BACKUP_TIMESTAMP_FORMAT).to_string();
    let backup_name = format!("backup_{timestamp}");
    let backup_path = backup_dir.join(&backup_name);

    copy_directory_filtered(save_dir, &backup_path, &["backups"])?;

    mark_backup_created(save_dir);

    info!(
        "Created backup: {} -> {}",
        save_dir.display(),
        backup_path.display()
    );
    Ok(backup_path)
}

pub fn create_pre_restore_backup(save_dir: &Path) -> SaveGameResult<PathBuf> {
    let backup_dir = get_backup_dir_for_save(save_dir);
    fs::create_dir_all(&backup_dir)?;

    let timestamp = Local::now().format(BACKUP_TIMESTAMP_FORMAT).to_string();
    let backup_name = format!("pre_restore_{timestamp}");
    let backup_path = backup_dir.join(&backup_name);

    copy_directory_filtered(save_dir, &backup_path, &["backups"])?;

    info!(
        "Created pre-restore backup: {} -> {}",
        save_dir.display(),
        backup_path.display()
    );
    Ok(backup_path)
}

pub fn list_backups(save_dir: &Path) -> SaveGameResult<Vec<BackupInfo>> {
    let backup_dir = get_backup_dir_for_save(save_dir);

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();

            if name.starts_with("backup_") || name.starts_with("pre_restore_") {
                let timestamp = name
                    .strip_prefix("backup_")
                    .or_else(|| name.strip_prefix("pre_restore_"))
                    .unwrap_or(name)
                    .to_string();

                let metadata = fs::metadata(&path)?;
                let created_at = metadata
                    .created()
                    .or_else(|_| metadata.modified())
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs() as i64);

                let size_bytes = calculate_dir_size(&path);

                backups.push(BackupInfo {
                    path,
                    timestamp,
                    size_bytes,
                    created_at,
                });
            }
        }
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));

    Ok(backups)
}

pub fn restore_from_backup(
    backup_path: &Path,
    save_dir: &Path,
    create_pre_restore: bool,
) -> SaveGameResult<RestoreResult> {
    if !backup_path.exists() {
        return Err(SaveGameError::NotFound {
            path: backup_path.to_path_buf(),
        });
    }

    // Defense-in-depth: refuse to restore a backup into a save_dir it doesn't
    // belong to. The backup's parent folder is named after its origin save;
    // it must match the target save_dir's folder name.
    let backup_origin = backup_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());
    let target_name = save_dir.file_name().and_then(|n| n.to_str());
    if let (Some(origin), Some(target)) = (backup_origin, target_name)
        && origin != target
    {
        return Err(SaveGameError::RestoreFailed(format!(
            "Backup belongs to save '{origin}' but target is '{target}'; refusing cross-save restore"
        )));
    }

    let pre_restore_backup = if create_pre_restore && save_dir.exists() {
        Some(create_pre_restore_backup(save_dir)?)
    } else {
        None
    };

    if save_dir.exists() {
        // Clear save dir contents but preserve the inner backups/ folder
        // (module/globals backups live there)
        for entry in fs::read_dir(save_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "backups") {
                    continue;
                }
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
    } else {
        fs::create_dir_all(save_dir)?;
    }

    let files_restored = copy_directory(backup_path, save_dir)?;

    info!(
        "Restored backup: {} -> {} ({} files)",
        backup_path.display(),
        save_dir.display(),
        files_restored
    );

    Ok(RestoreResult {
        success: true,
        pre_restore_backup,
        files_restored,
        message: format!("Successfully restored {files_restored} files"),
    })
}

pub fn restore_modules_from_backup(
    backup_path: &Path,
    save_dir: &Path,
) -> SaveGameResult<RestoreResult> {
    if !backup_path.exists() {
        return Err(SaveGameError::NotFound {
            path: backup_path.to_path_buf(),
        });
    }

    let mut files_restored = 0usize;

    for entry in fs::read_dir(backup_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("z"))
        {
            let filename = path.file_name().unwrap();
            let dest = save_dir.join(filename);
            fs::copy(&path, &dest)?;
            files_restored += 1;
            debug!("Restored module file: {}", filename.to_string_lossy());
        }
    }

    info!(
        "Restored {} module .z files from {} -> {}",
        files_restored,
        backup_path.display(),
        save_dir.display()
    );

    Ok(RestoreResult {
        success: true,
        pre_restore_backup: None,
        files_restored,
        message: format!("Successfully restored {files_restored} module files"),
    })
}

pub fn cleanup_old_backups(save_dir: &Path, keep_count: usize) -> SaveGameResult<CleanupResult> {
    let keep_count = if keep_count == 0 {
        DEFAULT_KEEP_COUNT
    } else {
        keep_count
    };

    let mut backups = list_backups(save_dir)?;

    if backups.len() <= keep_count {
        return Ok(CleanupResult {
            removed_count: 0,
            remaining_count: backups.len(),
            freed_bytes: 0,
        });
    }

    let mut removed_count = 0;
    let mut freed_bytes = 0u64;

    while backups.len() > keep_count {
        if let Some(oldest) = backups.pop()
            && oldest.path.exists()
        {
            freed_bytes += oldest.size_bytes;
            fs::remove_dir_all(&oldest.path)?;
            removed_count += 1;
            debug!("Removed old backup: {}", oldest.path.display());
        }
    }

    info!(
        "Cleaned up {} old backups, freed {} bytes",
        removed_count, freed_bytes
    );

    Ok(CleanupResult {
        removed_count,
        remaining_count: backups.len(),
        freed_bytes,
    })
}

pub fn infer_save_path_from_backup(backup_path: &Path) -> Option<PathBuf> {
    let backup_dir = backup_path.parent()?;
    let save_name = backup_dir.file_name()?;

    let backups_root = backup_dir.parent()?;

    let saves_dir = backups_root.parent()?.join("saves");
    if saves_dir.exists() {
        let inferred = saves_dir.join(save_name);
        if inferred.exists() {
            return Some(inferred);
        }
    }

    let sibling_save = backups_root.parent()?.join(save_name);
    if sibling_save.exists() {
        return Some(sibling_save);
    }

    None
}

fn copy_directory(src: &Path, dst: &Path) -> SaveGameResult<usize> {
    copy_directory_filtered(src, dst, &[])
}

fn copy_directory_filtered(src: &Path, dst: &Path, skip_dirs: &[&str]) -> SaveGameResult<usize> {
    fs::create_dir_all(dst)?;

    let mut count = 0;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            if skip_dirs.iter().any(|&s| entry.file_name() == s) {
                continue;
            }
            count += copy_directory_filtered(&src_path, &dst_path, skip_dirs)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
            count += 1;
        }
    }

    Ok(count)
}

fn calculate_dir_size(path: &Path) -> u64 {
    let mut size = 0u64;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += calculate_dir_size(&path);
            } else if let Ok(metadata) = fs::metadata(&path) {
                size += metadata.len();
            }
        }
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_dir_structure() {
        let save_dir = PathBuf::from("/saves/mysave");
        let backup_dir = get_backup_dir_for_save(&save_dir);

        assert!(backup_dir.to_string_lossy().contains("backups"));
        assert!(backup_dir.to_string_lossy().contains("mysave"));
    }

    #[test]
    fn test_backup_tracking() {
        clear_backup_tracking();

        let path = PathBuf::from("/test/save");
        assert!(!has_backup_been_created(&path));

        mark_backup_created(&path);
        assert!(has_backup_been_created(&path));

        clear_backup_tracking();
        assert!(!has_backup_been_created(&path));
    }

    #[test]
    fn test_restore_refuses_cross_save() {
        let tmp = std::env::temp_dir().join(format!(
            "nwn2_restore_guard_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let saves_root = tmp.join("saves");
        let backups_root = saves_root.join("backups");
        let backup_for_b = backups_root.join("b").join("backup_20260425_000000");
        let save_a = saves_root.join("a");

        fs::create_dir_all(&backup_for_b).unwrap();
        fs::create_dir_all(&save_a).unwrap();
        fs::write(backup_for_b.join("resgff.zip"), b"b-data").unwrap();
        fs::write(save_a.join("resgff.zip"), b"a-data").unwrap();

        let result = restore_from_backup(&backup_for_b, &save_a, false);
        assert!(
            matches!(result, Err(SaveGameError::RestoreFailed(_))),
            "cross-save restore should be rejected, got: {result:?}"
        );
        assert_eq!(
            fs::read(save_a.join("resgff.zip")).unwrap(),
            b"a-data",
            "target save must not be modified when guard trips"
        );

        fs::remove_dir_all(&tmp).ok();
    }
}
