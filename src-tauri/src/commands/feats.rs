use crate::character::feats::{AbilityChange, AutoAddedFeat, build_domain_feat_sets};
use crate::character::{
    Character, DomainId, DomainInfo, FeatAvailability, FeatEntry, FeatId, FeatInfo, FeatSlots,
    FeatSummary, PrerequisiteResult,
};
use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::state::{AppState, SessionState};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FeatActionResult {
    pub success: bool,
    pub message: String,
    pub feat_id: i32,
    pub auto_added_feats: Vec<AutoAddedFeat>,
    pub auto_modified_abilities: Vec<AbilityChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FilteredFeatsResponse {
    pub feats: Vec<FeatInfo>,
    pub total: i32,
    pub page: i32,
    pub limit: i32,
    pub pages: i32,
    pub has_next: bool,
    pub has_previous: bool,
}

#[tauri::command]
pub async fn get_feat_list(state: State<'_, AppState>) -> CommandResult<Vec<FeatEntry>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.feat_entries())
}

#[tauri::command]
pub async fn has_feat(state: State<'_, AppState>, feat_id: i32) -> CommandResult<bool> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.has_feat(FeatId(feat_id)))
}

#[tauri::command]
pub async fn get_feat_info(state: State<'_, AppState>, feat_id: i32) -> CommandResult<FeatInfo> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character
        .get_feat_info(FeatId(feat_id), &game_data)
        .ok_or_else(|| CommandError::NotFound {
            item: format!("Feat {feat_id}"),
        })
}

#[tauri::command]
pub async fn get_feat_summary(state: State<'_, AppState>) -> CommandResult<FeatSummary> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_feat_summary(&game_data))
}

#[tauri::command]
pub async fn get_feat_slots(state: State<'_, AppState>) -> CommandResult<FeatSlots> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_feat_slots(&game_data))
}

#[tauri::command]
pub async fn validate_feat_prerequisites(
    state: State<'_, AppState>,
    feat_id: i32,
) -> CommandResult<PrerequisiteResult> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.validate_feat_prerequisites(FeatId(feat_id), &game_data))
}

#[tauri::command]
pub async fn add_feat(state: State<'_, AppState>, feat_id: i32) -> CommandResult<FeatActionResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let feat_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_feat_name(crate::character::FeatId(feat_id), &game_data)
    };
    session.record_history(format!("Add feat: {feat_label}"), None);

    // Background feats only take effect when CharBackground (Dword on the character root)
    // points at the matching backgrounds.2da row; adding the feat alone is not enough.
    let background_id = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.find_background_for_feat(FeatId(feat_id), &game_data)
    };

    if let Some(background_id) = background_id {
        let bg_result = {
            let character = session
                .character
                .as_mut()
                .ok_or(CommandError::NoCharacterLoaded)?;
            character.add_background(background_id, &game_data)
        };
        session.invalidate_feat_cache();
        return build_feat_group_add_result(&session, &game_data, feat_id, "Background", bg_result);
    }

    // Domain feats reach the game engine via Domain1/Domain2 class fields, not the feat list;
    // routing through add_domain keeps both in sync.
    let domain_id = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.find_domain_for_feat(FeatId(feat_id), &game_data)
    };

    if let Some(domain_id) = domain_id {
        let domain_result = {
            let character = session
                .character
                .as_mut()
                .ok_or(CommandError::NoCharacterLoaded)?;
            character.add_domain(domain_id, &game_data)
        };
        session.invalidate_feat_cache();
        return build_feat_group_add_result(&session, &game_data, feat_id, "Domain", domain_result);
    }

    let result = {
        let character = session
            .character
            .as_mut()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.add_feat_with_prerequisites(
            FeatId(feat_id),
            crate::character::feats::FeatSource::Manual,
            &game_data,
        )
    };
    session.invalidate_feat_cache();
    match result {
        Ok(add_result) => {
            let mut parts = Vec::new();
            if !add_result.auto_added_feats.is_empty() {
                let names: Vec<&str> = add_result
                    .auto_added_feats
                    .iter()
                    .map(|f| f.label.as_str())
                    .collect();
                parts.push(format!("feats: {}", names.join(", ")));
            }
            if !add_result.auto_modified_abilities.is_empty() {
                let changes: Vec<String> = add_result
                    .auto_modified_abilities
                    .iter()
                    .map(|a| format!("{} {} -> {}", a.ability, a.old_value, a.new_value))
                    .collect();
                parts.push(format!("abilities: {}", changes.join(", ")));
            }
            let message = if parts.is_empty() {
                "Feat added successfully".to_string()
            } else {
                format!("Feat added with prerequisites - {}", parts.join("; "))
            };
            Ok(FeatActionResult {
                success: true,
                message,
                feat_id,
                auto_added_feats: add_result.auto_added_feats,
                auto_modified_abilities: add_result.auto_modified_abilities,
            })
        }
        Err(e) => Ok(FeatActionResult {
            success: false,
            message: e.to_string(),
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        }),
    }
}

