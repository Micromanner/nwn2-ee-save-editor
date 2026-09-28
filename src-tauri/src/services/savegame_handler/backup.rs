use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::Local;
use tracing::{info, warn};

use super::backup_manifest::{self, BackupReason};
use super::error::{SaveGameError, SaveGameResult};

pub const BACKUP_TIMESTAMP_FORMAT: &str = "%Y%m%d_%H%M%S";
/// Folder inside a save that holds the per-file globals/module backups. Full
/// backups never copy it and restores never touch it.
pub const INNER_BACKUPS_DIR: &str = "backups";
const LOAD_PREFIX: &str = "backup_";
const PRE_RESTORE_PREFIX: &str = "pre_restore_";
const RESTORE_STAGING_DIR: &str = ".restore_staging";
const RESTORE_DISPLACED_DIR: &str = ".restore_displaced";

pub struct RestoreResult {
    pub pre_restore_backup: PathBuf,
    pub files_restored: usize,
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

/// Timestamp part of a full-backup folder name, `None` for any other folder.
pub fn backup_timestamp(dir_name: &str) -> Option<&str> {
    dir_name
        .strip_prefix(LOAD_PREFIX)
        .or_else(|| dir_name.strip_prefix(PRE_RESTORE_PREFIX))
}

/// Creation time (modification time where unsupported) as Unix seconds, 0 if unknown.
pub fn created_at_secs(metadata: &fs::Metadata) -> i64 {
    metadata
        .created()
        .or_else(|_| metadata.modified())
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64)
}

pub fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(ft) if ft.is_dir() => dir_size(&entry.path()),
            Ok(_) => entry.metadata().map_or(0, |m| m.len()),
            Err(_) => 0,
        })
        .sum()
}

pub fn create_backup(save_dir: &Path, reason: BackupReason) -> SaveGameResult<PathBuf> {
    if !save_dir.exists() {
        return Err(SaveGameError::NotFound {
            path: save_dir.to_path_buf(),
        });
    }

    let prefix = match reason {
        BackupReason::Load => LOAD_PREFIX,
        BackupReason::PreRestore => PRE_RESTORE_PREFIX,
    };
    let timestamp = Local::now().format(BACKUP_TIMESTAMP_FORMAT);
    let backup_path = get_backup_dir_for_save(save_dir).join(format!("{prefix}{timestamp}"));

    copy_save_files(save_dir, &backup_path)?;
    backup_manifest::write_initial(&backup_path, reason);

    info!(
        "Created backup: {} -> {}",
        save_dir.display(),
        backup_path.display()
    );
    Ok(backup_path)
}

/// Replace the contents of `save_dir` with `backup_path`, after taking a
/// pre-restore backup. The backup is copied to a staging folder first and
/// swapped in by renames, so a failed copy leaves the save untouched and a
/// failed swap moves the original files back.
pub fn restore_from_backup(backup_path: &Path, save_dir: &Path) -> SaveGameResult<RestoreResult> {
    if !backup_path.exists() {
        return Err(SaveGameError::NotFound {
            path: backup_path.to_path_buf(),
        });
    }

    // The backup's parent folder is named after its origin save; it must match
    // the target, or a restore would overwrite an unrelated save.
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

    let pre_restore_backup = create_backup(save_dir, BackupReason::PreRestore)?;

    let work_dir = get_backup_dir_for_save(save_dir);
    let staging = work_dir.join(RESTORE_STAGING_DIR);
    let displaced = work_dir.join(RESTORE_DISPLACED_DIR);
    for leftover in [&staging, &displaced] {
        if leftover.exists() {
            fs::remove_dir_all(leftover)?;
        }
    }

    let files_restored = match copy_save_files(backup_path, &staging) {
        Ok(count) => count,
        Err(e) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    };
    let swapped = swap_save_contents(save_dir, &staging, &displaced);
    for temp in [&staging, &displaced] {
        if temp.exists()
            && let Err(e) = fs::remove_dir_all(temp)
        {
            warn!("Failed to remove {}: {e}", temp.display());
        }
    }
    swapped?;

    info!(
        "Restored backup: {} -> {} ({} files)",
        backup_path.display(),
        save_dir.display(),
        files_restored
    );

    Ok(RestoreResult {
        pre_restore_backup,
        files_restored,
    })
}

