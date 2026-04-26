use crate::character::classes::{
    AlignmentRestriction, ClassProgression, LevelUpResult, PrestigeClassOption,
    PrestigeClassValidation, PrestigeRequirements, ResolvedLevelHistoryEntry,
    get_class_progression,
};
use crate::character::{
    Character, ClassEntry, ClassId, ClassSummaryEntry, FeatId, SkillId, XpProgress,
};
use crate::commands::{CommandError, CommandResult};
use crate::services::class_categorizer::{CategorizedClasses, get_categorized_classes};
use crate::state::AppState;
use crate::utils::parsing::{row_bool, row_int, row_str};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

#[tauri::command]
pub async fn get_total_level(state: State<'_, AppState>) -> CommandResult<i32> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.total_level())
}

#[tauri::command]
pub async fn get_class_entries(state: State<'_, AppState>) -> CommandResult<Vec<ClassEntry>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.class_entries())
}

#[tauri::command]
pub async fn get_class_level(state: State<'_, AppState>, class_id: i32) -> CommandResult<i32> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.class_level(ClassId(class_id)))
}

#[tauri::command]
pub async fn get_class_summary(
    state: State<'_, AppState>,
) -> CommandResult<Vec<ClassSummaryEntry>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_class_summary(&game_data))
}

#[tauri::command]
pub async fn get_class_name(state: State<'_, AppState>, class_id: i32) -> CommandResult<String> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_class_name(ClassId(class_id), &game_data))
}

#[tauri::command]
pub async fn get_xp_progress(state: State<'_, AppState>) -> CommandResult<XpProgress> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_xp_progress(&game_data))
}

#[tauri::command]
pub async fn get_level_history(
    state: State<'_, AppState>,
) -> CommandResult<Vec<ResolvedLevelHistoryEntry>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.level_history_resolved(&game_data))
}

