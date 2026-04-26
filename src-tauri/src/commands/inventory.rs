use crate::character::{
    AddItemResult, BaseItemId, BasicItemInfo, EncumbranceInfo, EquipResult, EquipmentSlot,
    FullInventorySummary, InventoryItem, ItemProficiencyInfo, RemoveItemResult, UnequipResult,
};
use crate::commands::{CommandError, CommandResult};
use crate::services::item_property_decoder::{
    EditorContext, ItemBonuses, PropertyMetadata, is_invalid_label, load_2da_options_from_rm,
};
use crate::state::AppState;
use crate::utils::parsing::{row_int, row_str};
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use specta::Type;
use std::collections::HashMap;
use tauri::State;

pub(super) async fn ensure_decoder_initialized(state: &AppState) {
    let needs_init = {
        let session = state.session.read();
        !session.item_property_decoder.has_lookup_tables()
    };
    if needs_init {
        let (skills, classes, feats, racial_groups) = {
            let game_data = state.game_data.read();
            (
                load_skills_from_game_data(&game_data),
                load_classes_from_game_data(&game_data),
                load_feats_from_game_data(&game_data),
                load_racial_groups_from_game_data(&game_data),
            )
        };
        let spells = {
            let rm = state.resource_manager.read().await;
            load_2da_options_from_rm(&rm, "iprp_spells")
        };
        let rm = state.resource_manager.read().await;
        let mut session = state.session.write();
        session.item_property_decoder.initialize_with_rm(&rm);
        session.item_property_decoder.set_lookup_tables(
            skills,
            classes,
            feats,
            spells,
            racial_groups,
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTemplate {
    pub resref: String,
    pub name: String,
    pub base_item: i32,
    pub category: i32,
    pub sub_category: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquippedItem {
    pub slot: EquipmentSlot,
    pub item: InventoryItem,
    pub bonuses: ItemBonuses,
}

#[tauri::command]
pub async fn get_inventory_items(state: State<'_, AppState>) -> CommandResult<Vec<BasicItemInfo>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.inventory_items())
}

#[tauri::command]
pub async fn get_equipped_items(
    state: State<'_, AppState>,
) -> CommandResult<Vec<(usize, BasicItemInfo)>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.equipped_items())
}

#[tauri::command]
pub async fn get_inventory_summary(
    state: State<'_, AppState>,
) -> CommandResult<FullInventorySummary> {
    ensure_decoder_initialized(&state).await;

    let rm = state.resource_manager.read().await;
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let summary =
        character.get_full_inventory_summary(&game_data, &session.item_property_decoder, Some(&rm));
    tracing::info!(
        "Inventory summary retrieved. Items: {}, Equipped: {}, Gold: {}",
        summary.inventory.len(),
        summary.equipped.len(),
        summary.gold
    );
    if let Some(first) = summary.inventory.first() {
        tracing::debug!(
            "First item: {}, raw data keys: {:?}",
            first.name,
            first.item.0.keys().collect::<Vec<_>>()
        );
    }
    Ok(summary)
}

#[tauri::command]
pub async fn get_gold(state: State<'_, AppState>) -> CommandResult<u32> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.gold())
}

#[tauri::command]
pub async fn set_gold(state: State<'_, AppState>, amount: u32) -> CommandResult<u32> {
    let mut session = state.session.write();
    session.record_history(format!("Set gold to {amount}"), Some("gold"));
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    character.set_gold(amount);
    Ok(character.gold())
}

#[tauri::command]
pub async fn add_gold(state: State<'_, AppState>, amount: i32) -> CommandResult<u32> {
    let mut session = state.session.write();
    session.record_history(format!("Add {amount} gold"), Some("gold"));
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let current = character.gold();
    let new_amount = if amount < 0 {
        current.saturating_sub(amount.unsigned_abs())
    } else {
        current.saturating_add(amount as u32)
    };
    character.set_gold(new_amount);
    Ok(new_amount)
}

#[tauri::command]
pub async fn get_equipment_bonuses(state: State<'_, AppState>) -> CommandResult<ItemBonuses> {
    ensure_decoder_initialized(&state).await;

    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_equipment_bonuses(&game_data, &session.item_property_decoder))
}

#[tauri::command]
pub async fn get_item_proficiency_info(
    state: State<'_, AppState>,
    base_item_id: i32,
) -> CommandResult<ItemProficiencyInfo> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_item_proficiency_info(base_item_id, &game_data))
}

