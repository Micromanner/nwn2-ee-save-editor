use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;

use tracing::{debug, warn};

use super::Character;
use crate::character::gff_helpers::gff_value_to_i32;
use crate::character::inventory::EquipmentSlot;
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::row_str;

use super::appearance_helpers::{
    TintChannels, build_nested_tint, build_tintable_struct, mesh_number, read_tint_from_tintable,
    resolve_armor_prefix, swap_tint_2_3,
};

fn raw_variation(fields: &IndexMap<String, GffValue<'static>>) -> i32 {
    fields
        .get("Variation")
        .and_then(gff_value_to_i32)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AppearanceState {
    pub race_id: i32,
    pub race_name: String,
    /// `racialsubtypes.2da` row of the `Subrace` byte (row 0 is Gold Dwarf). Several
    /// subraces can share one appearance row, so the UI matches on this first.
    pub subrace_id: i32,
    pub gender: i32,
    pub gender_name: String,

    pub appearance_type: i32,
    /// `appearance.2da` row name (TLK `STRING_REF`, else `LABEL`) so rows outside the
    /// playable race list (companion-specific rows) still display a name.
    pub appearance_type_name: String,

    pub appearance_head: i32,
    pub appearance_hair: i32,
    pub appearance_fhair: i32,

    pub tint_head: TintChannels,
    pub tint_hair: TintChannels,
    pub tint_body: TintChannels,

    pub color_tattoo1: i32,
    pub color_tattoo2: i32,

    pub height: f32,
    pub girth: f32,

    pub soundset: i32,

    pub wings: i32,
    pub wings_name: String,
    pub tail: i32,
    pub tail_name: String,

    pub available_heads: Vec<i32>,
    pub available_hairs: Vec<i32>,
    pub is_parts_based: bool,
    pub has_fhair_meshes: bool,
    pub never_draw_helmet: bool,
    pub cloak_tint: Option<TintChannels>,
    pub armor_tint: Option<TintChannels>,
    pub color_capabilities: crate::services::tint_analysis::ColorCapabilities,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AppearanceOption {
    pub id: i32,
    pub name: String,
    /// False for non-humanoid creature parts (dragon tails, gargoyle/solar
    /// wings, etc.) whose base-anim set differs from the body's. These are
    /// rigged for their own creature body and do not attach cleanly to a
    /// player character; the UI surfaces a note when one is selected.
    pub fits_humanoid: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CharacterModelParts {
    pub body_parts: Vec<String>,
    pub naked_body_resref: String,
    /// `None` for headless creature appearances (`nwn2_model_head = ****`).
    pub head_resref: Option<String>,
    pub hair_resref: Option<String>,
    pub show_fhair: bool,
    pub skeleton_resref: String,
    pub wings_resref: Option<String>,
    pub wings_skeleton_resref: Option<String>,
    pub wings_base_anims: Option<String>,
    pub tail_resref: Option<String>,
    pub tail_skeleton_resref: Option<String>,
    pub tail_base_anims: Option<String>,
    pub helm_candidates: Vec<String>,
    pub show_helmet: bool,
    pub boots_candidates: Vec<String>,
    pub gloves_candidates: Vec<String>,
    pub boots_tint: Option<TintChannels>,
    pub gloves_tint: Option<TintChannels>,
    pub has_standalone_boots: bool,
    pub has_standalone_gloves: bool,
    pub helm_tint: Option<TintChannels>,
    pub cloak_resref: Option<String>,
    pub cloak_tint: Option<TintChannels>,
    pub armor_tint: Option<TintChannels>,
    /// Accessories parsed off the equipped chest item (pauldrons, bracers, …).
    /// Empty / defaulted when no chest armor is equipped.
    pub chest_accessories: super::item_appearance::ArmorAccessories,
}

struct PartVisual {
    armor_prefix: Option<String>,
    variation: i32,
}

struct EquippedVisuals {
    armor_prefix: Option<String>,
    armor_variation: i32,
    helm_part: Option<PartVisual>,
    boots: Option<PartVisual>,
    gloves: Option<PartVisual>,
    cloak_variation: Option<i32>,
    cloak_tint: Option<TintChannels>,
    armor_tint: Option<TintChannels>,
    boots_tint: Option<TintChannels>,
    gloves_tint: Option<TintChannels>,
    has_standalone_boots: bool,
    has_standalone_gloves: bool,
    helm_tint: Option<TintChannels>,
    chest_accessories: super::item_appearance::ArmorAccessories,
}

impl Character {
    pub fn appearance_type(&self) -> i32 {
        self.get_i32("Appearance_Type").unwrap_or(0)
    }

    pub fn set_appearance_type(&mut self, value: i32) {
        self.set_u16("Appearance_Type", value as u16);
    }

    pub fn appearance_head(&self) -> i32 {
        self.get_i32("Appearance_Head").unwrap_or(0)
    }

    pub fn set_appearance_head(&mut self, value: i32) {
        self.set_byte("Appearance_Head", value as u8);
    }

    pub fn appearance_hair(&self) -> i32 {
        self.get_i32("Appearance_Hair").unwrap_or(0)
    }

    pub fn set_appearance_hair(&mut self, value: i32) {
        self.set_byte("Appearance_Hair", value as u8);
    }

    pub fn appearance_fhair(&self) -> i32 {
        self.get_i32("Appearance_FHair").unwrap_or(0)
    }

    pub fn set_appearance_fhair(&mut self, value: i32) {
        self.set_byte("Appearance_FHair", value as u8);
    }

    pub fn color_tattoo1(&self) -> i32 {
        self.get_i32("Color_Tattoo1").unwrap_or(0)
    }

    pub fn set_color_tattoo1(&mut self, value: i32) {
        self.set_byte("Color_Tattoo1", value as u8);
    }

    pub fn color_tattoo2(&self) -> i32 {
        self.get_i32("Color_Tattoo2").unwrap_or(0)
    }

    pub fn set_color_tattoo2(&mut self, value: i32) {
        self.set_byte("Color_Tattoo2", value as u8);
    }

    pub fn soundset(&self) -> i32 {
        self.get_i32("SoundSetFile").unwrap_or(0)
    }

    pub fn set_soundset(&mut self, value: i32) {
        self.set_u16("SoundSetFile", value as u16);
    }

    pub fn wings(&self) -> i32 {
        self.get_i32("Wings").unwrap_or(0)
    }

    pub fn set_wings(&mut self, value: i32) {
        self.set_byte("Wings", value.clamp(0, 255) as u8);
    }

    pub fn tail(&self) -> i32 {
        self.get_i32("Tail").unwrap_or(0)
    }

    pub fn set_tail(&mut self, value: i32) {
        self.set_byte("Tail", value.clamp(0, 255) as u8);
    }

    pub fn never_draw_helmet(&self) -> bool {
        self.get_i32("NeverDrawHelmet").unwrap_or(0) != 0
    }

    pub fn set_never_draw_helmet(&mut self, value: bool) {
        self.set_int("NeverDrawHelmet", i32::from(value));
    }

    // -- Tint reading --

    fn read_tint_channels_nested(&self, field: &str) -> TintChannels {
        let Some(outer) = self.get_struct_owned(field) else {
            return TintChannels::default();
        };
        let tintable = match outer.get("Tintable") {
            Some(GffValue::StructOwned(s)) => s.as_ref().clone(),
            Some(GffValue::Struct(lazy)) => lazy.force_load(),
            _ => return TintChannels::default(),
        };
        read_tint_from_tintable(&tintable)
    }

    /// GFF head tint stores channels as (skin, eyes, eyebrows). Our in-memory
    /// TintChannels is (skin, eyebrows, eyes) — the order the UI pickers and
    /// viewer masks use — so swap 2/3 on both read and write. Hair needs no
    /// swap. External refs: DragonCoast character-modification guide, NWN2Wiki.
    pub fn tint_head(&self) -> TintChannels {
        swap_tint_2_3(&self.read_tint_channels_nested("Tint_Head"))
    }

    pub fn tint_hair(&self) -> TintChannels {
        self.read_tint_channels_nested("Tint_Hair")
    }

    // -- Tint writing --

    pub fn set_tint_head(&mut self, tints: &TintChannels) {
        let nested = build_nested_tint(&swap_tint_2_3(tints));
        self.set_struct("Tint_Head", nested);
    }

    pub fn set_tint_hair(&mut self, tints: &TintChannels) {
        let nested = build_nested_tint(tints);
        self.set_struct("Tint_Hair", nested);
    }

    /// Write Tint_Head from raw GFF-order channels (no UI 2/3 swap). Used by
    /// the calibration diagnostic to probe engine channel -> region mapping.
    pub fn write_raw_tint_head(&mut self, tints: &TintChannels) {
        let nested = build_nested_tint(tints);
        self.set_struct("Tint_Head", nested);
    }

    /// Root-level body tint (`Tintable → Tint → 1/2/3`). The engine tints
    /// TAIL and WING models and the creature's own unarmored body (root
    /// `ArmorVisualType`/`Variation`) with this layer; companions ship real
    /// colours here (Neeshka's tan leathers). Raw GFF channel order — no 2/3 swap.
    pub fn tint_body(&self) -> TintChannels {
        let Some(tintable) = self.get_struct_owned("Tintable") else {
            return TintChannels::default();
        };
        read_tint_from_tintable(&tintable)
    }

    pub fn set_tint_body(&mut self, tints: &TintChannels) {
        self.set_struct("Tintable", build_tintable_struct(tints));
    }

    /// Copy the raw `Tint_Head` tint subtree into the root `Tintable` —
    /// byte-identical to the proven community workflow for skin-tone tails
    /// (Wings N Tails EX). No-op when the character has no `Tint_Head`.
    pub fn set_tint_body_from_head(&mut self) {
        if self.get_struct_owned("Tint_Head").is_none() {
            warn!("set_tint_body_from_head: character has no Tint_Head, skipping");
            return;
        }
        let raw_head = self.read_tint_channels_nested("Tint_Head");
        self.set_tint_body(&raw_head);
    }

    // -- ModelScale (height = z, girth = x synced with y) --

    fn model_scale_struct(&self) -> (f32, f32, f32) {
        let Some(scale_struct) = self.get_struct_owned("ModelScale") else {
            return (1.0, 1.0, 1.0);
        };
        let get_f = |key: &str| -> f32 {
            match scale_struct.get(key) {
                Some(GffValue::Float(v)) => *v,
                _ => 1.0,
            }
        };
        (get_f("x"), get_f("y"), get_f("z"))
    }

    pub fn height(&self) -> f32 {
        self.model_scale_struct().2
    }

    pub fn girth(&self) -> f32 {
        self.model_scale_struct().0
    }

    pub fn set_height(&mut self, height: f32) {
        let (x, y, _) = self.model_scale_struct();
        let mut map = IndexMap::new();
        map.insert("x".to_string(), GffValue::Float(x));
        map.insert("y".to_string(), GffValue::Float(y));
        map.insert("z".to_string(), GffValue::Float(height));
        self.set_struct("ModelScale", map);
    }

    pub fn set_girth(&mut self, girth: f32) {
        let (_, _, z) = self.model_scale_struct();
        let mut map = IndexMap::new();
        map.insert("x".to_string(), GffValue::Float(girth));
        map.insert("y".to_string(), GffValue::Float(girth));
        map.insert("z".to_string(), GffValue::Float(z));
        self.set_struct("ModelScale", map);
    }

    fn resolve_appearance_name(game_data: &GameData, row_id: i32) -> String {
        let name = game_data
            .get_table("appearance")
            .and_then(|table| table.get_by_id(row_id))
            .and_then(|row| {
                row_str(&row, "string_ref")
                    .and_then(|s| s.parse::<i32>().ok())
                    .and_then(|strref| game_data.get_string(strref))
                    .or_else(|| row_str(&row, "label"))
            });
        name.unwrap_or_else(|| format!("{row_id}"))
    }

    fn resolve_label_from_2da(game_data: &GameData, table_name: &str, row_id: i32) -> String {
        let Some(table) = game_data.get_table(table_name) else {
            return format!("{row_id}");
        };
        let Some(row) = table.get_by_id(row_id) else {
            return format!("{row_id}");
        };
        row_str(&row, "label").unwrap_or_else(|| format!("{row_id}"))
    }

    fn discover_available_variants(
        resource_manager: &crate::services::resource_manager::ResourceManager,
        prefix: &str,
    ) -> Vec<i32> {
        let mdbs = resource_manager.list_resources_by_prefix(&prefix.to_lowercase(), "mdb");
        let prefix_lower = prefix.to_lowercase();
        let mut variants: Vec<i32> = mdbs
            .iter()
            .filter_map(|filename| {
                let name = filename.trim_end_matches(".mdb");
                let num_str = name.strip_prefix(&prefix_lower)?;
                num_str.parse::<i32>().ok()
            })
            .collect();
        variants.sort_unstable();
        variants.dedup();
        variants
    }

    pub fn get_appearance_state(
        &self,
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
        model_parts: Option<&CharacterModelParts>,
    ) -> AppearanceState {
        let gender_id = self.gender();
        let gender_name = match gender_id {
            0 => "Male",
            1 => "Female",
            _ => "Unknown",
        }
        .to_string();

        let wings_id = self.wings();
        let wings_name = Self::resolve_label_from_2da(game_data, "wingmodel", wings_id);

        let tail_id = self.tail();
        let tail_name = Self::resolve_label_from_2da(game_data, "tailmodel", tail_id);

        let (available_heads, available_hairs, is_parts_based) =
            self.discover_model_variants(game_data, resource_manager);

        let has_fhair_meshes = model_parts
            .and_then(|parts| parts.head_resref.as_deref())
            .map(|head| {
                crate::services::model_loader::head_has_fhair_meshes(resource_manager, head)
            })
            .unwrap_or(false);
        let cloak_tint = model_parts.and_then(|p| p.cloak_tint.clone());
        let armor_tint = model_parts.and_then(|p| p.armor_tint.clone());
        debug!("AppearanceState: armor_tint={armor_tint:?}, cloak_tint={cloak_tint:?}");

        AppearanceState {
            race_id: self.race_id().0,
            race_name: self.race_display_name(game_data),
            subrace_id: self.subrace_index().unwrap_or(0),
            gender: gender_id,
            gender_name,

            appearance_type: self.appearance_type(),
            appearance_type_name: Self::resolve_appearance_name(game_data, self.appearance_type()),

            appearance_head: self.appearance_head(),
            appearance_hair: self.appearance_hair(),
            appearance_fhair: self.appearance_fhair(),

            tint_head: self.tint_head(),
            tint_hair: self.tint_hair(),
            tint_body: self.tint_body(),

            color_tattoo1: self.color_tattoo1(),
            color_tattoo2: self.color_tattoo2(),

            height: self.height(),
            girth: self.girth(),

            soundset: self.soundset(),

            wings: wings_id,
            wings_name,
            tail: tail_id,
            tail_name,

            available_heads,
            available_hairs,
            is_parts_based,
            has_fhair_meshes,
            never_draw_helmet: self.never_draw_helmet(),
            cloak_tint,
            armor_tint,
            color_capabilities: crate::services::tint_analysis::ColorCapabilities::default(),
        }
    }

    pub(crate) fn discover_model_variants(
        &self,
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
    ) -> (Vec<i32>, Vec<i32>, bool) {
        let appearance_id = self.appearance_type();
        let Some(appearance_table) = game_data.get_table("appearance") else {
            return (Vec::new(), Vec::new(), false);
        };
        let Some(row) = appearance_table.get_by_id(appearance_id) else {
            return (Vec::new(), Vec::new(), false);
        };

        let model_type = row_str(&row, "modeltype").unwrap_or_default();
        if model_type.to_uppercase() != "P" {
            return (Vec::new(), Vec::new(), false);
        }

        let gender_id = self.gender();
        let gender_letter = if let Some(gender_table) = game_data.get_table("gender")
            && let Some(gender_row) = gender_table.get_by_id(gender_id)
        {
            row_str(&gender_row, "gender").unwrap_or_else(|| "M".to_string())
        } else {
            "M".to_string()
        };

        let head_prefix = row_str(&row, "nwn2_model_head")
            .unwrap_or_default()
            .replace('?', &gender_letter);
        let hair_prefix = row_str(&row, "nwn2_model_hair")
            .unwrap_or_default()
            .replace('?', &gender_letter);
        let available_heads = Self::discover_available_variants(resource_manager, &head_prefix);
        let available_hairs = Self::discover_available_variants(resource_manager, &hair_prefix);

        (available_heads, available_hairs, true)
    }

    /// Every parts-based (`MODELTYPE = P`) `appearance.2da` row that has a head
    /// template. Covers the companion/NPC rows (`N_Human`, `N_Elf`, ...) that no
    /// playable race or subrace points at, so the appearance dropdown can offer
    /// them and a character switched away from one can be switched back.
    pub fn get_parts_based_appearances(game_data: &GameData) -> Vec<AppearanceOption> {
        let Some(table) = game_data.get_table("appearance") else {
            return Vec::new();
        };
        (0..table.row_count())
            .filter_map(|i| {
                let id = i as i32;
                let row = table.get_by_id(id)?;
                let model_type = row_str(&row, "modeltype")?;
                if !model_type.eq_ignore_ascii_case("P") {
                    return None;
                }
                row_str(&row, "nwn2_model_head")?;
                Some(AppearanceOption {
                    id,
                    name: Self::resolve_appearance_name(game_data, id),
                    fits_humanoid: true,
                })
            })
            .collect()
    }

    /// Enumerate rows from a 2DA table, skipping any whose `model` MDB is
    /// missing from the resource index. Row 0 ("None") is kept regardless, so
    /// the user can still clear the selection.
    pub fn get_available_model_options(
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
        table_name: &str,
        body_skeleton_resref: Option<&str>,
    ) -> Vec<AppearanceOption> {
        let Some(table) = game_data.get_table(table_name) else {
            return Vec::new();
        };

        let mut options = Vec::new();
        for i in 0..table.row_count() {
            let id = i as i32;
            let Some(row) = table.get_by_id(id) else {
                continue;
            };
            let Some(label) = row_str(&row, "label") else {
                continue;
            };
            if id == 0 {
                options.push(AppearanceOption {
                    id,
                    name: label,
                    fits_humanoid: true,
                });
                continue;
            }
            let Some(model) = row_str(&row, "model") else {
                continue;
            };
            if !resource_manager.has_resource(&model, "mdb") {
                continue;
            }
            let base_anims = row_str(&row, "nwn2_baseanims");
            let fits_humanoid = match body_skeleton_resref {
                Some(skel) => {
                    crate::services::model_loader::is_synced_base_anims(skel, base_anims.as_deref())
                }
                None => true,
            };
            options.push(AppearanceOption {
                id,
                name: label,
                fits_humanoid,
            });
        }
        options
    }

    /// Resolve this character's race/gender body prefix
    /// (e.g. `P_EEM` for elf male, `P_HHF` for human female).
    /// Returns `None` if the appearance/gender lookup tables are missing.
    /// Drives the item viewer so previewed armor sits on the same body the
    /// character actually has in-game — drow uses the elf model so a
    /// hardcoded `P_HHM` would produce a totally different silhouette.
    pub fn body_prefix(&self, game_data: &GameData) -> Option<String> {
        let appearance_id = self.appearance_type();
        let row = game_data
            .get_table("appearance")?
            .get_by_id(appearance_id)?;
        let gender_id = self.gender();
        let gender_row = game_data.get_table("gender")?.get_by_id(gender_id)?;
        let gender_letter = row_str(&gender_row, "gender").unwrap_or_else(|| "M".to_string());
        let body_template = row_str(&row, "nwn2_model_body")?;
        Some(body_template.replace('?', &gender_letter))
    }

    /// Resolve this character's body skeleton resref (e.g. `P_HHF_skel`), the
    /// same value `resolve_model_parts` derives. Used to decide whether a
    /// tail/wing appearance's base-anim set matches the body (fits a humanoid).
    pub fn body_skeleton_resref(&self, game_data: &GameData) -> Option<String> {
        let appearance_id = self.appearance_type();
        let row = game_data
            .get_table("appearance")?
            .get_by_id(appearance_id)?;
        let gender_id = self.gender();
        let gender_row = game_data.get_table("gender")?.get_by_id(gender_id)?;
        let gender_letter = row_str(&gender_row, "gender").unwrap_or_else(|| "M".to_string());
        let skel_template = row_str(&row, "nwn2_skeleton_file")?;
        Some(skel_template.replace('?', &gender_letter))
    }

    pub fn resolve_model_parts(
        &self,
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
    ) -> Option<CharacterModelParts> {
        let appearance_id = self.appearance_type();
        let appearance_table = game_data.get_table("appearance").or_else(|| {
            warn!("appearance.2da not loaded");
            None
        })?;
        let row = appearance_table.get_by_id(appearance_id).or_else(|| {
            warn!("No appearance row for id {appearance_id}");
            None
        })?;

        let gender_id = self.gender();
        let gender_table = game_data.get_table("gender").or_else(|| {
            warn!("gender.2da not loaded");
            None
        })?;
        let gender_row = gender_table.get_by_id(gender_id).or_else(|| {
            warn!("No gender row for id {gender_id}");
            None
        })?;
        let gender_letter = row_str(&gender_row, "gender").unwrap_or_else(|| "M".to_string());

        let body_template = row_str(&row, "nwn2_model_body").or_else(|| {
            warn!("No nwn2_model_body for appearance {appearance_id}");
            None
        })?;
        let skel_template = row_str(&row, "nwn2_skeleton_file").or_else(|| {
            warn!("No nwn2_skeleton_file for appearance {appearance_id}");
            None
        })?;

        let body_prefix = body_template.replace('?', &gender_letter);
        let skeleton_resref = skel_template.replace('?', &gender_letter);

        let model_type = row_str(&row, "modeltype").unwrap_or_default();

        // Creature bodies (modeltype L/F/S) follow the same
        // `{body}_{armor}_Body{NN}` naming as player bodies but ship no `_NK_`
        // variant; their bare fallback is the cloth body.
        let naked_body_resref = if model_type.eq_ignore_ascii_case("P") {
            format!("{body_prefix}_NK_Body01")
        } else {
            format!("{body_prefix}_CL_Body01")
        };

        // Extract all equipped visual info in one pass
        let equip_visuals = self.resolve_equipped_visuals(game_data, resource_manager);
        let armor_prefix = equip_visuals.armor_prefix.as_deref();
        let var = equip_visuals.armor_variation;
        let mut body_parts: Vec<String> = armor_prefix
            .iter()
            .map(|pfx| format!("{body_prefix}_{pfx}_Body{var:02}"))
            .collect();
        // Fallback to variation 01 if the requested variation doesn't exist
        if var != 1
            && let Some(pfx) = armor_prefix
        {
            body_parts.push(format!("{body_prefix}_{pfx}_Body01"));
        }
        let cloth_default = format!("{body_prefix}_CL_Body01");
        if !body_parts.contains(&cloth_default) {
            body_parts.push(cloth_default);
        }
        debug!("Body model candidates: {body_parts:?}");

        let head_id = self.appearance_head();
        let head_resref = row_str(&row, "nwn2_model_head").map(|head_template| {
            let head_prefix = head_template.replace('?', &gender_letter);
            format!("{head_prefix}{head_id:02}")
        });

        // Hair variants start at 1; NPC heads with hair baked into the head
        // mesh store 0, which has no mesh and therefore nothing to tint.
        let hair_id = self.appearance_hair();
        let hair_resref = row_str(&row, "nwn2_model_hair")
            .filter(|_| hair_id > 0)
            .map(|hair_template| {
                let hair_prefix = hair_template.replace('?', &gender_letter);
                format!("{hair_prefix}{hair_id:02}")
            });

        let show_fhair = self.appearance_fhair() > 0;

        let (wings_resref, wings_skeleton_resref, wings_base_anims) = if self.wings() > 0 {
            let row = game_data
                .get_table("wingmodel")
                .and_then(|t| t.get_by_id(self.wings()));
            let model = row.as_ref().and_then(|r| row_str(r, "model"));
            let skel = row.as_ref().and_then(|r| row_str(r, "nwn2_skeleton_file"));
            let anims = row.as_ref().and_then(|r| row_str(r, "nwn2_baseanims"));
            (model, skel, anims)
        } else {
            (None, None, None)
        };

        let (tail_resref, tail_skeleton_resref, tail_base_anims) = if self.tail() > 0 {
            let row = game_data
                .get_table("tailmodel")
                .and_then(|t| t.get_by_id(self.tail()));
            let model = row.as_ref().and_then(|r| row_str(r, "model"));
            let skel = row.as_ref().and_then(|r| row_str(r, "nwn2_skeleton_file"));
            let anims = row.as_ref().and_then(|r| row_str(r, "nwn2_baseanims"));
            (model, skel, anims)
        } else {
            (None, None, None)
        };

        // Legacy helms without ArmorVisualType borrow the chest material so
        // the viewer still renders something.
        let helm_candidates: Vec<String> = equip_visuals
            .helm_part
            .as_ref()
            .and_then(|pv| {
                let pfx = pv.armor_prefix.as_deref().or(armor_prefix)?;
                Some(format!("{body_prefix}_{pfx}_Helm{:02}", pv.variation))
            })
            .into_iter()
            .collect();

        let boots_candidates = Self::build_part_candidates(
            &body_prefix,
            armor_prefix,
            "Boots",
            equip_visuals.boots.as_ref(),
        );
        let gloves_candidates = Self::build_part_candidates(
            &body_prefix,
            armor_prefix,
            "Gloves",
            equip_visuals.gloves.as_ref(),
        );

        let cloak_resref = equip_visuals
            .cloak_variation
            .map(|var| format!("{body_prefix}_CL_Cloak{var:02}"));

        let show_helmet = !self.never_draw_helmet();
        debug!(
            "Helm={helm_candidates:?}, Boots={boots_candidates:?}, Gloves={gloves_candidates:?}, Cloak={cloak_resref:?}"
        );

        Some(CharacterModelParts {
            body_parts,
            naked_body_resref,
            head_resref,
            hair_resref,
            show_fhair,
            skeleton_resref,
            wings_resref,
            wings_skeleton_resref,
            wings_base_anims,
            tail_resref,
            tail_skeleton_resref,
            tail_base_anims,
            helm_candidates,
            show_helmet,
            boots_candidates,
            gloves_candidates,
            boots_tint: equip_visuals.boots_tint,
            gloves_tint: equip_visuals.gloves_tint,
            has_standalone_boots: equip_visuals.has_standalone_boots,
            has_standalone_gloves: equip_visuals.has_standalone_gloves,
            helm_tint: equip_visuals.helm_tint,
            cloak_resref,
            cloak_tint: equip_visuals.cloak_tint,
            armor_tint: equip_visuals.armor_tint,
            chest_accessories: equip_visuals.chest_accessories,
        })
    }

    fn build_part_candidates(
        body_prefix: &str,
        chest_armor_prefix: Option<&str>,
        part_name: &str,
        part_visual: Option<&PartVisual>,
    ) -> Vec<String> {
        let mut candidates = Vec::new();
        let mut push = |c: String| {
            if !candidates.contains(&c) {
                candidates.push(c);
            }
        };

        if let Some(pv) = part_visual {
            let var = pv.variation;
            if let Some(pfx) = &pv.armor_prefix {
                push(format!("{body_prefix}_{pfx}_{part_name}{var:02}"));
                if var != 1 {
                    push(format!("{body_prefix}_{pfx}_{part_name}01"));
                }
                return candidates;
            }
            if let Some(pfx) = chest_armor_prefix {
                push(format!("{body_prefix}_{pfx}_{part_name}{var:02}"));
            }
        }

        // Default: the chest armor prefix with variation 01
        if let Some(pfx) = chest_armor_prefix {
            push(format!("{body_prefix}_{pfx}_{part_name}01"));
        }
        // Last resort: CL (Cloth) which always has boots/gloves
        push(format!("{body_prefix}_CL_{part_name}01"));

        candidates
    }

    fn parse_part_visual(
        part_struct: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Option<PartVisual> {
        let visual_type = part_struct
            .get("ArmorVisualType")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        let variation = raw_variation(part_struct);
        if variation < 0 {
            return None;
        }

        Some(PartVisual {
            armor_prefix: resolve_armor_prefix(game_data, visual_type),
            variation: mesh_number(variation),
        })
    }

    fn resolve_equipped_visuals(
        &self,
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
    ) -> EquippedVisuals {
        let mut result = EquippedVisuals {
            armor_prefix: None,
            armor_variation: 1,
            helm_part: None,
            boots: None,
            gloves: None,
            cloak_variation: None,
            cloak_tint: None,
            armor_tint: None,
            boots_tint: None,
            gloves_tint: None,
            has_standalone_boots: false,
            has_standalone_gloves: false,
            helm_tint: None,
            chest_accessories: super::item_appearance::ArmorAccessories::default(),
        };

        let equip_list = self.get_list_owned("Equip_ItemList").unwrap_or_default();
        let never_show_armor = self.get_i32("NeverShowArmor") == Some(1);
        if never_show_armor {
            debug!("NeverShowArmor set: chest item visuals ignored, using creature body");
        }

        let baseitems_table = game_data.get_table("baseitems");
        let mut chest_boots = None;
        let mut chest_gloves = None;
        let mut chest_boots_tint = None;
        let mut chest_gloves_tint = None;

        for item_struct in &equip_list {
            // Determine equipment slot from BaseItem -> baseitems.2da EquipableSlots
            // This is more reliable than __struct_id__ which some saves don't set correctly
            let base_item = item_struct
                .get("BaseItem")
                .and_then(gff_value_to_i32)
                .unwrap_or(-1);
            let equip_slots = baseitems_table
                .as_ref()
                .and_then(|t| t.get_by_id(base_item))
                .and_then(|row| {
                    row_str(&row, "equipableslots").and_then(|s| {
                        let s = s.trim();
                        if let Some(hex) = s.strip_prefix("0x") {
                            u32::from_str_radix(hex, 16).ok()
                        } else {
                            s.parse::<u32>().ok()
                        }
                    })
                })
                .unwrap_or(0);

            let is_chest = equip_slots & EquipmentSlot::Chest.to_bitmask() != 0;
            let is_head = equip_slots & EquipmentSlot::Head.to_bitmask() != 0;
            let is_boots = equip_slots & EquipmentSlot::Boots.to_bitmask() != 0;
            let is_gloves = equip_slots & EquipmentSlot::Gloves.to_bitmask() != 0;
            let is_cloak = equip_slots & EquipmentSlot::Cloak.to_bitmask() != 0;

            let raw_equip_str = baseitems_table
                .as_ref()
                .and_then(|t| t.get_by_id(base_item))
                .and_then(|row| row_str(&row, "equipableslots"));
            debug!(
                "Equip item: BaseItem={base_item}, raw_equipslots={raw_equip_str:?}, parsed=0x{equip_slots:04x}, chest={is_chest}, cloak={is_cloak}"
            );

            if is_chest && !never_show_armor {
                let visual_type = item_struct
                    .get("ArmorVisualType")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                result.armor_prefix = resolve_armor_prefix(game_data, visual_type);
                result.armor_variation = mesh_number(raw_variation(item_struct));
                result.armor_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                debug!(
                    "Chest ArmorVisualType {visual_type}, Variation {} -> prefix {:?}, tint {:?}",
                    result.armor_variation, result.armor_prefix, result.armor_tint
                );

                // Parse all 22 AC* accessory slots (pauldrons, bracers, greaves,
                // …) so the character viewer can render them the same way the
                // item viewer does.
                result.chest_accessories =
                    super::item_appearance::ItemAppearance::from_gff(item_struct).accessories;

                // Boots and Gloves are nested structs with their own ArmorVisualType + Variation
                let boots_fields = match item_struct.get("Boots") {
                    Some(GffValue::StructOwned(s)) => Some(s.as_ref().clone()),
                    Some(GffValue::Struct(lazy)) => Some(lazy.force_load()),
                    _ => None,
                };
                if let Some(ref fields) = boots_fields {
                    chest_boots = Self::parse_part_visual(fields, game_data);
                    chest_boots_tint =
                        super::item_appearance::read_nested_armor_part(item_struct, "Boots")
                            .and_then(|p| p.tints);
                    debug!(
                        "Boots part: {:?}",
                        chest_boots.as_ref().map(|b| (&b.armor_prefix, b.variation))
                    );
                }
                let gloves_fields = match item_struct.get("Gloves") {
                    Some(GffValue::StructOwned(s)) => Some(s.as_ref().clone()),
                    Some(GffValue::Struct(lazy)) => Some(lazy.force_load()),
                    _ => None,
                };
                if let Some(ref fields) = gloves_fields {
                    chest_gloves = Self::parse_part_visual(fields, game_data);
                    chest_gloves_tint =
                        super::item_appearance::read_nested_armor_part(item_struct, "Gloves")
                            .and_then(|p| p.tints);
                    debug!(
                        "Gloves part: {:?}",
                        chest_gloves
                            .as_ref()
                            .map(|g| (&g.armor_prefix, g.variation))
                    );
                }
            }

            if is_head {
                result.helm_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                result.helm_part = Some(Self::parse_item_part_visual(item_struct, game_data));
            }

            if is_boots {
                result.has_standalone_boots = true;
                result.boots = Some(Self::parse_item_part_visual(item_struct, game_data));
                result.boots_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                debug!(
                    "Boots from slot: {:?}, tint={:?}",
                    result
                        .boots
                        .as_ref()
                        .map(|b| (&b.armor_prefix, b.variation)),
                    result.boots_tint
                );
            }
            if is_gloves {
                result.has_standalone_gloves = true;
                result.gloves = Some(Self::parse_item_part_visual(item_struct, game_data));
                let is_bracer = baseitems_table
                    .as_ref()
                    .and_then(|t| t.get_by_id(base_item))
                    .and_then(|row| row_str(&row, "label"))
                    .is_some_and(|label| super::item_appearance::is_bracer_label(&label));
                if is_bracer && let Some(ref mut gloves) = result.gloves {
                    gloves.variation = 1;
                }
                result.gloves_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                debug!(
                    "Gloves from slot: {:?}, tint={:?}",
                    result
                        .gloves
                        .as_ref()
                        .map(|g| (&g.armor_prefix, g.variation)),
                    result.gloves_tint
                );
            }

            if is_cloak {
                let variation = mesh_number(raw_variation(item_struct));
                result.cloak_variation = Some(variation);
                result.cloak_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                debug!(
                    "Cloak variation: {variation}, tint: {:?}",
                    result.cloak_tint
                );
            }
        }

        if !result.has_standalone_boots {
            result.boots = chest_boots;
            result.boots_tint = chest_boots_tint;
        }
        if !result.has_standalone_gloves {
            result.gloves = chest_gloves;
            result.gloves_tint = chest_gloves_tint;
        }
        if result.armor_prefix.is_none() {
            self.apply_creature_body_visual(game_data, &mut result);
        }
        result
    }

    /// The creature root struct carries its own `ArmorVisualType` and
    /// `Variation`: the outfit the engine renders when no chest item is
    /// equipped (Elanee's robe, Neeshka's leathers), coloured by the root
    /// `Tintable` (the root `ArmorTint` is all white on every companion).
    /// Optional root `Boots`/`Gloves` structs (same shape as a chest item's
    /// nested parts: Qara `LE_Boots07`, Shandra `CL_Boots04`) supply the
    /// footwear; without them the material's `*01` parts (bare hands and
    /// feet for cloth) apply, since bodies like Elanee's robe ship none.
    /// Verified on live saves 2026-09-11; the fields are 0/0 for most
    /// characters, which resolves to the plain `CL_Body01` cloth body.
    fn apply_creature_body_visual(&self, game_data: &GameData, result: &mut EquippedVisuals) {
        let visual_type = self.get_i32("ArmorVisualType").unwrap_or(0);
        let variation = mesh_number(self.get_i32("Variation").unwrap_or(0));
        result.armor_prefix = resolve_armor_prefix(game_data, visual_type);
        result.armor_variation = variation;

        let root_part = |key: &str| {
            let fields = self.get_struct_owned(key)?;
            let visual = Self::parse_part_visual(&fields, game_data)?;
            let tint = match fields.get("ArmorTint") {
                Some(GffValue::StructOwned(s)) => {
                    Some(super::appearance_helpers::read_tint_channels(s))
                }
                Some(GffValue::Struct(lazy)) => Some(
                    super::appearance_helpers::read_tint_channels(&lazy.force_load()),
                ),
                _ => None,
            };
            debug!(
                "Creature root {key}: {:?} var {}, tint {tint:?}",
                visual.armor_prefix, visual.variation
            );
            Some((visual, tint))
        };
        if !result.has_standalone_boots
            && let Some((visual, tint)) = root_part("Boots")
        {
            result.boots = Some(visual);
            result.boots_tint = tint;
        }
        if !result.has_standalone_gloves
            && let Some((visual, tint)) = root_part("Gloves")
        {
            result.gloves = Some(visual);
            result.gloves_tint = tint;
        }
        // Same channel order as a chest item's `Tintable`: consumers swap 2/3
        // exactly as they do for item tints (Neeshka's tan vest, Qara's red
        // tunic verified in-game 2026-09-11).
        let tint = self.tint_body();
        if tint.has_color() {
            result.armor_tint = Some(tint);
        }
        debug!(
            "Creature body: ArmorVisualType {visual_type}, Variation {variation} -> prefix {:?}, tint {:?}",
            result.armor_prefix, result.armor_tint
        );
    }

    /// Read an item's tint from its `Tintable` struct, falling back to the
    /// item template (.uti) when the saved tint is all zero.
    pub(crate) fn read_item_tint_with_fallback(
        item_struct: &IndexMap<String, GffValue<'static>>,
        resource_manager: &crate::services::resource_manager::ResourceManager,
    ) -> Option<TintChannels> {
        let tintable = match item_struct.get("Tintable") {
            Some(GffValue::StructOwned(s)) => Some(s.as_ref().clone()),
            Some(GffValue::Struct(lazy)) => Some(lazy.force_load()),
            _ => None,
        };
        if let Some(ref t) = tintable {
            let tint = read_tint_from_tintable(t);
            if tint.has_color() {
                return Some(tint);
            }
        }
        Self::load_template_tint(item_struct, resource_manager)
    }

    pub(crate) fn load_template_tint(
        item_struct: &IndexMap<String, GffValue<'static>>,
        resource_manager: &crate::services::resource_manager::ResourceManager,
    ) -> Option<TintChannels> {
        let resref = match item_struct.get("TemplateResRef") {
            Some(GffValue::ResRef(s)) => s.to_string(),
            _ => return None,
        };
        let uti_bytes = resource_manager.get_resource_bytes(&resref, "uti").ok()?;
        let gff = crate::parsers::gff::parser::GffParser::from_bytes(uti_bytes).ok()?;
        let fields = gff.read_struct_fields(0).ok()?;
        let tintable = match fields.get("Tintable") {
            Some(GffValue::Struct(lazy)) => lazy.force_load(),
            _ => return None,
        };
        let tint = read_tint_from_tintable(&tintable);
        if tint.has_color() {
            debug!(
                "Template '{}' tint: ch1=({},{},{},{}), ch2=({},{},{},{}), ch3=({},{},{},{})",
                resref,
                tint.channel1.r,
                tint.channel1.g,
                tint.channel1.b,
                tint.channel1.a,
                tint.channel2.r,
                tint.channel2.g,
                tint.channel2.b,
                tint.channel2.a,
                tint.channel3.r,
                tint.channel3.g,
                tint.channel3.b,
                tint.channel3.a,
            );
            Some(tint)
        } else {
            None
        }
    }

    fn parse_item_part_visual(
        item_struct: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> PartVisual {
        let visual_type = item_struct
            .get("ArmorVisualType")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        let variation = mesh_number(raw_variation(item_struct));

        debug!("Item part visual: ArmorVisualType={visual_type}, Variation(mesh)={variation}");

        PartVisual {
            armor_prefix: resolve_armor_prefix(game_data, visual_type),
            variation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::Character;
    use crate::character::appearance_helpers::TintChannel;
    use crate::parsers::gff::GffValue;
    use indexmap::IndexMap;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("Appearance_Head".to_string(), GffValue::Byte(3));
        fields.insert("Appearance_Hair".to_string(), GffValue::Byte(2));
        fields.insert("Appearance_FHair".to_string(), GffValue::Byte(1));
        fields.insert("Color_Tattoo1".to_string(), GffValue::Byte(2));
        fields.insert("Color_Tattoo2".to_string(), GffValue::Byte(7));
        fields.insert("SoundSetFile".to_string(), GffValue::Word(42));
        fields.insert("Wings".to_string(), GffValue::Byte(0));
        fields.insert("Tail".to_string(), GffValue::Byte(0));
        fields.insert("Race".to_string(), GffValue::Byte(1));
        fields.insert("Gender".to_string(), GffValue::Byte(0));
        Character::from_gff(fields)
    }

    #[test]
    fn test_appearance_head() {
        let character = create_test_character();
        assert_eq!(character.appearance_head(), 3);
    }

    fn game_data_with_armor_and_baseitems() -> GameData {
        use crate::loaders::types::LoadedTable;
        use crate::parsers::tda::TDAParser;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let mut game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));
        let tables = [
            (
                "armor",
                "2DA V2.0

   Label   Prefix
0  Cloth   CL
1  Padded  CP
2  Leather LE
",
            ),
            (
                "baseitems",
                "2DA V2.0

   Label   EquipableSlots
0  Armor   0x00002
",
            ),
            (
                "appearance",
                "2DA V2.0

   LABEL  modeltype  nwn2_model_body  nwn2_skeleton_file
0  Human  P          P_HH?            P_HH?_Skel
",
            ),
            (
                "gender",
                "2DA V2.0

   LABEL   gender
0  Male    M
1  Female  F
",
            ),
        ];
        for (name, content) in tables {
            let mut parser = TDAParser::new();
            parser.parse_from_bytes(content.as_bytes()).unwrap();
            game_data.tables.insert(
                name.to_string(),
                LoadedTable::new(name.to_string(), Arc::new(parser)),
            );
        }
        game_data
    }

    fn test_resource_manager() -> crate::services::resource_manager::ResourceManager {
        use crate::config::nwn2_paths::NWN2Paths;
        use std::sync::Arc;
        use tauri::async_runtime::RwLock;
        crate::services::resource_manager::ResourceManager::new(Arc::new(RwLock::new(
            NWN2Paths::default(),
        )))
    }

    fn chest_item(visual_type: u8, variation: u8) -> GffValue<'static> {
        let mut item = IndexMap::new();
        item.insert("BaseItem".to_string(), GffValue::Int(0));
        item.insert("ArmorVisualType".to_string(), GffValue::Byte(visual_type));
        item.insert("Variation".to_string(), GffValue::Byte(variation));
        GffValue::ListOwned(vec![item])
    }

    #[test]
    fn creature_root_variation_selects_unarmored_body() {
        let mut fields = IndexMap::new();
        fields.insert("ArmorVisualType".to_string(), GffValue::Byte(0));
        fields.insert("Variation".to_string(), GffValue::Byte(8));
        let character = Character::from_gff(fields);
        let visuals = character.resolve_equipped_visuals(
            &game_data_with_armor_and_baseitems(),
            &test_resource_manager(),
        );
        assert_eq!(visuals.armor_prefix.as_deref(), Some("CL"));
        assert_eq!(visuals.armor_variation, 9);
    }

    #[test]
    fn creature_root_leather_body_when_chest_empty() {
        let mut fields = IndexMap::new();
        fields.insert("ArmorVisualType".to_string(), GffValue::Byte(2));
        fields.insert("Variation".to_string(), GffValue::Byte(2));
        fields.insert("Equip_ItemList".to_string(), GffValue::ListOwned(vec![]));
        let character = Character::from_gff(fields);
        let visuals = character.resolve_equipped_visuals(
            &game_data_with_armor_and_baseitems(),
            &test_resource_manager(),
        );
        assert_eq!(visuals.armor_prefix.as_deref(), Some("LE"));
        assert_eq!(visuals.armor_variation, 3);
    }

    #[test]
    fn creature_body_tint_is_root_tintable_raw_order() {
        let mut fields = IndexMap::new();
        fields.insert("Variation".to_string(), GffValue::Byte(2));
        let mut character = Character::from_gff(fields);
        let mut body = TintChannels::default();
        body.channel2 = TintChannel {
            r: 166,
            g: 124,
            b: 82,
            a: 255,
        };
        body.channel3 = TintChannel {
            r: 33,
            g: 33,
            b: 33,
            a: 255,
        };
        character.set_tint_body(&body);
        let visuals = character.resolve_equipped_visuals(
            &game_data_with_armor_and_baseitems(),
            &test_resource_manager(),
        );
        let tint = visuals.armor_tint.expect("root body tint applied");
        assert_eq!(tint.channel2.r, 166);
        assert_eq!(tint.channel3.r, 33);
    }

    #[test]
    fn equipped_chest_item_wins_over_creature_root() {
        let mut fields = IndexMap::new();
        fields.insert("ArmorVisualType".to_string(), GffValue::Byte(0));
        fields.insert("Variation".to_string(), GffValue::Byte(8));
        fields.insert("Equip_ItemList".to_string(), chest_item(1, 4));
        let character = Character::from_gff(fields);
        let visuals = character.resolve_equipped_visuals(
            &game_data_with_armor_and_baseitems(),
            &test_resource_manager(),
        );
        assert_eq!(visuals.armor_prefix.as_deref(), Some("CP"));
        assert_eq!(visuals.armor_variation, 5);
    }

    #[test]
    fn creature_root_body_without_part_structs_uses_material_defaults() {
        let mut fields = IndexMap::new();
        fields.insert("Gender".to_string(), GffValue::Byte(0));
        fields.insert("ArmorVisualType".to_string(), GffValue::Byte(2));
        fields.insert("Variation".to_string(), GffValue::Byte(4));
        let character = Character::from_gff(fields);
        let parts = character
            .resolve_model_parts(
                &game_data_with_armor_and_baseitems(),
                &test_resource_manager(),
            )
            .expect("model parts");
        assert_eq!(parts.body_parts[0], "P_HHM_LE_Body05");
        assert_eq!(
            parts.boots_candidates,
            vec![
                "P_HHM_LE_Boots01".to_string(),
                "P_HHM_CL_Boots01".to_string()
            ]
        );
        assert_eq!(
            parts.gloves_candidates,
            vec![
                "P_HHM_LE_Gloves01".to_string(),
                "P_HHM_CL_Gloves01".to_string()
            ]
        );
    }

    #[test]
    fn creature_root_boots_struct_selects_default_boots() {
        let mut boots = IndexMap::new();
        boots.insert("ArmorVisualType".to_string(), GffValue::Byte(2));
        boots.insert("Variation".to_string(), GffValue::Byte(6));
        let mut fields = IndexMap::new();
        fields.insert("Gender".to_string(), GffValue::Byte(1));
        fields.insert("Variation".to_string(), GffValue::Byte(13));
        fields.insert("Boots".to_string(), GffValue::StructOwned(Box::new(boots)));
        let character = Character::from_gff(fields);
        let parts = character
            .resolve_model_parts(
                &game_data_with_armor_and_baseitems(),
                &test_resource_manager(),
            )
            .expect("model parts");
        assert_eq!(
            parts.boots_candidates,
            vec![
                "P_HHF_LE_Boots07".to_string(),
                "P_HHF_LE_Boots01".to_string()
            ]
        );
        assert!(parts.boots_tint.is_none());
        assert_eq!(
            parts.gloves_candidates,
            vec!["P_HHF_CL_Gloves01".to_string()]
        );
    }

    #[test]
    fn chest_item_keeps_default_boots_and_gloves() {
        let mut fields = IndexMap::new();
        fields.insert("Gender".to_string(), GffValue::Byte(0));
        fields.insert("Equip_ItemList".to_string(), chest_item(1, 4));
        let character = Character::from_gff(fields);
        let parts = character
            .resolve_model_parts(
                &game_data_with_armor_and_baseitems(),
                &test_resource_manager(),
            )
            .expect("model parts");
        assert_eq!(parts.body_parts[0], "P_HHM_CP_Body05");
        assert_eq!(
            parts.boots_candidates,
            vec![
                "P_HHM_CP_Boots01".to_string(),
                "P_HHM_CL_Boots01".to_string()
            ]
        );
        assert_eq!(
            parts.gloves_candidates,
            vec![
                "P_HHM_CP_Gloves01".to_string(),
                "P_HHM_CL_Gloves01".to_string()
            ]
        );
    }

    #[test]
    fn never_show_armor_uses_creature_root() {
        let mut fields = IndexMap::new();
        fields.insert("ArmorVisualType".to_string(), GffValue::Byte(2));
        fields.insert("Variation".to_string(), GffValue::Byte(4));
        fields.insert("NeverShowArmor".to_string(), GffValue::Byte(1));
        fields.insert("Equip_ItemList".to_string(), chest_item(1, 4));
        let character = Character::from_gff(fields);
        let visuals = character.resolve_equipped_visuals(
            &game_data_with_armor_and_baseitems(),
            &test_resource_manager(),
        );
        assert_eq!(visuals.armor_prefix.as_deref(), Some("LE"));
        assert_eq!(visuals.armor_variation, 5);
    }

    #[test]
    fn test_set_appearance_head() {
        let mut character = create_test_character();
        character.set_appearance_head(7);
        assert_eq!(character.appearance_head(), 7);
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_never_draw_helmet_writes_int_type() {
        let mut character = create_test_character();
        character.set_never_draw_helmet(true);
        match character.gff.get("NeverDrawHelmet") {
            Some(GffValue::Int(v)) => assert_eq!(*v, 1),
            other => panic!("NeverDrawHelmet must be Int, got {other:?}"),
        }
    }

    #[test]
    fn test_set_never_draw_helmet_repairs_byte_type() {
        let mut fields = IndexMap::new();
        fields.insert("NeverDrawHelmet".to_string(), GffValue::Byte(1));
        let mut character = Character::from_gff(fields);
        character.set_never_draw_helmet(false);
        match character.gff.get("NeverDrawHelmet") {
            Some(GffValue::Int(v)) => assert_eq!(*v, 0),
            other => panic!("NeverDrawHelmet must be Int, got {other:?}"),
        }
    }

    #[test]
    fn test_appearance_hair() {
        let character = create_test_character();
        assert_eq!(character.appearance_hair(), 2);
    }

    #[test]
    fn test_appearance_fhair() {
        let character = create_test_character();
        assert_eq!(character.appearance_fhair(), 1);
    }

    #[test]
    fn test_set_appearance_fhair() {
        let mut character = create_test_character();
        character.set_appearance_fhair(3);
        assert_eq!(character.appearance_fhair(), 3);
        assert!(character.is_modified());
    }

    #[test]
    fn test_tattoo_colors() {
        let character = create_test_character();
        assert_eq!(character.color_tattoo1(), 2);
        assert_eq!(character.color_tattoo2(), 7);
    }

    #[test]
    fn test_height_girth_default() {
        let character = create_test_character();
        assert!((character.height() - 1.0).abs() < f32::EPSILON);
        assert!((character.girth() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_set_height() {
        let mut character = create_test_character();
        character.set_height(1.1);
        assert!((character.height() - 1.1).abs() < f32::EPSILON);
        assert!((character.girth() - 1.0).abs() < f32::EPSILON);
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_girth() {
        let mut character = create_test_character();
        character.set_girth(0.85);
        assert!((character.girth() - 0.85).abs() < f32::EPSILON);
        assert!((character.height() - 1.0).abs() < f32::EPSILON);
        assert!(character.is_modified());
    }

    #[test]
    fn test_tint_head_default() {
        let character = create_test_character();
        let tint = character.tint_head();
        assert_eq!(tint.channel1.r, 0);
        assert_eq!(tint.channel1.g, 0);
        assert_eq!(tint.channel1.b, 0);
    }

    #[test]
    fn test_set_tint_head_roundtrip() {
        let mut character = create_test_character();
        let tints = TintChannels {
            channel1: TintChannel {
                r: 255,
                g: 219,
                b: 212,
                a: 0,
            },
            channel2: TintChannel {
                r: 252,
                g: 146,
                b: 32,
                a: 0,
            },
            channel3: TintChannel {
                r: 48,
                g: 43,
                b: 42,
                a: 0,
            },
        };
        character.set_tint_head(&tints);
        let read_back = character.tint_head();
        assert_eq!(read_back.channel1.r, 255);
        assert_eq!(read_back.channel1.g, 219);
        assert_eq!(read_back.channel1.b, 212);
        assert_eq!(read_back.channel2.r, 252);
        assert_eq!(read_back.channel3.r, 48);
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_tint_hair_roundtrip() {
        let mut character = create_test_character();
        let tints = TintChannels {
            channel1: TintChannel {
                r: 127,
                g: 93,
                b: 84,
                a: 0,
            },
            channel2: TintChannel {
                r: 114,
                g: 31,
                b: 0,
                a: 0,
            },
            channel3: TintChannel {
                r: 164,
                g: 53,
                b: 0,
                a: 0,
            },
        };
        character.set_tint_hair(&tints);
        let read_back = character.tint_hair();
        assert_eq!(read_back.channel1.r, 127);
        assert_eq!(read_back.channel2.g, 31);
        assert_eq!(read_back.channel3.r, 164);
    }

    #[test]
    fn test_soundset() {
        let character = create_test_character();
        assert_eq!(character.soundset(), 42);
    }

    #[test]
    fn test_wings_and_tail() {
        let character = create_test_character();
        assert_eq!(character.wings(), 0);
        assert_eq!(character.tail(), 0);
    }

    #[test]
    fn test_set_wings_and_tail() {
        let mut character = create_test_character();
        character.set_wings(2);
        character.set_tail(3);
        assert_eq!(character.wings(), 2);
        assert_eq!(character.tail(), 3);
        assert!(matches!(
            character.gff.get("Wings"),
            Some(GffValue::Byte(2))
        ));
        assert!(character.gff.get("Wings_NewID").is_none());
        assert!(character.gff.get("Tail_NewID").is_none());
    }

    #[test]
    fn test_tint_body_default_when_missing() {
        let character = create_test_character();
        let tint = character.tint_body();
        assert_eq!(tint.channel1.r, 0);
    }

    #[test]
    fn test_set_tint_body_roundtrip_and_shape() {
        let mut character = create_test_character();
        let tints = TintChannels {
            channel1: TintChannel {
                r: 255,
                g: 219,
                b: 212,
                a: 0,
            },
            channel2: TintChannel {
                r: 1,
                g: 2,
                b: 3,
                a: 0,
            },
            channel3: TintChannel {
                r: 4,
                g: 5,
                b: 6,
                a: 0,
            },
        };
        character.set_tint_body(&tints);

        let read_back = character.tint_body();
        assert_eq!(read_back.channel1.r, 255);
        assert_eq!(read_back.channel2.g, 2);
        assert_eq!(read_back.channel3.b, 6);
        assert!(character.is_modified());

        // Root field IS the Tintable struct: Tintable -> Tint -> "1".r as Byte
        let Some(GffValue::StructOwned(tintable)) = character.gff.get("Tintable") else {
            panic!("Tintable must be a struct");
        };
        let Some(GffValue::StructOwned(tint)) = tintable.get("Tint") else {
            panic!("Tintable.Tint must be a struct");
        };
        let Some(GffValue::StructOwned(ch1)) = tint.get("1") else {
            panic!("Tint.1 must be a struct");
        };
        assert!(matches!(ch1.get("r"), Some(GffValue::Byte(255))));
    }

    #[test]
    fn test_set_tint_body_from_head_copies_raw_gff_order() {
        let mut character = create_test_character();
        let head_ui = TintChannels {
            channel1: TintChannel {
                r: 200,
                g: 180,
                b: 160,
                a: 0,
            }, // skin
            channel2: TintChannel {
                r: 10,
                g: 20,
                b: 30,
                a: 0,
            }, // eyebrows (UI)
            channel3: TintChannel {
                r: 40,
                g: 50,
                b: 60,
                a: 0,
            }, // eyes (UI)
        };
        character.set_tint_head(&head_ui);
        character.set_tint_body_from_head();

        // Body copies the RAW GFF subtree = swap of the UI representation
        let body = character.tint_body();
        let expected = crate::character::appearance_helpers::swap_tint_2_3(&head_ui);
        assert_eq!(body.channel1.r, expected.channel1.r);
        assert_eq!(body.channel2.r, expected.channel2.r);
        assert_eq!(body.channel3.r, expected.channel3.r);
    }

    #[test]
    fn test_set_tint_body_from_head_noop_without_head() {
        let mut character = create_test_character();
        character.set_tint_body_from_head();
        assert!(character.gff.get("Tintable").is_none());
    }
}
