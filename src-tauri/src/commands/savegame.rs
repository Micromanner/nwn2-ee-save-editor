use crate::commands::{CommandError, CommandResult};
use crate::services::savegame_handler::{BackupInfo, FileInfo, RestoreResult};
use crate::state::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub async fn list_backups(state: State<'_, AppState>) -> CommandResult<Vec<BackupInfo>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(handler.list_backups()?)
}

#[tauri::command]
pub async fn create_backup(state: State<'_, AppState>) -> CommandResult<()> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    crate::services::savegame_handler::backup::create_backup(handler.save_dir())?;
    Ok(())
}

#[tauri::command]
pub async fn restore_backup(
    state: State<'_, AppState>,
    backup_path: String,
    create_pre_restore_backup: bool,
) -> CommandResult<RestoreResult> {
    let backup = PathBuf::from(&backup_path);

    // If a character is loaded, use its save_dir directly
    let save_dir = {
        let session = state.session.read();
        session
            .savegame_handler
            .as_ref()
            .map(|h| h.save_dir().to_path_buf())
    };

    let save_dir = save_dir
        .or_else(|| crate::services::savegame_handler::backup::infer_save_path_from_backup(&backup))
        .ok_or(CommandError::NotFound {
            item: format!("Could not determine save directory for backup: {backup_path}"),
        })?;

    Ok(
        crate::services::savegame_handler::backup::restore_from_backup(
            &backup,
            &save_dir,
            create_pre_restore_backup,
        )?,
    )
}

#[tauri::command]
pub async fn restore_modules_from_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> CommandResult<RestoreResult> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let backup = PathBuf::from(backup_path);
    Ok(
        crate::services::savegame_handler::backup::restore_modules_from_backup(
            &backup,
            handler.save_dir(),
        )?,
    )
}

#[tauri::command]
pub async fn cleanup_backups(
    state: State<'_, AppState>,
    keep_count: usize,
) -> CommandResult<crate::services::savegame_handler::CleanupResult> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(handler.cleanup_old_backups(keep_count)?)
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
pub async fn get_save_info(
    state: State<'_, AppState>,
) -> CommandResult<Option<crate::services::savegame_handler::CharacterSummary>> {
    let session = state.session.read();
    let handler = session
        .savegame_handler
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(handler.read_character_summary()?)
}

#[tauri::command]
pub async fn delete_backup(state: State<'_, AppState>, backup_path: String) -> CommandResult<bool> {
    let _session = state.session.read();
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
}