#[tauri::command]
pub async fn remove_feat(
    state: State<'_, AppState>,
    feat_id: i32,
) -> CommandResult<FeatActionResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let feat_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_feat_name(FeatId(feat_id), &game_data)
    };
    session.record_history(format!("Remove feat: {feat_label}"), None);

    // Background feats only take effect when CharBackground points at the matching row;
    // clearing that field is required for the game to treat the character as having no background.
    let background_id = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.find_background_for_feat(FeatId(feat_id), &game_data)
    };

    if let Some(background_id) = background_id {
        let current = {
            let character = session
                .character
                .as_ref()
                .ok_or(CommandError::NoCharacterLoaded)?;
            character.get_character_background()
        };

        if current == Some(background_id) {
            let bg_result = {
                let character = session
                    .character
                    .as_mut()
                    .ok_or(CommandError::NoCharacterLoaded)?;
                character.remove_background(&game_data)
            };
            session.invalidate_feat_cache();
            return Ok(build_feat_group_remove_result(
                feat_id,
                "Background",
                bg_result,
            ));
        }
        // CharBackground points elsewhere - fall through so the feat still gets dropped.
    }

    // Domain feats reach the game engine via Domain1/Domain2 class fields, not the feat list;
    // routing through remove_domain keeps both in sync and cascades to domain spells.
    let domain_id = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.find_domain_for_feat(FeatId(feat_id), &game_data)
    };

    if let Some(domain_id) = domain_id {
        let domain_result = {
            let character = session
                .character
                .as_mut()
                .ok_or(CommandError::NoCharacterLoaded)?;
            character.remove_domain(domain_id, &game_data)
        };
        session.invalidate_feat_cache();
        return Ok(build_feat_group_remove_result(
            feat_id,
            "Domain",
            domain_result,
        ));
    }

    let result = {
        let character = session
            .character
            .as_mut()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.remove_feat(FeatId(feat_id))
    };
    session.invalidate_feat_cache();
    match result {
        Ok(()) => Ok(FeatActionResult {
            success: true,
            message: "Feat removed successfully".to_string(),
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        }),
        Err(e) => Ok(FeatActionResult {
            success: false,
            message: e.to_string(),
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        }),
    }
}

#[tauri::command]
pub async fn swap_feat(
    state: State<'_, AppState>,
    old_feat_id: i32,
    new_feat_id: i32,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let (old_label, new_label) = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        (
            character.get_feat_name(FeatId(old_feat_id), &game_data),
            character.get_feat_name(FeatId(new_feat_id), &game_data),
        )
    };
    session.record_history(format!("Swap feat: {old_label} -> {new_label}"), None);
    let result = {
        let character = session
            .character
            .as_mut()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character
            .swap_feat(FeatId(old_feat_id), FeatId(new_feat_id))
            .map(|_| ())
            .map_err(|e| CommandError::OperationFailed {
                operation: format!("swap_feat({old_feat_id} -> {new_feat_id})"),
                reason: e.to_string(),
            })
    };
    session.invalidate_feat_cache();
    result
}

#[tauri::command]
pub async fn check_feat_progression(
    state: State<'_, AppState>,
    new_feat_id: i32,
) -> CommandResult<Option<i32>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let old_feat = character.check_feat_progression(FeatId(new_feat_id), &game_data);
    Ok(old_feat.map(|f| f.0))
}

#[tauri::command]
pub async fn get_character_domains(state: State<'_, AppState>) -> CommandResult<Vec<i32>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let domains = character.get_character_domains();
    Ok(domains.into_iter().map(|d| d.0).collect())
}

