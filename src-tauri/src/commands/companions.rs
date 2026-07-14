use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use tauri::{AppHandle, State};
use tracing::{error, info, instrument};

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct RosterClassInfo {
    pub class_id: i32,
    pub level: i32,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct RosterEntryInfo {
    pub ros_name: String,
    pub char_name: String,
    pub classes: Vec<RosterClassInfo>,
}

#[tauri::command]
pub async fn list_roster(state: State<'_, AppState>) -> CommandResult<Vec<RosterEntryInfo>> {
    let session = state.session.read();
    let listings = session
        .list_roster()
        .map_err(|message| CommandError::FileError {
            message,
            path: None,
            diagnostics_path: None,
        })?;
    Ok(listings
        .into_iter()
        .map(|l| RosterEntryInfo {
            ros_name: l.ros_name,
            char_name: l.char_name,
            classes: l
                .classes
                .into_iter()
                .map(|(class_id, level)| RosterClassInfo { class_id, level })
                .collect(),
        })
        .collect())
}

#[tauri::command]
#[instrument(name = "load_companion_command", skip(state), fields(ros_name = %ros_name))]
pub async fn load_companion(
    state: State<'_, AppState>,
    ros_name: String,
    force: bool,
) -> CommandResult<bool> {
    info!("Load companion command invoked");
    let mut session = state.session.write();
    session
        .load_companion(&ros_name, force)
        .map_err(|message| {
            error!("Failed to load companion: {message}");
            CommandError::FileError {
                message,
                path: None,
                diagnostics_path: None,
            }
        })?;
    Ok(true)
}

#[tauri::command]
#[instrument(name = "load_player_command", skip(state, app))]
pub async fn load_player(
    state: State<'_, AppState>,
    app: AppHandle,
    force: bool,
) -> CommandResult<bool> {
    info!("Load player command invoked");
    let (file_path, player_index) = {
        let session = state.session.read();
        if !force && session.has_unsaved_changes() {
            return Err(CommandError::FileError {
                message: "Unsaved changes present; save or discard before switching characters"
                    .into(),
                path: None,
                diagnostics_path: None,
            });
        }
        let file_path = session
            .current_file_path
            .clone()
            .ok_or(CommandError::NoCharacterLoaded)?
            .to_string_lossy()
            .to_string();
        (file_path, session.selected_player_index)
    };
    super::session::load_character(state, app, file_path, Some(player_index)).await
}
