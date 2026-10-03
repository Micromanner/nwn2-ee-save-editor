use crate::character::{CombatSummary, NaturalArmorChange, SaveChange, SaveSummary, SaveType};
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_combat_summary(state: State<'_, AppState>) -> CommandResult<CombatSummary> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_combat_summary(&game_data, decoder))
}

#[tauri::command]
pub async fn update_natural_armor(
    state: State<'_, AppState>,
    value: i32,
) -> CommandResult<NaturalArmorChange> {
    let mut session = state.session.write();
    session.record_edit(format!("Set natural armor to {value}"), |character| {
        let old_value = character.natural_ac();
        character.set_natural_ac(value)?;
        Ok(NaturalArmorChange {
            old_value,
            new_value: value,
        })
    })
}

#[tauri::command]
pub async fn get_save_summary(state: State<'_, AppState>) -> CommandResult<SaveSummary> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_save_summary(&game_data, decoder))
}

#[tauri::command]
pub async fn set_misc_save_bonus(
    state: State<'_, AppState>,
    save_type: i32,
    value: i32,
) -> CommandResult<SaveChange> {
    let save_enum = match save_type {
        1 => SaveType::Fortitude,
        2 => SaveType::Reflex,
        3 => SaveType::Will,
        _ => {
            return Err(CommandError::InvalidValue {
                field: "save_type".to_string(),
                expected: "1 (Fortitude), 2 (Reflex), or 3 (Will)".to_string(),
                actual: save_type.to_string(),
            });
        }
    };

    let mut session = state.session.write();
    session.record_edit(format!("Set misc save bonus to {value}"), |character| {
        let old_misc = match save_enum {
            SaveType::Fortitude => character.base_fortitude(),
            SaveType::Reflex => character.base_reflex(),
            SaveType::Will => character.base_will(),
        };

        match save_enum {
            SaveType::Fortitude => character.set_fortitude(value)?,
            SaveType::Reflex => character.set_reflex(value)?,
            SaveType::Will => character.set_will(value)?,
        }

        Ok(SaveChange {
            save_type: save_enum,
            old_misc,
            new_misc: value,
        })
    })
}
