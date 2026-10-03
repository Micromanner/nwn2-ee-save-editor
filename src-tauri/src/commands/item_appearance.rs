use tauri::State;
use tracing::{debug, info};

use crate::character::{
    AccessorySlot, ArmorSlot, ItemAppearance, ItemAppearanceOptions, TintChannels,
    accessory_resref, swap_tint_2_3,
};
use crate::commands::{CommandError, CommandResult};
use crate::services::model_loader::{self, ModelData};
use crate::state::AppState;

fn resref_is_boots_or_gloves(resref: &str) -> bool {
    let lower = resref.to_lowercase();
    lower.contains("_boots") || lower.contains("_gloves")
}

// The model commands are async so their MDB parsing, texture-free mesh
// serialization and the one-off resource prefix index build run on the
// async runtime instead of the window's main thread, where a sync command
// would freeze the UI for the duration of every thumbnail.
#[tauri::command]
pub async fn get_item_appearance_options(
    state: State<'_, AppState>,
    base_item_id: i32,
    armor_visual_type: Option<i32>,
) -> CommandResult<ItemAppearanceOptions> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let body_prefix = current_body_prefix(&state, &game_data);
    debug!(
        "Getting appearance options for base item: {} (vt={:?}, body={:?})",
        base_item_id, armor_visual_type, body_prefix
    );

    Ok(ItemAppearance::get_options(
        base_item_id,
        armor_visual_type,
        body_prefix.as_deref(),
        &game_data,
        &rm,
    ))
}

/// Look up the loaded character's race/gender body prefix (e.g. `P_EEM`)
/// so the item viewer previews armor on the same body it sits on in-game.
/// Returns `None` when there's no character loaded — the resolver then
/// falls back to its `P_HHM` default.
fn current_body_prefix(
    state: &State<'_, AppState>,
    game_data: &crate::loaders::GameData,
) -> Option<String> {
    let session = state.session.read();
    let character = session.character.as_ref()?;
    character.body_prefix(game_data)
}

pub fn load_item_model_impl(
    base_item_id: i32,
    appearance: &ItemAppearance,
    body_prefix: Option<&str>,
    game_data: &crate::loaders::GameData,
    rm: &crate::services::resource_manager::ResourceManager,
) -> CommandResult<ModelData> {
    let base_label = game_data
        .get_table("baseitems")
        .and_then(|t| t.get_by_id(base_item_id))
        .and_then(|row| {
            crate::utils::parsing::row_str(&row, "label")
                .or_else(|| crate::utils::parsing::row_str(&row, "modeltype"))
        })
        .unwrap_or_else(|| format!("base_item_{base_item_id}"));

    // Gloves and bracer items share the glove MDB pipeline. In-game ch3 is
    // ignored for both, so their meshes get their own tint group and the
    // frontend applies that rule at render time.
    let is_gloves_like =
        crate::character::is_bracer_label(&base_label) || base_label.eq_ignore_ascii_case("gloves");

    // Boots/gloves that ARE the edited item render their own tints live, so
    // they must stay in the "item" tint group the frontend updates. Only the
    // boots/gloves nested inside a chest item get baked, frozen tints.
    let own_slot = ItemAppearance::base_item_armor_slot(base_item_id, game_data);
    let is_own_boots_or_gloves = matches!(own_slot, Some(ArmorSlot::Boots | ArmorSlot::Gloves));

    let groups = appearance.resolve_model_resrefs(base_item_id, game_data, body_prefix);

    if groups.is_empty() {
        info!(
            target: "item_model",
            "ITEM base={base_label}(#{base_item_id}) BODY={body_prefix:?} VARIATION={} MP={:?} VT={:?} -> NO_MODEL",
            appearance.variation, appearance.model_parts, appearance.armor_visual_type
        );
        return Ok(ModelData::default());
    }

    let mut combined_data = ModelData::default();
    let mut loaded_resrefs: Vec<String> = Vec::new();

    for group in &groups {
        let mut group_hit: Option<String> = None;
        for resref in group {
            let part_tag = match resref.to_lowercase().as_bytes() {
                [.., b'_', b'a'] => "item_a",
                [.., b'_', b'b'] => "item_b",
                [.., b'_', b'c'] => "item_c",
                _ => "item",
            };
            match model_loader::load_model_static(rm, resref, part_tag, "item") {
                Ok(mut data) => {
                    if let Some((bone, slot)) =
                        crate::character::resref_attach_bone_and_slot(resref)
                    {
                        let tints = appearance.accessories.get_tints(slot).map(swap_tint_2_3);
                        for m in &mut data.meshes {
                            m.attach_bone = Some(bone.to_string());
                            m.accessory_slot = Some(slot);
                            if let Some(ref t) = tints {
                                m.override_tints = Some(t.clone());
                            }
                        }
                    } else if is_gloves_like && resref.to_lowercase().contains("_gloves") {
                        // The game ignores channel 3 on gloves and bracers. Own
                        // tint group so the frontend can keep applying that rule
                        // live (see `cosmetic_gloves_tint`) instead of us baking
                        // a value the color pickers can no longer move.
                        for m in &mut data.meshes {
                            m.tint_group = "item_gloves".to_string();
                        }
                    } else if resref_is_boots_or_gloves(resref) && !is_own_boots_or_gloves {
                        let nested = if resref.to_lowercase().contains("_boots") {
                            appearance.boots.as_ref()
                        } else {
                            appearance.gloves.as_ref()
                        };
                        let tints = nested
                            .and_then(|p| p.tints.as_ref())
                            .unwrap_or(&appearance.tints);
                        let swapped = swap_tint_2_3(tints);
                        for m in &mut data.meshes {
                            m.override_tints = Some(swapped.clone());
                            m.tint_group = "item_frozen".to_string();
                        }
                    }
                    combined_data.meshes.extend(data.meshes);
                    combined_data.hooks.extend(data.hooks);
                    combined_data.hair.extend(data.hair);
                    combined_data.helm.extend(data.helm);
                    if combined_data.skeleton.is_none() {
                        combined_data.skeleton = data.skeleton;
                        combined_data.animations = data.animations;
                    }
                    group_hit = Some(resref.clone());
                    break;
                }
                Err(e) => {
                    debug!("Candidate '{resref}' failed: {e}. Trying next in group.");
                }
            }
        }
        loaded_resrefs.push(group_hit.unwrap_or_else(|| format!("MISS({group:?})")));
    }

    info!(
        target: "item_model",
        "ITEM base={base_label}(#{base_item_id}) BODY={body_prefix:?} VARIATION={} MP={:?} VT={:?} BOOTS={:?} GLOVES={:?} GROUPS={} LOADED={:?}",
        appearance.variation,
        appearance.model_parts,
        appearance.armor_visual_type,
        appearance.boots,
        appearance.gloves,
        groups.len(),
        loaded_resrefs
    );

    if combined_data.meshes.is_empty() {
        info!("No meshes loaded for base item {base_item_id}; returning empty ModelData");
        return Ok(ModelData::default());
    }

    info!(
        "Loaded item model with {} meshes",
        combined_data.meshes.len()
    );
    Ok(combined_data)
}

