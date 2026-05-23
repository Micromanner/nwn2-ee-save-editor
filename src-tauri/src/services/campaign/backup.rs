use crate::services::campaign::settings::CampaignBackupInfo;
use crate::services::savegame_handler::SaveGameHandler;
use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

fn get_campaign_variables_backup_dir(save_dir: &Path) -> PathBuf {
    save_dir.join("backups").join("campaign_variable_backups")
}

fn get_module_backup_dir(save_dir: &Path) -> PathBuf {
    save_dir.join("backups").join("module_backups")
}

pub fn backup_campaign_variables(handler: &SaveGameHandler) -> Result<(), String> {
    let save_dir = handler.save_dir();
    let source = save_dir.join("globals.xml");

    if !source.exists() {
        return Ok(());
    }

    let backup_dir = get_campaign_variables_backup_dir(save_dir);
    fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("globals_{timestamp}.xml");
    let dest = backup_dir.join(&filename);

    fs::copy(&source, &dest).map_err(|e| format!("Failed to backup globals.xml: {e}"))?;
    info!("Created globals.xml backup at {:?}", dest);
    Ok(())
}

pub fn list_campaign_variable_backups(
    handler: &SaveGameHandler,
) -> Result<Vec<CampaignBackupInfo>, String> {
    let backup_dir = get_campaign_variables_backup_dir(handler.save_dir());

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    collect_backups(&backup_dir, "xml")
}

pub fn restore_campaign_variable_backup(
    handler: &SaveGameHandler,
    backup_path: &str,
) -> Result<(), String> {
    let backup = PathBuf::from(backup_path);
    if !backup.exists() {
        return Err(format!("Backup file not found: {backup_path}"));
    }

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

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{module_id}_{timestamp}.{ext}");
    let dest = backup_dir.join(&filename);

    fs::copy(&source, &dest).map_err(|e| format!("Failed to backup {module_id}.{ext}: {e}"))?;
    info!("Created module .{} backup at {:?}", ext, dest);
    Ok(())
}

pub fn list_module_backups(handler: &SaveGameHandler) -> Result<Vec<CampaignBackupInfo>, String> {
    let backup_dir = get_module_backup_dir(handler.save_dir());

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    collect_backups(&backup_dir, "z")
}

pub fn restore_module_backup(handler: &SaveGameHandler, backup_path: &str) -> Result<(), String> {
    let backup = PathBuf::from(backup_path);
    if !backup.exists() {
        return Err(format!("Backup file not found: {backup_path}"));
    }

    let filename = backup
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("Invalid backup filename")?;

    // Filename format: {module_id}_{YYYYMMDD}_{HHMMSS}
    // Strip the last two underscore-separated segments to get module_id
    let parts: Vec<&str> = filename.rsplitn(3, '_').collect();
    let module_id = if parts.len() == 3 {
        parts[2]
    } else {
        return Err(format!(
            "Cannot determine module ID from backup filename: {filename}"
        ));
    };

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

fn collect_backups(dir: &Path, extension: &str) -> Result<Vec<CampaignBackupInfo>, String> {
    let mut backups = Vec::new();

    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
        {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();

            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
            let created_at = metadata
                .created()
                .or_else(|_| metadata.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs() as i64);

            backups.push(CampaignBackupInfo {
                filename,
                path: path.to_string_lossy().to_string(),
                size_bytes: metadata.len(),
                created_at,
            });
        }
    }

    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    Ok(backups)
}
