use crate::character::{
    AddItemResult, EquipResult, EquipmentSlot, FullInventorySummary, RemoveItemResult,
    TransformResult, UnequipResult,
};
use crate::commands::{CommandError, CommandResult};
use crate::services::item_property_decoder::PropertyMetadata;
use crate::state::AppState;
use crate::utils::parsing::{row_int, row_str};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use specta::Type;
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedTemplate {
    pub resref: String,
    pub name: String,
    pub base_item: i32,
    pub category: i32,
    pub sub_category: String,
    pub source: String,
}

#[tauri::command]
pub async fn get_inventory_summary(
    state: State<'_, AppState>,
) -> CommandResult<FullInventorySummary> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let session = state.session.read();
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
pub async fn set_gold(state: State<'_, AppState>, amount: u32) -> CommandResult<u32> {
    let mut session = state.session.write();
    session.record_edit(format!("Set gold to {amount}"), |character| {
        character.set_gold(amount);
        Ok(character.gold())
    })
}

#[tauri::command]
pub async fn equip_item(
    state: State<'_, AppState>,
    inventory_index: usize,
    slot: EquipmentSlot,
) -> CommandResult<EquipResult> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_edit(format!("Equip item to {slot:?}"), |character| {
        Ok(character.equip_item(inventory_index, slot, &game_data)?)
    })
}

#[tauri::command]
pub async fn unequip_item(
    state: State<'_, AppState>,
    slot: EquipmentSlot,
) -> CommandResult<UnequipResult> {
    let mut session = state.session.write();
    session.record_edit(format!("Unequip {slot:?}"), |character| {
        Ok(character.unequip_item(slot)?)
    })
}

#[tauri::command]
pub async fn add_to_inventory(
    state: State<'_, AppState>,
    base_item_id: i32,
    stack_size: i32,
    icon_template_resref: Option<String>,
) -> CommandResult<AddItemResult> {
    // Base-type creation produces a minimal item; without a real Icon the engine
    // renders a red question mark or "MISSING 2D TEXTURE" in-game, and without
    // an ArmorVisualType a chest/boots/gloves item has no material to render,
    // so we borrow both from a .uti template matching this base item.
    let (icon_id, armor_visual_type): (Option<u32>, Option<u8>) =
        if let Some(resref) = icon_template_resref {
            let rm = state.resource_manager.read().await;
            rm.get_item_template(&resref)
                .and_then(|info| rm.get_item_template_fields(&info).ok())
                .map(|fields| {
                    (
                        fields
                            .get("Icon")
                            .and_then(crate::character::gff_helpers::gff_value_to_u32),
                        fields
                            .get("ArmorVisualType")
                            .and_then(crate::character::gff_helpers::gff_value_to_u32)
                            .and_then(|v| u8::try_from(v).ok()),
                    )
                })
                .unwrap_or((None, None))
        } else {
            (None, None)
        };

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let item_label = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?
        .get_base_item_name(base_item_id, &game_data);
    session.record_edit(format!("Add {item_label} (x{stack_size})"), |character| {
        Ok(character.add_item(
            base_item_id,
            stack_size,
            icon_id,
            armor_visual_type,
            &game_data,
        )?)
    })
}

#[tauri::command]
pub async fn remove_from_inventory(
    state: State<'_, AppState>,
    index: usize,
    container_index: Option<usize>,
) -> CommandResult<RemoveItemResult> {
    let mut session = state.session.write();
    session.record_edit(format!("Remove item from slot {index}"), |character| {
        Ok(character.remove_item(index, container_index)?)
    })
}

