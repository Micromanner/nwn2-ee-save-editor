use std::time::{Duration, Instant};

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use tracing::{debug, info, warn};

use crate::character::{
    AccessorySlot, AppearanceOption, AppearanceState, Character, CharacterModelParts, TintChannel,
    TintChannels, cosmetic_gloves_tint, swap_tint_2_3,
};
use crate::commands::{CommandError, CommandResult};
use crate::loaders::GameData;
use crate::services::model_loader::{
    self, AttachedPart, BonePalettes, MeshData, ModelData, NamedSkeleton, SkeletonData,
};
use crate::services::resource_manager::ResourceManager;
use crate::services::tint_analysis::{self, TintCapabilityCache};
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
    base_anims_prefix: &str,
) -> Option<AttachedPart> {
    // tint_group must match the viewer's tint map keys ("tail"/"wings") so the
    // root Tintable colors bind and update; "none" leaves the mesh untinted.
    let result = match skeleton_resref {
        Some(skel) => model_loader::load_model_with_skeleton(rm, resref, skel, name, name),
        None => model_loader::load_model(rm, resref, name, name),
    };
    match result {
        Ok(data) => Some(AttachedPart {
            name: name.to_string(),
            meshes: data.meshes,
            skeleton: data.skeleton,
            animations: model_loader::load_idle_animations_for_prefix(rm, base_anims_prefix),
            attach_bone: Some(attach_bone.to_string()),
        }),
        Err(e) => {
            warn!("Failed to load {name} model '{resref}': {e}");
            None
        }
    }
}

struct PartLoader<'a> {
    rm: &'a ResourceManager,
    skeleton: Option<SkeletonData>,
    palettes: Option<BonePalettes>,
}

impl<'a> PartLoader<'a> {
    fn new(rm: &'a ResourceManager, skeleton_resref: &str) -> Self {
        let skeleton = model_loader::load_skeleton(rm, skeleton_resref);
        let palettes = skeleton.as_ref().map(model_loader::build_bone_palettes);
        Self {
            rm,
            skeleton,
            palettes,
        }
    }

    fn load(&self, resref: &str, part: &str, tint: &str) -> Result<Vec<MeshData>, String> {
        if let (Some(skel), Some(pal)) = (&self.skeleton, &self.palettes) {
            model_loader::load_meshes_with_existing_skeleton(self.rm, resref, part, tint, skel, pal)
        } else {
            model_loader::load_model_static(self.rm, resref, part, tint).map(|d| d.meshes)
        }
    }

    fn load_first<'c>(
        &self,
        candidates: impl IntoIterator<Item = &'c String>,
        part: &str,
        tint: &str,
        override_tints: Option<&TintChannels>,
    ) -> Option<Vec<MeshData>> {
        candidates.into_iter().find_map(|resref| {
            let mut meshes = self.load(resref, part, tint).ok()?;
            if let Some(t) = override_tints {
                for m in &mut meshes {
                    m.override_tints = Some(t.clone());
                }
            }
            Some(meshes)
        })
    }
}

#[derive(Default)]
struct ExtraParts {
    attached: Vec<AttachedPart>,
    secondary_skeletons: Vec<NamedSkeleton>,
}

fn load_body_meshes(loader: &PartLoader, parts: &CharacterModelParts) -> Vec<MeshData> {
    if let Some(meshes) = loader.load_first(&parts.body_parts, "body", "body", None) {
        return meshes;
    }
    warn!(
        "Body candidates all failed: {:?}, falling back to naked: {}",
        parts.body_parts, parts.naked_body_resref
    );
    loader
        .load(&parts.naked_body_resref, "body", "body")
        .unwrap_or_default()
}

/// Head meshes without facial hair, and the facial hair meshes (empty unless shown).
fn load_head_meshes(
    loader: &PartLoader,
    parts: &CharacterModelParts,
) -> (Vec<MeshData>, Vec<MeshData>) {
    let mut head = Vec::new();
    let mut fhair = Vec::new();
    let Some(head_resref) = &parts.head_resref else {
        return (head, fhair);
    };
    match loader.load(head_resref, "head", "head") {
        Ok(meshes) => {
            for mut mesh in meshes {
                if mesh.name.to_lowercase().contains("_fhair") {
                    if parts.show_fhair {
                        mesh.part = "fhair".to_string();
                        mesh.tint_group = "hair".to_string();
                        fhair.push(mesh);
                    }
                } else {
                    head.push(mesh);
                }
            }
        }
        Err(e) => warn!("Failed to load head model '{head_resref}': {e}"),
    }
    (head, fhair)
}