#[tauri::command]
pub async fn get_available_domains(state: State<'_, AppState>) -> CommandResult<Vec<DomainInfo>> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_available_domains(&game_data))
}

#[tauri::command]
pub async fn add_domain(state: State<'_, AppState>, domain_id: i32) -> CommandResult<Vec<i32>> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let domain_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_domain_name(DomainId(domain_id), &game_data)
    };
    session.record_history(format!("Add domain: {domain_label}"), None);
    let feats = {
        let character = session
            .character
            .as_mut()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character
            .add_domain(DomainId(domain_id), &game_data)
            .map_err(|e| CommandError::OperationFailed {
                operation: format!("add_domain({domain_id})"),
                reason: e.to_string(),
            })?
    };
    session.invalidate_feat_cache();
    Ok(feats.into_iter().map(|f| f.0).collect())
}

#[tauri::command]
pub async fn remove_domain(state: State<'_, AppState>, domain_id: i32) -> CommandResult<Vec<i32>> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let domain_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_domain_name(DomainId(domain_id), &game_data)
    };
    session.record_history(format!("Remove domain: {domain_label}"), None);
    let feats = {
        let character = session
            .character
            .as_mut()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character
            .remove_domain(DomainId(domain_id), &game_data)
            .map_err(|e| CommandError::OperationFailed {
                operation: format!("remove_domain({domain_id})"),
                reason: e.to_string(),
            })?
    };
    session.invalidate_feat_cache();
    Ok(feats.into_iter().map(|f| f.0).collect())
}

/// Build a `FeatActionResult` for an add-via-group operation (background or domain).
/// On success, resolves the names of any auto-added feats (excluding the triggering feat).
fn build_feat_group_add_result(
    session: &SessionState,
    game_data: &GameData,
    feat_id: i32,
    group_label: &str,
    result: Result<Vec<FeatId>, crate::character::CharacterError>,
) -> CommandResult<FeatActionResult> {
    match result {
        Ok(added_feats) => {
            let character = session
                .character
                .as_ref()
                .ok_or(CommandError::NoCharacterLoaded)?;
            let auto_added_feats: Vec<AutoAddedFeat> = added_feats
                .iter()
                .filter(|f| f.0 != feat_id)
                .map(|f| AutoAddedFeat {
                    feat_id: *f,
                    label: character.get_feat_name(*f, game_data),
                })
                .collect();
            let message = if auto_added_feats.is_empty() {
                format!("{group_label} added")
            } else {
                let names: Vec<&str> = auto_added_feats.iter().map(|f| f.label.as_str()).collect();
                format!("{group_label} added with feats: {}", names.join(", "))
            };
            Ok(FeatActionResult {
                success: true,
                message,
                feat_id,
                auto_added_feats,
                auto_modified_abilities: vec![],
            })
        }
        Err(e) => Ok(FeatActionResult {
            success: false,
            message: e.to_string(),
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        }),
    }
}

/// Build a `FeatActionResult` for a remove-via-group operation (background or domain).
fn build_feat_group_remove_result(
    feat_id: i32,
    group_label: &str,
    result: Result<Vec<FeatId>, crate::character::CharacterError>,
) -> FeatActionResult {
    match result {
        Ok(_) => FeatActionResult {
            success: true,
            message: format!("{group_label} removed"),
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        },
        Err(e) => FeatActionResult {
            success: false,
            message: e.to_string(),
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        },
    }
}