#[tauri::command]
pub async fn destroy_equipped_item(
    state: State<'_, AppState>,
    slot: EquipmentSlot,
) -> CommandResult<()> {
    let mut session = state.session.write();
    session.record_edit(format!("Destroy {slot:?}"), |character| {
        Ok(character.destroy_equipped_item(slot)?)
    })
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

    let zip_read_start = Instant::now();

    let mut grouped: HashMap<PathBuf, Vec<(String, String, String)>> = HashMap::new();
    let mut other_templates = Vec::new();
    for (resref, location) in &templates {
        let source = location.source.key().to_string();
        match (&location.container_type, &location.internal_path) {
            (crate::services::resource_manager::ContainerType::Zip, Some(internal_path)) => {
                grouped
                    .entry(location.container_path.clone())
                    .or_default()
                    .push((resref.clone(), internal_path.clone(), source));
            }
            _ => other_templates.push((resref.clone(), location.clone(), source)),
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
                let Some(data_start) = entry.data_start() else {
                    continue;
                };
                jobs.push(DecompJob {
                    zip_path: zip_path.clone(),
                    resref: resref.clone(),
                    source: source.clone(),
                    data_start,
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

    let mut raw_data: Vec<(String, Vec<u8>, String)> = jobs
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

    let other_read_start = Instant::now();
    let other_count = other_templates.len();
    let other_raw: Vec<(String, Vec<u8>, String)> = {
        let resource_manager = state.resource_manager.read().await;
        other_templates
            .par_iter()
            .filter_map(|(resref, location, source)| {
                let bytes = resource_manager.read_location(location).ok()?;
                Some((resref.clone(), bytes, source.clone()))
            })
            .collect()
    };
    tracing::info!(
        "Directory/ERF template read: {:?}, requested: {}, loaded: {}",
        other_read_start.elapsed(),
        other_count,
        other_raw.len()
    );
    raw_data.extend(other_raw);

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
            let name = crate::utils::strip_markup(&name);

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

/// Loads a .uti template's GFF fields plus a display label for history entries.
/// Holds the resource-manager read lock only for the duration of the load.
async fn load_item_template(
    state: &State<'_, AppState>,
    resref: &str,
) -> Result<
    (
        indexmap::IndexMap<String, crate::parsers::gff::GffValue<'static>>,
        String,
    ),
    CommandError,
> {
    let rm = state.resource_manager.read().await;
    let template_info = rm
        .get_item_template(resref)
        .ok_or_else(|| CommandError::NotFound {
            item: format!("Item template '{resref}'"),
        })?;
    let item_fields = rm
        .get_item_template_fields(&template_info)
        .map_err(|e| CommandError::Internal(format!("Failed to load item template: {e}")))?;

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
        .unwrap_or_else(|| resref.to_string());

    Ok((item_fields, item_label))
}

#[tauri::command]
pub async fn add_item_from_template(
    state: State<'_, AppState>,
    resref: String,
) -> CommandResult<AddItemResult> {
    let (item_fields, item_label) = load_item_template(&state, &resref).await?;

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_edit(format!("Add {item_label} from template"), |character| {
        Ok(character.add_new_item(item_fields, &game_data)?)
    })
}

/// Response for the item editor metadata endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ItemEditorMetadataResponse {
    pub property_types: Vec<PropertyMetadata>,
}

/// Get editor metadata for the item property editor.
#[tauri::command]
pub async fn get_editor_metadata(
    state: State<'_, AppState>,
) -> CommandResult<ItemEditorMetadataResponse> {
    let session = state.session.read();
    Ok(ItemEditorMetadataResponse {
        property_types: session.item_property_decoder.editor_metadata().to_vec(),
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
    /// Backpack index of the container holding the item at `item_index`.
    #[serde(default)]
    pub container_index: Option<usize>,
    pub slot: Option<String>,
    pub item_data: HashMap<String, JsonValue>,
    /// Optional typed appearance patch. When present, the backend writes Variation,
    /// ModelPart1/2/3, and Tintable via the typed GFF builders so byte-typed fields
    /// survive the update even if the original item never had a Tintable struct.
    #[serde(default)]
    pub appearance: Option<crate::character::ItemAppearance>,
}

#[tauri::command]
pub async fn update_item(
    state: State<'_, AppState>,
    request: UpdateItemRequest,
) -> CommandResult<()> {
    let game_data = state.game_data.read();
    let mut session = state.session.write();
    let label = match (&request.slot, request.item_index) {
        (Some(slot), _) => format!("Update item at slot {slot}"),
        (None, Some(idx)) => format!("Update item at slot {idx}"),
        _ => "Update item".to_string(),
    };
    session.record_edit(label, |character| {
        if let Some(slot_str) = &request.slot {
            let slot =
                parse_equipment_slot(slot_str).ok_or_else(|| CommandError::InvalidValue {
                    field: "slot".to_string(),
                    expected: "valid equipment slot".to_string(),
                    actual: slot_str.clone(),
                })?;
            character.update_equipped_item(slot, &request.item_data, &game_data)?;
            if let Some(appearance) = &request.appearance {
                character.apply_equipped_item_appearance(slot, appearance, &game_data)?;
            }
        } else if let Some(index) = request.item_index {
            let container = request.container_index;
            character.update_inventory_item(index, container, &request.item_data, &game_data)?;
            if let Some(appearance) = &request.appearance {
                character
                    .apply_inventory_item_appearance(index, container, appearance, &game_data)?;
            }
        } else {
            return Err(CommandError::InvalidValue {
                field: "request".to_string(),
                expected: "either slot or item_index".to_string(),
                actual: "neither provided".to_string(),
            });
        }
        Ok(())
    })
}

#[derive(Debug, Deserialize)]
pub struct TransformItemRequest {
    pub item_index: Option<usize>,
    /// Backpack index of the container holding the item at `item_index`.
    #[serde(default)]
    pub container_index: Option<usize>,
    pub slot: Option<String>,
    pub target_resref: String,
}

#[tauri::command]
pub async fn transform_item(
    state: State<'_, AppState>,
    request: TransformItemRequest,
) -> CommandResult<TransformResult> {
    let (item_fields, item_label) = load_item_template(&state, &request.target_resref).await?;

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_edit(format!("Transform item into {item_label}"), |character| {
        if let Some(slot_str) = &request.slot {
            let slot =
                parse_equipment_slot(slot_str).ok_or_else(|| CommandError::InvalidValue {
                    field: "slot".to_string(),
                    expected: "valid equipment slot".to_string(),
                    actual: slot_str.clone(),
                })?;
            Ok(character.transform_equipped_item(slot, item_fields, &game_data)?)
        } else if let Some(index) = request.item_index {
            Ok(character.transform_inventory_item(
                index,
                request.container_index,
                item_fields,
                &game_data,
            )?)
        } else {
            Err(CommandError::InvalidValue {
                field: "request".to_string(),
                expected: "either slot or item_index".to_string(),
                actual: "neither provided".to_string(),
            })
        }
    })
}