#[tauri::command]
pub async fn set_experience(state: State<'_, AppState>, xp: i32) -> CommandResult<()> {
    let mut session = state.session.write();
    session.record_history(format!("Set XP to {xp}"), Some("xp"));
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character
        .set_experience(xp)
        .map_err(|e| CommandError::ValidationError {
            field: "experience".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn add_class_entry(
    state: State<'_, AppState>,
    class_id: i32,
    level: i32,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let class_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_class_name(ClassId(class_id), &game_data)
    };
    session.record_history(format!("Add level: {class_label}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character
        .add_class_entry(ClassId(class_id), level)
        .map_err(|e| CommandError::ValidationError {
            field: "class_entry".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn set_class_level(
    state: State<'_, AppState>,
    class_id: i32,
    new_level: i32,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let class_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_class_name(ClassId(class_id), &game_data)
    };
    session.record_history(format!("Set {class_label} to level {new_level}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character
        .set_class_level(ClassId(class_id), new_level)
        .map_err(|e| CommandError::ValidationError {
            field: "class_level".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn remove_class_entry(state: State<'_, AppState>, class_id: i32) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let class_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_class_name(ClassId(class_id), &game_data)
    };
    session.record_history(format!("Remove class: {class_label}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character
        .remove_class(ClassId(class_id), &game_data)
        .map_err(|e| CommandError::ValidationError {
            field: "class".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn is_prestige_class(state: State<'_, AppState>, class_id: i32) -> CommandResult<bool> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.is_prestige_class(ClassId(class_id), &game_data))
}

#[tauri::command]
pub async fn check_prestige_class_requirements(
    state: State<'_, AppState>,
    class_id: i32,
) -> CommandResult<PrestigeClassValidation> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.validate_prestige_class_requirements(ClassId(class_id), &game_data))
}

#[tauri::command]
pub async fn get_available_prestige_classes(
    state: State<'_, AppState>,
) -> CommandResult<Vec<PrestigeClassOption>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_prestige_class_options(&game_data))
}

#[tauri::command]
pub async fn decode_alignment_restriction(bits: i32) -> CommandResult<Option<String>> {
    Ok(AlignmentRestriction(bits).decode_to_string())
}

#[tauri::command]
pub async fn add_class_level(
    state: State<'_, AppState>,
    class_id: i32,
    count: Option<i32>,
) -> CommandResult<Vec<LevelUpResult>> {
    let count = count.unwrap_or(1);
    if count < 1 {
        return Err(CommandError::ValidationError {
            field: "count".to_string(),
            reason: format!("count must be >= 1, got {count}"),
        });
    }

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let class_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_class_name(ClassId(class_id), &game_data)
    };
    session.record_history(format!("Add {count} {class_label} level(s)"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    character
        .transactional(|c| -> Result<_, crate::character::CharacterError> {
            let mut results = Vec::with_capacity(count as usize);
            for _ in 0..count {
                results.push(c.level_up(ClassId(class_id), &game_data)?);
            }
            Ok(results)
        })
        .map_err(|e| CommandError::ValidationError {
            field: "level_up".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn change_class(
    state: State<'_, AppState>,
    old_class_id: i32,
    new_class_id: i32,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let (old_label, new_label) = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        (
            character.get_class_name(ClassId(old_class_id), &game_data),
            character.get_class_name(ClassId(new_class_id), &game_data),
        )
    };
    session.record_history(format!("Change class: {old_label} -> {new_label}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    character
        .swap_class(ClassId(old_class_id), ClassId(new_class_id), &game_data)
        .map_err(|e| CommandError::ValidationError {
            field: "change_class".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn remove_class_levels(
    state: State<'_, AppState>,
    class_id: i32,
    count: i32,
) -> CommandResult<()> {
    if count < 1 {
        return Err(CommandError::ValidationError {
            field: "count".to_string(),
            reason: format!("count must be >= 1, got {count}"),
        });
    }

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let class_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_class_name(ClassId(class_id), &game_data)
    };
    session.record_history(format!("Remove {count} {class_label} level(s)"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    character
        .transactional(|c| -> Result<(), crate::character::CharacterError> {
            for _ in 0..count {
                c.level_down(ClassId(class_id), &game_data)?;
            }
            Ok(())
        })
        .map_err(|e| CommandError::ValidationError {
            field: "level_down".to_string(),
            reason: e.to_string(),
        })
}

#[tauri::command]
pub async fn get_class_progression_details(
    state: State<'_, AppState>,
    class_id: i32,
    max_level: Option<i32>,
) -> CommandResult<ClassProgression> {
    let _session = state.session.read();
    let game_data = state.game_data.read();
    let max = max_level.unwrap_or(20);
    get_class_progression(class_id, max, &game_data).ok_or_else(|| CommandError::NotFound {
        item: format!("Class {class_id}"),
    })
}

#[tauri::command]
pub async fn get_all_categorized_classes(
    state: State<'_, AppState>,
) -> CommandResult<CategorizedClasses> {
    let game_data = state.game_data.read();
    Ok(get_categorized_classes(&game_data))
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrerequisiteCheck {
    pub label: String,
    pub met: bool,
    pub current_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassDetailResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub hit_die: i32,
    pub skill_points: i32,
    pub bab_progression: String,
    pub is_spellcaster: bool,
    pub spell_type: Option<String>,
    pub max_level: i32,
    pub is_prestige: bool,
    pub alignment_restriction: Option<String>,
    pub prerequisites: PrestigeRequirements,
    pub prerequisite_status: Vec<PrerequisiteCheck>,
    pub class_skills: Vec<String>,
    pub save_progression: SaveProgressionType,
    pub progression: Option<ClassProgression>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SaveProgressionType {
    pub fortitude: String,
    pub reflex: String,
    pub will: String,
}

#[tauri::command]
pub async fn get_class_detail(
    state: State<'_, AppState>,
    class_id: i32,
) -> CommandResult<ClassDetailResponse> {
    let session = state.session.read();
    let game_data = state.game_data.read();

    let classes_table = game_data
        .get_table("classes")
        .ok_or(CommandError::NotFound {
            item: "classes table".to_string(),
        })?;
    let class_data = classes_table
        .get_by_id(class_id)
        .ok_or(CommandError::NotFound {
            item: format!("Class {class_id}"),
        })?;

    let name_strref = row_int(&class_data, "name", -1);
    let name = if name_strref >= 0 {
        game_data.get_string(name_strref).filter(|n| !n.is_empty())
    } else {
        None
    }
    .or_else(|| row_str(&class_data, "label"))
    .unwrap_or_else(|| format!("Class{class_id}"));

    let description_strref = row_int(&class_data, "description", -1);
    let description = if description_strref >= 0 {
        game_data
            .get_string(description_strref)
            .filter(|d| !d.is_empty())
    } else {
        None
    };

    let has_arcane = row_bool(&class_data, "hasarcane", false);
    let has_divine = row_bool(&class_data, "hasdivine", false);
    let is_prestige =
        row_bool(&class_data, "isprestige", false) || row_int(&class_data, "maxlevel", 0) > 0;
    let align_restrict = row_int(&class_data, "alignrestrict", 0);

    let requirements = Character::get_prestige_requirements(&class_data, &game_data);

    let mut prerequisite_status = Vec::new();

    if let Some(character) = session.character.as_ref() {
        if let Some(min_bab) = requirements.base_attack_bonus {
            let current_bab = character.calculate_bab(&game_data);
            prerequisite_status.push(PrerequisiteCheck {
                label: format!("Base Attack Bonus +{min_bab}"),
                met: current_bab >= min_bab,
                current_value: Some(format!("+{current_bab}")),
            });
        }

        for (skill_name, min_ranks) in &requirements.skills {
            let current_ranks = game_data
                .get_table("skills")
                .and_then(|t| {
                    for row_idx in 0..t.row_count() {
                        let Ok(row) = t.get_row(row_idx) else {
                            continue;
                        };
                        let resolved = row
                            .get("name")
                            .and_then(|v| v.as_deref())
                            .and_then(|s| s.trim().parse::<i32>().ok())
                            .and_then(|strref| game_data.get_string(strref));
                        if resolved.as_deref() == Some(skill_name.as_str()) {
                            return Some(character.skill_rank(SkillId(row_idx as i32)));
                        }
                    }
                    None
                })
                .unwrap_or(0);

            prerequisite_status.push(PrerequisiteCheck {
                label: format!("{skill_name} {min_ranks} ranks"),
                met: current_ranks >= *min_ranks,
                current_value: Some(format!("{current_ranks} ranks")),
            });
        }

        for feat_name in &requirements.feats {
            let has_feat = game_data
                .get_table("feat")
                .map(|t| {
                    for row_idx in 0..t.row_count() {
                        let Ok(row) = t.get_row(row_idx) else {
                            continue;
                        };
                        let resolved = row
                            .get("feat")
                            .and_then(|v| v.as_deref())
                            .and_then(|s| s.trim().parse::<i32>().ok())
                            .and_then(|strref| game_data.get_string(strref));
                        if resolved.as_deref() == Some(feat_name.as_str()) {
                            return character.has_feat(FeatId(row_idx as i32));
                        }
                    }
                    false
                })
                .unwrap_or(false);

            prerequisite_status.push(PrerequisiteCheck {
                label: feat_name.clone(),
                met: has_feat,
                current_value: None,
            });
        }

        if align_restrict > 0 {
            let restriction = AlignmentRestriction(align_restrict);
            let alignment = character.alignment();
            let met = restriction.check_alignment(&alignment);
            if let Some(text) = restriction.decode_to_string() {
                prerequisite_status.push(PrerequisiteCheck {
                    label: format!("Alignment: {text}"),
                    met,
                    current_value: Some(character.alignment().alignment_string()),
                });
            }
        }
    }

    let spell_type = if has_arcane {
        Some("arcane".to_string())
    } else if has_divine {
        Some("divine".to_string())
    } else {
        None
    };

    // Class skills from cls_skill_* table
    let class_skills = row_str(&class_data, "skillstable")
        .and_then(|table_name| {
            let table = game_data.get_table(&table_name.to_lowercase())?;
            let skills_table = game_data.get_table("skills")?;
            let mut skills = Vec::new();
            for row_idx in 0..table.row_count() {
                let Ok(row) = table.get_row(row_idx) else {
                    continue;
                };
                let is_class_skill = row_int(&row, "classsflag", 0) == 1;
                if is_class_skill {
                    let skill_name = skills_table
                        .get_by_id(row_idx as i32)
                        .and_then(|skill_row| {
                            let strref = row_int(&skill_row, "name", -1);
                            if strref >= 0 {
                                game_data.get_string(strref)
                            } else {
                                row_str(&skill_row, "label")
                            }
                        })
                        .unwrap_or_else(|| format!("Skill {row_idx}"));
                    skills.push(skill_name);
                }
            }
            skills.sort();
            Some(skills)
        })
        .unwrap_or_default();

    // Save progression (high/low per save)
    let save_progression = row_str(&class_data, "savingthrowtable")
        .and_then(|table_name| {
            let table = game_data.get_table(&table_name.to_lowercase())?;
            // Check level 2 saves to determine high/low (high = 3 at level 2)
            let row = table.get_row(1).ok()?;
            let fort = row_int(&row, "fortsave", 0);
            let reflex = row_int(&row, "refsave", 0);
            let will = row_int(&row, "willsave", 0);
            Some(SaveProgressionType {
                fortitude: if fort >= 3 { "High" } else { "Low" }.to_string(),
                reflex: if reflex >= 3 { "High" } else { "Low" }.to_string(),
                will: if will >= 3 { "High" } else { "Low" }.to_string(),
            })
        })
        .unwrap_or(SaveProgressionType {
            fortitude: "Low".to_string(),
            reflex: "Low".to_string(),
            will: "Low".to_string(),
        });

    let max_level_val = row_int(&class_data, "maxlevel", 0);
    let progression_max = if max_level_val > 0 { max_level_val } else { 20 };
    let progression = get_class_progression(class_id, progression_max, &game_data);

    Ok(ClassDetailResponse {
        id: class_id,
        name,
        description,
        hit_die: row_int(&class_data, "hitdie", 8),
        skill_points: row_int(&class_data, "skillpointbase", 2),
        bab_progression: row_str(&class_data, "attackbonustable")
            .unwrap_or_else(|| "CLS_ATK_2".to_string()),
        is_spellcaster: has_arcane || has_divine,
        spell_type,
        max_level: max_level_val,
        is_prestige,
        alignment_restriction: if align_restrict > 0 {
            AlignmentRestriction(align_restrict).decode_to_string()
        } else {
            None
        },
        prerequisites: requirements,
        prerequisite_status,
        class_skills,
        save_progression,
        progression,
    })
}
