use crate::character::feats::{AbilityChange, AutoAddedFeat};
use crate::character::{Character, FeatId, FeatInfo, FeatListContext};
use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::state::{AppState, SessionState};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FeatActionResult {
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
pub async fn get_feat_info(state: State<'_, AppState>, feat_id: i32) -> CommandResult<FeatInfo> {
    let game_data = state.game_data.read();
    let session = state.session.read();
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
pub async fn add_feat(state: State<'_, AppState>, feat_id: i32) -> CommandResult<FeatActionResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let feat_label = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?
        .get_feat_name(FeatId(feat_id), &game_data);

    session.record_edit(format!("Add feat: {feat_label}"), |character| {
        // Background feats only take effect when CharBackground (Dword on the character root)
        // points at the matching backgrounds.2da row; adding the feat alone is not enough.
        if let Some(background_id) = character.find_background_for_feat(FeatId(feat_id), &game_data)
        {
            let added = character.add_background(background_id, &game_data)?;
            return Ok(group_add_result(character, &game_data, feat_id, &added));
        }

        // Domain feats reach the game engine via Domain1/Domain2 class fields, not the feat list;
        // routing through add_domain keeps both in sync.
        if let Some(domain_id) = character.find_domain_for_feat(FeatId(feat_id), &game_data) {
            let added = character.add_domain(domain_id, &game_data)?;
            return Ok(group_add_result(character, &game_data, feat_id, &added));
        }

        let add_result = character.add_feat_with_prerequisites(
            FeatId(feat_id),
            crate::character::feats::FeatSource::Manual,
            &game_data,
        )?;
        Ok(FeatActionResult {
            auto_added_feats: add_result.auto_added_feats,
            auto_modified_abilities: add_result.auto_modified_abilities,
        })
    })
}

#[tauri::command]
pub async fn remove_feat(state: State<'_, AppState>, feat_id: i32) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let feat_label = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?
        .get_feat_name(FeatId(feat_id), &game_data);

    session.record_edit(format!("Remove feat: {feat_label}"), |character| {
        // Background feats only take effect when CharBackground points at the matching row;
        // clearing that field is required for the game to treat the character as having no background.
        if let Some(background_id) = character.find_background_for_feat(FeatId(feat_id), &game_data)
            && character.get_character_background() == Some(background_id)
        {
            character.remove_background(&game_data)?;
            return Ok(());
        }

        // Domain feats reach the game engine via Domain1/Domain2 class fields, not the feat list;
        // routing through remove_domain keeps both in sync.
        if let Some(domain_id) = character.find_domain_for_feat(FeatId(feat_id), &game_data) {
            character.remove_domain(domain_id, &game_data)?;
            return Ok(());
        }

        character.remove_feat(FeatId(feat_id))?;
        Ok(())
    })
}

/// Resolves the names of feats auto-added by a background or domain add, excluding the triggering feat.
fn group_add_result(
    character: &Character,
    game_data: &GameData,
    feat_id: i32,
    added_feats: &[FeatId],
) -> FeatActionResult {
    FeatActionResult {
        auto_added_feats: added_feats
            .iter()
            .filter(|f| f.0 != feat_id)
            .map(|f| AutoAddedFeat {
                feat_id: *f,
                label: character.get_feat_name(*f, game_data),
            })
            .collect(),
        auto_modified_abilities: vec![],
    }
}

pub(super) fn build_feat_list(
    character: &Character,
    game_data: &GameData,
) -> Result<Vec<FeatInfo>, CommandError> {
    let feats_table = game_data.get_table("feat").ok_or(CommandError::NotFound {
        item: "Feat table".to_string(),
    })?;

    let ctx = FeatListContext::new(character, game_data);
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

        if let Some(feat_info) = character.get_feat_info_display(feat_id, game_data, &ctx) {
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
pub async fn get_filtered_feats(
    state: State<'_, AppState>,
    page: i32,
    limit: i32,
    feat_type: Option<i32>,
    search: Option<String>,
) -> CommandResult<FilteredFeatsResponse> {
    let game_data = state.game_data.read();
    {
        let session = state.session.read();
        if let Some(cached) = &session.feat_cache {
            return Ok(filter_feats(cached, page, limit, feat_type, search));
        }
    }
    let mut session = state.session.write();
    ensure_feat_cache(&mut session, &game_data)?;
    let cached = session.feat_cache.as_ref().unwrap();
    Ok(filter_feats(cached, page, limit, feat_type, search))
}

fn filter_feats(
    cached: &[FeatInfo],
    page: i32,
    limit: i32,
    feat_type: Option<i32>,
    search: Option<String>,
) -> FilteredFeatsResponse {
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

    FilteredFeatsResponse {
        feats: paged_feats,
        total,
        page,
        limit,
        pages,
        has_next: page < pages,
        has_previous: page > 1,
    }
}
