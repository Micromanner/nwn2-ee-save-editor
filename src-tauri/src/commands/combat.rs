use crate::character::{
    ArmorClass, AttackBonuses, CombatSummary, DamageReduction, Initiative, InitiativeChange,
    NaturalArmorChange, SaveChange, SaveCheck, SaveSummary, SaveType, SavingThrows,
};
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_combat_summary(state: State<'_, AppState>) -> CommandResult<CombatSummary> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_combat_summary(&game_data, decoder))
}

#[tauri::command]
pub async fn calculate_base_attack_bonus(state: State<'_, AppState>) -> CommandResult<i32> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.calculate_bab(&game_data))
}

#[tauri::command]
pub async fn get_attack_sequence(state: State<'_, AppState>) -> CommandResult<Vec<i32>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_attack_sequence(&game_data))
}

#[tauri::command]
pub async fn get_damage_reduction(
    state: State<'_, AppState>,
) -> CommandResult<Vec<DamageReduction>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_damage_reduction_list(&game_data))
}

#[tauri::command]
pub async fn update_natural_armor(
    state: State<'_, AppState>,
    value: i32,
) -> CommandResult<NaturalArmorChange> {
    let mut session = state.session.write();
    session.record_history(
        format!("Set natural armor to {value}"),
        Some("combat:natural_armor"),
    );
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let old_value = character.natural_ac();
    character.set_natural_ac(value)?;

    Ok(NaturalArmorChange {
        old_value,
        new_value: value,
    })
}

#[tauri::command]
pub async fn update_initiative_bonus(
    state: State<'_, AppState>,
    value: i32,
) -> CommandResult<InitiativeChange> {
    let mut session = state.session.write();
    session.record_history(
        format!("Set initiative bonus to {value}"),
        Some("combat:initiative"),
    );
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let old_value = character.get_i32("initbonus").unwrap_or(0);
    character.set_i32("initbonus", value);

    Ok(InitiativeChange {
        old_value,
        new_value: value,
    })
}

#[tauri::command]
pub async fn get_save_summary(state: State<'_, AppState>) -> CommandResult<SaveSummary> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
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
    session.record_history(
        format!("Set misc save bonus to {value}"),
        Some("combat:misc_save"),
    );
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

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
}

#[tauri::command]
pub async fn check_save(
    state: State<'_, AppState>,
    save_type: i32,
    dc: i32,
    modifier: i32,
    take_20: bool,
) -> CommandResult<SaveCheck> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

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

    let base_bonus = match save_enum {
        SaveType::Fortitude => character.base_fortitude(),
        SaveType::Reflex => character.base_reflex(),
        SaveType::Will => character.base_will(),
    };

    let total_bonus = base_bonus + modifier;
    Ok(SaveCheck::evaluate(save_enum, total_bonus, dc, take_20))
}

#[tauri::command]
pub async fn get_armor_class(state: State<'_, AppState>) -> CommandResult<ArmorClass> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_armor_class(&game_data, decoder))
}

#[tauri::command]
pub async fn get_attack_bonuses(state: State<'_, AppState>) -> CommandResult<AttackBonuses> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_attack_bonuses(&game_data, decoder))
}

#[tauri::command]
pub async fn get_initiative(state: State<'_, AppState>) -> CommandResult<Initiative> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_initiative_breakdown(&game_data, decoder))
}

#[tauri::command]
pub async fn get_attacks_per_round(state: State<'_, AppState>) -> CommandResult<i32> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let attack_sequence = character.get_attack_sequence(&game_data);
    Ok(attack_sequence.len() as i32)
}

#[tauri::command]
pub async fn get_saving_throws(state: State<'_, AppState>) -> CommandResult<SavingThrows> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_saving_throws(&game_data, decoder))
}

#[tauri::command]
pub async fn get_save_breakdown(
    state: State<'_, AppState>,
    save_type: i32,
) -> CommandResult<crate::character::SaveBreakdown> {
    super::inventory::ensure_decoder_initialized(&state).await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;

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

    Ok(character.get_save_breakdown(&game_data, decoder, save_enum))
}
