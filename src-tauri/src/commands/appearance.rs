use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tracing::{debug, info, warn};

use crate::character::{
    AccessorySlot, AppearanceOption, AppearanceState, Character, TintChannels,
    cosmetic_gloves_tint, swap_tint_2_3,
};
use crate::commands::{CommandError, CommandResult};
use crate::services::model_loader::{self, AttachedPart, MeshData, ModelData, NamedSkeleton};
use crate::services::resource_manager::ResourceManager;
use crate::state::AppState;

/// Load a wing/tail as an AttachedPart with its own skeleton (from the 2DA
/// NWN2_Skeleton_File column when available, else from the MDB's stored name).
/// Binding them to the body skeleton scrambles deformation.
fn load_attached_part(
    rm: &ResourceManager,
    name: &str,
    resref: &str,
    skeleton_resref: Option<&str>,
    attach_bone: &str,
) -> Option<AttachedPart> {
    let result = match skeleton_resref {
        Some(skel) => model_loader::load_model_with_skeleton(rm, resref, skel, name, "none"),
        None => model_loader::load_model(rm, resref, name, "none"),
    };
    match result {
        Ok(data) => Some(AttachedPart {
            name: name.to_string(),
            meshes: data.meshes,
            skeleton: data.skeleton,
            animations: data.animations,
            attach_bone: Some(attach_bone.to_string()),
        }),
        Err(e) => {
            warn!("Failed to load {name} model '{resref}': {e}");
            None
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AppearanceUpdates {
    pub appearance_type: Option<i32>,
    pub appearance_head: Option<i32>,
    pub appearance_hair: Option<i32>,
    pub appearance_fhair: Option<i32>,
    pub tint_head: Option<TintChannels>,
    pub tint_hair: Option<TintChannels>,
    pub color_tattoo1: Option<i32>,
    pub color_tattoo2: Option<i32>,
    pub height: Option<f32>,
    pub girth: Option<f32>,
    pub soundset: Option<i32>,
    pub wings: Option<i32>,
    pub tail: Option<i32>,
    pub never_draw_helmet: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VoiceSetInfo {
    pub id: u32,
    pub name: String,
    pub gender: u8,
    pub resref: String,
    pub voice_type: u8,
}

#[tauri::command]
pub fn get_appearance_state(state: State<'_, AppState>) -> CommandResult<AppearanceState> {
    let rm = state.resource_manager.blocking_read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let game_data = state.game_data.read();

    Ok(character.get_appearance_state(&game_data, &rm))
}

#[tauri::command]
pub fn update_appearance(
    state: State<'_, AppState>,
    updates: AppearanceUpdates,
) -> CommandResult<AppearanceState> {
    let rm = state.resource_manager.blocking_read();

    let mut session = state.session.write();
    session.record_history("Update appearance", None);
    let character = session
        .character
        .as_mut()
        .ok_or(CommandError::NoCharacterLoaded)?;

    if let Some(v) = updates.appearance_type {
        character.set_appearance_type(v);
    }
    if let Some(v) = updates.appearance_head {
        character.set_appearance_head(v);
    }
    if let Some(v) = updates.appearance_hair {
        character.set_appearance_hair(v);
    }
    if let Some(v) = updates.appearance_fhair {
        character.set_appearance_fhair(v);
    }
    if let Some(ref tints) = updates.tint_head {
        character.set_tint_head(tints);
    }
    if let Some(ref tints) = updates.tint_hair {
        character.set_tint_hair(tints);
    }
    if let Some(v) = updates.color_tattoo1 {
        character.set_color_tattoo1(v);
    }
    if let Some(v) = updates.color_tattoo2 {
        character.set_color_tattoo2(v);
    }
    if let Some(v) = updates.height {
        character.set_height(v);
    }
    if let Some(v) = updates.girth {
        character.set_girth(v);
    }
    if let Some(v) = updates.soundset {
        character.set_soundset(v);
    }
    if let Some(v) = updates.wings {
        character.set_wings(v);
    }
    if let Some(v) = updates.tail {
        character.set_tail(v);
    }
    if let Some(v) = updates.never_draw_helmet {
        character.set_never_draw_helmet(v);
    }

    let game_data = state.game_data.read();

    let mut appearance_state = character.get_appearance_state(&game_data, &rm);

    // On appearance_type change, stored head/hair may not exist for the new
    // race prefix — clamp to the first available to keep the model resolvable.
    if updates.appearance_type.is_some() {
        if !appearance_state
            .available_heads
            .contains(&character.appearance_head())
            && let Some(&first) = appearance_state.available_heads.first()
        {
            character.set_appearance_head(first);
            appearance_state.appearance_head = first;
        }
        if !appearance_state
            .available_hairs
            .contains(&character.appearance_hair())
            && let Some(&first) = appearance_state.available_hairs.first()
        {
            character.set_appearance_hair(first);
            appearance_state.appearance_hair = first;
        }
    }

    Ok(appearance_state)
}

#[tauri::command]
pub async fn get_available_wings(
    state: State<'_, AppState>,
) -> CommandResult<Vec<AppearanceOption>> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    Ok(Character::get_available_model_options(
        &game_data,
        &rm,
        "wingmodel",
    ))
}

#[tauri::command]
pub async fn get_available_tails(
    state: State<'_, AppState>,
) -> CommandResult<Vec<AppearanceOption>> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    Ok(Character::get_available_model_options(
        &game_data,
        &rm,
        "tailmodel",
    ))
}

#[tauri::command]
pub fn load_character_model(state: State<'_, AppState>) -> CommandResult<ModelData> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let game_data = state.game_data.read();

    let rm = state.resource_manager.blocking_read();
    let parts = character
        .resolve_model_parts(&game_data, &rm)
        .ok_or_else(|| CommandError::Internal("Failed to resolve character model parts".into()))?;

    info!(
        "Model parts: body={:?}, boots={:?}, gloves={:?}, helm={:?}, cloak={:?}",
        parts.body_parts,
        parts.boots_candidates,
        parts.gloves_candidates,
        parts.helm_candidates,
        parts.cloak_resref,
    );

    // Load skeleton + bone palettes + animations once
    let skeleton = model_loader::load_skeleton(&rm, &parts.skeleton_resref);
    let palettes = skeleton.as_ref().map(model_loader::build_bone_palettes);
    let animations = skeleton
        .as_ref()
        .map(|_| model_loader::load_idle_animations(&rm, &parts.skeleton_resref))
        .unwrap_or_default();

    let mut all_meshes: Vec<MeshData> = Vec::new();

    let load_part = |resref: &str, part: &str, tint: &str| -> Result<Vec<MeshData>, String> {
        if let (Some(skel), Some(pal)) = (&skeleton, &palettes) {
            model_loader::load_meshes_with_existing_skeleton(&rm, resref, part, tint, skel, pal)
        } else {
            model_loader::load_model(&rm, resref, part, tint).map(|d| d.meshes)
        }
    };

    // Body
    let mut body_loaded = false;
    for part_resref in &parts.body_parts {
        if let Ok(meshes) = load_part(part_resref, "body", "body") {
            all_meshes.extend(meshes);
            body_loaded = true;
            break;
        }
    }
    if !body_loaded {
        warn!(
            "Body candidates all failed: {:?}, falling back to naked: {}",
            parts.body_parts, parts.naked_body_resref
        );
        if let Ok(meshes) = load_part(&parts.naked_body_resref, "body", "body") {
            all_meshes.extend(meshes);
        }
    }

    // Head
    match load_part(&parts.head_resref, "head", "head") {
        Ok(head_meshes) => {
            for mut mesh in head_meshes {
                if mesh.name.to_lowercase().contains("_fhair") {
                    if parts.show_fhair {
                        mesh.part = "fhair".to_string();
                        mesh.tint_group = "hair".to_string();
                        all_meshes.push(mesh);
                    }
                } else {
                    all_meshes.push(mesh);
                }
            }
        }
        Err(e) => warn!("Failed to load head model '{}': {}", parts.head_resref, e),
    }

    // Hair
    if let Some(ref hair_resref) = parts.hair_resref {
        match load_part(hair_resref, "hair", "hair") {
            Ok(meshes) => all_meshes.extend(meshes),
            Err(e) => debug!("Hair model '{}' not found: {}", hair_resref, e),
        }
    }

    let mut attached_parts: Vec<AttachedPart> = Vec::new();
    if let Some(ref resref) = parts.wings_resref {
        attached_parts.extend(load_attached_part(
            &rm,
            "wings",
            resref,
            parts.wings_skeleton_resref.as_deref(),
            "ap_wings",
        ));
    }
    if let Some(ref resref) = parts.tail_resref {
        attached_parts.extend(load_attached_part(
            &rm,
            "tail",
            resref,
            parts.tail_skeleton_resref.as_deref(),
            "ap_tail",
        ));
    }

    // Helm
    if parts.show_helmet {
        for helm_resref in &parts.helm_candidates {
            if let Ok(meshes) = load_part(helm_resref, "helm", "body") {
                all_meshes.extend(meshes);
                break;
            }
        }
    }

    let mut load_body_part_with_tint =
        |candidates: &[String], part: &str, tint: Option<&TintChannels>| -> bool {
            for resref in candidates {
                if let Ok(mut meshes) = load_part(resref, part, "body") {
                    if let Some(t) = tint {
                        for m in &mut meshes {
                            m.override_tints = Some(t.clone());
                        }
                    }
                    all_meshes.extend(meshes);
                    return true;
                }
            }
            false
        };

    if !parts.boots_candidates.is_empty() {
        let tint = parts.boots_tint.as_ref().map(swap_tint_2_3);
        if !load_body_part_with_tint(&parts.boots_candidates, "boots", tint.as_ref()) {
            warn!(
                "Failed to load boots from candidates: {:?}",
                parts.boots_candidates
            );
        }
    }

    if !parts.gloves_candidates.is_empty() {
        let tint = parts.gloves_tint.as_ref().map(cosmetic_gloves_tint);
        if !load_body_part_with_tint(&parts.gloves_candidates, "gloves", tint.as_ref()) {
            warn!(
                "Failed to load gloves from candidates: {:?}",
                parts.gloves_candidates
            );
        }
    }

    // Armor accessories (pauldrons, bracers, greaves, …): same renderable
    // iterator the item viewer uses, so resref format / bone mapping / tint
    // source stay in one place.
    let accessory_body_prefix = character
        .body_prefix(&game_data)
        .unwrap_or_else(|| "P_HHM".to_string());
    // Shin accessories clip through standalone boots; hide them when a
    // boots item contributed its own tint.
    let has_standalone_boots = parts.boots_tint.is_some();
    for acc in parts
        .chest_accessories
        .iter_renderable(&accessory_body_prefix)
    {
        if has_standalone_boots && matches!(acc.slot, AccessorySlot::LtShin | AccessorySlot::RtShin)
        {
            debug!(
                "Hiding shin accessory '{}' because standalone boots are equipped",
                acc.resref
            );
            continue;
        }
        let tints = if matches!(acc.slot, AccessorySlot::LtBracer | AccessorySlot::RtBracer) {
            swap_tint_2_3(acc.tints)
        } else {
            acc.tints.clone()
        };
        match load_part(&acc.resref, "accessory", "body") {
            Ok(meshes) => {
                for mut m in meshes {
                    m.attach_bone = Some(acc.attach_bone.to_string());
                    m.override_tints = Some(tints.clone());
                    all_meshes.push(m);
                }
            }
            Err(e) => debug!("Accessory '{}' skipped: {e}", acc.resref),
        }
    }

    let mut secondary_skeletons: Vec<NamedSkeleton> = Vec::new();
    if let (Some(cloak_resref), Some(body_skel), Some(body_pal)) =
        (&parts.cloak_resref, skeleton.as_ref(), palettes.as_ref())
    {
        let cape = model_loader::load_cape_skeleton_for_body(&rm, &parts.naked_body_resref);
        match model_loader::load_cloak(&rm, cloak_resref, body_skel, body_pal, cape) {
            Ok((meshes, cape_ns)) => {
                all_meshes.extend(meshes);
                if let Some(ns) = cape_ns {
                    secondary_skeletons.push(ns);
                }
            }
            Err(e) => warn!("Failed to load cloak '{}': {}", cloak_resref, e),
        }
    }

    if all_meshes.is_empty() {
        return Err(CommandError::Internal(
            "No model meshes could be loaded".to_string(),
        ));
    }

    Ok(ModelData {
        meshes: all_meshes,
        hooks: Vec::new(),
        hair: Vec::new(),
        helm: Vec::new(),
        skeleton,
        animations,
        attached_parts,
        secondary_skeletons,
    })
}

#[tauri::command]
pub fn load_character_part(state: State<'_, AppState>, part: String) -> CommandResult<ModelData> {
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    let game_data = state.game_data.read();

    let rm = state.resource_manager.blocking_read();
    let parts = character
        .resolve_model_parts(&game_data, &rm)
        .ok_or_else(|| {
            CommandError::Internal("Failed to resolve character model parts".to_string())
        })?;

    let skeleton = model_loader::load_skeleton(&rm, &parts.skeleton_resref);
    let palettes = skeleton.as_ref().map(model_loader::build_bone_palettes);

    let load_with_skel =
        |resref: &str, part_name: &str, tint: &str| -> Result<Vec<MeshData>, String> {
            if let (Some(skel), Some(pal)) = (&skeleton, &palettes) {
                model_loader::load_meshes_with_existing_skeleton(
                    &rm, resref, part_name, tint, skel, pal,
                )
            } else {
                model_loader::load_model(&rm, resref, part_name, tint).map(|d| d.meshes)
            }
        };

    let mut meshes: Vec<MeshData> = Vec::new();
    let mut attached_parts: Vec<AttachedPart> = Vec::new();
    let mut secondary_skeletons: Vec<NamedSkeleton> = Vec::new();

    match part.as_str() {
        "head" => {
            if let Ok(head_meshes) = load_with_skel(&parts.head_resref, "head", "head") {
                meshes.extend(
                    head_meshes
                        .into_iter()
                        .filter(|m| !m.name.to_lowercase().contains("_fhair")),
                );
            }
        }
        "hair" => {
            if let Some(ref resref) = parts.hair_resref
                && let Ok(hair_meshes) = load_with_skel(resref, "hair", "hair")
            {
                meshes.extend(hair_meshes);
            }
        }
        "fhair" => {
            if parts.show_fhair
                && let Ok(head_meshes) = load_with_skel(&parts.head_resref, "fhair", "hair")
            {
                for mesh in head_meshes {
                    if mesh.name.to_lowercase().contains("_fhair") {
                        meshes.push(mesh);
                    }
                }
            }
        }
        "wings" => {
            if let Some(ref resref) = parts.wings_resref {
                attached_parts.extend(load_attached_part(
                    &rm,
                    "wings",
                    resref,
                    parts.wings_skeleton_resref.as_deref(),
                    "ap_wings",
                ));
            }
        }
        "tail" => {
            if let Some(ref resref) = parts.tail_resref {
                attached_parts.extend(load_attached_part(
                    &rm,
                    "tail",
                    resref,
                    parts.tail_skeleton_resref.as_deref(),
                    "ap_tail",
                ));
            }
        }
        "helm" => {
            if parts.show_helmet {
                for resref in &parts.helm_candidates {
                    if let Ok(helm_meshes) = load_with_skel(resref, "helm", "body") {
                        meshes.extend(helm_meshes);
                        break;
                    }
                }
            }
        }
        "body" => {
            let mut loaded = false;
            for part_resref in &parts.body_parts {
                if let Ok(body_meshes) = load_with_skel(part_resref, "body", "body") {
                    meshes.extend(body_meshes);
                    loaded = true;
                    break;
                }
            }
            if !loaded
                && let Ok(body_meshes) = load_with_skel(&parts.naked_body_resref, "body", "body")
            {
                meshes.extend(body_meshes);
            }
        }
        "cloak" => {
            if let (Some(resref), Some(body_skel), Some(body_pal)) =
                (&parts.cloak_resref, skeleton.as_ref(), palettes.as_ref())
            {
                let cape = model_loader::load_cape_skeleton_for_body(&rm, &parts.naked_body_resref);
                match model_loader::load_cloak(&rm, resref, body_skel, body_pal, cape) {
                    Ok((ms, cape_ns)) => {
                        meshes.extend(ms);
                        if let Some(ns) = cape_ns {
                            secondary_skeletons.push(ns);
                        }
                    }
                    Err(e) => warn!("Failed to load cloak '{}': {}", resref, e),
                }
            }
        }
        _ => {
            return Err(CommandError::Internal(format!("Unknown part type: {part}")));
        }
    }

    Ok(ModelData {
        meshes,
        hooks: Vec::new(),
        hair: Vec::new(),
        helm: Vec::new(),
        skeleton: None,
        animations: Vec::new(),
        attached_parts,
        secondary_skeletons,
    })
}

#[tauri::command]
pub async fn get_available_voicesets(
    state: State<'_, AppState>,
) -> CommandResult<Vec<VoiceSetInfo>> {
    // Extract all soundset rows first so we can drop game_data before the await
    let candidates: Vec<(i32, String, u8, i32, String)> = {
        let game_data = state.game_data.read();
        let soundset_table = game_data
            .get_table("soundset")
            .ok_or_else(|| CommandError::Internal("soundset.2da not loaded".to_string()))?;

        let mut rows = Vec::new();
        for i in 0..soundset_table.row_count() {
            let id = i as i32;
            let Some(row) = soundset_table.get_by_id(id) else {
                continue;
            };
            let Some(resref) = crate::utils::parsing::row_str(&row, "resref") else {
                continue;
            };
            if resref.eq_ignore_ascii_case("none") {
                continue;
            }
            let type_val = crate::utils::parsing::row_int(&row, "type", -1) as u8;
            let gender = crate::utils::parsing::row_int(&row, "gender", 0) as u8;
            let strref = crate::utils::parsing::row_int(&row, "strref", -1);
            let label = crate::utils::parsing::row_str(&row, "label")
                .unwrap_or_else(|| format!("Voice {id}"));
            rows.push((id, resref, type_val | (gender << 4), strref, label));
        }
        rows
    };

    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let mut voicesets = Vec::new();

    for (id, resref, packed, strref, label) in &candidates {
        let type_val = *packed & 0x0F;
        let gender = *packed >> 4;

        let Ok(ssf_data) = rm.get_resource_bytes(resref, "ssf") else {
            continue;
        };

        let has_audio = crate::parsers::ssf::parse_ssf(&ssf_data)
            .map(|wav_resrefs| {
                wav_resrefs
                    .iter()
                    .any(|wr| rm.get_resource_bytes(wr, "wav").is_ok())
            })
            .unwrap_or(false);
        if !has_audio {
            continue;
        }

        let name = if *strref >= 0 {
            game_data
                .get_string(*strref)
                .unwrap_or_else(|| label.clone())
        } else {
            label.clone()
        };

        voicesets.push(VoiceSetInfo {
            id: *id as u32,
            name,
            gender,
            resref: resref.clone(),
            voice_type: type_val,
        });
    }

    voicesets.sort_by_key(|a| a.name.to_lowercase());
    Ok(voicesets)
}

#[tauri::command]
pub async fn preview_voiceset(
    state: State<'_, AppState>,
    resref: String,
) -> CommandResult<Vec<u8>> {
    let rm = state.resource_manager.read().await;

    let ssf_data = rm
        .get_resource_bytes(&resref, "ssf")
        .map_err(|e| CommandError::Internal(format!("SSF file not found: {e}")))?;

    let wav_resrefs = crate::parsers::ssf::parse_ssf(&ssf_data)
        .map_err(|e| CommandError::Internal(format!("Failed to parse SSF: {e}")))?;

    if wav_resrefs.is_empty() {
        return Err(CommandError::Internal(
            "No voice lines found in SSF".to_string(),
        ));
    }

    let mut rng = rand::rng();
    let mut shuffled = wav_resrefs;
    shuffled.shuffle(&mut rng);

    for wav_resref in &shuffled {
        if let Ok(data) = rm.get_resource_bytes(wav_resref, "wav") {
            return Ok(data);
        }
    }

    Err(CommandError::Internal(format!(
        "No WAV files found for voiceset: {resref}"
    )))
}