fn load_hair_meshes(loader: &PartLoader, parts: &CharacterModelParts) -> Vec<MeshData> {
    let Some(hair_resref) = &parts.hair_resref else {
        return Vec::new();
    };
    loader
        .load(hair_resref, "hair", "hair")
        .unwrap_or_else(|e| {
            debug!("Hair model '{hair_resref}' not found: {e}");
            Vec::new()
        })
}

fn load_helm_meshes(loader: &PartLoader, parts: &CharacterModelParts) -> Vec<MeshData> {
    if !parts.show_helmet {
        return Vec::new();
    }
    let helm_tint = parts.helm_tint.as_ref().map(swap_tint_2_3);
    loader
        .load_first(&parts.helm_candidates, "helm", "body", helm_tint.as_ref())
        .unwrap_or_default()
}

/// Wings or tail, as meshes bound to a synced secondary skeleton or as an
/// attached part with its own skeleton, depending on the base animations.
fn load_wings_or_tail(
    loader: &PartLoader,
    parts: &CharacterModelParts,
    name: &str,
    meshes: &mut Vec<MeshData>,
    extra: &mut ExtraParts,
) {
    let (resref, skeleton_resref, base_anims, attach_bone) = if name == "wings" {
        (
            parts.wings_resref.as_deref(),
            parts.wings_skeleton_resref.as_deref(),
            parts.wings_base_anims.as_deref(),
            "ap_wings",
        )
    } else {
        (
            parts.tail_resref.as_deref(),
            parts.tail_skeleton_resref.as_deref(),
            parts.tail_base_anims.as_deref(),
            "ap_tail",
        )
    };
    let Some(resref) = resref else {
        return;
    };
    if model_loader::is_synced_base_anims(&parts.skeleton_resref, base_anims) {
        if let (true, Some(skel_resref)) = (loader.skeleton.is_some(), skeleton_resref) {
            match model_loader::load_synced_part(loader.rm, resref, skel_resref, name) {
                Ok((ms, ns)) => {
                    meshes.extend(ms);
                    extra.secondary_skeletons.push(ns);
                }
                Err(e) => warn!("Failed to load synced {name} '{resref}': {e}"),
            }
        } else {
            warn!("Synced {name} '{resref}' skipped: body or {name} skeleton missing");
        }
    } else {
        extra.attached.extend(load_attached_part(
            loader.rm,
            name,
            resref,
            skeleton_resref,
            attach_bone,
            base_anims.unwrap_or_default(),
        ));
    }
}

