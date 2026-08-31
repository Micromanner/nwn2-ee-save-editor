#![allow(dead_code)]
pub mod context_maps;
pub mod error;
pub mod property_types;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::{debug, warn};

use crate::services::resource_manager::ResourceManager;
use crate::utils::parsing::{row_int, row_str};

pub use context_maps::*;
pub use error::{ItemPropertyError, ItemPropertyResult};
pub use property_types::{DecodedProperty, PropertyDefinition, PropertyMetadata};

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct EditorContext {
    pub abilities: HashMap<u32, String>,
    pub skills: HashMap<u32, String>,
    pub spells: HashMap<u32, String>,
    pub damage_types: HashMap<u32, String>,
    pub immunity_types: HashMap<u32, String>,
    pub saving_throws: HashMap<u32, String>,
    pub save_elements: HashMap<u32, String>,
    pub classes: HashMap<u32, String>,
    pub racial_groups: HashMap<u32, String>,
    pub alignment_groups: HashMap<u32, String>,
    pub alignments: HashMap<u32, String>,
    pub light: HashMap<u32, String>,
    pub feats: HashMap<u32, String>,
}

fn get_subtype_context_key(subtype_ref: &str) -> Option<&'static str> {
    match subtype_ref.to_lowercase().as_str() {
        "ability" | "decreaseabilityscore" | "abilitybonus" => Some("abilities"),
        "skill" | "decreasedskill" => Some("skills"),
        "castspell" | "spellimmunity_specific" | "onhitcastspell" => Some("spells"),
        "damagetype"
        | "armordamagetype"
        | "damageresist"
        | "damageimmunity"
        | "damageimmunity_fixed"
        | "damagepenalty"
        | "damage_vulnerability"
        | "damage_vulnerability_fixed"
        | "damagemelee"
        | "damageranged"
        | "damage"
        | "damagereduced"
        | "damagenone"
        | "damage_reduction"
        | "damagereduction"
        | "massive_criticals" => Some("damage_types"),
        "immunity" => Some("immunity_types"),
        "savingthrow"
        | "improvedsavingthrowsspecific"
        | "reducedspecificsavingthrow"
        | "reducedspecificsaving_throw" => Some("saving_throws"),
        "saveselement" | "improvedsavingthrows" | "reducedsavingthrows" => Some("save_elements"),
        "uselimitationclass" | "classes" | "singlebonusspellofle" => Some("classes"),
        "uselimitationracial"
        | "racialtype"
        | "racialtypes"
        | "armorracinggroup"
        | "armorracialgroup"
        | "enhancementracialgroup"
        | "damageracialgroup"
        | "attackbonusracialgroup"
        | "damageracialtype" => Some("racial_groups"),
        "iprp_acmodtype" | "acmodtype" | "decreaseac" => Some("ac_types"),
        "armoralignmentgroup"
        | "damagealignmentgroup"
        | "enhancementalignmentgroup"
        | "attackbonusalignmentgroup"
        | "uselimitationalignmentgroup" => Some("alignment_groups"),
        "armorspecificalignment"
        | "damagespecificalignment"
        | "enhancementspecificalignment"
        | "attackbonusspecificalignment"
        | "uselimitationspecificalignment"
        | "specificalignment" => Some("alignments"),
        "bonusfeats" => Some("feats"),
        "light" => Some("light"),
        _ => None,
    }
}

#[allow(dead_code)]
const PROPERTY_ID_ABILITY_BONUS: u32 = 0;
const PROPERTY_ID_AC_BONUS: u32 = 1;
const PROPERTY_ID_AC_BONUS_VS_ALIGNMENT: u32 = 5;
/// `baseitems.2da` `AC_Enchant` column value the engine treats as armor.
const AC_ENCHANT_ARMOR: i32 = 2;
/// `baseitems.2da` `AC_Enchant` column value the engine treats as a shield.
const AC_ENCHANT_SHIELD: i32 = 3;
const PROPERTY_ID_ENHANCEMENT: u32 = 6;
const PROPERTY_ID_ATTACK_PENALTY: u32 = 10;
const PROPERTY_ID_BONUS_FEAT: u32 = 12;
const PROPERTY_ID_BONUS_SPELL_SLOT: u32 = 13;
const PROPERTY_ID_CAST_SPELL: u32 = 15;
const PROPERTY_ID_DAMAGE_BONUS: u32 = 16;
const PROPERTY_ID_DAMAGE_RESISTANCE: u32 = 23;
const PROPERTY_ID_DAMAGE_VULNERABILITY: u32 = 24;
const PROPERTY_ID_ABILITY_PENALTY: u32 = 27;
const PROPERTY_ID_HASTE: u32 = 35;
const PROPERTY_ID_IMMUNITY: u32 = 37;
const PROPERTY_ID_SPELL_RESISTANCE: u32 = 39;
const PROPERTY_ID_SAVING_THROW_VS: u32 = 40;
const PROPERTY_ID_SAVING_THROW_BONUS: u32 = 41;
const PROPERTY_ID_LIGHT: u32 = 44;
const PROPERTY_ID_REGENERATION: u32 = 51;
const PROPERTY_ID_SKILL_BONUS: u32 = 52;
const PROPERTY_ID_ATTACK_BONUS: u32 = 56;
const PROPERTY_ID_USE_LIMIT_ALIGNMENT: u32 = 62;
const PROPERTY_ID_USE_LIMIT_CLASS: u32 = 63;
const PROPERTY_ID_TRAP: u32 = 70;
const PROPERTY_ID_TRUE_SEEING: u32 = 71;
const PROPERTY_ID_FREEDOM_OF_MOVEMENT: u32 = 75;
const PROPERTY_ID_WEIGHT_MODIFIER: u32 = 81;
const PROPERTY_ID_DAMAGE_REDUCTION: u32 = 90;

