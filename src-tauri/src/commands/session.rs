use crate::character::Character;
use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::parsers::gff::GffParser;
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

    let mut session = state.session.write();
    match session.load_character(&file_path, player_index) {
        Ok(()) => {
            info!("Character loaded successfully via command");
            drop(session);
            {
                let game_data = state.game_data.read();
                let mut session = state.session.write();
                session.normalize_loaded_skill_points(&game_data);
            }
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
        Err(e) => {
            error!("Failed to load character: {}", e);
            Err(CommandError::FileError {
                message: e,
                path: Some(file_path),
            })
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
    pub name: String,
    pub race: String,
    pub total_level: i32,
    pub classes: Vec<SaveCharacterClass>,
}

fn summarize_save_character(
    player_index: usize,
    character: Character,
    game_data: &GameData,
) -> SaveCharacterOption {
    let name = {
        let full_name = character.full_name();
        if full_name.trim().is_empty() {
            format!("Player {}", player_index + 1)
        } else {
            full_name
        }
    };

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
    })?;

    let mut player_entries =
        crate::state::session_state::read_playerlist_entries(gff).map_err(|message| {
            CommandError::ParseError {
                message,
                context: Some(file_path.clone()),
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

#[tauri::command]
#[instrument(name = "save_character_command", skip(state))]
pub async fn save_character(
    state: State<'_, AppState>,
    _file_path: Option<String>,
) -> CommandResult<bool> {
    info!("Save character command invoked");

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    match session.save_character(&game_data) {
        Ok(()) => {
            info!("Character saved successfully via command");
            Ok(true)
        }
        Err(e) => {
            error!("Failed to save character: {}", e);
            Err(CommandError::FileError {
                message: e.clone(),
                path: session
                    .current_file_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string()),
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
            })
        }
    }
}
