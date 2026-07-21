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
    TintChannels, build_nested_tint, build_tintable_struct, read_tint_from_tintable,
    resolve_armor_prefix, swap_tint_2_3,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AppearanceState {
    pub race_id: i32,
    pub race_name: String,
    pub gender: i32,
    pub gender_name: String,

    pub appearance_type: i32,

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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CharacterModelParts {
    pub body_parts: Vec<String>,
    pub naked_body_resref: String,
    pub head_resref: String,
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
    pub cloak_resref: Option<String>,
    pub cloak_tint: Option<TintChannels>,
    pub armor_tint: Option<TintChannels>,
    /// Accessories parsed off the equipped chest item (pauldrons, bracers, …).
    /// Empty / defaulted when no chest armor is equipped.
    pub chest_accessories: super::item_appearance::ArmorAccessories,
}

struct PartVisual {
    armor_prefixes: Vec<String>,
    variation: i32,
}

struct EquippedVisuals {
    armor_prefixes: Vec<String>,
    armor_variation: i32,
    helm_part: Option<PartVisual>,
    boots: Option<PartVisual>,
    gloves: Option<PartVisual>,
    cloak_variation: Option<i32>,
    cloak_tint: Option<TintChannels>,
    armor_tint: Option<TintChannels>,
    boots_tint: Option<TintChannels>,
    gloves_tint: Option<TintChannels>,
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
        self.get_i32("Wings_NewID")
            .or_else(|| self.get_i32("Wings"))
            .unwrap_or(0)
    }

    pub fn set_wings(&mut self, value: i32) {
        // NWN2-EE stores wings in both the original Byte field (Wings) and the
        // extended Int field (Wings_NewID). The engine reads Wings; writing only
        // _NewID leaves the stale Byte value in place.
        self.set_i32("Wings_NewID", value);
        self.set_byte("Wings", value.clamp(0, 255) as u8);
    }

    pub fn tail(&self) -> i32 {
        self.get_i32("Tail_NewID")
            .or_else(|| self.get_i32("Tail"))
            .unwrap_or(0)
    }

    pub fn set_tail(&mut self, value: i32) {
        self.set_i32("Tail_NewID", value);
        self.set_byte("Tail", value.clamp(0, 255) as u8);
    }

    pub fn never_draw_helmet(&self) -> bool {
        self.get_i32("NeverDrawHelmet").unwrap_or(0) != 0
    }

    pub fn set_never_draw_helmet(&mut self, value: bool) {
        self.set_int("NeverDrawHelmet", i32::from(value));
    }

    pub fn body_part_value(&self, gff_field: &str) -> i32 {
        self.get_i32(gff_field).unwrap_or(0)
    }

    pub fn set_body_part_value(&mut self, gff_field: &str, value: i32) {
        self.set_byte(gff_field, value as u8);
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
    /// TAIL and WING models (and the naked body) with this layer; the game
    /// leaves it white. Raw GFF channel order — no 2/3 swap.
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

        let model_parts = self.resolve_model_parts(game_data, resource_manager);
        let has_fhair_meshes = model_parts
            .as_ref()
            .map(|parts| {
                crate::services::model_loader::head_has_fhair_meshes(
                    resource_manager,
                    &parts.head_resref,
                )
            })
            .unwrap_or(false);
        let cloak_tint = model_parts.as_ref().and_then(|p| p.cloak_tint.clone());
        let armor_tint = model_parts.and_then(|p| p.armor_tint);
        debug!("AppearanceState: armor_tint={armor_tint:?}, cloak_tint={cloak_tint:?}");

        AppearanceState {
            race_id: self.race_id().0,
            race_name: self.race_name(game_data),
            gender: gender_id,
            gender_name,

            appearance_type: self.appearance_type(),

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

    fn discover_model_variants(
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

    /// Enumerate rows from a 2DA table, skipping any whose `model` MDB is
    /// missing from the resource index. Row 0 ("None") is kept regardless, so
    /// the user can still clear the selection.
    pub fn get_available_model_options(
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
        table_name: &str,
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
                options.push(AppearanceOption { id, name: label });
                continue;
            }
            let Some(model) = row_str(&row, "model") else {
                continue;
            };
            if !resource_manager.has_resource(&model, "mdb") {
                continue;
            }
            options.push(AppearanceOption { id, name: label });
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
        let head_template = row_str(&row, "nwn2_model_head").or_else(|| {
            warn!("No nwn2_model_head for appearance {appearance_id}");
            None
        })?;
        let skel_template = row_str(&row, "nwn2_skeleton_file").or_else(|| {
            warn!("No nwn2_skeleton_file for appearance {appearance_id}");
            None
        })?;

        let body_prefix = body_template.replace('?', &gender_letter);
        let head_prefix = head_template.replace('?', &gender_letter);
        let skeleton_resref = skel_template.replace('?', &gender_letter);

        let model_type = row_str(&row, "modeltype").unwrap_or_default();

        let naked_body_resref = format!("{body_prefix}_NK_Body01");

        // Extract all equipped visual info in one pass
        let equip_visuals = self.resolve_equipped_visuals(game_data, resource_manager);
        let armor_prefixes = &equip_visuals.armor_prefixes;
        let body_parts = if model_type.to_uppercase() == "P" {
            let var = equip_visuals.armor_variation;
            let mut parts: Vec<String> = armor_prefixes
                .iter()
                .map(|pfx| format!("{body_prefix}_{pfx}_Body{var:02}"))
                .collect();
            // Fallback to variation 01 if the requested variation doesn't exist
            if var != 1 {
                for pfx in armor_prefixes {
                    parts.push(format!("{body_prefix}_{pfx}_Body01"));
                }
            }
            if parts.is_empty() {
                parts.push(format!("{body_prefix}_CL_Body01"));
            }
            debug!("Body model candidates: {parts:?}");
            parts
        } else {
            vec![body_prefix.clone()]
        };

        let head_id = self.appearance_head();
        let head_resref = format!("{head_prefix}{head_id:02}");

        let hair_resref = row_str(&row, "nwn2_model_hair").map(|hair_template| {
            let hair_prefix = hair_template.replace('?', &gender_letter);
            let hair_id = self.appearance_hair();
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

        let build_helm_candidates = |prefixes: &[String], var: i32| -> Vec<String> {
            prefixes
                .iter()
                .map(|pfx| format!("{body_prefix}_{pfx}_Helm{var:02}"))
                .collect()
        };
        let helm_candidates: Vec<String> = match &equip_visuals.helm_part {
            Some(pv) => {
                let cs = build_helm_candidates(&pv.armor_prefixes, pv.variation);
                // Legacy helms without ArmorVisualType: borrow chest material
                // so the viewer still renders something.
                if cs.is_empty() {
                    build_helm_candidates(armor_prefixes, pv.variation)
                } else {
                    cs
                }
            }
            None => Vec::new(),
        };

        let boots_candidates = Self::build_part_candidates(
            &body_prefix,
            armor_prefixes,
            "Boots",
            equip_visuals.boots.as_ref(),
        );
        let gloves_candidates = Self::build_part_candidates(
            &body_prefix,
            armor_prefixes,
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
            cloak_resref,
            cloak_tint: equip_visuals.cloak_tint,
            armor_tint: equip_visuals.armor_tint,
            chest_accessories: equip_visuals.chest_accessories,
        })
    }

    fn build_part_candidates(
        body_prefix: &str,
        chest_armor_prefixes: &[String],
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
            for pfx in &pv.armor_prefixes {
                push(format!("{body_prefix}_{pfx}_{part_name}{var:02}"));
            }
            for pfx in chest_armor_prefixes {
                push(format!("{body_prefix}_{pfx}_{part_name}{var:02}"));
            }
        }

        // Default: try each chest armor prefix with variation 01
        for pfx in chest_armor_prefixes {
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
        let raw_variation = part_struct
            .get("Variation")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);

        if raw_variation == 0 {
            return None;
        }

        let variation = raw_variation + 1;
        let armor_prefixes = if visual_type > 0 {
            resolve_armor_prefix(game_data, visual_type, false)
        } else {
            Vec::new()
        };

        Some(PartVisual {
            armor_prefixes,
            variation,
        })
    }

    fn resolve_equipped_visuals(
        &self,
        game_data: &GameData,
        resource_manager: &crate::services::resource_manager::ResourceManager,
    ) -> EquippedVisuals {
        let mut result = EquippedVisuals {
            armor_prefixes: Vec::new(),
            armor_variation: 1,
            helm_part: None,
            boots: None,
            gloves: None,
            cloak_variation: None,
            cloak_tint: None,
            armor_tint: None,
            boots_tint: None,
            gloves_tint: None,
            chest_accessories: super::item_appearance::ArmorAccessories::default(),
        };

        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return result;
        };

        let baseitems_table = game_data.get_table("baseitems");

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

            if is_chest {
                let visual_type = item_struct
                    .get("ArmorVisualType")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                result.armor_prefixes = resolve_armor_prefix(game_data, visual_type, false);
                // GFF stores Variation 0-indexed (toolset displays the raw
                // value as "Variation 0", "Variation 1", ...) while the on-
                // disk mesh filenames are 1-indexed (Body01 is the first
                // variant). The engine adds 1 at render time; mirror that
                // here so the character viewer matches the in-game look.
                result.armor_variation = item_struct
                    .get("Variation")
                    .and_then(gff_value_to_i32)
                    .map(|v| v + 1)
                    .unwrap_or(1)
                    .max(1);
                debug!(
                    "Chest ArmorVisualType {visual_type}, Variation {} -> prefixes: {:?}",
                    result.armor_variation, result.armor_prefixes
                );

                let tintable_raw = item_struct.get("Tintable");
                debug!(
                    "Chest Tintable raw variant: {:?}",
                    tintable_raw.map(std::mem::discriminant)
                );
                if let Some(val) = tintable_raw {
                    debug!(
                        "Chest Tintable keys: {:?}",
                        match val {
                            GffValue::StructOwned(s) => s.keys().cloned().collect::<Vec<_>>(),
                            GffValue::Struct(lazy) =>
                                lazy.force_load().keys().cloned().collect::<Vec<_>>(),
                            _ => vec!["NOT_A_STRUCT".to_string()],
                        }
                    );
                }
                let tintable = match tintable_raw {
                    Some(GffValue::StructOwned(s)) => Some(s.as_ref().clone()),
                    Some(GffValue::Struct(lazy)) => Some(lazy.force_load()),
                    _ => None,
                };
                if let Some(ref t) = tintable {
                    debug!("Tintable has Tint key: {}", t.contains_key("Tint"));
                    let tint = read_tint_from_tintable(t);
                    debug!(
                        "Read tint: ch1.a={}, ch2.a={}, ch3.a={}",
                        tint.channel1.a, tint.channel2.a, tint.channel3.a
                    );
                    let has_color = [&tint.channel1, &tint.channel2, &tint.channel3]
                        .iter()
                        .any(|ch| ch.r > 0 || ch.g > 0 || ch.b > 0);
                    if has_color {
                        result.armor_tint = Some(tint);
                    }
                }

                // Fallback: when save item has no tints (all zero), use item template tints
                if result.armor_tint.is_none()
                    && let Some(template_tint) =
                        Self::load_template_tint(item_struct, resource_manager)
                {
                    debug!("Using template tint fallback");
                    result.armor_tint = Some(template_tint);
                }
                debug!("Armor tint: {:?}", result.armor_tint);

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
                    result.boots = Self::parse_part_visual(fields, game_data);
                    debug!(
                        "Boots part: {:?}",
                        result
                            .boots
                            .as_ref()
                            .map(|b| (&b.armor_prefixes, b.variation))
                    );
                }
                let gloves_fields = match item_struct.get("Gloves") {
                    Some(GffValue::StructOwned(s)) => Some(s.as_ref().clone()),
                    Some(GffValue::Struct(lazy)) => Some(lazy.force_load()),
                    _ => None,
                };
                if let Some(ref fields) = gloves_fields {
                    result.gloves = Self::parse_part_visual(fields, game_data);
                    debug!(
                        "Gloves part: {:?}",
                        result
                            .gloves
                            .as_ref()
                            .map(|g| (&g.armor_prefixes, g.variation))
                    );
                }
            }

            if is_head {
                let visual_type = item_struct
                    .get("ArmorVisualType")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                let variation = item_struct
                    .get("Variation")
                    .and_then(gff_value_to_i32)
                    .map(|v| v + 1)
                    .unwrap_or(1)
                    .max(1);
                let armor_prefixes = if visual_type > 0 {
                    resolve_armor_prefix(game_data, visual_type, false)
                } else {
                    Vec::new()
                };
                if !armor_prefixes.is_empty() || variation > 0 {
                    result.helm_part = Some(PartVisual {
                        armor_prefixes,
                        variation,
                    });
                }
            }

            if is_boots {
                result.boots = Self::parse_item_part_visual(item_struct, game_data);
                result.boots_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                debug!(
                    "Boots from slot: {:?}, tint={:?}",
                    result
                        .boots
                        .as_ref()
                        .map(|b| (&b.armor_prefixes, b.variation)),
                    result.boots_tint
                );
            }
            if is_gloves {
                result.gloves = Self::parse_item_part_visual(item_struct, game_data);
                result.gloves_tint =
                    Self::read_item_tint_with_fallback(item_struct, resource_manager);
                debug!(
                    "Gloves from slot: {:?}, tint={:?}",
                    result
                        .gloves
                        .as_ref()
                        .map(|g| (&g.armor_prefixes, g.variation)),
                    result.gloves_tint
                );
            }

            if is_cloak {
                // For cloaks, the mesh variation lives in ModelPart1 — the
                // top-level Variation field holds an item sub-type and does
                // not map to `P_???_CL_Cloak{NN}.mdb`. Fall back to Variation
                // only if ModelPart1 is missing/zero.
                let model_part1 = item_struct
                    .get("ModelPart1")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                let variation = if model_part1 > 0 {
                    model_part1
                } else {
                    item_struct
                        .get("Variation")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(0)
                };
                if variation > 0 {
                    result.cloak_variation = Some(variation);
                    result.cloak_tint =
                        Self::read_item_tint_with_fallback(item_struct, resource_manager);
                    debug!(
                        "Cloak variation: {variation}, tint: {:?}",
                        result.cloak_tint
                    );
                }
            }
        }

        result
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
            let has_color = [&tint.channel1, &tint.channel2, &tint.channel3]
                .iter()
                .any(|ch| ch.r > 0 || ch.g > 0 || ch.b > 0);
            if has_color {
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
        let has_color = [&tint.channel1, &tint.channel2, &tint.channel3]
            .iter()
            .any(|ch| ch.r > 0 || ch.g > 0 || ch.b > 0);
        if has_color {
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
    ) -> Option<PartVisual> {
        let visual_type = item_struct
            .get("ArmorVisualType")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        let variation = item_struct
            .get("Variation")
            .and_then(gff_value_to_i32)
            .map(|v| v + 1)
            .unwrap_or(1)
            .max(1);

        debug!("Item part visual: ArmorVisualType={visual_type}, Variation(mesh)={variation}");

        let armor_prefixes = if visual_type > 0 {
            resolve_armor_prefix(game_data, visual_type, false)
        } else {
            Vec::new()
        };

        Some(PartVisual {
            armor_prefixes,
            variation,
        })
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
        fields.insert("Wings_NewID".to_string(), GffValue::Int(0));
        fields.insert("Tail_NewID".to_string(), GffValue::Int(0));
        fields.insert("Race".to_string(), GffValue::Byte(1));
        fields.insert("Gender".to_string(), GffValue::Byte(0));
        Character::from_gff(fields)
    }

    #[test]
    fn test_appearance_head() {
        let character = create_test_character();
        assert_eq!(character.appearance_head(), 3);
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
    }

    #[test]
    fn test_body_part_value() {
        let mut character = create_test_character();
        assert_eq!(character.body_part_value("BodyPart_Torso"), 0);
        character.set_body_part_value("BodyPart_Torso", 5);
        assert_eq!(character.body_part_value("BodyPart_Torso"), 5);
        assert!(character.is_modified());
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