/// One `iprp_*cost` table: the display label per row plus the numeric column some
/// of those tables carry (`Amount` in `iprp_resistcost`, `Value` in
/// `iprp_damvulcost`/`iprp_bonuscost`/...).
#[derive(Debug, Clone, Default)]
pub struct CostTableData {
    pub labels: HashMap<u32, String>,
    pub amounts: HashMap<u32, i32>,
}

pub struct ItemPropertyDecoder {
    rm: Arc<tokio::sync::RwLock<ResourceManager>>,
    property_cache: HashMap<u32, PropertyDefinition>,
    /// Keyed by `iprp_costtable.2da` row, i.e. `PropertyDefinition::cost_table_ref`.
    cost_tables: HashMap<u32, CostTableData>,
    /// Subtype tables that have no shared context map, keyed by lowercase
    /// `SubTypeResRef` (e.g. `iprp_combatdam`).
    subtype_tables: HashMap<String, HashMap<u32, String>>,
    initialized: bool,
    skills: HashMap<u32, String>,
    classes: HashMap<u32, String>,
    feats: HashMap<u32, String>,
    spells: HashMap<u32, String>,
    abilities: HashMap<u32, String>,
    damage_types: HashMap<u32, String>,
    immunity_types: HashMap<u32, String>,
    ac_types: HashMap<u32, String>,
    save_types: HashMap<u32, String>,
    save_elements: HashMap<u32, String>,
    alignment_groups: HashMap<u32, String>,
    alignments: HashMap<u32, String>,
    racial_groups: HashMap<u32, String>,
    light: HashMap<u32, String>,
    /// `baseitems.2da` `AC_Enchant` column: 2 = armor enchantment, 3 = shield
    /// enchantment, anything else (including missing rows) = deflection.
    ac_enchant: HashMap<i32, i32>,
}

impl ItemPropertyDecoder {
    pub fn new(rm: Arc<tokio::sync::RwLock<ResourceManager>>) -> Self {
        Self {
            rm,
            property_cache: HashMap::new(),
            cost_tables: HashMap::new(),
            subtype_tables: HashMap::new(),
            initialized: false,
            skills: HashMap::new(),
            classes: HashMap::new(),
            feats: HashMap::new(),
            spells: HashMap::new(),
            abilities: HashMap::new(),
            damage_types: HashMap::new(),
            immunity_types: HashMap::new(),
            ac_types: HashMap::new(),
            save_types: HashMap::new(),
            save_elements: HashMap::new(),
            alignment_groups: HashMap::new(),
            alignments: HashMap::new(),
            racial_groups: HashMap::new(),
            light: HashMap::new(),
            ac_enchant: HashMap::new(),
        }
    }

    pub fn set_lookup_tables(
        &mut self,
        skills: HashMap<u32, String>,
        classes: HashMap<u32, String>,
        feats: HashMap<u32, String>,
        spells: HashMap<u32, String>,
        racial_groups: HashMap<u32, String>,
    ) {
        self.skills = skills;
        self.classes = classes;
        self.feats = feats;
        self.spells = spells;
        self.racial_groups = racial_groups;
    }

    pub fn set_2da_tables(
        &mut self,
        abilities: HashMap<u32, String>,
        damage_types: HashMap<u32, String>,
        immunity_types: HashMap<u32, String>,
        ac_types: HashMap<u32, String>,
        save_types: HashMap<u32, String>,
        save_elements: HashMap<u32, String>,
        alignment_groups: HashMap<u32, String>,
        alignments: HashMap<u32, String>,
    ) {
        self.abilities = abilities;
        self.damage_types = damage_types;
        self.immunity_types = immunity_types;
        self.ac_types = ac_types;
        self.save_types = save_types;
        self.save_elements = save_elements;
        self.alignment_groups = alignment_groups;
        self.alignments = alignments;
    }

    pub fn has_lookup_tables(&self) -> bool {
        !self.skills.is_empty()
            || !self.classes.is_empty()
            || !self.feats.is_empty()
            || !self.spells.is_empty()
    }