pub(super) fn build_feat_list(
    character: &Character,
    game_data: &GameData,
) -> Result<Vec<FeatInfo>, CommandError> {
    let feats_table = game_data.get_table("feat").ok_or(CommandError::NotFound {
        item: "Feat table".to_string(),
    })?;

    let (_, epithet_feats) = build_domain_feat_sets(game_data);

    let feat_entries = character.feat_entries();
    let mut feat_sources = std::collections::HashMap::with_capacity(feat_entries.len());
    let mut owned_feats = std::collections::HashSet::with_capacity(feat_entries.len());
    for entry in &feat_entries {
        feat_sources.insert(entry.feat_id, entry.source);
        owned_feats.insert(entry.feat_id);
    }

    let mut all_feats: Vec<FeatInfo> = Vec::new();
    let mut seen_names: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for i in 0..feats_table.row_count() {
        let feat_id = FeatId(i as i32);

        if let Some(feat_data) = feats_table.get_by_id(feat_id.0) {
            let removed = feat_data
                .get("removed")
                .and_then(|s| s.as_ref())
                .is_some_and(|s| s == "1");

            if removed {
                continue;
            }
        }

        if let Some(feat_info) = character.get_feat_info_display(
            feat_id,
            game_data,
            &epithet_feats,
            &feat_sources,
            &owned_feats,
        ) {
            if feat_info.label.is_empty()
                || feat_info.label.starts_with("****")
                || feat_info.label.starts_with("DEL_")
                || feat_info.label == "DELETED"
            {
                continue;
            }

            if let Some(&existing_idx) = seen_names.get(&feat_info.name) {
                if feat_info.has_feat && !all_feats[existing_idx].has_feat {
                    all_feats[existing_idx] = feat_info;
                }
                continue;
            }
            seen_names.insert(feat_info.name.clone(), all_feats.len());
            all_feats.push(feat_info);
        }
    }
    Ok(all_feats)
}

pub(super) fn ensure_feat_cache(
    session: &mut SessionState,
    game_data: &GameData,
) -> Result<(), CommandError> {
    if session.feat_cache.is_some() {
        return Ok(());
    }
    let cache = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        build_feat_list(character, game_data)?
    };
    session.feat_cache = Some(cache);
    Ok(())
}

#[tauri::command]
pub async fn get_all_feats(state: State<'_, AppState>) -> CommandResult<Vec<FeatInfo>> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    ensure_feat_cache(&mut session, &game_data)?;
    Ok(session.feat_cache.clone().unwrap())
}

#[tauri::command]
pub async fn get_filtered_feats(
    state: State<'_, AppState>,
    page: i32,
    limit: i32,
    feat_type: Option<i32>,
    search: Option<String>,
) -> CommandResult<FilteredFeatsResponse> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    ensure_feat_cache(&mut session, &game_data)?;

    let cached = session.feat_cache.as_ref().unwrap();
    let search_lower = search.as_ref().map(|s| s.to_lowercase());

    let filtered_feats: Vec<&FeatInfo> = cached
        .iter()
        .filter(|feat_info| {
            if let Some(ft) = feat_type
                && (feat_info.feat_type.0 & ft) == 0
            {
                return false;
            }
            if let Some(ref search_str) = search_lower {
                let name_lower = feat_info.name.to_lowercase();
                let label_lower = feat_info.label.to_lowercase();
                let desc_lower = feat_info.description.to_lowercase();

                if !name_lower.contains(search_str)
                    && !label_lower.contains(search_str)
                    && !desc_lower.contains(search_str)
                {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered_feats.len() as i32;
    let pages = if total == 0 {
        1
    } else {
        (total + limit - 1) / limit
    };
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(filtered_feats.len());

    let paged_feats = if start < filtered_feats.len() {
        filtered_feats[start..end]
            .iter()
            .map(|f| (*f).clone())
            .collect()
    } else {
        Vec::new()
    };

    Ok(FilteredFeatsResponse {
        feats: paged_feats,
        total,
        page,
        limit,
        pages,
        has_next: page < pages,
        has_previous: page > 1,
    })
}

#[tauri::command]
pub async fn check_feat_availability(
    state: State<'_, AppState>,
    feat_id: i32,
) -> CommandResult<FeatAvailability> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    let feats_table = game_data.get_table("feat").ok_or(CommandError::NotFound {
        item: "Feat table".to_string(),
    })?;

    let feat_data = feats_table
        .get_by_id(feat_id)
        .ok_or(CommandError::NotFound {
            item: format!("Feat {feat_id}"),
        })?;

    let label = feat_data
        .get("label")
        .and_then(|s| s.as_ref())
        .map_or_else(|| format!("feat_{feat_id}"), |s| s.clone());

    let description = feat_data
        .get("description")
        .and_then(|s| s.as_ref()?.parse::<i32>().ok())
        .and_then(|strref| game_data.get_string(strref))
        .unwrap_or_default();

    let feat_type = Character::parse_feat_type(&feat_data, &description);

    Ok(character.get_feat_availability(FeatId(feat_id), feat_type, &label, &game_data))
}