/// Move the save's files (except the inner backups folder) to `displaced`,
/// then the staged files into the save, undoing both moves on failure.
fn swap_save_contents(save_dir: &Path, staging: &Path, displaced: &Path) -> SaveGameResult<()> {
    fs::create_dir_all(displaced)?;

    let mut displaced_names = Vec::new();
    if let Err(e) = move_entries(save_dir, displaced, &mut displaced_names) {
        move_back(displaced, save_dir, &displaced_names);
        return Err(SaveGameError::RestoreFailed(format!(
            "Could not move the current save files aside, save left unchanged: {e}"
        )));
    }

    let mut restored_names = Vec::new();
    if let Err(e) = move_entries(staging, save_dir, &mut restored_names) {
        move_back(save_dir, staging, &restored_names);
        move_back(displaced, save_dir, &displaced_names);
        return Err(SaveGameError::RestoreFailed(format!(
            "Could not move the restored files into the save, original files put back: {e}"
        )));
    }
    Ok(())
}

fn move_entries(from: &Path, to: &Path, moved: &mut Vec<OsString>) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let name = entry?.file_name();
        if name == INNER_BACKUPS_DIR {
            continue;
        }
        fs::rename(from.join(&name), to.join(&name))?;
        moved.push(name);
    }
    Ok(())
}

fn move_back(from: &Path, to: &Path, names: &[OsString]) {
    for name in names {
        if let Err(e) = fs::rename(from.join(name), to.join(name)) {
            warn!(
                "Failed to move {} back to {}: {e}",
                from.join(name).display(),
                to.display()
            );
        }
    }
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

/// Copy a save (or backup) folder, skipping the inner backups folder and the
/// backup manifest. Returns the number of files copied.
pub(crate) fn copy_save_files(src: &Path, dst: &Path) -> SaveGameResult<usize> {
    fs::create_dir_all(dst)?;

    let mut count = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            if entry.file_name() == INNER_BACKUPS_DIR {
                continue;
            }
            count += copy_save_files(&src_path, &dst_path)?;
        } else if entry.file_name() != backup_manifest::MANIFEST_FILE {
            fs::copy(&src_path, &dst_path)?;
            count += 1;
        }
    }

    Ok(count)
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
    fn test_backup_timestamp() {
        assert_eq!(
            backup_timestamp("backup_20260925_120000"),
            Some("20260925_120000")
        );
        assert_eq!(
            backup_timestamp("pre_restore_20260925_120000"),
            Some("20260925_120000")
        );
        assert_eq!(backup_timestamp(RESTORE_STAGING_DIR), None);
    }

    #[test]
    fn test_restore_refuses_cross_save() {
        let tmp = tempfile::tempdir().unwrap();
        let saves_root = tmp.path().join("saves");
        let backups_root = saves_root.join("backups");
        let backup_for_b = backups_root.join("b").join("backup_20260425_000000");
        let save_a = saves_root.join("a");

        fs::create_dir_all(&backup_for_b).unwrap();
        fs::create_dir_all(&save_a).unwrap();
        fs::write(backup_for_b.join("resgff.zip"), b"b-data").unwrap();
        fs::write(save_a.join("resgff.zip"), b"a-data").unwrap();

        let result = restore_from_backup(&backup_for_b, &save_a);
        assert!(
            matches!(result, Err(SaveGameError::RestoreFailed(_))),
            "cross-save restore should be rejected"
        );
        assert_eq!(
            fs::read(save_a.join("resgff.zip")).unwrap(),
            b"a-data",
            "target save must not be modified when guard trips"
        );
    }

    #[test]
    fn test_restore_replaces_save_and_keeps_inner_backups() {
        let tmp = tempfile::tempdir().unwrap();
        let save = tmp.path().join("saves").join("a");
        let inner = save.join(INNER_BACKUPS_DIR).join("module_backups");
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join("m_20260925_120000.z"), b"old-module").unwrap();
        fs::write(save.join("globals.xml"), b"original").unwrap();

        let backup = create_backup(&save, BackupReason::Load).unwrap();
        assert!(!backup.join(INNER_BACKUPS_DIR).exists());

        fs::write(save.join("globals.xml"), b"edited").unwrap();
        fs::write(save.join("new.z"), b"added after backup").unwrap();

        let result = restore_from_backup(&backup, &save).unwrap();

        assert_eq!(result.files_restored, 1);
        assert_eq!(fs::read(save.join("globals.xml")).unwrap(), b"original");
        assert!(!save.join("new.z").exists());
        assert!(!save.join(backup_manifest::MANIFEST_FILE).exists());
        assert!(inner.join("m_20260925_120000.z").exists());
        assert_eq!(
            fs::read(result.pre_restore_backup.join("globals.xml")).unwrap(),
            b"edited"
        );
        let work_dir = get_backup_dir_for_save(&save);
        assert!(!work_dir.join(RESTORE_STAGING_DIR).exists());
        assert!(!work_dir.join(RESTORE_DISPLACED_DIR).exists());
    }
}