    pub fn initialize_with_rm(&mut self, rm: &ResourceManager) {
        if self.initialized {
            return;
        }

        self.load_property_definitions_from_rm(rm);
        self.load_cost_tables_from_rm(rm);
        self.load_subtype_tables_from_rm(rm);
        self.abilities = load_2da_options_from_rm(rm, "iprp_abilities");
        self.damage_types = load_2da_options_from_rm(rm, "iprp_damagetype");
        self.immunity_types = load_2da_options_from_rm(rm, "iprp_immunity");
        self.ac_types = load_2da_options_from_rm(rm, "iprp_acmodtype");
        self.save_types = load_2da_options_from_rm(rm, "iprp_savingthrow");
        self.save_elements = load_2da_options_from_rm(rm, "iprp_saveelement");
        self.alignment_groups = load_2da_options_from_rm(rm, "iprp_aligngrp");
        self.alignments = load_2da_options_from_rm(rm, "iprp_alignment");
        self.light = load_2da_options_from_rm(rm, "iprp_lightcost");
        self.ac_enchant = load_baseitem_ac_enchant_from_rm(rm);
        self.initialized = true;
        debug!(
            "ItemPropertyDecoder initialized with {} property definitions",
            self.property_cache.len()
        );
    }

    /// Every table here is read WITH overrides (override/, workshop, HAKs), the
    /// same view `GameData` and the cost calculator use, so a modded
    /// `itempropdef` is named, decoded and priced from one and the same file.
    fn load_property_definitions_from_rm(&mut self, rm: &ResourceManager) {
        let itempropdef = match rm.get_2da_with_overrides("itempropdef") {
            Ok(parser) => parser,
            Err(e) => {
                warn!("Failed to load itempropdef.2da: {}", e);
                return;
            }
        };

        for row_idx in 0..itempropdef.row_count() {
            if let Ok(row) = itempropdef.get_row_dict(row_idx) {
                let id = row_idx as u32;

                let label = row_str(&row, "label").unwrap_or_default();

                if label.is_empty() || is_invalid_label(&label) {
                    continue;
                }

                let subtype_ref = row_str(&row, "subtyperesref").filter(|s| s != "****");

                let cost_table_ref = row_str(&row, "costtableresref").filter(|s| s != "****");

                let param1_ref = row_str(&row, "param1resref").filter(|s| s != "****");

                let game_str_ref_val = row_int(&row, "gamestrref", -1);
                let game_str_ref = if game_str_ref_val >= 0 {
                    Some(game_str_ref_val)
                } else {
                    None
                };

                let description = if let Some(str_ref) = game_str_ref {
                    rm.get_string(str_ref)
                } else {
                    String::new()
                };

                // The Name strref carries the property name the game itself shows
                // ("AC Bonus vs. Damage Type"), which beats deriving one from the
                // engineer-facing label column ("ArmorDamageType").
                let name_str_ref = row_int(&row, "name", -1);
                let raw_name = if name_str_ref > 0 && !ignores_2da_name(id) {
                    let s = rm.get_string(name_str_ref);
                    if s.is_empty() { None } else { Some(s) }
                } else {
                    None
                };

                let display_label = raw_name.clone().unwrap_or_else(|| clean_label(&label));

                self.property_cache.insert(
                    id,
                    PropertyDefinition {
                        id,
                        label: display_label,
                        subtype_ref,
                        cost_table_ref,
                        param1_ref,
                        description,
                        game_str_ref,
                        raw_label: label,
                        raw_name,
                    },
                );
            }
        }
    }

    /// Preload every cost table referenced by `itempropdef.2da` so both decoding
    /// and the editor read values from the same 2DA-backed source.
    fn load_cost_tables_from_rm(&mut self, rm: &ResourceManager) {
        let refs: Vec<u32> = self
            .property_cache
            .values()
            .filter_map(|def| def.cost_table_ref.as_ref())
            .filter_map(|r| r.parse::<u32>().ok())
            .collect();

        for table_idx in refs {
            if self.cost_tables.contains_key(&table_idx) {
                continue;
            }
            let Some(table_name) = resolve_cost_table_from_rm(rm, &table_idx.to_string()) else {
                continue;
            };
            self.cost_tables
                .insert(table_idx, load_cost_table_from_rm(rm, &table_name));
        }
    }

    fn load_subtype_tables_from_rm(&mut self, rm: &ResourceManager) {
        let refs: Vec<String> = self
            .property_cache
            .values()
            .filter_map(|def| def.subtype_ref.as_ref())
            .filter(|r| get_subtype_context_key(r).is_none())
            .map(|r| r.to_lowercase())
            .collect();

        for table_name in refs {
            if self.subtype_tables.contains_key(&table_name) {
                continue;
            }
            let options = load_2da_options_from_rm(rm, &table_name);
            self.subtype_tables.insert(table_name, options);
        }
    }

    /// Subtype display name for any property, via its context map when one
    /// exists and otherwise via the property's own `SubTypeResRef` table.
    fn subtype_name(&self, def: &PropertyDefinition, subtype: u32) -> Option<String> {
        let subtype_ref = def.subtype_ref.as_ref()?;
        if let Some(key) = get_subtype_context_key(subtype_ref) {
            return self.resolve_lookup(key, subtype);
        }
        self.subtype_tables
            .get(&subtype_ref.to_lowercase())?
            .get(&subtype)
            .cloned()
    }

