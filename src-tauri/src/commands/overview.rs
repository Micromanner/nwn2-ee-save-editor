use tauri::State;
use tracing::warn;

use crate::character::overview::CampaignOverviewInfo;
use crate::character::{AbilitiesState, ClassesState, FeatsState, OverviewState, SpellsState};
use crate::commands::campaign::cached_module_info;
use crate::commands::{CommandError, CommandResult};
use crate::services::campaign::CampaignManager;
use crate::state::AppState;

use super::types::CharacterUpdates;

#[tauri::command]
pub async fn get_overview_state(state: State<'_, AppState>) -> CommandResult<OverviewState> {
    let module_info_result = cached_module_info(&state)
        .inspect_err(|e| warn!("Module info unavailable for overview: {e}"))
        .ok();

    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;

    let mut overview = character.get_overview_state(&game_data, decoder);

    if let Some(handler) = session.savegame_handler.as_ref() {
        let mut info = CampaignOverviewInfo::default();

        if let Some((module_info, _)) = module_info_result {
            info.campaign_name = Some(module_info.campaign);
            info.module_name = Some(module_info.module_name);
            info.area_name = Some(module_info.area_name);
            info.game_year = Some(module_info.game_year);
            info.game_month = Some(module_info.game_month);
            info.game_day = Some(module_info.game_day);
            info.game_hour = Some(module_info.game_hour);
        }

        match CampaignManager::get_summary(handler) {
            Ok(summary) => {
                info.game_act = summary.general_info.get("game_act").cloned().flatten();
                info.last_saved = summary.general_info.get("last_saved").cloned().flatten();
                info.difficulty = summary.general_info.get("difficulty").cloned().flatten();
            }
            Err(e) => warn!("Campaign summary unavailable for overview: {e}"),
        }

        overview.campaign_info = Some(info);
    }

    Ok(overview)
}

#[tauri::command]
pub async fn update_character(
    state: State<'_, AppState>,
    updates: CharacterUpdates,
) -> CommandResult<OverviewState> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let changed_fields: Vec<&str> = [
        ("first_name", updates.first_name.is_some()),
        ("last_name", updates.last_name.is_some()),
        ("age", updates.age.is_some()),
        ("gender", updates.gender.is_some()),
        ("deity", updates.deity.is_some()),
        ("description", updates.description.is_some()),
        ("alignment", updates.alignment.is_some()),
        ("experience", updates.experience.is_some()),
    ]
    .into_iter()
    .filter_map(|(name, changed)| changed.then_some(name))
    .collect();
    let label = if changed_fields.is_empty() {
        "Update character".to_string()
    } else {
        format!("Update character: {}", changed_fields.join(", "))
    };
    session.record_edit(label, |character| {
        if let Some(first_name) = updates.first_name {
            character.set_first_name(first_name);
        }
        if let Some(last_name) = updates.last_name {
            character.set_last_name(last_name);
        }
        if let Some(age) = updates.age {
            character.set_age(age)?;
        }
        if let Some(gender) = updates.gender {
            character.set_gender(gender)?;
        }
        if let Some(deity) = updates.deity {
            character.set_deity(deity);
        }
        if let Some(description) = updates.description {
            character.set_description(description);
        }
        if let Some((law_chaos, good_evil)) = updates.alignment {
            character.set_alignment(Some(law_chaos), Some(good_evil))?;
        }
        if let Some(experience) = updates.experience {
            character.set_experience(experience)?;
        }
        Ok(())
    })?;

    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_overview_state(&game_data, decoder))
}

#[tauri::command]
pub async fn get_abilities_state(state: State<'_, AppState>) -> CommandResult<AbilitiesState> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;

    Ok(character.get_abilities_state(&game_data, decoder))
}

#[tauri::command]
pub async fn apply_point_buy(
    state: State<'_, AppState>,
    new_scores: crate::character::types::AbilityScores,
) -> CommandResult<AbilitiesState> {
    use crate::character::abilities::{
        POINT_BUY_BUDGET, POINT_BUY_MAX, POINT_BUY_MIN, calculate_point_buy_cost,
    };
    use crate::character::types::AbilityIndex;

    let cost = calculate_point_buy_cost(&new_scores);
    if cost > POINT_BUY_BUDGET {
        return Err(CommandError::ValidationError {
            field: "point_buy_cost".to_string(),
            reason: format!("Point buy cost {cost} exceeds budget {POINT_BUY_BUDGET}"),
        });
    }

    for score in [
        new_scores.str_,
        new_scores.dex,
        new_scores.con,
        new_scores.int,
        new_scores.wis,
        new_scores.cha,
    ] {
        if !(POINT_BUY_MIN..=POINT_BUY_MAX).contains(&score) {
            return Err(CommandError::ValidationError {
                field: "ability_score".to_string(),
                reason: format!("Scores must be between {POINT_BUY_MIN} and {POINT_BUY_MAX}"),
            });
        }
    }

    let game_data = state.game_data.read();

    {
        let mut session = state.session.write();
        session.record_edit("Apply point buy", |character| {
            character.clear_ability_level_up_history()?;

            let old_con = character.base_ability(AbilityIndex::CON);
            character.set_ability(AbilityIndex::STR, new_scores.str_)?;
            character.set_ability(AbilityIndex::DEX, new_scores.dex)?;
            character.set_ability(AbilityIndex::CON, new_scores.con)?;
            character.set_ability(AbilityIndex::INT, new_scores.int)?;
            character.set_ability(AbilityIndex::WIS, new_scores.wis)?;
            character.set_ability(AbilityIndex::CHA, new_scores.cha)?;

            if new_scores.con != old_con {
                character.recalculate_hit_points(old_con, new_scores.con);
            }
            Ok(())
        })?;
    }

    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;
    Ok(character.get_abilities_state(&game_data, decoder))
}

#[tauri::command]
pub async fn get_classes_state(state: State<'_, AppState>) -> CommandResult<ClassesState> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    Ok(character.get_classes_state(&game_data))
}

#[tauri::command]
pub async fn get_feats_state(state: State<'_, AppState>) -> CommandResult<FeatsState> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    Ok(character.get_feats_state(&game_data))
}

#[tauri::command]
pub async fn get_spells_state(state: State<'_, AppState>) -> CommandResult<SpellsState> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    Ok(character.get_spells_state(&game_data, &session.item_property_decoder))
}
