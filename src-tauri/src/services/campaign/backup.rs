use crate::services::campaign::settings::CampaignBackupInfo;
use crate::services::savegame_handler::SaveGameHandler;
use crate::services::savegame_handler::backup::{
    BACKUP_TIMESTAMP_FORMAT, INNER_BACKUPS_DIR, created_at_secs,
};
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

const GLOBALS_BACKUP_PREFIX: &str = "globals_";

fn get_campaign_variables_backup_dir(save_dir: &Path) -> PathBuf {
    save_dir
        .join(INNER_BACKUPS_DIR)
        .join("campaign_variable_backups")
}

fn get_module_backup_dir(save_dir: &Path) -> PathBuf {
    save_dir.join(INNER_BACKUPS_DIR).join("module_backups")
}

/// Module id of a `{module_id}_{YYYYMMDD}_{HHMMSS}.z` backup file name.
fn module_id_of_backup(file_name: &str) -> Option<&str> {
    let stem = Path::new(file_name).file_stem()?.to_str()?;
    stem.rsplitn(3, '_').nth(2)
}

/// Refuse backup paths that are not directly inside `dir`, so a restore can
/// only ever use this save's own backups.
pub(crate) fn ensure_backup_in_dir(backup: &Path, dir: &Path) -> Result<(), String> {
    let backup = fs::canonicalize(backup)
        .map_err(|e| format!("Backup file not found: {} ({e})", backup.display()))?;
    let dir = fs::canonicalize(dir).map_err(|e| format!("Backup folder not found: {e}"))?;
    if backup.parent() != Some(dir.as_path()) {
        return Err(format!(
            "Backup {} does not belong to this save",
            backup.display()
        ));
    }
    Ok(())
}

pub fn backup_campaign_variables(handler: &SaveGameHandler) -> Result<(), String> {
    let save_dir = handler.save_dir();
    let source = save_dir.join("globals.xml");

    if !source.exists() {
        return Ok(());
    }

    let backup_dir = get_campaign_variables_backup_dir(save_dir);
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format(BACKUP_TIMESTAMP_FORMAT);
    let dest = backup_dir.join(format!("{GLOBALS_BACKUP_PREFIX}{timestamp}.xml"));
    // Edits within the same second keep the first backup, the state before them.
    if dest.exists() {
        return Ok(());
    }

    fs::copy(&source, &dest).map_err(|e| format!("Failed to backup globals.xml: {e}"))?;
    info!("Created globals.xml backup at {:?}", dest);
    Ok(())
}

pub fn list_campaign_variable_backups(
    handler: &SaveGameHandler,
) -> Result<Vec<CampaignBackupInfo>, String> {
    collect_backups(
        &get_campaign_variables_backup_dir(handler.save_dir()),
        "xml",
    )
}

pub fn restore_campaign_variable_backup(
    handler: &SaveGameHandler,
    backup_path: &str,
) -> Result<(), String> {
    let backup = PathBuf::from(backup_path);
    ensure_backup_in_dir(
        &backup,
        &get_campaign_variables_backup_dir(handler.save_dir()),
    )?;

    let dest = handler.save_dir().join("globals.xml");
    fs::copy(&backup, &dest).map_err(|e| format!("Failed to restore globals.xml backup: {e}"))?;
    info!("Restored globals.xml from backup: {}", backup.display());
    Ok(())
}

pub fn backup_module_z(handler: &SaveGameHandler, module_id: &str) -> Result<(), String> {
    let save_dir = handler.save_dir();

    // Try both casings: EE uses `.z`, original NWN2 uses `.Z`. Preserve whichever the
    // game wrote so a restored backup matches what the engine expects.
    let (source, ext) = {
        let lower = save_dir.join(format!("{module_id}.z"));
        let upper = save_dir.join(format!("{module_id}.Z"));
        if lower.exists() {
            (lower, "z")
        } else if upper.exists() {
            (upper, "Z")
        } else {
            return Ok(());
        }
    };

    let backup_dir = get_module_backup_dir(save_dir);
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format(BACKUP_TIMESTAMP_FORMAT);
    let dest = backup_dir.join(format!("{module_id}_{timestamp}.{ext}"));
    if dest.exists() {
        return Ok(());
    }

    fs::copy(&source, &dest).map_err(|e| format!("Failed to backup {module_id}.{ext}: {e}"))?;
    info!("Created module .{} backup at {:?}", ext, dest);
    Ok(())
}

pub fn list_module_backups(
    handler: &SaveGameHandler,
    module_id: &str,
) -> Result<Vec<CampaignBackupInfo>, String> {
    let mut backups = collect_backups(&get_module_backup_dir(handler.save_dir()), "z")?;
    backups.retain(|b| {
        module_id_of_backup(&b.filename).is_some_and(|id| id.eq_ignore_ascii_case(module_id))
    });
    Ok(backups)
}

pub fn restore_module_backup(handler: &SaveGameHandler, backup_path: &str) -> Result<(), String> {
    let backup = PathBuf::from(backup_path);
    ensure_backup_in_dir(&backup, &get_module_backup_dir(handler.save_dir()))?;

    let filename = backup
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid backup filename")?;
    let module_id = module_id_of_backup(filename)
        .ok_or_else(|| format!("Cannot determine module ID from backup filename: {filename}"))?;

    // Preserve the on-disk casing: restore to .Z if an original-NWN2-style file exists,
    // otherwise .z (EE default, or new save).
    let upper = handler.save_dir().join(format!("{module_id}.Z"));
    let ext = if upper.exists() { "Z" } else { "z" };
    let dest = handler.save_dir().join(format!("{module_id}.{ext}"));
    fs::copy(&backup, &dest)
        .map_err(|e| format!("Failed to restore {module_id}.{ext} backup: {e}"))?;
    info!(
        "Restored {}.{} from backup: {}",
        module_id,
        ext,
        backup.display()
    );
    Ok(())
}

/// Backup files with `extension` in `dir`, newest first; empty if `dir` is missing.
pub(crate) fn collect_backups(
    dir: &Path,
    extension: &str,
) -> Result<Vec<CampaignBackupInfo>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|e| e.to_string())?;

        if metadata.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
        {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            backups.push(CampaignBackupInfo {
                filename,
                path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                created_at: created_at_secs(&metadata),
            });
        }
    }

    // Same-second ties fall back to the timestamp in the file name.
    backups.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| b.filename.cmp(&a.filename))
    });
    Ok(backups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_id_keeps_underscores() {
        assert_eq!(
            module_id_of_backup("1000_Neverwinter_A1_20260925_120000.z"),
            Some("1000_Neverwinter_A1")
        );
        assert_eq!(module_id_of_backup("broken.z"), None);
    }

    #[test]
    fn backup_outside_dir_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let own = tmp.path().join("a").join("module_backups");
        let other = tmp.path().join("b").join("module_backups");
        fs::create_dir_all(&own).unwrap();
        fs::create_dir_all(&other).unwrap();
        let foreign = other.join("m_20260925_120000.z");
        let local = own.join("m_20260925_120000.z");
        fs::write(&foreign, b"x").unwrap();
        fs::write(&local, b"x").unwrap();

        assert!(ensure_backup_in_dir(&foreign, &own).is_err());
        assert!(ensure_backup_in_dir(&local, &own).is_ok());
    }
}
