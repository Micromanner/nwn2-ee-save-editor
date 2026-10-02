use crate::commands::{CommandError, CommandResult};
use crate::services::load_diagnostics::{LoadInput, LoadReport};
use crate::services::savegame_handler::FileInfo;
use crate::services::savegame_handler::backup;
use crate::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn restore_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<bool> {
    let reloaded = restore_and_reload(&state, &backup_path)?;
    if reloaded {
        super::session::sync_module_content(&state).await;
    }
    Ok(reloaded)
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

#[tauri::command]
pub async fn list_save_files(state: State<'_, AppState>) -> CommandResult<Vec<FileInfo>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(handler.list_files()?)
}

#[tauri::command]
pub async fn delete_backup(
    _state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<bool> {
    tokio::task::spawn_blocking(move || {
        let path = PathBuf::from(&backup_path);

        if !path.exists() {
            return Err(CommandError::NotFound {
                item: format!("Backup path: {backup_path}"),
            });
        }

        if !path.is_dir() {
            return Err(CommandError::FileError {
                message: "Backup path is not a directory".to_string(),
                path: Some(backup_path),
                diagnostics_path: None,
            });
        }

        std::fs::remove_dir_all(&path)?;
        Ok(true)
    })
    .await
    .map_err(|e| CommandError::Internal(e.to_string()))?
}

#[tauri::command]
pub async fn delete_backups(
    _state: State<'_, AppState>,
    backup_paths: Vec<String>,
) -> CommandResult<usize> {
    tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;

        let deleted_count = backup_paths
            .par_iter()
            .filter_map(|backup_path| {
                let path = PathBuf::from(backup_path);
                if path.exists() && path.is_dir() {
                    match std::fs::remove_dir_all(&path) {
                        Ok(()) => Some(1),
                        Err(err) => {
                            tracing::warn!("Failed to delete backup dir {}: {err}", path.display());
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .sum();

        Ok(deleted_count)
    })
    .await
    .map_err(|e| CommandError::Internal(e.to_string()))?
}
