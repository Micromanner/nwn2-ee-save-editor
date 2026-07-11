use crate::character::Character;
use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::parsers::gff::GffParser;
use crate::services::load_diagnostics::{self, LoadInput, LoadReport, LoadStage, LoadStatus};
use crate::services::savegame_handler::SaveGameHandler;
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};
use tracing::{error, info, instrument, warn};

#[tauri::command]
#[instrument(name = "load_character_command", skip(state, app), fields(file_path = %file_path))]
pub async fn load_character(
    state: State<'_, AppState>,
    app: AppHandle,
    file_path: String,
    player_index: Option<usize>,
) -> CommandResult<bool> {
    info!("Load character command invoked");

    let file_size = std::fs::metadata(&file_path).ok().map(|m| m.len());
    let mut report = LoadReport::new(LoadInput {
        file_path: file_path.clone(),
        player_index,
        file_size,
    });

    let load_result = {
        let mut session = state.session.write();
        session.load_character(&file_path, player_index, &mut report)
    };

    // Snapshot capture walks override/hak/workshop directories — skip on clean
    // loads so heavy mod installs don't pay 100ms+ on every successful load.
    if report.status != LoadStatus::Ok {
        report.snapshot = Some(load_diagnostics::snapshot::capture(&state).await);
    }
    report.finalize();
    let diagnostics_path =
        load_diagnostics::writer::write(&report).map(|p| p.display().to_string());

    match load_result {
        Ok(()) => {
            info!("Character loaded successfully via command");
            tokio::spawn(async move {
                let state = app.state::<AppState>();

                // Pre-warm feat cache
                let cache = {
                    let game_data = state.game_data.read();
                    let session = state.session.read();
                    let Some(character) = session.character.as_ref() else {
                        return;
                    };
                    match super::feats::build_feat_list(character, &game_data) {
                        Ok(c) => c,
                        Err(e) => {
                            warn!("Failed to pre-warm feat cache: {e}");
                            return;
                        }
                    }
                };
                let mut session = state.session.write();
                if session.feat_cache.is_none() {
                    session.feat_cache = Some(cache);
                }
                drop(session);
            });
            Ok(true)
        }
        Err(load_err) => {
            error!("Failed to load character: {}", load_err.message);
            let cmd_err = match load_err.stage {
                LoadStage::Gff | LoadStage::Playerlist | LoadStage::Bic => {
                    CommandError::ParseError {
                        message: load_err.message,
                        context: Some(load_err.stage.to_string()),
                        diagnostics_path,
                    }
                }
                _ => CommandError::FileError {
                    message: load_err.message,
                    path: Some(file_path),
                    diagnostics_path,
                },
            };
            Err(cmd_err)
        }
    }
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct SaveCharacterClass {
    pub name: String,
    pub level: u8,
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct SaveCharacterOption {
    pub player_index: usize,
    pub name: Option<String>,
    pub race: String,
    pub total_level: i32,
    pub classes: Vec<SaveCharacterClass>,
}

fn summarize_save_character(
    player_index: usize,
    character: Character,
    game_data: &GameData,
) -> SaveCharacterOption {
    let full_name = character.full_name();
    let name = (!full_name.trim().is_empty()).then_some(full_name);

    let classes = character
        .class_entries()
        .into_iter()
        .map(|entry| SaveCharacterClass {
            name: character.get_class_name(entry.class_id, game_data),
            level: entry.level.clamp(0, i32::from(u8::MAX)) as u8,
        })
        .collect();

    SaveCharacterOption {
        player_index,
        name,
        race: character.race_name(game_data),
        total_level: character.total_level(),
        classes,
    }
}

#[tauri::command]
#[instrument(name = "list_save_characters_command", skip(state), fields(file_path = %file_path))]
pub async fn list_save_characters(
    state: State<'_, AppState>,
    file_path: String,
) -> CommandResult<Vec<SaveCharacterOption>> {
    let handler = SaveGameHandler::new(&file_path, false, false).map_err(CommandError::from)?;
    let playerlist_data = handler.extract_player_data().map_err(CommandError::from)?;
    let gff = GffParser::from_bytes(playerlist_data).map_err(|e| CommandError::ParseError {
        message: format!("Failed to parse playerlist.ifo: {e}"),
        context: Some(file_path.clone()),
        diagnostics_path: None,
    })?;

    let mut player_entries =
        crate::state::session_state::read_playerlist_entries(gff).map_err(|message| {
            CommandError::ParseError {
                message,
                context: Some(file_path.clone()),
                diagnostics_path: None,
            }
        })?;

    if let Ok(Some(player_bic_data)) = handler.extract_player_bic()
        && let Ok(primary_fields) =
            crate::state::session_state::read_player_bic_entry(player_bic_data)
        && let Some(primary_index) = crate::state::session_state::resolve_primary_player_index(
            &player_entries,
            Some(&primary_fields),
        )
        && let Some(primary_entry) = player_entries.get_mut(primary_index)
    {
        *primary_entry = primary_fields;
    }

    let game_data = state.game_data.read();
    Ok(player_entries
        .into_iter()
        .enumerate()
        .map(|(player_index, fields)| {
            summarize_save_character(player_index, Character::from_gff(fields), &game_data)
        })
        .collect())
}

#[derive(Clone, serde::Serialize, specta::Type)]
pub struct SaveCharacterResult {
    pub saved: bool,
    pub warning: Option<String>,
}

#[tauri::command]
#[instrument(name = "save_character_command", skip(state))]
pub async fn save_character(
    state: State<'_, AppState>,
    _file_path: Option<String>,
) -> CommandResult<SaveCharacterResult> {
    info!("Save character command invoked");

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    match session.save_character(&game_data) {
        Ok(warning) => {
            info!("Character saved successfully via command");
            Ok(SaveCharacterResult {
                saved: true,
                warning,
            })
        }
        Err(e) => {
            error!("Failed to save character: {}", e);
            Err(CommandError::FileError {
                message: e.clone(),
                path: session
                    .current_file_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
                diagnostics_path: None,
            })
        }
    }
}

#[tauri::command]
#[instrument(name = "close_character_command", skip(state))]
pub async fn close_character(state: State<'_, AppState>) -> CommandResult<bool> {
    info!("Close character command invoked");
    let mut session = state.session.write();
    session.close_character();
    info!("Character closed successfully");
    Ok(true)
}

#[derive(serde::Serialize, specta::Type)]
pub struct SessionInfo {
    pub character_loaded: bool,
    pub file_path: Option<String>,
    pub dirty: bool,
    pub player_index: Option<usize>,
}

#[tauri::command]
pub async fn get_session_info(state: State<'_, AppState>) -> CommandResult<SessionInfo> {
    let session = state.session.read();
    Ok(SessionInfo {
        character_loaded: session.character.is_some(),
        file_path: session
            .current_file_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        dirty: session.has_unsaved_changes(),
        player_index: session
            .character
            .as_ref()
            .map(|_| session.selected_player_index),
    })
}

#[tauri::command]
pub async fn has_unsaved_changes(state: State<'_, AppState>) -> CommandResult<bool> {
    let session = state.session.read();
    Ok(session.has_unsaved_changes())
}

#[derive(serde::Serialize, specta::Type)]
pub struct UndoResult {
    pub applied: bool,
    pub label: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
}

#[derive(serde::Serialize, specta::Type)]
pub struct HistoryState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_label: Option<String>,
    pub redo_label: Option<String>,
}

#[tauri::command]
pub async fn undo(state: State<'_, AppState>) -> CommandResult<UndoResult> {
    let mut session = state.session.write();
    let label = session.undo();
    Ok(UndoResult {
        applied: label.is_some(),
        label,
        can_undo: session.can_undo(),
        can_redo: session.can_redo(),
    })
}

#[tauri::command]
pub async fn redo(state: State<'_, AppState>) -> CommandResult<UndoResult> {
    let mut session = state.session.write();
    let label = session.redo();
    Ok(UndoResult {
        applied: label.is_some(),
        label,
        can_undo: session.can_undo(),
        can_redo: session.can_redo(),
    })
}

#[tauri::command]
pub async fn get_history_state(state: State<'_, AppState>) -> CommandResult<HistoryState> {
    let session = state.session.read();
    Ok(HistoryState {
        can_undo: session.can_undo(),
        can_redo: session.can_redo(),
        undo_label: session.undo_label().map(str::to_owned),
        redo_label: session.redo_label().map(str::to_owned),
    })
}

#[tauri::command]
#[instrument(name = "export_to_localvault_command", skip(state))]
pub async fn export_to_localvault(state: State<'_, AppState>) -> CommandResult<String> {
    info!("Export to localvault command invoked");

    let session = state.session.read();
    let paths = state.paths.read();
    match session.export_to_localvault(&paths) {
        Ok(path) => {
            info!("Character exported to vault: {}", path);
            Ok(path)
        }
        Err(e) => {
            error!("Failed to export to localvault: {}", e);
            Err(CommandError::FileError {
                message: e,
                path: None,
                diagnostics_path: None,
            })
        }
    }
}