    fn cost_table_for(&self, property_id: u32) -> Option<&CostTableData> {
        let idx = self
            .property_cache
            .get(&property_id)?
            .cost_table_ref
            .as_ref()?
            .parse::<u32>()
            .ok()?;
        self.cost_tables.get(&idx)
    }

    /// Display text for a property's cost value ("1d4", "+3", "25%"), straight
    /// from the property's own cost table.
    fn cost_label(&self, property_id: u32, cost_value: u32) -> Option<&str> {
        self.cost_table_for(property_id)?
            .labels
            .get(&cost_value)
            .map(String::as_str)
    }

    /// Numeric column of a property's cost value, where its table has one.
    fn cost_amount(&self, property_id: u32, cost_value: u32) -> Option<i32> {
        self.cost_table_for(property_id)?
            .amounts
            .get(&cost_value)
            .copied()
    }

    /// AC type the engine applies to an AC bonus property on this base item:
    /// armor enchantment on armor, shield enchantment on shields, deflection
    /// everywhere else.
    fn ac_bonus_kind(&self, base_item_id: i32) -> &'static str {
        match self.ac_enchant.get(&base_item_id) {
            Some(&AC_ENCHANT_ARMOR) => "Armor",
            Some(&AC_ENCHANT_SHIELD) => "Shield",
            _ => "Deflection",
        }
    }

    fn resolve_lookup(&self, context_key: &str, subtype: u32) -> Option<String> {
        let table = match context_key {
            "abilities" => &self.abilities,
            "skills" => &self.skills,
            "spells" => &self.spells,
            "damage_types" => &self.damage_types,
            "immunity_types" => &self.immunity_types,
            "saving_throws" => &self.save_types,
            "save_elements" => &self.save_elements,
            "classes" => &self.classes,
            "racial_groups" => &self.racial_groups,
            "alignment_groups" => &self.alignment_groups,
            "alignments" => &self.alignments,
            "feats" => &self.feats,
            "ac_types" => &self.ac_types,
            "light" => &self.light,
            _ => return None,
        };
        table.get(&subtype).cloned()
    }

    pub fn decode_property(
        &self,
        property_data: &HashMap<String, serde_json::Value>,
    ) -> Option<DecodedProperty> {
        let property_id = get_u32(property_data, "PropertyName")?;
        let subtype = get_u32(property_data, "Subtype").unwrap_or(0);
        let cost_value = get_u32(property_data, "CostValue").unwrap_or(0);
        let param1_value = get_u32(property_data, "Param1Value").unwrap_or(0);

        let raw_data = property_data.clone();

        let decoded = match property_id {
            PROPERTY_ID_ABILITY_BONUS => {
                let name = self.abilities.get(&subtype).map_or("Unknown", |s| s);
                property_types::decode_ability_bonus(name, cost_value, raw_data)
            }
            PROPERTY_ID_ABILITY_PENALTY => {
                let name = self.abilities.get(&subtype).map_or("Unknown", |s| s);
                property_types::decode_ability_penalty(name, cost_value, raw_data)
            }
            PROPERTY_ID_AC_BONUS => property_types::decode_ac_bonus(cost_value as i32, raw_data),
            PROPERTY_ID_ENHANCEMENT => {
                property_types::decode_enhancement_bonus(cost_value, raw_data)
            }
            PROPERTY_ID_ATTACK_BONUS => property_types::decode_attack_bonus(cost_value, raw_data),
            PROPERTY_ID_DAMAGE_BONUS => {
                let name = self.damage_types.get(&subtype).map_or("Unknown", |s| s);
                let dice = self
                    .cost_label(property_id, cost_value)
                    .map_or_else(|| cost_value.to_string(), str::to_string);
                property_types::decode_damage_bonus(name, &dice, raw_data)
            }
            PROPERTY_ID_DAMAGE_RESISTANCE => {
                let name = self.damage_types.get(&subtype).map_or("Unknown", |s| s);
                let amount = self
                    .cost_amount(property_id, cost_value)
                    .unwrap_or(cost_value as i32);
                property_types::decode_damage_resistance(name, amount, raw_data)
            }
            PROPERTY_ID_DAMAGE_VULNERABILITY => {
                let name = self.damage_types.get(&subtype).map_or("Unknown", |s| s);
                let percent = self
                    .cost_amount(property_id, cost_value)
                    .unwrap_or(cost_value as i32);
                property_types::decode_damage_vulnerability(name, percent, raw_data)
            }
            PROPERTY_ID_SAVING_THROW_BONUS => {
                let name = self.save_types.get(&subtype).map_or("Unknown", |s| s);
                property_types::decode_saving_throw_bonus_named(name, cost_value, raw_data)
            }
            PROPERTY_ID_SAVING_THROW_VS => {
                let name = self.save_elements.get(&subtype).map_or("Unknown", |s| s);
                property_types::decode_saving_throw_vs_element_named(name, cost_value, raw_data)
            }
            PROPERTY_ID_SKILL_BONUS => {
                let skill_name = self
                    .skills
                    .get(&subtype)
                    .cloned()
                    .unwrap_or_else(|| format!("Skill_{subtype}"));
                property_types::decode_skill_bonus(&skill_name, cost_value, raw_data)
            }
            PROPERTY_ID_IMMUNITY => {
                let name = self.immunity_types.get(&subtype).map_or("Unknown", |s| s);
                property_types::decode_immunity(name, raw_data)
            }
            PROPERTY_ID_CAST_SPELL => {
                let spell_name = self
                    .spells
                    .get(&subtype)
                    .cloned()
                    .unwrap_or_else(|| format!("Spell_{subtype}"));
                let prefix = self
                    .property_cache
                    .get(&property_id)
                    .map(|d| d.description.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("Use:");
                property_types::decode_cast_spell(
                    prefix,
                    &spell_name,
                    self.cost_label(property_id, cost_value),
                    raw_data,
                )
            }
            PROPERTY_ID_BONUS_FEAT => {
                let feat_name = self
                    .feats
                    .get(&subtype)
                    .cloned()
                    .unwrap_or_else(|| format!("Feat_{subtype}"));
                property_types::decode_bonus_feat(&feat_name, raw_data)
            }
            PROPERTY_ID_SPELL_RESISTANCE => {
                property_types::decode_spell_resistance(cost_value, raw_data)
            }
            PROPERTY_ID_REGENERATION => property_types::decode_regeneration(cost_value, raw_data),
            PROPERTY_ID_FREEDOM_OF_MOVEMENT => property_types::decode_freedom_of_movement(raw_data),
            PROPERTY_ID_HASTE => property_types::decode_haste(raw_data),
            PROPERTY_ID_TRUE_SEEING => property_types::decode_true_seeing(raw_data),
            PROPERTY_ID_USE_LIMIT_CLASS => {
                let class_name = self
                    .classes
                    .get(&subtype)
                    .cloned()
                    .unwrap_or_else(|| format!("Class_{subtype}"));
                property_types::decode_use_limitation_class(&class_name, raw_data)
            }
            PROPERTY_ID_USE_LIMIT_ALIGNMENT => {
                let name = self.alignment_groups.get(&subtype).map_or("Unknown", |s| s);
                property_types::decode_use_limitation_alignment(name, raw_data)
            }
            PROPERTY_ID_DAMAGE_REDUCTION => {
                let bypass = format!("Bypass_{param1_value}");
                property_types::decode_damage_reduction(cost_value, &bypass, raw_data)
            }
            _ => {
                let def = self.property_cache.get(&property_id);
                let label = def
                    .map(|d| d.label.clone())
                    .unwrap_or_else(|| format!("Property {property_id}"));

                let subtype_name = def.and_then(|d| self.subtype_name(d, subtype));

                // The GameStrRef text is what the game prints in front of the
                // subtype/value on the item itself ("Armor Bonus vs.", "Use:").
                let prefix = def
                    .map(|d| d.description.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&label);

                property_types::decode_generic_with_context(
                    property_id,
                    &label,
                    prefix,
                    subtype_name.as_deref(),
                    self.cost_label(property_id, cost_value),
                    raw_data,
                )
            }
        };

        Some(decoded)
    }

    pub fn decode_all_properties(
        &self,
        properties: &[HashMap<String, serde_json::Value>],
    ) -> Vec<DecodedProperty> {
        properties
            .iter()
            .filter_map(|prop| self.decode_property(prop))
            .collect()
    }

    /// Decodes an item's properties and stamps the AC family (rows 1-5) with
    /// the AC type the engine derives from the base item, since the property
    /// itself does not store one.
    pub fn decode_item_properties(
        &self,
        properties: &[HashMap<String, serde_json::Value>],
        base_item_id: i32,
    ) -> Vec<DecodedProperty> {
        let ac_kind = self.ac_bonus_kind(base_item_id);
        let mut decoded = self.decode_all_properties(properties);
        for prop in &mut decoded {
            if (PROPERTY_ID_AC_BONUS..=PROPERTY_ID_AC_BONUS_VS_ALIGNMENT)
                .contains(&prop.property_id)
            {
                prop.description = format!("{} ({ac_kind})", prop.description);
                prop.ac_type = Some(ac_kind.to_string());
            }
        }
        decoded
    }

    pub fn get_item_bonuses(
        &self,
        properties: &[HashMap<String, serde_json::Value>],
        base_item_id: i32,
    ) -> ItemBonuses {
        let decoded = self.decode_all_properties(properties);
        let mut bonuses = ItemBonuses::default();

        let ac_kind = self.ac_bonus_kind(base_item_id);
        let is_armor = ac_kind == "Armor";
        let is_shield = ac_kind == "Shield";

        for prop in decoded {
            match prop.bonus_type.as_str() {
                "ability" => {
                    if let (Some(ability), Some(value)) = (&prop.ability, prop.bonus_value) {
                        match normalize_ability_name(ability) {
                            "Str" => bonuses.str_bonus += value,
                            "Dex" => bonuses.dex_bonus += value,
                            "Con" => bonuses.con_bonus += value,
                            "Int" => bonuses.int_bonus += value,
                            "Wis" => bonuses.wis_bonus += value,
                            "Cha" => bonuses.cha_bonus += value,
                            _ => {}
                        }
                    }
                }
                "ability_penalty" => {
                    if let (Some(ability), Some(value)) = (&prop.ability, prop.penalty_value) {
                        match normalize_ability_name(ability) {
                            "Str" => bonuses.str_bonus -= value,
                            "Dex" => bonuses.dex_bonus -= value,
                            "Con" => bonuses.con_bonus -= value,
                            "Int" => bonuses.int_bonus -= value,
                            "Wis" => bonuses.wis_bonus -= value,
                            "Cha" => bonuses.cha_bonus -= value,
                            _ => {}
                        }
                    }
                }
                "ac" => {
                    if let Some(value) = prop.bonus_value {
                        // AC Bonus carries no AC type of its own; the engine picks
                        // one from the item: armor enchantment on armor, shield
                        // enchantment on shields, deflection everywhere else.
                        if is_armor {
                            bonuses.ac_armor_bonus += value;
                        } else if is_shield {
                            bonuses.ac_shield_bonus += value;
                        } else {
                            bonuses.ac_deflection_bonus += value;
                        }
                    }
                }
                "enhancement" => {
                    if let Some(value) = prop.bonus_value {
                        if is_armor {
                            bonuses.ac_armor_bonus += value;
                        } else if is_shield {
                            bonuses.ac_shield_bonus += value;
                        } else {
                            bonuses.attack_bonus += value;
                        }
                    }
                }
                "attack" => {
                    if let Some(value) = prop.bonus_value {
                        bonuses.attack_bonus += value;
                    }
                }
                "saving_throw" => {
                    if let (Some(save_type), Some(value)) = (&prop.save_type, prop.bonus_value) {
                        match save_type.as_str() {
                            "Fortitude" => bonuses.fortitude_bonus += value,
                            "Reflex" => bonuses.reflex_bonus += value,
                            "Will" => bonuses.will_bonus += value,
                            _ => {}
                        }
                    }
                }
                "saving_throw_element" => {
                    if let Some(value) = prop.bonus_value {
                        let element = prop.save_element.as_deref().unwrap_or("");
                        let element_lower = element.to_lowercase();
                        if element_lower == "universal" || element_lower == "all" {
                            bonuses.fortitude_bonus += value;
                            bonuses.reflex_bonus += value;
                            bonuses.will_bonus += value;
                        }
                    }
                }
                "skill" => {
                    // Fix: Use Skill_<ID> key format to match skills.rs expectation
                    // The Subtype in raw_data is the Skill ID
                    let skill_id = prop.raw_data.get("Subtype").and_then(|v| match v {
                        serde_json::Value::Number(n) => n.as_u64().map(|u| u as u32),
                        serde_json::Value::String(s) => s.parse::<u32>().ok(),
                        _ => None,
                    });

                    if let Some(subtype) = skill_id {
                        if let Some(value) = prop.bonus_value {
                            let key = format!("Skill_{subtype}");
                            *bonuses.skill_bonuses.entry(key).or_insert(0) += value;
                        }
                    } else if let (Some(skill), Some(value)) = (&prop.skill_name, prop.bonus_value)
                    {
                        // Fallback for logic where subtype might be missing but name exists (unlikely for proper items)
                        *bonuses.skill_bonuses.entry(skill.clone()).or_insert(0) += value;
                    }
                }
                "spell_resistance" => {
                    if let Some(value) = prop.bonus_value {
                        bonuses.spell_resistance = bonuses.spell_resistance.max(value);
                    }
                }
                "damage_resistance" => {
                    if let (Some(dtype), Some(value)) = (&prop.damage_type, prop.resistance_value) {
                        *bonuses.damage_resistances.entry(dtype.clone()).or_insert(0) =
                            (*bonuses.damage_resistances.get(dtype).unwrap_or(&0)).max(value);
                    }
                }
                _ => {}
            }
        }

        bonuses
    }

    pub fn get_property_definition(&self, id: u32) -> Option<&PropertyDefinition> {
        self.property_cache.get(&id)
    }

    /// Editor property metadata with full 2DA resolution (subtypes, cost tables,
    /// param tables).
    pub fn get_editor_property_metadata_with_rm(
        &self,
        context: &EditorContext,
        rm: &ResourceManager,
    ) -> Vec<PropertyMetadata> {
        let mut metadata = Vec::new();
        let allowed_columns = load_itemprops_columns_from_rm(rm);

        for (id, def) in &self.property_cache {
            if is_hidden_property(*id) {
                continue;
            }

            let mut meta = PropertyMetadata {
                id: *id,
                label: def.label.clone(),
                original_label: def.raw_label.clone(),
                description: def.description.clone(),
                allowed_columns: allowed_columns
                    .as_ref()
                    .map(|cols| cols.get(id).cloned().unwrap_or_default()),
                ..Default::default()
            };

            if let Some(ref subtype_ref) = def.subtype_ref {
                let mapping_key = get_subtype_context_key(subtype_ref);

                meta.subtype_options = if let Some(key) = mapping_key {
                    match key {
                        "abilities" => context.abilities.clone(),
                        "skills" => context.skills.clone(),
                        "spells" => context.spells.clone(),
                        "damage_types" => context.damage_types.clone(),
                        "immunity_types" => context.immunity_types.clone(),
                        "saving_throws" => context.saving_throws.clone(),
                        "save_elements" => context.save_elements.clone(),
                        "classes" => context.classes.clone(),
                        "racial_groups" => context.racial_groups.clone(),
                        "alignment_groups" => context.alignment_groups.clone(),
                        "alignments" => context.alignments.clone(),
                        "light" => context.light.clone(),
                        "feats" => context.feats.clone(),
                        _ => load_2da_options_from_rm(rm, subtype_ref),
                    }
                } else {
                    load_2da_options_from_rm(rm, subtype_ref)
                };

                meta.has_subtype = !meta.subtype_options.is_empty();
            }

            if let Some(cost_idx) = def
                .cost_table_ref
                .as_ref()
                .and_then(|r| r.parse::<u32>().ok())
            {
                meta.cost_table_idx = Some(cost_idx);
                if let Some(table) = self.cost_tables.get(&cost_idx) {
                    meta.cost_table_options = table.labels.clone();
                    meta.has_cost_table = !meta.cost_table_options.is_empty();
                }
            }

            if let Some(ref param_ref) = def.param1_ref
                && let Some(table_name) = resolve_param_table_from_rm(rm, param_ref)
            {
                meta.param1_idx = param_ref.parse::<u32>().ok();
                let param_key = get_subtype_context_key(&table_name);
                meta.param1_options = if let Some(key) = param_key {
                    match key {
                        "racial_groups" => context.racial_groups.clone(),
                        "classes" => context.classes.clone(),
                        _ => load_2da_options_from_rm(rm, &table_name),
                    }
                } else {
                    load_2da_options_from_rm(rm, &table_name)
                };
                meta.has_param1 = !meta.param1_options.is_empty();
            }

            meta.is_flat = !meta.has_subtype && !meta.has_cost_table && !meta.has_param1;
            metadata.push(meta);
        }

        metadata.sort_by(|a, b| a.label.cmp(&b.label));
        metadata
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ItemBonuses {
    pub str_bonus: i32,
    pub dex_bonus: i32,
    pub con_bonus: i32,
    pub int_bonus: i32,
    pub wis_bonus: i32,
    pub cha_bonus: i32,
    pub ac_bonus: i32,
    pub ac_armor_bonus: i32,
    pub ac_shield_bonus: i32,
    pub ac_deflection_bonus: i32,
    pub ac_natural_bonus: i32,
    pub ac_dodge_bonus: i32,
    pub attack_bonus: i32,
    pub damage_bonus: i32,
    pub fortitude_bonus: i32,
    pub reflex_bonus: i32,
    pub will_bonus: i32,
    pub spell_resistance: i32,
    pub skill_bonuses: HashMap<String, i32>,
    pub damage_resistances: HashMap<String, i32>,
}

fn normalize_ability_name(name: &str) -> &str {
    match name {
        "Str" | "Strength" => "Str",
        "Dex" | "Dexterity" => "Dex",
        "Con" | "Constitution" => "Con",
        "Int" | "Intelligence" => "Int",
        "Wis" | "Wisdom" => "Wis",
        "Cha" | "Charisma" => "Cha",
        _ => name,
    }
}

/// `itemprops.2da`: one row per `itempropdef` row, one column per base-item
/// class (`0_Melee` ... `21_Glove`, indexed by `baseitems.PropColumn`), `1`
/// where the engine honours the property on that class.
///
/// Returns `None` if the table itself failed to load, distinct from a loaded
/// table where a property's row is empty (genuinely disallowed everywhere).
pub fn load_itemprops_columns_from_rm(rm: &ResourceManager) -> Option<HashMap<u32, Vec<u32>>> {
    let mut allowed = HashMap::new();
    let Ok(table) = rm.get_2da_with_overrides("itemprops") else {
        warn!("itemprops.2da not available; property/base-item compatibility unknown");
        return None;
    };

    let columns: Vec<(u32, String)> = table
        .column_names()
        .into_iter()
        .filter_map(|name| {
            let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
            digits
                .parse::<u32>()
                .ok()
                .map(|idx| (idx, name.to_string()))
        })
        .collect();

    for row_idx in 0..table.row_count() {
        let cols: Vec<u32> = columns
            .iter()
            .filter(|(_, name)| {
                table
                    .get_cell_by_name(row_idx, name)
                    .ok()
                    .flatten()
                    .is_some_and(|v| v == "1")
            })
            .map(|(idx, _)| *idx)
            .collect();
        allowed.insert(row_idx as u32, cols);
    }
    Some(allowed)
}

/// `baseitems.2da` `AC_Enchant` column, keyed by base item row: the engine's
/// own record of which AC family (armor/shield/deflection) an item's AC
/// enchantment belongs to.
fn load_baseitem_ac_enchant_from_rm(rm: &ResourceManager) -> HashMap<i32, i32> {
    let mut ac_enchant = HashMap::new();
    let Ok(table) = rm.get_2da_with_overrides("baseitems") else {
        warn!("baseitems.2da not available; AC bonuses will default to deflection");
        return ac_enchant;
    };

    for row_idx in 0..table.row_count() {
        let Ok(row) = table.get_row_dict(row_idx) else {
            continue;
        };
        let value = row_int(&row, "ac_enchant", -1);
        if value >= 0 {
            ac_enchant.insert(row_idx as i32, value);
        }
    }
    ac_enchant
}

pub fn load_2da_options_from_rm(rm: &ResourceManager, table_name: &str) -> HashMap<u32, String> {
    let mut options = HashMap::new();
    let Ok(table) = rm.get_2da_with_overrides(table_name) else {
        return options;
    };

    for row_idx in 0..table.row_count() {
        let Ok(row) = table.get_row_dict(row_idx) else {
            continue;
        };

        let name_str_ref = row_int(&row, "name", -1);
        let name = if name_str_ref > 100 {
            let s = rm.get_string(name_str_ref);
            if s.is_empty() || s.chars().all(|c| c.is_ascii_digit()) {
                None
            } else {
                Some(s)
            }
        } else {
            None
        };

        let game_str = name.clone().or_else(|| {
            let sr = row_int(&row, "gamestring", -1);
            if sr < 0 {
                return None;
            }
            let s = rm.get_string(sr);
            if s.is_empty() || s.chars().all(|c| c.is_ascii_digit()) {
                None
            } else {
                Some(s)
            }
        });

        let label = game_str.or_else(|| row_str(&row, "label").filter(|s| s != "****"));

        if let Some(display_name) = label
            && !is_invalid_label(&display_name)
        {
            options.insert(row_idx as u32, clean_label(&display_name));
        }
    }
    options
}

/// Load one `iprp_*cost` table.
///
/// `amounts` is recorded for every row that carries a parsable numeric column,
/// independent of whether the row has a usable display name - `cost_amount`
/// consumers (damage resistance/vulnerability) need the value even for rows
/// `labels` rejects.
///
/// `labels` drops rows without a resolvable `Name` strref. The engine builds an
/// item property's description from that strref, so a row it cannot resolve
/// renders as "Bad StrRef" on the item in game - which is exactly what row 0
/// ("Random", Name `****`) of every cost table does. Random is an
/// authoring-time instruction for the treasure generator, not a value a
/// finished item can carry.
fn load_cost_table_from_rm(rm: &ResourceManager, table_name: &str) -> CostTableData {
    let mut data = CostTableData::default();
    let Ok(table) = rm.get_2da_with_overrides(table_name) else {
        return data;
    };

    for row_idx in 0..table.row_count() {
        let Ok(row) = table.get_row_dict(row_idx) else {
            continue;
        };

        for column in ["amount", "value"] {
            if let Some(amount) = row_str(&row, column).and_then(|s| s.parse::<i32>().ok()) {
                data.amounts.insert(row_idx as u32, amount);
                break;
            }
        }

        let name_str_ref = row_int(&row, "name", -1);
        if name_str_ref <= 100 {
            continue;
        }

        let resolved = rm.get_string(name_str_ref);
        if resolved.is_empty() {
            continue;
        }

        // Purely numeric strings ("1", "5") read better from the label column,
        // which clean_label turns into "+1"/"+5".
        let display = if resolved.chars().all(|c| c.is_ascii_digit()) {
            row_str(&row, "label").filter(|s| s != "****")
        } else {
            Some(resolved)
        };

        let Some(display) = display.filter(|s| !is_invalid_label(s)) else {
            continue;
        };

        data.labels.insert(row_idx as u32, clean_label(&display));
    }

    data
}

fn resolve_cost_table_from_rm(rm: &ResourceManager, cost_table_idx: &str) -> Option<String> {
    let idx: usize = cost_table_idx.parse().ok()?;
    let mapping = rm.get_2da_with_overrides("iprp_costtable").ok()?;
    let row = mapping.get_row_dict(idx).ok()?;
    row_str(&row, "name")
}

fn resolve_param_table_from_rm(rm: &ResourceManager, param_idx: &str) -> Option<String> {
    let idx: usize = param_idx.parse().ok()?;
    let mapping = rm.get_2da_with_overrides("iprp_paramtable").ok()?;
    let row = mapping.get_row_dict(idx).ok()?;
    row_str(&row, "tableresref")
}

fn get_u32(data: &HashMap<String, serde_json::Value>, key: &str) -> Option<u32> {
    data.get(key).and_then(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_i64().map(|n| n as u32))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_u32() {
        let mut data = HashMap::new();
        data.insert(
            "PropertyName".to_string(),
            serde_json::Value::Number(serde_json::Number::from(5u64)),
        );

        assert_eq!(get_u32(&data, "PropertyName"), Some(5));
        assert_eq!(get_u32(&data, "Missing"), None);
    }

    #[test]
    fn test_item_bonuses_default() {
        let bonuses = ItemBonuses::default();
        assert_eq!(bonuses.str_bonus, 0);
        assert_eq!(bonuses.ac_bonus, 0);
    }
}