#[tauri::command]
pub async fn calculate_total_weight(state: State<'_, AppState>) -> CommandResult<f32> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.calculate_total_weight(&game_data))
}

#[tauri::command]
pub async fn equip_item(
    state: State<'_, AppState>,
    inventory_index: usize,
    slot: EquipmentSlot,
) -> CommandResult<EquipResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_history(format!("Equip item to {slot:?}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.equip_item(inventory_index, slot, &game_data)?)
}

#[tauri::command]
pub async fn unequip_item(
    state: State<'_, AppState>,
    slot: EquipmentSlot,
) -> CommandResult<UnequipResult> {
    let mut session = state.session.write();
    session.record_history(format!("Unequip {slot:?}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.unequip_item(slot)?)
}

#[tauri::command]
pub async fn add_to_inventory(
    state: State<'_, AppState>,
    base_item_id: i32,
    stack_size: i32,
    icon_template_resref: Option<String>,
) -> CommandResult<AddItemResult> {
    // Base-type creation produces a minimal item; without a real Icon the engine
    // renders a red question mark or "MISSING 2D TEXTURE" in-game, so we borrow
    // one from a .uti template matching this base item.
    let icon_id: Option<u32> = if let Some(resref) = icon_template_resref {
        let rm = state.resource_manager.read().await;
        rm.get_item_template(&resref)
            .and_then(|info| rm.get_item_template_fields(&info).ok())
            .and_then(|fields| {
                fields
                    .get("Icon")
                    .and_then(crate::character::gff_helpers::gff_value_to_u32)
            })
    } else {
        None
    };

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let item_label = {
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character.get_base_item_name(base_item_id, &game_data)
    };
    session.record_history(format!("Add {item_label} (x{stack_size})"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.add_item(base_item_id, stack_size, icon_id, &game_data)?)
}

#[tauri::command]
pub async fn remove_from_inventory(
    state: State<'_, AppState>,
    index: usize,
) -> CommandResult<RemoveItemResult> {
    let mut session = state.session.write();
    session.record_history(format!("Remove item from slot {index}"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.remove_item(index)?)
}

#[tauri::command]
pub async fn calculate_encumbrance(state: State<'_, AppState>) -> CommandResult<EncumbranceInfo> {
    let session = state.session.read();
    let game_data = state.game_data.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(character.get_encumbrance_info(&game_data))
}

#[tauri::command]
pub async fn get_equipped_item(
    state: State<'_, AppState>,
    slot: EquipmentSlot,
) -> CommandResult<Option<EquippedItem>> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    if let Some(basic_info) = character.get_equipped_item_by_slot(slot) {
        Ok(Some(EquippedItem {
            slot,
            item: InventoryItem {
                index: 0,
                base_item_id: BaseItemId(basic_info.base_item),
                name: basic_info.tag.clone(),
                tag: basic_info.tag,
                stack_size: basic_info.stack_size,
                identified: basic_info.identified,
            },
            bonuses: ItemBonuses::default(),
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_available_templates(
    state: State<'_, AppState>,
) -> CommandResult<Vec<IndexedTemplate>> {
    use std::path::PathBuf;
    use std::time::Instant;

    let total_start = Instant::now();

    let (templates, category_map, sub_category_map) = {
        let resource_manager = state.resource_manager.read().await;
        let game_data = state.game_data.read();
        let templates = resource_manager.get_all_item_templates();
        tracing::info!(
            "get_all_item_templates: {:?}, count: {}",
            total_start.elapsed(),
            templates.len()
        );

        let mut cat_map: HashMap<i32, i32> = HashMap::new();
        let mut sub_map: HashMap<i32, String> = HashMap::new();
        if let Some(t) = game_data.get_table("baseitems") {
            for i in 0..t.row_count() {
                if let Ok(row) = t.get_row(i) {
                    let store_panel = row_int(&row, "storepanel", 4);
                    let label = row_str(&row, "label").unwrap_or_default();
                    let item_class = row_str(&row, "itemclass");
                    cat_map.insert(i as i32, store_panel);
                    sub_map.insert(
                        i as i32,
                        super::gamedata::compute_sub_category(
                            store_panel,
                            &label,
                            item_class.as_deref(),
                        ),
                    );
                }
            }
        }

        (templates, cat_map, sub_map)
    };

    let tag_regex = Regex::new(r"<[^>]+>").unwrap();

    let zip_read_start = Instant::now();

    let mut grouped: HashMap<PathBuf, Vec<(String, String, String)>> = HashMap::new();
    for (resref, info) in &templates {
        if let crate::services::resource_manager::ContainerType::Zip = &info.container_type
            && let Some(internal_path) = &info.internal_path
        {
            let source = format!("{:?}", info.source);
            grouped
                .entry(info.container_path.clone())
                .or_default()
                .push((resref.clone(), internal_path.clone(), source));
        }
    }

    use std::io::Cursor;
    use std::sync::Arc;

    let mut zip_buffers: HashMap<PathBuf, Arc<Vec<u8>>> = HashMap::new();
    for zip_path in grouped.keys() {
        if let Ok(data) = std::fs::read(zip_path) {
            zip_buffers.insert(zip_path.clone(), Arc::new(data));
        }
    }
    tracing::info!(
        "ZIP preload: {:?}, {} ZIPs, {:.2} MB",
        zip_read_start.elapsed(),
        zip_buffers.len(),
        zip_buffers.values().map(|v| v.len()).sum::<usize>() as f64 / 1_048_576.0
    );

    let offset_start = Instant::now();

    struct DecompJob {
        zip_path: PathBuf,
        resref: String,
        source: String,
        data_start: u64,
        compressed_size: u64,
        uncompressed_size: usize,
        is_stored: bool,
    }

    let mut jobs: Vec<DecompJob> = Vec::with_capacity(templates.len());

    for (zip_path, files) in &grouped {
        let Some(zip_data) = zip_buffers.get(zip_path) else {
            continue;
        };
        let reader = Cursor::new(zip_data.as_ref().as_slice());
        let Ok(mut archive) = zip::ZipArchive::new(reader) else {
            continue;
        };

        for (resref, internal_path, source) in files {
            if let Ok(entry) = archive.by_name(internal_path) {
                jobs.push(DecompJob {
                    zip_path: zip_path.clone(),
                    resref: resref.clone(),
                    source: source.clone(),
                    data_start: entry.data_start(),
                    compressed_size: entry.compressed_size(),
                    uncompressed_size: entry.size() as usize,
                    is_stored: entry.compression() == zip::CompressionMethod::Stored,
                });
            }
        }
    }

    tracing::info!(
        "Offset map: {:?}, {} jobs",
        offset_start.elapsed(),
        jobs.len()
    );

    let decomp_start = Instant::now();

    let raw_data: Vec<(String, Vec<u8>, String)> = jobs
        .par_iter()
        .filter_map(|job| {
            let zip_data = zip_buffers.get(&job.zip_path)?;
            let start = job.data_start as usize;
            let end = start + job.compressed_size as usize;

            if end > zip_data.len() {
                return None;
            }

            let compressed = &zip_data[start..end];

            if job.is_stored {
                return Some((job.resref.clone(), compressed.to_vec(), job.source.clone()));
            }

            let mut output = vec![0u8; job.uncompressed_size];
            let mut decompressor = libdeflater::Decompressor::new();

            match decompressor.deflate_decompress(compressed, &mut output) {
                Ok(_) => Some((job.resref.clone(), output, job.source.clone())),
                Err(_) => None,
            }
        })
        .collect();

    tracing::info!(
        "Decompress phase (libdeflater): {:?}, files: {}",
        decomp_start.elapsed(),
        raw_data.len()
    );

    let parse_start = Instant::now();

    struct ParsedItem {
        resref: String,
        name: Option<String>,
        string_ref: Option<i32>,
        base_item: i32,
        category: i32,
        sub_category: String,
        source: String,
    }

    let parsed_items: Vec<ParsedItem> = raw_data
        .par_iter()
        .filter_map(|(resref, data, source)| {
            let gff = crate::parsers::gff::GffParser::from_bytes(data.clone()).ok()?;

            let base_item = gff
                .read_field_by_label(0, "BaseItem")
                .ok()
                .and_then(|v| match v {
                    crate::parsers::gff::GffValue::Int(i) => Some(i),
                    crate::parsers::gff::GffValue::Short(s) => Some(i32::from(s)),
                    crate::parsers::gff::GffValue::Byte(b) => Some(i32::from(b)),
                    crate::parsers::gff::GffValue::Word(w) => Some(i32::from(w)),
                    crate::parsers::gff::GffValue::Dword(d) => Some(d as i32),
                    _ => None,
                })
                .unwrap_or(0);

            let (name, string_ref) = gff
                .read_field_by_label(0, "LocalizedName")
                .ok()
                .map(|v| {
                    if let crate::parsers::gff::GffValue::LocString(ls) = v {
                        if !ls.substrings.is_empty() {
                            (Some(ls.substrings[0].string.to_string()), None)
                        } else if ls.string_ref >= 0 {
                            (None, Some(ls.string_ref))
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    }
                })
                .unwrap_or((None, None));

            let category = category_map.get(&base_item).copied().unwrap_or(4);
            let sub_category = sub_category_map
                .get(&base_item)
                .cloned()
                .unwrap_or_else(|| "other".to_string());

            Some(ParsedItem {
                resref: resref.clone(),
                name,
                string_ref,
                base_item,
                category,
                sub_category,
                source: source.clone(),
            })
        })
        .collect();

    tracing::info!("GFF parse phase (parallel): {:?}", parse_start.elapsed());

    let tlk_start = Instant::now();
    let string_refs: Vec<i32> = parsed_items.iter().filter_map(|p| p.string_ref).collect();

    let resource_manager = state.resource_manager.read().await;
    let tlk_strings = resource_manager.get_strings_batch(&string_refs);
    drop(resource_manager);
    tracing::info!(
        "TLK batch resolve: {:?}, {} refs",
        tlk_start.elapsed(),
        string_refs.len()
    );

    let mut indexed: Vec<IndexedTemplate> = parsed_items
        .into_iter()
        .map(|p| {
            let name = p
                .name
                .or_else(|| p.string_ref.and_then(|sr| tlk_strings.get(&sr).cloned()))
                .unwrap_or_else(|| p.resref.replace(".uti", ""));
            let name = tag_regex.replace_all(&name, "").to_string();

            IndexedTemplate {
                resref: p.resref,
                name,
                base_item: p.base_item,
                category: p.category,
                sub_category: p.sub_category,
                source: p.source,
            }
        })
        .collect();

    indexed.sort_by_key(|a| a.name.to_lowercase());
    tracing::info!("TOTAL get_available_templates: {:?}", total_start.elapsed());
    Ok(indexed)
}

#[tauri::command]
pub async fn add_item_from_template(
    state: State<'_, AppState>,
    resref: String,
) -> CommandResult<AddItemResult> {
    // Load template first (async operation)
    let item_fields = {
        let rm = state.resource_manager.read().await;
        let templates = rm.get_all_item_templates();
        let template_info =
            templates
                .get(&resref)
                .cloned()
                .ok_or_else(|| CommandError::NotFound {
                    item: format!("Item template '{resref}'"),
                })?;
        rm.get_item_template_fields(&template_info)
            .map_err(|e| CommandError::Internal(format!("Failed to load item template: {e}")))?
    };

    let item_label = item_fields
        .get("LocalizedName")
        .and_then(|v| {
            if let crate::parsers::gff::GffValue::LocString(ls) = v {
                ls.substrings.first().map(|s| s.string.to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| resref.clone());

    // Now acquire sync locks and add item
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_history(format!("Add {item_label} from template"), None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    Ok(character.add_item_from_struct(item_fields, &game_data)?)
}

/// Response for the item editor metadata endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ItemEditorMetadataResponse {
    pub property_types: Vec<PropertyMetadata>,
    pub abilities: HashMap<u32, String>,
    pub skills: HashMap<u32, String>,
    pub damage_types: HashMap<u32, String>,
    pub alignment_groups: HashMap<u32, String>,
    pub alignments: HashMap<u32, String>,
    pub racial_groups: HashMap<u32, String>,
    pub saving_throws: HashMap<u32, String>,
    pub save_elements: HashMap<u32, String>,
    pub immunity_types: HashMap<u32, String>,
    pub classes: HashMap<u32, String>,
    pub spells: HashMap<u32, String>,
    pub light: HashMap<u32, String>,
}

/// Load skills from game data skills.2da table.
fn load_skills_from_game_data(game_data: &crate::loaders::GameData) -> HashMap<u32, String> {
    let mut skills = HashMap::new();
    let Some(skill_table) = game_data.get_table("skills") else {
        return skills;
    };

    for row_idx in 0..skill_table.row_count() {
        let Ok(row) = skill_table.get_row(row_idx) else {
            continue;
        };

        let name: Option<String> = row
            .get("name")
            .and_then(|v| v.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .and_then(|str_ref| game_data.get_string(str_ref));

        let label = name.or_else(|| {
            row.get("label")
                .and_then(|v| v.clone())
                .filter(|s| !s.is_empty() && !s.starts_with("DEL_"))
        });

        if let Some(skill_name) = label.filter(|s| !is_invalid_label(s)) {
            skills.insert(row_idx as u32, skill_name);
        }
    }
    skills
}

/// Load classes from game data classes.2da table.
fn load_classes_from_game_data(game_data: &crate::loaders::GameData) -> HashMap<u32, String> {
    let mut classes = HashMap::new();
    let Some(class_table) = game_data.get_table("classes") else {
        return classes;
    };

    for row_idx in 0..class_table.row_count() {
        let Ok(row) = class_table.get_row(row_idx) else {
            continue;
        };

        let name: Option<String> = row
            .get("name")
            .and_then(|v| v.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .and_then(|str_ref| game_data.get_string(str_ref));

        let label = name.or_else(|| {
            row.get("label")
                .and_then(|v| v.clone())
                .filter(|s| !s.is_empty() && !s.starts_with("DEL_"))
        });

        if let Some(class_name) = label.filter(|s| !is_invalid_label(s)) {
            classes.insert(row_idx as u32, class_name);
        }
    }
    classes
}

/// Load racial types from game data racialtypes.2da table.
fn load_racial_groups_from_game_data(game_data: &crate::loaders::GameData) -> HashMap<u32, String> {
    let mut races = HashMap::new();
    let Some(race_table) = game_data.get_table("racialtypes") else {
        return races;
    };

    for row_idx in 0..race_table.row_count() {
        let Ok(row) = race_table.get_row(row_idx) else {
            continue;
        };

        let name: Option<String> = row
            .get("name")
            .and_then(|v| v.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .and_then(|str_ref| game_data.get_string(str_ref));

        let label = name.or_else(|| {
            row.get("label")
                .and_then(|v| v.clone())
                .filter(|s| !s.is_empty() && !s.starts_with("DEL_"))
        });

        if let Some(race_name) = label.filter(|s| !is_invalid_label(s)) {
            races.insert(row_idx as u32, race_name);
        }
    }
    races
}

/// Load feats from game data feat.2da table.
fn load_feats_from_game_data(game_data: &crate::loaders::GameData) -> HashMap<u32, String> {
    let mut feats = HashMap::new();
    let Some(feat_table) = game_data.get_table("feat") else {
        return feats;
    };

    for row_idx in 0..feat_table.row_count() {
        let Ok(row) = feat_table.get_row(row_idx) else {
            continue;
        };

        let name: Option<String> = row
            .get("feat")
            .and_then(|v| v.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .and_then(|str_ref| game_data.get_string(str_ref));

        let label = name.or_else(|| {
            row.get("label")
                .and_then(|v| v.clone())
                .filter(|s| !s.is_empty() && !s.starts_with("DEL_"))
        });

        if let Some(feat_name) = label.filter(|s| !is_invalid_label(s)) {
            feats.insert(row_idx as u32, feat_name);
        }
    }
    feats
}

/// Get editor metadata for the item property editor.
#[tauri::command]
pub fn get_editor_metadata(
    state: State<'_, AppState>,
) -> CommandResult<ItemEditorMetadataResponse> {
    let rm = state.resource_manager.blocking_read();

    let abilities = load_2da_options_from_rm(&rm, "iprp_abilities");
    let saving_throws = load_2da_options_from_rm(&rm, "iprp_savingthrow");
    let damage_types = load_2da_options_from_rm(&rm, "iprp_damagetype");
    let immunity_types = load_2da_options_from_rm(&rm, "iprp_immunity");
    let save_elements = load_2da_options_from_rm(&rm, "iprp_saveelement");
    let alignment_groups = load_2da_options_from_rm(&rm, "iprp_aligngrp");
    let alignments = load_2da_options_from_rm(&rm, "iprp_alignment");
    let light = load_2da_options_from_rm(&rm, "iprp_lightcost");

    let (skills, classes, racial_groups, feats) = {
        let game_data = state.game_data.read();
        (
            load_skills_from_game_data(&game_data),
            load_classes_from_game_data(&game_data),
            load_racial_groups_from_game_data(&game_data),
            load_feats_from_game_data(&game_data),
        )
    };

    let spells = load_2da_options_from_rm(&rm, "iprp_spells");

    {
        let mut session = state.session.write();
        session.item_property_decoder.initialize_with_rm(&rm);
        session.item_property_decoder.set_lookup_tables(
            skills.clone(),
            classes.clone(),
            feats.clone(),
            spells.clone(),
            racial_groups.clone(),
        );
    }

    let context = EditorContext {
        abilities: abilities.clone(),
        skills: skills.clone(),
        spells: spells.clone(),
        damage_types: damage_types.clone(),
        immunity_types: immunity_types.clone(),
        saving_throws: saving_throws.clone(),
        save_elements: save_elements.clone(),
        classes: classes.clone(),
        racial_groups: racial_groups.clone(),
        alignment_groups: alignment_groups.clone(),
        alignments: alignments.clone(),
        light: light.clone(),
        feats: feats.clone(),
    };

    let property_types = {
        let session = state.session.read();
        session
            .item_property_decoder
            .get_editor_property_metadata_with_rm(&context, &rm)
    };

    Ok(ItemEditorMetadataResponse {
        property_types,
        abilities,
        skills,
        damage_types,
        alignment_groups,
        alignments,
        racial_groups,
        saving_throws,
        save_elements,
        immunity_types,
        classes,
        spells,
        light,
    })
}

fn parse_equipment_slot(s: &str) -> Option<EquipmentSlot> {
    match s {
        "Head" | "head" => Some(EquipmentSlot::Head),
        "Chest" | "chest" => Some(EquipmentSlot::Chest),
        "Boots" | "boots" => Some(EquipmentSlot::Boots),
        "Gloves" | "gloves" => Some(EquipmentSlot::Gloves),
        "RightHand" | "right_hand" => Some(EquipmentSlot::RightHand),
        "LeftHand" | "left_hand" => Some(EquipmentSlot::LeftHand),
        "Cloak" | "cloak" => Some(EquipmentSlot::Cloak),
        "LeftRing" | "left_ring" => Some(EquipmentSlot::LeftRing),
        "RightRing" | "right_ring" => Some(EquipmentSlot::RightRing),
        "Neck" | "neck" => Some(EquipmentSlot::Neck),
        "Belt" | "belt" => Some(EquipmentSlot::Belt),
        "Arrows" | "arrows" => Some(EquipmentSlot::Arrows),
        "Bullets" | "bullets" => Some(EquipmentSlot::Bullets),
        "Bolts" | "bolts" => Some(EquipmentSlot::Bolts),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemRequest {
    pub item_index: Option<usize>,
    pub slot: Option<String>,
    pub item_data: HashMap<String, JsonValue>,
    /// Optional typed appearance patch. When present, the backend writes Variation,
    /// ModelPart1/2/3, and Tintable via the typed GFF builders so byte-typed fields
    /// survive the update even if the original item never had a Tintable struct.
    #[serde(default)]
    pub appearance: Option<crate::character::ItemAppearance>,
}

#[derive(Debug, Serialize, Type)]
pub struct UpdateItemResponse {
    pub success: bool,
    pub message: String,
    pub has_unsaved_changes: bool,
}

#[tauri::command]
pub async fn update_item(
    state: State<'_, AppState>,
    request: UpdateItemRequest,
) -> CommandResult<UpdateItemResponse> {
    let mut session = state.session.write();
    let label = match (&request.slot, request.item_index) {
        (Some(slot), _) => format!("Update item at slot {slot}"),
        (None, Some(idx)) => format!("Update item at slot {idx}"),
        _ => "Update item".to_string(),
    };
    session.record_history(label, None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    if let Some(slot_str) = &request.slot {
        let slot = parse_equipment_slot(slot_str).ok_or_else(|| CommandError::InvalidValue {
            field: "slot".to_string(),
            expected: "valid equipment slot".to_string(),
            actual: slot_str.clone(),
        })?;
        character.update_equipped_item(slot, &request.item_data)?;
        if let Some(appearance) = &request.appearance {
            let game_data = state.game_data.read();
            character.apply_equipped_item_appearance(slot, appearance, &game_data)?;
        }
        Ok(UpdateItemResponse {
            success: true,
            message: format!("Updated equipped item in {}", slot.display_name()),
            has_unsaved_changes: true,
        })
    } else if let Some(index) = request.item_index {
        character.update_inventory_item(index, &request.item_data)?;
        if let Some(appearance) = &request.appearance {
            let game_data = state.game_data.read();
            character.apply_inventory_item_appearance(index, appearance, &game_data)?;
        }
        Ok(UpdateItemResponse {
            success: true,
            message: "Updated inventory item".to_string(),
            has_unsaved_changes: true,
        })
    } else {
        Err(CommandError::InvalidValue {
            field: "request".to_string(),
            expected: "either slot or item_index".to_string(),
            actual: "neither provided".to_string(),
        })
    }
}