fn load_cloak_meshes(
    loader: &PartLoader,
    parts: &CharacterModelParts,
    meshes: &mut Vec<MeshData>,
    extra: &mut ExtraParts,
) {
    let (Some(resref), Some(body_skel), Some(body_pal)) = (
        &parts.cloak_resref,
        loader.skeleton.as_ref(),
        loader.palettes.as_ref(),
    ) else {
        return;
    };
    let cape = model_loader::load_cape_skeleton_for_body(loader.rm, &parts.naked_body_resref);
    match model_loader::load_cloak(loader.rm, resref, body_skel, body_pal, cape) {
        Ok((ms, cape_ns)) => {
            meshes.extend(ms);
            extra.secondary_skeletons.extend(cape_ns);
        }
        Err(e) => warn!("Failed to load cloak '{resref}': {e}"),
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
    pub tint_body: Option<TintChannels>,
    pub tint_body_match_skin: Option<bool>,
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

fn build_appearance_state(
    character: &Character,
    game_data: &GameData,
    rm: &ResourceManager,
    tint_capability_cache: &TintCapabilityCache,
) -> AppearanceState {
    let parts = character.resolve_model_parts(game_data, rm);
    let mut appearance_state = character.get_appearance_state(game_data, rm, parts.as_ref());
    if let Some(parts) = &parts {
        appearance_state.color_capabilities =
            tint_analysis::color_capabilities(rm, tint_capability_cache, parts);
    }
    appearance_state
}

#[tauri::command]
pub async fn get_appearance_state(state: State<'_, AppState>) -> CommandResult<AppearanceState> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let session = state.session.read();
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;

    Ok(build_appearance_state(
        character,
        &game_data,
        &rm,
        &state.tint_capability_cache,
    ))
}

#[tauri::command]
pub async fn update_appearance(
    state: State<'_, AppState>,
    updates: AppearanceUpdates,
) -> CommandResult<AppearanceState> {
    let rm = state.resource_manager.read().await;

    let game_data = state.game_data.read();
    let mut session = state.session.write();
    session.record_edit("Update appearance", |character| {
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
        if let Some(ref tints) = updates.tint_body {
            character.set_tint_body(tints);
        }
        if updates.tint_body_match_skin == Some(true) {
            character.set_tint_body_from_head();
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

        // On appearance_type change, stored head/hair may not exist for the new
        // race prefix; clamp to the first available to keep the model resolvable.
        if updates.appearance_type.is_some() {
            let (available_heads, available_hairs, _) =
                character.discover_model_variants(&game_data, &rm);
            if !available_heads.contains(&character.appearance_head())
                && let Some(&first) = available_heads.first()
            {
                character.set_appearance_head(first);
            }
            if !available_hairs.contains(&character.appearance_hair())
                && let Some(&first) = available_hairs.first()
            {
                character.set_appearance_hair(first);
            }
        }

        Ok(())
    })?;

    let session = parking_lot::RwLockWriteGuard::downgrade(session);
    let character = session
        .character
        .as_ref()
        .ok_or(CommandError::NoCharacterLoaded)?;
    Ok(build_appearance_state(
        character,
        &game_data,
        &rm,
        &state.tint_capability_cache,
    ))
}

#[tauri::command]
pub async fn get_available_wings(
    state: State<'_, AppState>,
) -> CommandResult<Vec<AppearanceOption>> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let session = state.session.read();
    let body_skel = session
        .character
        .as_ref()
        .and_then(|c| c.body_skeleton_resref(&game_data));
    Ok(Character::get_available_model_options(
        &game_data,
        &rm,
        "wingmodel",
        body_skel.as_deref(),
    ))
}

#[tauri::command]
pub async fn get_available_tails(
    state: State<'_, AppState>,
) -> CommandResult<Vec<AppearanceOption>> {
    let rm = state.resource_manager.read().await;
    let game_data = state.game_data.read();
    let session = state.session.read();
    let body_skel = session
        .character
        .as_ref()
        .and_then(|c| c.body_skeleton_resref(&game_data));
    Ok(Character::get_available_model_options(
        &game_data,
        &rm,
        "tailmodel",
        body_skel.as_deref(),
    ))
}

#[tauri::command]
pub async fn get_parts_based_appearances(
    state: State<'_, AppState>,
) -> CommandResult<Vec<AppearanceOption>> {
    let game_data = state.game_data.read();
    Ok(Character::get_parts_based_appearances(&game_data))
}

#[tauri::command]
pub async fn load_character_model(state: State<'_, AppState>) -> CommandResult<ModelData> {
    let rm = state.resource_manager.read().await;
    let (parts, accessory_body_prefix) = {
        let game_data = state.game_data.read();
        let session = state.session.read();
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let parts = character
            .resolve_model_parts(&game_data, &rm)
            .ok_or_else(|| {
                CommandError::Internal("Failed to resolve character model parts".into())
            })?;
        let prefix = character
            .body_prefix(&game_data)
            .unwrap_or_else(|| "P_HHM".to_string());
        (parts, prefix)
    };

    info!(
        "Model parts: body={:?}, boots={:?}, gloves={:?}, helm={:?}, cloak={:?}",
        parts.body_parts,
        parts.boots_candidates,
        parts.gloves_candidates,
        parts.helm_candidates,
        parts.cloak_resref,
    );

    let loader = PartLoader::new(&rm, &parts.skeleton_resref);
    let animations = loader
        .skeleton
        .as_ref()
        .map(|_| model_loader::load_idle_animations(&rm, &parts.skeleton_resref))
        .unwrap_or_default();

    let mut all_meshes = load_body_meshes(&loader, &parts);
    let (head, fhair) = load_head_meshes(&loader, &parts);
    all_meshes.extend(head);
    all_meshes.extend(fhair);
    all_meshes.extend(load_hair_meshes(&loader, &parts));

    let mut extra = ExtraParts::default();
    load_wings_or_tail(&loader, &parts, "wings", &mut all_meshes, &mut extra);
    load_wings_or_tail(&loader, &parts, "tail", &mut all_meshes, &mut extra);
    all_meshes.extend(load_helm_meshes(&loader, &parts));

    if !parts.boots_candidates.is_empty() {
        let tint = parts.boots_tint.as_ref().map(swap_tint_2_3);
        if let Some(meshes) =
            loader.load_first(&parts.boots_candidates, "boots", "body", tint.as_ref())
        {
            all_meshes.extend(meshes);
        } else {
            warn!(
                "Failed to load boots from candidates: {:?}",
                parts.boots_candidates
            );
        }
    }

    if !parts.gloves_candidates.is_empty() {
        let tint = parts.gloves_tint.as_ref().map(|t| {
            if parts.has_standalone_gloves {
                cosmetic_gloves_tint(t)
            } else {
                swap_tint_2_3(t)
            }
        });
        if let Some(meshes) =
            loader.load_first(&parts.gloves_candidates, "gloves", "body", tint.as_ref())
        {
            all_meshes.extend(meshes);
        } else {
            warn!(
                "Failed to load gloves from candidates: {:?}",
                parts.gloves_candidates
            );
        }
    }

    // Armor accessories (pauldrons, bracers, greaves, …): same renderable
    // iterator the item viewer uses, so resref format / bone mapping / tint
    // source stay in one place.
    let has_standalone_boots = parts.has_standalone_boots;
    for acc in parts
        .chest_accessories
        .iter_renderable(&accessory_body_prefix)
    {
        // Shin accessories clip through standalone boots.
        if has_standalone_boots && matches!(acc.slot, AccessorySlot::LtShin | AccessorySlot::RtShin)
        {
            debug!(
                "Hiding shin accessory '{}' because standalone boots are equipped",
                acc.resref
            );
            continue;
        }
        let tints = swap_tint_2_3(acc.tints);
        match loader.load(&acc.resref, "accessory", "body") {
            Ok(meshes) => {
                for mut m in meshes {
                    m.attach_bone = Some(acc.attach_bone.to_string());
                    m.accessory_slot = Some(acc.slot);
                    m.override_tints = Some(tints.clone());
                    all_meshes.push(m);
                }
            }
            Err(e) => debug!("Accessory '{}' skipped: {e}", acc.resref),
        }
    }

    load_cloak_meshes(&loader, &parts, &mut all_meshes, &mut extra);

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
        skeleton: loader.skeleton,
        animations,
        attached_parts: extra.attached,
        secondary_skeletons: extra.secondary_skeletons,
    })
}