#[tauri::command]
pub async fn load_item_model(
    state: State<'_, AppState>,
    appearance: ItemAppearance,
    base_item_id: i32,
) -> CommandResult<ModelData> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let body_prefix = current_body_prefix(&state, &game_data);
    load_item_model_impl(
        base_item_id,
        &appearance,
        body_prefix.as_deref(),
        &game_data,
        &rm,
    )
}

#[tauri::command]
pub async fn load_item_part(
    state: State<'_, AppState>,
    base_item_id: i32,
    part_index: u8,
    variant: i32,
) -> CommandResult<ModelData> {
    debug!(
        "Loading item part base_item_id={} part_index={} variant={}",
        base_item_id, part_index, variant
    );
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();

    let resref = ItemAppearance::resolve_weapon_part_resref(
        base_item_id,
        part_index as usize,
        variant,
        &game_data,
    )
    .ok_or_else(|| CommandError::NotFound {
        item: format!("resref for part {part_index} variant {variant}"),
    })?;

    let part_tag = match part_index {
        0 => "item_a",
        1 => "item_b",
        2 => "item_c",
        _ => "item",
    };
    model_loader::load_model_static(&rm, &resref, part_tag, "item").map_err(|e| {
        CommandError::OperationFailed {
            operation: format!("load item part '{resref}'"),
            reason: e.clone(),
        }
    })
}

/// One accessory mesh on its own (no body) for the per-slot thumbnail
/// gallery and for swapping a single slot in the item viewer, tinted the way
/// the viewer tints it and tagged with the slot's bone.
#[tauri::command]
pub async fn load_accessory_model(
    state: State<'_, AppState>,
    slot: AccessorySlot,
    id: i32,
    tints: TintChannels,
) -> CommandResult<ModelData> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let body_prefix = current_body_prefix(&state, &game_data)
        .unwrap_or_else(|| crate::character::DEFAULT_BODY_PREFIX.to_string());
    let resref =
        accessory_resref(&body_prefix, slot, id).ok_or_else(|| CommandError::NotFound {
            item: format!("accessory mesh for {slot:?} id {id}"),
        })?;
    debug!("Loading accessory model {resref}");

    let mut data = model_loader::load_model_static(&rm, &resref, "item", "item").map_err(|e| {
        CommandError::OperationFailed {
            operation: format!("load accessory '{resref}'"),
            reason: e.clone(),
        }
    })?;
    let tints = swap_tint_2_3(&tints);
    let attach_bone = slot.attach_bone().map(str::to_string);
    for m in &mut data.meshes {
        m.accessory_slot = Some(slot);
        m.attach_bone = attach_bone.clone();
        m.override_tints = Some(tints.clone());
    }
    Ok(data)
}
