use crate::character::{
    AbilityIndex, AbilityScores, Alignment, Character, HitPoints, resolved_full_name,
};
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaceChangedEvent {
    pub old_race_id: Option<i32>,
    pub new_race_id: i32,
    pub old_subrace: Option<String>,
    pub new_subrace: Option<String>,
}

// Identity Commands
#[tauri::command]
pub async fn get_character_name(state: State<'_, AppState>) -> CommandResult<String> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(resolved_full_name(character, &game_data))
}

#[tauri::command]
pub async fn get_alignment(state: State<'_, AppState>) -> CommandResult<Alignment> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.alignment())
}

#[tauri::command]
pub async fn set_alignment(
    state: State<'_, AppState>,
    law_chaos: Option<i32>,
    good_evil: Option<i32>,
) -> CommandResult<Alignment> {
    let mut session = state.session.write();
    session.record_edit("Set alignment", |character| {
        character.set_alignment(law_chaos, good_evil)?;
        Ok(character.alignment())
    })
}

#[tauri::command]
pub async fn set_deity(state: State<'_, AppState>, deity: String) -> CommandResult<String> {
    let mut session = state.session.write();
    session.record_edit(format!("Set deity to {deity}"), |character| {
        character.set_deity(deity);
        Ok(character.deity())
    })
}

#[tauri::command]
pub async fn set_biography(
    state: State<'_, AppState>,
    description: String,
) -> CommandResult<String> {
    let mut session = state.session.write();
    session.record_edit("Update biography", |character| {
        character.set_description(description);
        Ok(character.description())
    })
}

// Ability Commands
#[tauri::command]
pub async fn get_base_ability_scores(state: State<'_, AppState>) -> CommandResult<AbilityScores> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.base_scores())
}

#[tauri::command]
pub async fn update_hit_points(
    state: State<'_, AppState>,
    current: Option<i32>,
    max: Option<i32>,
) -> CommandResult<HitPoints> {
    let mut session = state.session.write();
    let label = match (current, max) {
        (Some(c), Some(m)) => format!("Set HP to {c}/{m}"),
        (Some(c), None) => format!("Set HP to {c}/{c}"),
        (None, Some(m)) => format!("Set HP to {m}/{m}"),
        (None, None) => "Set HP".to_string(),
    };
    session.record_edit(label, |character| {
        if let Some(hp) = current {
            character.set_current_hp(hp);
        }
        if let Some(hp) = max {
            character.set_max_hp(hp);
        }
        Ok(character.hit_points())
    })
}

#[tauri::command]
pub async fn set_all_ability_scores(
    state: State<'_, AppState>,
    scores: AbilityScores,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let current = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let changes: Vec<(&str, AbilityIndex, i32)> = [
        ("Strength", AbilityIndex::STR, scores.str_),
        ("Dexterity", AbilityIndex::DEX, scores.dex),
        ("Constitution", AbilityIndex::CON, scores.con),
        ("Intelligence", AbilityIndex::INT, scores.int),
        ("Wisdom", AbilityIndex::WIS, scores.wis),
        ("Charisma", AbilityIndex::CHA, scores.cha),
    ]
    .into_iter()
    .filter(|(_, idx, val)| current.base_ability(*idx) != *val)
    .collect();
    let label = match changes.as_slice() {
        [] => "Update abilities".to_string(),
        [(name, _, value)] => format!("Set {name} to {value}"),
        multi => format!(
            "Set {}",
            multi
                .iter()
                .map(|(n, _, v)| format!("{n} to {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    session.record_edit(label, |character| {
        for (_, idx, value) in &changes {
            character.set_ability_with_cascades(*idx, *value, &game_data)?;
        }
        Ok(())
    })
}

// Race Commands
#[tauri::command]
pub async fn change_race(
    state: State<'_, AppState>,
    race_id: i32,
    subrace: Option<String>,
) -> CommandResult<RaceChangedEvent> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let race_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character
            .validate_race_change(race_id, subrace.as_deref(), &game_data)
            .into_result("race")?;
        let subrace_id = match subrace.as_deref().filter(|s| !s.is_empty()) {
            Some(name) => character
                .get_subrace_data(name, &game_data)
                .map(|data| data.id),
            None => Character::default_subrace_for_race(race_id, &game_data),
        };
        subrace_id
            .and_then(|id| character.get_subrace_name_by_index(id, &game_data))
            .unwrap_or_else(|| character.get_race_name_by_id(race_id, &game_data))
    };
    session.record_edit(format!("Change race to {race_label}"), |character| {
        let result = character.change_race(race_id, subrace, false, &game_data)?;
        Ok(RaceChangedEvent {
            old_race_id: Some(result.old_race_id),
            new_race_id: result.new_race_id,
            old_subrace: result.old_subrace,
            new_subrace: result.new_subrace,
        })
    })
}
