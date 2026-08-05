use crate::character::PetsState;
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_pets(state: State<'_, AppState>) -> CommandResult<PetsState> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.pets_state(&game_data))
}

#[tauri::command]
pub async fn set_animal_companion(
    state: State<'_, AppState>,
    name: Option<String>,
    type_id: Option<i32>,
) -> CommandResult<PetsState> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_history("Set animal companion", Some("pets:animal_companion"));
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    if let Some(name) = name {
        character.set_animal_companion_name(name);
    }
    if let Some(type_id) = type_id {
        character.set_animal_companion_type(type_id);
    }

    Ok(character.pets_state(&game_data))
}

#[tauri::command]
pub async fn set_familiar(
    state: State<'_, AppState>,
    name: Option<String>,
    type_id: Option<i32>,
) -> CommandResult<PetsState> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_history("Set familiar", Some("pets:familiar"));
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    if let Some(name) = name {
        character.set_familiar_name(name);
    }
    if let Some(type_id) = type_id {
        character.set_familiar_type(type_id);
    }

    Ok(character.pets_state(&game_data))
}
