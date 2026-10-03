use crate::character::SkillSummaryEntry;
use crate::character::types::SkillId;
use crate::commands::{CommandError, CommandResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillChangeResult {
    pub skill_id: SkillId,
    pub old_ranks: i32,
    pub new_ranks: i32,
    pub points_spent: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsStateResponse {
    pub class_skills: Vec<SkillSummaryEntry>,
    pub cross_class_skills: Vec<SkillSummaryEntry>,
    pub total_available: i32,
    pub spent_points: i32,
    /// Class new ranks are bought as (the class taken at the last level).
    pub levelling_class_id: Option<i32>,
    pub levelling_class_name: Option<String>,
}

#[tauri::command]
pub async fn get_skills_state(state: State<'_, AppState>) -> CommandResult<SkillsStateResponse> {
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let decoder = &session.item_property_decoder;

    let all_skills = character.get_skill_summary(&game_data, Some(decoder));
    let (class_skills, cross_class_skills): (Vec<_>, Vec<_>) =
        all_skills.into_iter().partition(|s| s.is_class_skill);

    let available_points = character.get_available_skill_points();
    let total_spent = character.calculate_total_spent_with_costs(&game_data);
    let levelling_class = character.levelling_class();

    Ok(SkillsStateResponse {
        class_skills,
        cross_class_skills,
        total_available: available_points + total_spent,
        spent_points: total_spent,
        levelling_class_id: levelling_class.map(|class_id| class_id.0),
        levelling_class_name: levelling_class
            .map(|class_id| character.get_class_name(class_id, &game_data)),
    })
}

#[tauri::command]
pub async fn set_skill_rank(
    state: State<'_, AppState>,
    skill_id: i32,
    ranks: i32,
) -> CommandResult<SkillChangeResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();

    let (old_ranks, skill_label) = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let old_ranks = character.skill_rank(SkillId(skill_id));
        let skill_label = character.get_skill_name(SkillId(skill_id), &game_data);
        (old_ranks, skill_label)
    };

    session.record_edit(format!("Set {skill_label} to rank {ranks}"), |character| {
        let points_spent = character.change_skill_rank(SkillId(skill_id), ranks, &game_data)?;
        Ok(SkillChangeResult {
            skill_id: SkillId(skill_id),
            old_ranks,
            new_ranks: ranks,
            points_spent,
        })
    })
}

#[tauri::command]
pub async fn reset_all_skills(state: State<'_, AppState>) -> CommandResult<i32> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_edit("Reset all skills", |character| {
        Ok(character.reset_all_skills(&game_data))
    })
}
