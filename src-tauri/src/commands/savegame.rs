use crate::commands::{CommandError, CommandResult};
use crate::services::load_diagnostics::{LoadInput, LoadReport};
use crate::services::savegame_handler::backup;
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

#[tauri::command]
pub async fn restore_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<bool> {
    let reloaded = restore_and_reload(&state, &backup_path);
    super::session::sync_module_content(&state).await;
    reloaded
}

fn restore_and_reload(state: &AppState, backup_path: &str) -> CommandResult<bool> {
    let backup = PathBuf::from(backup_path);

    // A backup's target save is determined by its filesystem location
    // (`saves/backups/<save_name>/backup_XXX`), not by whatever save happens to
    // be loaded. Routing a backup to a mismatched save_dir silently overwrites
    // the wrong save.
    let inferred_save_dir =
        backup::infer_save_path_from_backup(&backup).ok_or(CommandError::NotFound {
            item: format!("Could not determine save directory for backup: {backup_path}"),
        })?;

    // Held for the whole restore so no edit or save lands between the file swap
    // and the reload.
    let mut session = state.session.write();
    let loaded_save_dir = session
        .savegame_handler
        .as_ref()
        .map(|h| h.save_dir().to_path_buf());

    // If a character is loaded, it must be the save this backup belongs to,
    // otherwise the user is about to overwrite an unrelated save.
    if let Some(loaded) = &loaded_save_dir
        && !crate::config::nwn2_paths::paths_equivalent(loaded, &inferred_save_dir)
    {
        let loaded_name = loaded.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let target_name = inferred_save_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");
        return Err(CommandError::OperationFailed {
            operation: "restore backup".to_string(),
            reason: format!(
                "Backup belongs to save '{target_name}' but '{loaded_name}' is currently loaded. Load '{target_name}' (or close the current save) before restoring."
            ),
        });
    }

    backup::restore_from_backup(&backup, &inferred_save_dir)?;

    // Returns whether the loaded save was reloaded from the restored files.
    let reloaded = loaded_save_dir.is_some();
    if reloaded {
        let file_path = session
            .current_file_path
            .clone()
            .unwrap_or_else(|| inferred_save_dir.clone())
            .to_string_lossy()
            .to_string();
        let player_index = Some(session.selected_player_index);
        let mut report = LoadReport::new(LoadInput {
            file_path: crate::commands::debug::redact(&file_path),
            player_index,
            file_size: None,
        });
        // No load backup: it would duplicate the backup just restored.
        if let Err(e) = session.load_character(&file_path, player_index, false, &mut report) {
            // The in-memory session describes the files the restore replaced;
            // closing it keeps a later save from writing them back.
            session.close_character();
            return Err(CommandError::OperationFailed {
                operation: "reload restored save".to_string(),
                reason: e.message,
            });
        }
    }

    Ok(reloaded)
}

/// Canonical path of a full-backup folder (`<saves>/backups/<save>/backup_*`),
/// refusing anything else so a delete can never leave the backups root.
fn resolve_backup_dir(backup_path: &str, backups_root: &Path) -> CommandResult<PathBuf> {
    let path = PathBuf::from(backup_path);
    let canonical = std::fs::canonicalize(&path).map_err(|_| CommandError::NotFound {
        item: format!("Backup path: {backup_path}"),
    })?;
    let root = std::fs::canonicalize(backups_root).map_err(|_| CommandError::NotFound {
        item: format!("Backups folder: {}", backups_root.display()),
    })?;
    let is_backup_folder = canonical.is_dir()
        && canonical
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| backup::backup_timestamp(n).is_some())
        && canonical.parent().and_then(Path::parent) == Some(root.as_path());
    if !is_backup_folder {
        return Err(CommandError::FileError {
            message: "Path is not a save backup folder".to_string(),
            path: Some(backup_path.to_string()),
            diagnostics_path: None,
        });
    }
    Ok(canonical)
}

fn remove_backup(backup_path: &str, backups_root: &Path) -> CommandResult<()> {
    std::fs::remove_dir_all(resolve_backup_dir(backup_path, backups_root)?)?;
    Ok(())
}

fn backups_root(state: &AppState) -> CommandResult<PathBuf> {
    state
        .paths
        .read()
        .saves()
        .map(|saves| saves.join(backup::INNER_BACKUPS_DIR))
        .ok_or(CommandError::NotFound {
            item: "Saves folder".to_string(),
        })
}

#[tauri::command]
pub async fn delete_backup(state: State<'_, AppState>, backup_path: String) -> CommandResult<bool> {
    let root = backups_root(&state)?;
    tokio::task::spawn_blocking(move || {
        remove_backup(&backup_path, &root)?;
        Ok(true)
    })
    .await
    .map_err(|e| CommandError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_backups(
    state: State<'_, AppState>,
    backup_paths: Vec<String>,
) -> CommandResult<usize> {
    let root = backups_root(&state)?;
    tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;

        let deleted_count = backup_paths
            .par_iter()
            .filter(|backup_path| {
                remove_backup(backup_path, &root)
                    .inspect_err(|err| {
                        tracing::warn!("Failed to delete backup {backup_path}: {err}");
                    })
                    .is_ok()
            })
            .count();

        Ok(deleted_count)
    })
    .await
    .map_err(|e| CommandError::Internal(e.to_string()))?
}