/// Head-and-shoulders model for a companion's Party Management portrait:
/// torso body part + head + hair/fhair + shoulder accessories, no helmet.
/// Every mesh gets its tints baked into `override_tints` so the shared
/// thumbnail renderer needs no per-group tint mapping. That renderer always
/// applies the shader G/B swap; head/armor tints already expect it (same as
/// the character viewer), hair is raw GFF order and is pre-swapped to cancel.
#[tauri::command]
pub async fn load_companion_portrait_model(
    state: State<'_, AppState>,
    ros_name: String,
) -> CommandResult<ModelData> {
    let started = Instant::now();
    let rm = state.resource_manager.read().await;
    let session = state.session.read();
    let character = session
        .companion_snapshot(&ros_name)
        .map_err(CommandError::Internal)?;
    drop(session);
    let (parts, accessory_body_prefix, head_tint, hair_tint) = {
        let game_data = state.game_data.read();
        let parts = character
            .resolve_model_parts(&game_data, &rm)
            .ok_or_else(|| {
                CommandError::Internal("Failed to resolve character model parts".into())
            })?;
        let prefix = character
            .body_prefix(&game_data)
            .unwrap_or_else(|| "P_HHM".to_string());
        (parts, prefix, character.tint_head(), character.tint_hair())
    };

    // Per-step timings for the summary line, so a slow portrait names the
    // step responsible.
    let mut timings: Vec<(String, Duration)> = vec![("prep".into(), started.elapsed())];
    let step = Instant::now();
    let skeleton = model_loader::load_skeleton(&rm, &parts.skeleton_resref);
    let palettes = skeleton.as_ref().map(model_loader::build_bone_palettes);
    timings.push(("skeleton".into(), step.elapsed()));
    let mut load_part = |resref: &str, part: &str, tint: &str| -> Result<Vec<MeshData>, String> {
        let step = Instant::now();
        let result = if let (Some(skel), Some(pal)) = (&skeleton, &palettes) {
            model_loader::load_meshes_with_existing_skeleton(&rm, resref, part, tint, skel, pal)
        } else {
            model_loader::load_model_static(&rm, resref, part, tint).map(|d| d.meshes)
        };
        timings.push((resref.to_string(), step.elapsed()));
        result
    };

    let white = TintChannels {
        channel1: TintChannel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        channel2: TintChannel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        channel3: TintChannel {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
    };
    let mut all_meshes: Vec<MeshData> = Vec::new();

    let body_tint = parts
        .armor_tint
        .as_ref()
        .map(swap_tint_2_3)
        .unwrap_or_else(|| white.clone());
    let mut body_loaded = false;
    for part_resref in parts
        .body_parts
        .iter()
        .chain(std::iter::once(&parts.naked_body_resref))
    {
        if let Ok(meshes) = load_part(part_resref, "body", "body") {
            for mut mesh in meshes {
                mesh.override_tints = Some(body_tint.clone());
                all_meshes.push(mesh);
            }
            body_loaded = true;
            break;
        }
    }
    if !body_loaded {
        warn!(
            "Portrait body candidates all failed for '{ros_name}': {:?}",
            parts.body_parts
        );
    }

    let fhair_tint = TintChannels {
        channel1: hair_tint.channel1.clone(),
        channel2: white.channel1.clone(),
        channel3: white.channel1.clone(),
    };
    if let Some(ref head_resref) = parts.head_resref {
        match load_part(head_resref, "head", "head") {
            Ok(head_meshes) => {
                for mut mesh in head_meshes {
                    if mesh.name.to_lowercase().contains("_fhair") {
                        if parts.show_fhair {
                            mesh.part = "fhair".to_string();
                            mesh.tint_group = "hair".to_string();
                            mesh.override_tints = Some(fhair_tint.clone());
                            all_meshes.push(mesh);
                        }
                    } else {
                        mesh.override_tints = Some(head_tint.clone());
                        all_meshes.push(mesh);
                    }
                }
            }
            Err(e) => warn!("Portrait head '{head_resref}' failed for '{ros_name}': {e}"),
        }
    }

    if let Some(ref hair_resref) = parts.hair_resref {
        match load_part(hair_resref, "hair", "hair") {
            Ok(meshes) => {
                let swapped = swap_tint_2_3(&hair_tint);
                for mut mesh in meshes {
                    mesh.override_tints = Some(swapped.clone());
                    all_meshes.push(mesh);
                }
            }
            Err(e) => debug!("Portrait hair '{hair_resref}' not found: {e}"),
        }
    }

    for acc in parts
        .chest_accessories
        .iter_renderable(&accessory_body_prefix)
    {
        if !matches!(
            acc.slot,
            AccessorySlot::LtShoulder | AccessorySlot::RtShoulder
        ) {
            continue;
        }
        match load_part(&acc.resref, "accessory", "body") {
            Ok(meshes) => {
                for mut m in meshes {
                    m.attach_bone = Some(acc.attach_bone.to_string());
                    m.accessory_slot = Some(acc.slot);
                    m.override_tints = Some(swap_tint_2_3(acc.tints));
                    all_meshes.push(m);
                }
            }
            Err(e) => debug!("Portrait accessory '{}' skipped: {e}", acc.resref),
        }
    }

    if all_meshes.is_empty() {
        return Err(CommandError::Internal(
            "No portrait meshes could be loaded".to_string(),
        ));
    }

    info!(
        "Portrait model '{ros_name}': {} meshes in {:?} [{}]",
        all_meshes.len(),
        started.elapsed(),
        timings
            .iter()
            .map(|(step, elapsed)| format!("{step} {elapsed:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(ModelData {
        meshes: all_meshes,
        hooks: Vec::new(),
        hair: Vec::new(),
        helm: Vec::new(),
        skeleton,
        animations: Vec::new(),
        attached_parts: Vec::new(),
        secondary_skeletons: Vec::new(),
    })
}

#[tauri::command]
pub async fn load_character_part(
    state: State<'_, AppState>,
    part: String,
) -> CommandResult<ModelData> {
    let rm = state.resource_manager.read().await;
    let parts = {
        let game_data = state.game_data.read();
        let session = state.session.read();
        let character = session
            .character
            .as_ref()
            .ok_or(CommandError::NoCharacterLoaded)?;
        character
            .resolve_model_parts(&game_data, &rm)
            .ok_or_else(|| {
                CommandError::Internal("Failed to resolve character model parts".to_string())
            })?
    };

    let loader = PartLoader::new(&rm, &parts.skeleton_resref);
    let mut meshes: Vec<MeshData> = Vec::new();
    let mut extra = ExtraParts::default();

    match part.as_str() {
        "head" => meshes = load_head_meshes(&loader, &parts).0,
        "hair" => meshes = load_hair_meshes(&loader, &parts),
        "fhair" => meshes = load_head_meshes(&loader, &parts).1,
        "wings" | "tail" => load_wings_or_tail(&loader, &parts, &part, &mut meshes, &mut extra),
        "helm" => meshes = load_helm_meshes(&loader, &parts),
        "body" => meshes = load_body_meshes(&loader, &parts),
        "cloak" => load_cloak_meshes(&loader, &parts, &mut meshes, &mut extra),
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
        attached_parts: extra.attached,
        secondary_skeletons: extra.secondary_skeletons,
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
            .map(|wav_resrefs| wav_resrefs.iter().any(|wr| rm.has_resource(wr, "wav")))
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
