use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use super::gff_helpers::gff_value_to_i32;
use super::types::{AbilityIndex, ClassId, DomainId, SpellId};
use super::{Character, CharacterError};
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::row_bool;

pub const MAX_SPELL_LEVEL: i32 = 9;

/// Mod-added spells use name prefixes to distinguish from vanilla content.
/// These are internal to modpacks and not selectable in the game's spell UI.
pub fn is_mod_prefixed_name(name: &str) -> bool {
    name.starts_with("K's ")
}

pub fn is_displayable_spell(spell_row: &ahash::AHashMap<String, Option<String>>) -> bool {
    if spell_row
        .get("removed")
        .and_then(|v| v.as_ref())
        .is_some_and(|v| v == "1")
    {
        return false;
    }
    if spell_row
        .get("deleted")
        .and_then(|v| v.as_ref())
        .is_some_and(|v| !v.is_empty() && v != "****")
    {
        return false;
    }
    if spell_row
        .get("master")
        .and_then(|v| v.as_ref())
        .is_some_and(|v| !v.is_empty() && v != "****" && v.parse::<i32>().is_ok_and(|n| n >= 0))
    {
        return false;
    }
    if spell_row
        .get("usertype")
        .and_then(|v| v.as_ref())
        .is_some_and(|v| v != "1")
    {
        return false;
    }
    if spell_row
        .get("featid")
        .and_then(|v| v.as_ref())
        .is_some_and(|v| !v.is_empty() && v != "****" && v.parse::<i32>().is_ok())
    {
        return false;
    }
    if spell_row
        .get("label")
        .and_then(|v| v.as_ref())
        .is_some_and(|v| v.starts_with("SPELLABILITY_"))
    {
        return false;
    }
    true
}

static SCHOOL_LETTER_MAP: LazyLock<HashMap<char, i32>> = LazyLock::new(|| {
    HashMap::from([
        ('G', 0),
        ('A', 1),
        ('C', 2),
        ('D', 3),
        ('E', 4),
        ('V', 5),
        ('I', 6),
        ('N', 7),
        ('T', 8),
    ])
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct MemorizedSpellRaw {
    pub spell_id: SpellId,
    pub meta_magic: u8,
    pub ready: bool,
    pub is_domain: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct KnownSpellEntry {
    pub level: i32,
    pub spell_id: SpellId,
    pub name: String,
    pub icon: String,
    pub school_name: Option<String>,
    pub description: Option<String>,
    pub class_id: ClassId,
    pub is_domain_spell: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MemorizedSpellEntry {
    pub level: i32,
    pub spell_id: SpellId,
    pub name: String,
    pub icon: String,
    pub school_name: Option<String>,
    pub description: Option<String>,
    pub class_id: ClassId,
    pub metamagic: i32,
    pub ready: bool,
}

impl Character {
    /// Get all known spells for a specific class at a given spell level.
    ///
    /// Returns Vec<SpellId> of spell IDs from the "KnownListN" field where N is the spell level.
    /// Returns empty Vec if class doesn't have that spell level or no spells known.
    pub fn known_spells(&self, class_id: ClassId, spell_level: i32) -> Vec<SpellId> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Vec::new();
        }

        let Some(class_list) = self.get_list("ClassList") else {
            return Vec::new();
        };

        let Some(class_entry) = class_list.iter().find(|entry| {
            gff_value_to_i32(entry.get("Class").unwrap_or(&GffValue::Int(-1))) == Some(class_id.0)
        }) else {
            return Vec::new();
        };

        let field_name = format!("KnownList{spell_level}");
        let Some(GffValue::ListOwned(known_list)) = class_entry.get(&field_name) else {
            return Vec::new();
        };

        known_list
            .iter()
            .filter_map(|entry| {
                let spell_id = gff_value_to_i32(entry.get("Spell").unwrap_or(&GffValue::Int(-1)))?;
                if spell_id >= 0 {
                    Some(SpellId(spell_id))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if a specific spell is known by a class.
    ///
    /// Searches all spell levels (0-9) to find the spell.
    pub fn has_known_spell(&self, class_id: ClassId, spell_id: SpellId) -> bool {
        for spell_level in 0..=MAX_SPELL_LEVEL {
            let known = self.known_spells(class_id, spell_level);
            if known.contains(&spell_id) {
                return true;
            }
        }
        false
    }

    /// Get all memorized spells for a specific class at a given spell level.
    ///
    /// Returns Vec<MemorizedSpellRaw> with spell ID, metamagic, and ready status.
    pub fn memorized_spells(&self, class_id: ClassId, spell_level: i32) -> Vec<MemorizedSpellRaw> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Vec::new();
        }

        let Some(class_list) = self.get_list("ClassList") else {
            return Vec::new();
        };

        let Some(class_entry) = class_list.iter().find(|entry| {
            gff_value_to_i32(entry.get("Class").unwrap_or(&GffValue::Int(-1))) == Some(class_id.0)
        }) else {
            return Vec::new();
        };

        let field_name = format!("MemorizedList{spell_level}");
        let Some(GffValue::ListOwned(mem_list)) = class_entry.get(&field_name) else {
            return Vec::new();
        };

        mem_list
            .iter()
            .filter_map(|entry| {
                let spell_id = gff_value_to_i32(entry.get("Spell").unwrap_or(&GffValue::Int(-1)))?;
                if spell_id < 0 {
                    return None;
                }

                let meta_magic = entry
                    .get("SpellMetaMagic")
                    .or_else(|| entry.get("SpellMetaMagicN2"))
                    .and_then(|v| match v {
                        GffValue::Byte(b) => Some(*b),
                        _ => gff_value_to_i32(v).map(|i| i as u8),
                    })
                    .unwrap_or(0);

                let ready = entry
                    .get("Ready")
                    .and_then(gff_value_to_i32)
                    .map(|v| v != 0)
                    .or_else(|| {
                        entry.get("SpellFlags").and_then(|v| match v {
                            GffValue::Byte(b) => Some(*b & 0x01 != 0),
                            _ => None,
                        })
                    })
                    .unwrap_or(true);

                let is_domain = entry
                    .get("SpellDomain")
                    .and_then(gff_value_to_i32)
                    .is_some_and(|v| v == 1);

                Some(MemorizedSpellRaw {
                    spell_id: SpellId(spell_id),
                    meta_magic,
                    ready,
                    is_domain,
                })
            })
            .collect()
    }

    /// Add a known spell to a class at a specific spell level.
    ///
    /// `class_index` is the index into ClassList (0, 1, or 2 for most characters).
    /// Returns error if class_index is out of bounds or spell already known.
    pub fn add_known_spell(
        &mut self,
        class_index: usize,
        spell_level: i32,
        spell_id: SpellId,
    ) -> Result<(), CharacterError> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Err(CharacterError::InvalidOperation(format!(
                "Spell level {spell_level} out of range [0, {MAX_SPELL_LEVEL}]"
            )));
        }

        let Some(mut class_list) = self.get_list_owned("ClassList") else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };

        if class_index >= class_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "Class index {} out of range [0, {})",
                class_index,
                class_list.len()
            )));
        }

        let class_entry = &mut class_list[class_index];
        let field_name = format!("KnownList{spell_level}");

        let mut known_list = match class_entry.get(&field_name) {
            Some(GffValue::ListOwned(list)) => list.clone(),
            _ => Vec::new(),
        };

        if known_list.iter().any(|e| {
            gff_value_to_i32(e.get("Spell").unwrap_or(&GffValue::Int(-1))) == Some(spell_id.0)
        }) {
            return Err(CharacterError::AlreadyExists {
                entity: "Spell",
                id: spell_id.0,
            });
        }

        let mut new_spell = IndexMap::new();
        new_spell.insert("Spell".to_string(), GffValue::Word(spell_id.0 as u16));
        known_list.push(new_spell);

        class_entry.insert(field_name, GffValue::ListOwned(known_list));
        self.set_list("ClassList", class_list);

        self.record_spell_change(spell_level, spell_id.0, true);

        Ok(())
    }

    /// Remove a known spell from a class at a specific spell level.
    ///
    /// `class_index` is the index into ClassList (0, 1, or 2 for most characters).
    /// Returns error if class_index is out of bounds or spell not found.
    pub fn remove_known_spell(
        &mut self,
        class_index: usize,
        spell_level: i32,
        spell_id: SpellId,
    ) -> Result<(), CharacterError> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Err(CharacterError::InvalidOperation(format!(
                "Spell level {spell_level} out of range [0, {MAX_SPELL_LEVEL}]"
            )));
        }

        let Some(mut class_list) = self.get_list_owned("ClassList") else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };

        if class_index >= class_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "Class index {} out of range [0, {})",
                class_index,
                class_list.len()
            )));
        }

        let class_entry = &mut class_list[class_index];
        let field_name = format!("KnownList{spell_level}");

        let mut known_list = match class_entry.get(&field_name) {
            Some(GffValue::ListOwned(list)) => list.clone(),
            _ => Vec::new(),
        };

        let original_len = known_list.len();
        known_list.retain(|e| {
            gff_value_to_i32(e.get("Spell").unwrap_or(&GffValue::Int(-1))) != Some(spell_id.0)
        });

        if known_list.len() == original_len {
            return Err(CharacterError::NotFound {
                entity: "Spell",
                id: spell_id.0,
            });
        }

        class_entry.insert(field_name, GffValue::ListOwned(known_list));
        self.set_list("ClassList", class_list);

        self.record_spell_change(spell_level, spell_id.0, false);

        Ok(())
    }

    /// Add a memorized spell to a class at a specific spell level.
    ///
    /// `class_index` is the index into ClassList (0, 1, or 2 for most characters).
    pub fn add_memorized_spell(
        &mut self,
        class_index: usize,
        spell_level: i32,
        spell: MemorizedSpellRaw,
    ) -> Result<(), CharacterError> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Err(CharacterError::InvalidOperation(format!(
                "Spell level {spell_level} out of range [0, {MAX_SPELL_LEVEL}]"
            )));
        }

        let Some(mut class_list) = self.get_list_owned("ClassList") else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };

        if class_index >= class_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "Class index {} out of range [0, {})",
                class_index,
                class_list.len()
            )));
        }

        let class_entry = &mut class_list[class_index];
        let field_name = format!("MemorizedList{spell_level}");

        let mut mem_list = match class_entry.get(&field_name) {
            Some(GffValue::ListOwned(list)) => list.clone(),
            _ => Vec::new(),
        };

        let mut new_spell = IndexMap::new();
        new_spell.insert("Spell".to_string(), GffValue::Word(spell.spell_id.0 as u16));
        // NWN2-EE renamed SpellMetaMagic (Byte) to SpellMetaMagicN2 (Dword) to
        // support more metamagic flags; writing the old name/type leaves a
        // phantom field that the engine ignores.
        new_spell.insert(
            "SpellMetaMagicN2".to_string(),
            GffValue::Dword(u32::from(spell.meta_magic)),
        );
        new_spell.insert("Ready".to_string(), GffValue::Byte(u8::from(spell.ready)));
        mem_list.push(new_spell);

        class_entry.insert(field_name, GffValue::ListOwned(mem_list));
        self.set_list("ClassList", class_list);

        Ok(())
    }

    /// Clear all memorized spells for a class at a specific spell level.
    ///
    /// `class_index` is the index into ClassList (0, 1, or 2 for most characters).
    pub fn clear_memorized_spells(
        &mut self,
        class_index: usize,
        spell_level: i32,
    ) -> Result<(), CharacterError> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Err(CharacterError::InvalidOperation(format!(
                "Spell level {spell_level} out of range [0, {MAX_SPELL_LEVEL}]"
            )));
        }

        let Some(mut class_list) = self.get_list_owned("ClassList") else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };

        if class_index >= class_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "Class index {} out of range [0, {})",
                class_index,
                class_list.len()
            )));
        }

        let class_entry = &mut class_list[class_index];
        let field_name = format!("MemorizedList{spell_level}");

        class_entry.insert(field_name, GffValue::ListOwned(Vec::new()));
        self.set_list("ClassList", class_list);

        Ok(())
    }

    /// Clear all memorized spells for a class across all spell levels.
    ///
    /// `class_index` is the index into ClassList (0, 1, or 2 for most characters).
    pub fn clear_all_memorized_spells(&mut self, class_index: usize) -> Result<(), CharacterError> {
        for spell_level in 0..=MAX_SPELL_LEVEL {
            self.clear_memorized_spells(class_index, spell_level)?;
        }
        Ok(())
    }

    /// Check if a class is a spellcaster.
    ///
    /// Returns true if the class has a SpellCaster field > 0 in the classes table.
    pub fn is_spellcaster(&self, class_id: ClassId, game_data: &GameData) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        if let Some(spell_caster_str) = class_data.get("spellcaster")
            && let Some(spell_caster_value) = spell_caster_str
            && spell_caster_value != "0"
            && spell_caster_value != "****"
        {
            return true;
        }

        false
    }

    /// Check if a class is a prepared caster (requires memorization).
    ///
    /// Returns true if the class has MemorizesSpells field set to 1.
    /// False indicates spontaneous casting (e.g., Sorcerer, Bard).
    pub fn is_prepared_caster(&self, class_id: ClassId, game_data: &GameData) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        row_bool(&class_data, "memorizesspells", false)
    }

    /// Check if a class uses "all spells known" mode (like Clerics/Druids).
    ///
    /// Classes with AllSpellsKnown=1 automatically know all spells from their list
    /// and don't need spells added to KnownList.
    pub fn uses_all_spells_known(&self, class_id: ClassId, game_data: &GameData) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        row_bool(&class_data, "allspellsknown", false)
    }

    /// Check if a class is a divine caster (has domains).
    ///
    /// Returns true if the class has HasDomains=1 or MaxDomains > 0.
    pub fn is_divine_caster(&self, class_id: ClassId, game_data: &GameData) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        if row_bool(&class_data, "hasdomains", false) {
            return true;
        }

        if let Some(max_domains_str) = class_data.get("maxdomains").and_then(|v| v.as_ref())
            && let Ok(max_domains) = max_domains_str.parse::<i32>()
            && max_domains > 0
        {
            return true;
        }

        false
    }

    /// Get the spell level for a specific spell and class.
    ///
    /// Looks up the spell level from the spells.2da table using the class's
    /// SpellTableColumn field. Returns None if spell is not available to the class.
    pub fn get_spell_level_for_class(
        &self,
        spell_id: SpellId,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Option<i32> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id.0)?;

        let spell_column = class_data
            .get("spelltablecolumn")
            .and_then(|v| v.as_ref())
            .map(|s| s.to_lowercase())
            .filter(|v| !v.is_empty() && *v != "****")?;

        let spells_table = game_data.get_table("spells")?;
        let spell_data = spells_table.get_by_id(spell_id.0)?;

        let level_str = spell_data.get(&spell_column).and_then(|v| v.as_ref())?;
        let level = level_str.parse::<i32>().ok()?;

        if (0..=MAX_SPELL_LEVEL).contains(&level) {
            Some(level)
        } else {
            None
        }
    }

    /// Check if a class has domains selected.
    ///
    /// Returns true if Domain1 or Domain2 is >= 0 for the given class.
    fn has_domains_selected(&self, class_id: ClassId) -> bool {
        let Some(class_list) = self.get_list("ClassList") else {
            return false;
        };

        class_list.iter().any(|entry| {
            let entry_class_id =
                gff_value_to_i32(entry.get("Class").unwrap_or(&GffValue::Int(-1))).unwrap_or(-1);
            if entry_class_id != class_id.0 {
                return false;
            }

            let domain1 = entry
                .get("Domain1")
                .and_then(gff_value_to_i32)
                .unwrap_or(-1);
            let domain2 = entry
                .get("Domain2")
                .and_then(gff_value_to_i32)
                .unwrap_or(-1);

            domain1 >= 0 || domain2 >= 0
        })
    }

    /// Calculate spell slots per day for all spellcasting classes.
    /// Returns a map of class_id -> spell_level -> slot_count.
    /// Includes bonus slots from ability modifiers and domain slots for divine casters.
    pub fn calculate_all_spell_slots(
        &self,
        game_data: &GameData,
    ) -> HashMap<i32, HashMap<i32, i32>> {
        let mut slots_by_class: HashMap<i32, HashMap<i32, i32>> = HashMap::new();

        let Some(class_list) = self.get_list("ClassList") else {
            return slots_by_class;
        };

        let Some(classes_table) = game_data.get_table("classes") else {
            return slots_by_class;
        };

        for entry in class_list {
            let class_id =
                gff_value_to_i32(entry.get("Class").unwrap_or(&GffValue::Int(-1))).unwrap_or(-1);
            if class_id < 0 {
                continue;
            }

            let class_level = entry
                .get("ClassLevel")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);
            if class_level == 0 {
                continue;
            }

            let Some(class_data) = classes_table.get_by_id(class_id) else {
                continue;
            };

            if !row_bool(&class_data, "spellcaster", false) {
                continue;
            }

            // Get spell gain table
            let spell_table_name = class_data
                .get("spellgaintable")
                .and_then(|v| v.as_ref())
                .filter(|s| !s.is_empty() && *s != "****");

            let Some(spell_table_name) = spell_table_name else {
                // Prestige caster without own spell table
                continue;
            };

            let Some(spell_table) = game_data.get_table(&spell_table_name.to_lowercase()) else {
                continue;
            };

            // Use SpellCasterLevel if available (for prestige class progression), otherwise use ClassLevel
            let effective_level = entry
                .get("SpellCasterLevel")
                .and_then(gff_value_to_i32)
                .filter(|&l| l > 0)
                .unwrap_or(class_level);

            let table_row_idx = (effective_level - 1).max(0) as usize;

            // Get casting ability
            let casting_ability = self.get_casting_ability_for_class(class_id, game_data);
            let ability_mod = casting_ability
                .map(|ability| self.ability_modifier(ability))
                .unwrap_or(0);

            // Check if has domains (for divine casters)
            let has_domains = entry
                .get("Domain1")
                .and_then(gff_value_to_i32)
                .is_some_and(|d| d >= 0)
                || entry
                    .get("Domain2")
                    .and_then(gff_value_to_i32)
                    .is_some_and(|d| d >= 0);

            let mut class_slots: HashMap<i32, i32> = HashMap::new();

            // Calculate slots for each spell level (0-9)
            for spell_level in 0..=MAX_SPELL_LEVEL {
                let column_name = format!("spelllevel{spell_level}");

                let base_slots = spell_table
                    .get_cell(table_row_idx, &column_name)
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);

                if base_slots <= 0 {
                    continue;
                }

                // Calculate bonus slots from ability modifier
                let bonus_slots = if spell_level > 0 && ability_mod >= spell_level {
                    1 + (ability_mod - spell_level) / 4
                } else {
                    0
                };

                // Add domain slot for divine casters (1 extra slot per level 1-9)
                let domain_slots = i32::from(has_domains && spell_level > 0);

                let total_slots = base_slots + bonus_slots + domain_slots;
                class_slots.insert(spell_level, total_slots);
            }

            if !class_slots.is_empty() {
                slots_by_class.insert(class_id, class_slots);
            }
        }

        slots_by_class
    }

    /// Get the casting ability for a class (Int for Wizard, Wis for Cleric, Cha for Sorcerer, etc.)
    fn get_casting_ability_for_class(
        &self,
        class_id: i32,
        game_data: &GameData,
    ) -> Option<AbilityIndex> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id)?;

        // Try different field names for the primary ability
        let ability_str = class_data
            .get("primaryabil")
            .or_else(|| class_data.get("spellability"))
            .or_else(|| class_data.get("spellcastingabil"))
            .and_then(|v| v.as_ref())
            .filter(|s| !s.is_empty() && *s != "****")?;

        // Map the ability name to AbilityIndex
        match ability_str.to_lowercase().as_str() {
            "str" | "strength" => Some(AbilityIndex::STR),
            "dex" | "dexterity" => Some(AbilityIndex::DEX),
            "con" | "constitution" => Some(AbilityIndex::CON),
            "int" | "intelligence" => Some(AbilityIndex::INT),
            "wis" | "wisdom" => Some(AbilityIndex::WIS),
            "cha" | "charisma" => Some(AbilityIndex::CHA),
            _ => None,
        }
    }

    /// Grant initial spellbook to a book caster (like Wizard).
    ///
    /// Adds all cantrips (level 0 spells) from the spell list to the known spells.
    /// Only applies to prepared casters that don't use "all spells known" mode.
    pub fn grant_initial_spellbook(
        &mut self,
        class_index: usize,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let Some(class_list) = self.get_list("ClassList") else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };

        if class_index >= class_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "Class index {} out of range [0, {})",
                class_index,
                class_list.len()
            )));
        }

        let class_id_val = gff_value_to_i32(
            class_list[class_index]
                .get("Class")
                .unwrap_or(&GffValue::Int(-1)),
        )
        .unwrap_or(-1);
        let class_id = ClassId(class_id_val);

        if !self.is_spellcaster(class_id, game_data) {
            return Ok(());
        }

        if self.uses_all_spells_known(class_id, game_data) {
            return Ok(());
        }

        if !self.is_prepared_caster(class_id, game_data) {
            return Ok(());
        }

        let Some(classes_table) = game_data.get_table("classes") else {
            return Ok(());
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return Ok(());
        };

        let Some(spell_column) = class_data
            .get("spelltablecolumn")
            .and_then(|v| v.as_ref())
            .map(|s| s.to_lowercase())
            .filter(|v| !v.is_empty() && *v != "****")
        else {
            return Ok(());
        };

        let Some(spells_table) = game_data.get_table("spells") else {
            return Ok(());
        };

        for row_id in 0..spells_table.row_count() {
            let Ok(spell_row) = spells_table.get_row(row_id) else {
                continue;
            };

            let level_str = spell_row.get(&spell_column).and_then(|v| v.as_ref());
            if let Some(level_str) = level_str
                && let Ok(level) = level_str.parse::<i32>()
                && level == 0
            {
                let _ = self.add_known_spell(class_index, 0, SpellId(row_id as i32));
            }
        }

        Ok(())
    }

    /// Calculate base spell slots for a class at the character's current level.
    ///
    /// Returns a Vec of 10 elements (spell levels 0-9) with base slot counts.
    /// This does NOT include bonus slots from ability scores.
    pub fn calculate_spell_slots(&self, class_id: ClassId, game_data: &GameData) -> Vec<i32> {
        let mut slots = vec![0; 10];

        let class_level = self.class_level(class_id);
        if class_level == 0 {
            return slots;
        }

        let Some(classes_table) = game_data.get_table("classes") else {
            return slots;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return slots;
        };

        let Some(spell_gain_table_name) = class_data.get("spellgaintable").and_then(|v| v.as_ref())
        else {
            return slots;
        };

        if spell_gain_table_name.is_empty() || spell_gain_table_name == "****" {
            return slots;
        }

        let table_name_lower = spell_gain_table_name.to_lowercase();
        let Some(spell_gain_table) = game_data.get_table(&table_name_lower) else {
            return slots;
        };

        let caster_level = self.get_caster_level(class_id, game_data);
        if caster_level <= 0 {
            return slots;
        }

        let table_row_index = (caster_level - 1) as usize;
        if table_row_index >= spell_gain_table.row_count() {
            return slots;
        }

        let Ok(row_data) = spell_gain_table.get_row(table_row_index) else {
            return slots;
        };

        for spell_level in 0..=MAX_SPELL_LEVEL {
            let field_name = format!("spelllevel{spell_level}");
            if let Some(slot_value_str) = row_data.get(&field_name).and_then(|v| v.as_ref())
                && let Ok(base_slots) = slot_value_str.parse::<i32>()
            {
                slots[spell_level as usize] = base_slots.max(0);
            }
        }

        // Add domain slot bonus for divine casters with domains selected
        if self.is_divine_caster(class_id, game_data) && self.has_domains_selected(class_id) {
            for spell_level in 1..=MAX_SPELL_LEVEL {
                if slots[spell_level as usize] > 0 {
                    slots[spell_level as usize] += 1;
                }
            }
        }

        slots
    }

    /// Calculate bonus spell slots for a specific spell level based on ability modifier.
    ///
    /// Formula: if ability_mod >= spell_level then 1 + (ability_mod - spell_level) / 4 else 0
    /// Returns 0 for cantrips (level 0).
    pub fn calculate_bonus_spell_slots(
        &self,
        class_id: ClassId,
        spell_level: i32,
        game_data: &GameData,
    ) -> i32 {
        if spell_level == 0 {
            return 0;
        }

        let Some(ability) = self.get_casting_ability(class_id, game_data) else {
            return 0;
        };

        let ability_score = self.base_ability(ability);
        let ability_modifier = (ability_score - 10).div_euclid(2);

        if ability_modifier >= spell_level {
            1 + ((ability_modifier - spell_level) / 4).max(0)
        } else {
            0
        }
    }

    /// Get the primary spellcasting ability for a class.
    ///
    /// Checks PrimaryAbil, SpellAbility, or SpellcastingAbil fields in the classes table.
    /// Returns None if no spellcasting ability is defined.
    pub fn get_casting_ability(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Option<AbilityIndex> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id.0)?;

        let ability_str = class_data
            .get("primaryabil")
            .and_then(|v| v.as_ref())
            .or_else(|| class_data.get("spellability").and_then(|v| v.as_ref()))
            .or_else(|| class_data.get("spellcastingabil").and_then(|v| v.as_ref()))?;

        if ability_str.is_empty() || ability_str == "****" {
            return None;
        }

        AbilityIndex::from_gff_field(ability_str)
    }

    /// Get the effective caster level for a class.
    ///
    /// Takes into account the SpellCaster field which determines spell progression:
    /// - Type 2: Bard-like (2/3 progression: caster_level = class_level - 3, min 0)
    /// - Type 3: Paladin/Ranger-like (1/2 progression: caster_level = class_level / 2)
    /// - Type 4: Full caster (1:1 progression: caster_level = class_level)
    /// - Other: Defaults to class_level
    ///
    /// Also checks for a "SpellCasterLevel" override field in the ClassList entry.
    pub fn get_caster_level(&self, class_id: ClassId, game_data: &GameData) -> i32 {
        let Some(classes_table) = game_data.get_table("classes") else {
            return 0;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return 0;
        };

        // Check for PrC/Homebrew override in ClassList
        let mut explicit_level = None;
        if let Some(class_list) = self.get_list("ClassList")
            && let Some(entry) = class_list.iter().find(|e| {
                gff_value_to_i32(e.get("Class").unwrap_or(&GffValue::Int(-1))) == Some(class_id.0)
            })
        {
            // Check "SpellCasterLevel" field
            if let Some(scl) = entry
                .get("SpellCasterLevel")
                .and_then(gff_value_to_i32)
                .filter(|l| *l > 0)
            {
                explicit_level = Some(scl);
            }
        }

        if let Some(lvl) = explicit_level {
            return lvl;
        }

        let class_level = self.class_level(class_id);
        if class_level == 0 {
            return 0;
        }

        if let Some(spell_caster_type_str) = class_data.get("spellcaster").and_then(|v| v.as_ref())
            && let Ok(sct) = spell_caster_type_str.parse::<i32>()
        {
            return match sct {
                2 => (class_level - 3).max(0),
                3 => class_level / 2,
                4 => class_level,
                _ => class_level,
            };
        }

        class_level
    }

    /// Get the SpellKnownTable name for a class (e.g., "cls_spkn_sorc").
    /// Returns None if the class doesn't use a spell known table (prepared casters).
    fn get_spell_known_table_name(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Option<String> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id.0)?;

        class_data
            .get("spellknowntable")
            .and_then(|v| v.as_ref())
            .filter(|s| !s.is_empty() && *s != "****")
            .map(|s| s.to_lowercase())
    }

    /// Get expected spells known per level from the SpellKnownTable.
    /// Returns HashMap of spell_level -> expected count at the current class level.
    fn get_expected_spells_known_from_table(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> HashMap<i32, i32> {
        let mut expected: HashMap<i32, i32> = HashMap::new();

        let Some(table_name) = self.get_spell_known_table_name(class_id, game_data) else {
            return expected;
        };

        let Some(spell_known_table) = game_data.get_table(&table_name) else {
            return expected;
        };

        let class_level = self.class_level(class_id);
        if class_level <= 0 {
            return expected;
        }

        let row_index = (class_level - 1) as usize;
        let Ok(row_data) = spell_known_table.get_row(row_index) else {
            return expected;
        };

        for spell_level in 0..=MAX_SPELL_LEVEL {
            let column_name = format!("spelllevel{spell_level}");
            if let Some(count_str) = row_data.get(&column_name).and_then(|v| v.as_ref())
                && let Ok(count) = count_str.parse::<i32>()
                && count > 0
            {
                expected.insert(spell_level, count);
            }
        }

        expected
    }

    /// Count actual spells known per level from KnownList fields.
    fn get_actual_spells_known_count(&self, class_id: ClassId) -> HashMap<i32, i32> {
        let mut actual: HashMap<i32, i32> = HashMap::new();

        for spell_level in 0..=MAX_SPELL_LEVEL {
            let known = self.known_spells(class_id, spell_level);
            if !known.is_empty() {
                actual.insert(spell_level, known.len() as i32);
            }
        }

        actual
    }

    /// Check if a class is a spellbook caster (learns spells into a spellbook).
    /// These are prepared casters that don't auto-know all spells and don't use SpellKnownTable.
    /// Vanilla: Wizard. Mods may add similar classes.
    fn is_spellbook_caster(&self, class_id: ClassId, game_data: &GameData) -> bool {
        let is_prepared = self.is_prepared_caster(class_id, game_data);
        let all_spells_known = self.uses_all_spells_known(class_id, game_data);
        let has_spell_known_table = self
            .get_spell_known_table_name(class_id, game_data)
            .is_some();

        // Spellbook casters: prepare spells, don't auto-know all, no SpellKnownTable
        is_prepared && !all_spells_known && !has_spell_known_table
    }

    /// Spellbook casters (Wizard) gain 2 spells per level (engine-hardcoded behavior).
    const SPELLBOOK_SPELLS_PER_LEVEL: i32 = 2;

    /// Get pending spells to learn for a class.
    /// Returns None if this class doesn't need to select spells (Cleric, Druid, etc.)
    pub fn get_pending_spells_to_learn(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Option<PendingSpellLearning> {
        // AllSpellsKnown classes (Cleric, Druid) auto-know all spells — nothing to learn
        if self.uses_all_spells_known(class_id, game_data) {
            return None;
        }

        let class_name = self.get_class_name(class_id, game_data);

        // Spellbook casters (Wizard): 2 spells per level
        if self.is_spellbook_caster(class_id, game_data) {
            let class_level = self.class_level(class_id);
            let expected_total = class_level * Self::SPELLBOOK_SPELLS_PER_LEVEL;

            let actual = self.get_actual_spells_known_count(class_id);
            let actual_total: i32 = actual
                .iter()
                .filter(|(lvl, _)| **lvl > 0)
                .map(|(_, &count)| count)
                .sum();

            let pending = expected_total - actual_total;
            if pending <= 0 {
                return None;
            }

            return Some(PendingSpellLearning {
                class_id,
                class_name,
                caster_type: "spellbook".to_string(),
                by_level: HashMap::new(),
                total: pending,
            });
        }

        // Spontaneous casters (Sorcerer, Bard, etc.): use SpellKnownTable
        let expected = self.get_expected_spells_known_from_table(class_id, game_data);
        if expected.is_empty() {
            return None;
        }

        let actual = self.get_actual_spells_known_count(class_id);

        let mut by_level: HashMap<i32, i32> = HashMap::new();
        let mut total = 0;

        for (spell_level, expected_count) in &expected {
            let actual_count = actual.get(spell_level).copied().unwrap_or(0);
            let pending = expected_count - actual_count;
            if pending > 0 {
                by_level.insert(*spell_level, pending);
                total += pending;
            }
        }

        if total == 0 {
            return None;
        }

        Some(PendingSpellLearning {
            class_id,
            class_name,
            caster_type: "spontaneous".to_string(),
            by_level,
            total,
        })
    }

    /// Get a comprehensive spell summary for all spellcasting classes (internal use).
    ///
    /// Iterates the raw ClassList with proper indices so that `class_list_index`
    /// matches the positional index used by `add_known_spell`/`remove_known_spell`.
    fn get_spell_summary_internal(&self, game_data: &GameData) -> Vec<ClassSpellInfoInternal> {
        let Some(class_list) = self.get_list("ClassList") else {
            return Vec::new();
        };

        let mut classes = Vec::new();

        for (list_idx, entry) in class_list.iter().enumerate() {
            let class_id_val =
                gff_value_to_i32(entry.get("Class").unwrap_or(&GffValue::Int(-1))).unwrap_or(-1);
            if class_id_val < 0 {
                continue;
            }
            let class_id = ClassId(class_id_val);

            if !self.is_spellcaster(class_id, game_data) {
                continue;
            }

            let caster_level = self.get_caster_level(class_id, game_data);
            let is_prepared = self.is_prepared_caster(class_id, game_data);
            let base_slots = self.calculate_spell_slots(class_id, game_data);

            let mut bonus_slots = vec![0; 10];
            for spell_level in 1..=MAX_SPELL_LEVEL {
                bonus_slots[spell_level as usize] =
                    self.calculate_bonus_spell_slots(class_id, spell_level, game_data);
            }

            classes.push(ClassSpellInfoInternal {
                class_list_index: list_idx,
                class_id,
                caster_level,
                is_prepared,
                slots: base_slots,
                bonus_slots,
            });
        }

        classes
    }

    const LEARNABLE_CLASS_COLUMNS: &[&str] = &[
        "bard",
        "cleric",
        "druid",
        "paladin",
        "ranger",
        "wiz_sorc",
        "favsoul",
        "spiritshm",
    ];

    fn is_on_learnable_caster_list(spell_row: &ahash::AHashMap<String, Option<String>>) -> bool {
        Self::LEARNABLE_CLASS_COLUMNS.iter().any(|&col| {
            spell_row
                .get(col)
                .and_then(|v| v.as_ref())
                .is_some_and(|v| {
                    !v.is_empty() && v != "****" && v.parse::<i32>().is_ok_and(|n| n >= 0)
                })
        })
    }

    pub fn get_ability_spells(&self, game_data: &GameData) -> Vec<AbilitySpellEntry> {
        let Some(spells_table) = game_data.get_table("spells") else {
            return Vec::new();
        };

        let char_feats: HashSet<i32> = self.feat_ids().iter().map(|f| f.0).collect();

        // Build reverse map: spell_id -> true for spells referenced by character's feats
        // via feat.2da's spellid column (e.g. racial feats like Aasimar Light)
        let mut feat_granted_spells: HashSet<i32> = HashSet::new();
        if let Some(feat_table) = game_data.get_table("feat") {
            for feat_id in &char_feats {
                if let Ok(feat_row) = feat_table.get_row(*feat_id as usize)
                    && let Some(spell_id) = feat_row
                        .get("spellid")
                        .and_then(|v| v.as_ref())
                        .and_then(|v| v.parse::<i32>().ok())
                {
                    feat_granted_spells.insert(spell_id);
                }
            }
        }

        let mut known_spell_ids: HashSet<i32> = HashSet::new();
        for class_entry in self.class_entries() {
            for level in 0..=9 {
                for spell_id in self.known_spells(class_entry.class_id, level) {
                    known_spell_ids.insert(spell_id.0);
                }
            }
        }

        let mut ability_spells = Vec::new();
        let mut seen = HashSet::new();

        for row_id in 0..spells_table.row_count() {
            let Ok(spell_row) = spells_table.get_row(row_id) else {
                continue;
            };

            if spell_row
                .get("removed")
                .and_then(|v| v.as_ref())
                .is_some_and(|v| v == "1")
            {
                continue;
            }

            let spell_id_val = row_id as i32;
            let user_type = spell_row
                .get("usertype")
                .and_then(|v| v.as_ref())
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(1);
            let feat_id = spell_row
                .get("featid")
                .and_then(|v| v.as_ref())
                .and_then(|v| v.parse::<i32>().ok());
            let is_spellability = spell_row
                .get("label")
                .and_then(|v| v.as_ref())
                .is_some_and(|v| v.starts_with("SPELLABILITY_"));
            let on_caster_list = Self::is_on_learnable_caster_list(&spell_row);

            let has_feat = feat_id.is_some_and(|fid| char_feats.contains(&fid));
            let has_feat_granted = feat_granted_spells.contains(&spell_id_val);
            let is_known_ability = known_spell_ids.contains(&spell_id_val)
                && (user_type != 1 || is_spellability || !on_caster_list);

            if !has_feat && !has_feat_granted && !is_known_ability {
                continue;
            }
            if !seen.insert(spell_id_val) {
                continue;
            }

            let spell_id = SpellId(spell_id_val);
            let Some(details) = self.get_spell_details(spell_id, game_data) else {
                continue;
            };

            let innate_level = spell_row
                .get("innate")
                .and_then(|v| v.as_ref())
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0);

            ability_spells.push(AbilitySpellEntry {
                spell_id: spell_id_val,
                name: details.name,
                icon: details.icon,
                description: details.description,
                school_name: details.school_name,
                innate_level,
            });
        }

        ability_spells.sort_by(|a, b| a.name.cmp(&b.name));
        ability_spells
    }

    pub fn get_spells_state(&self, game_data: &GameData) -> SpellsState {
        let internal_classes = self.get_spell_summary_internal(game_data);

        let mut spellcasting_classes = Vec::new();
        let mut known_spells = Vec::new();
        let mut memorized_spells = Vec::new();
        let mut caster_class_summaries = Vec::new();
        let mut total_spell_levels = 0;

        let domain_spell_map = self.collect_domain_spells(game_data);

        // Pre-compute class info for all spellcasting classes
        struct ClassSpellConfig {
            class_id: ClassId,
            all_spells_known: bool,
            spell_column: Option<String>,
            max_castable_level: i32,
        }

        let mut class_configs: Vec<ClassSpellConfig> = Vec::new();

        for class_info in &internal_classes {
            let class_name = self.get_class_name(class_info.class_id, game_data);
            let class_level = self.class_level(class_info.class_id);
            let all_spells_known = self.uses_all_spells_known(class_info.class_id, game_data);
            let can_edit_spells = !all_spells_known;
            let spell_type = if class_info.is_prepared {
                "prepared"
            } else {
                "spontaneous"
            };

            let mut total_slots = 0;
            let mut max_spell_level = 0;
            let mut slots_by_level = HashMap::new();

            for level in 0..=MAX_SPELL_LEVEL {
                let base = class_info.slots.get(level as usize).copied().unwrap_or(0);
                // Bonus spell slots only apply to levels where the caster has base slots
                let bonus = if base > 0 {
                    class_info
                        .bonus_slots
                        .get(level as usize)
                        .copied()
                        .unwrap_or(0)
                } else {
                    0
                };
                let slot_count = base + bonus;
                if slot_count > 0 {
                    total_slots += slot_count;
                    max_spell_level = level;
                    slots_by_level.insert(level, slot_count);
                }
            }
            total_spell_levels += max_spell_level;

            let expected_spells_known = if !class_info.is_prepared && !all_spells_known {
                self.get_expected_spells_known_from_table(class_info.class_id, game_data)
            } else {
                HashMap::new()
            };

            spellcasting_classes.push(SpellcastingClass {
                index: class_info.class_list_index as i32,
                class_id: class_info.class_id,
                class_name: class_name.clone(),
                class_level,
                caster_level: class_info.caster_level,
                spell_type: spell_type.to_string(),
                can_edit_spells,
                expected_spells_known,
            });

            caster_class_summaries.push(CasterClassSummary {
                id: class_info.class_id,
                name: class_name.clone(),
                total_slots,
                max_spell_level,
                slots_by_level,
            });

            // Get spell column for AllSpellsKnown classes
            let spell_column = if all_spells_known {
                self.get_spell_table_column_for_class(class_info.class_id, game_data)
            } else {
                None
            };

            class_configs.push(ClassSpellConfig {
                class_id: class_info.class_id,
                all_spells_known,
                spell_column,
                max_castable_level: max_spell_level,
            });
        }

        let spells_table_ref = game_data.get_table("spells");

        // Process explicit known spells (non-AllSpellsKnown classes)
        // Read all levels 0-9: a character's spellbook can contain spells above
        // their current castable level (e.g. a Wizard's spellbook).
        for config in &class_configs {
            if config.all_spells_known {
                continue;
            }
            for spell_level in 0..=MAX_SPELL_LEVEL {
                let spell_ids = self.known_spells(config.class_id, spell_level);
                for spell_id in spell_ids {
                    if let Some(table) = spells_table_ref
                        && let Some(spell_row) = table.get_by_id(spell_id.0)
                        && !is_displayable_spell(&spell_row)
                    {
                        continue;
                    }
                    if let Some(details) = self.get_spell_details(spell_id, game_data) {
                        let is_domain_spell =
                            domain_spell_map
                                .get(&config.class_id)
                                .is_some_and(|levels| {
                                    levels
                                        .get(&spell_level)
                                        .is_some_and(|spells| spells.contains(&spell_id))
                                });

                        known_spells.push(KnownSpellEntry {
                            level: spell_level,
                            spell_id,
                            name: details.name,
                            icon: details.icon,
                            school_name: details.school_name,
                            description: details.description,
                            class_id: config.class_id,
                            is_domain_spell,
                        });
                    }
                }
            }
        }

        // Single pass through spells table for AllSpellsKnown classes
        let all_spells_classes: Vec<_> = class_configs
            .iter()
            .filter(|c| c.all_spells_known && c.spell_column.is_some())
            .collect();

        if !all_spells_classes.is_empty()
            && let Some(spells_table) = game_data.get_table("spells")
        {
            for row_id in 0..spells_table.row_count() {
                let Ok(spell_row) = spells_table.get_row(row_id) else {
                    continue;
                };

                if !is_displayable_spell(&spell_row) {
                    continue;
                }

                for config in &all_spells_classes {
                    let col = config.spell_column.as_ref().unwrap();
                    if let Some(level_str) = spell_row.get(col).and_then(|v| v.as_ref())
                        && let Ok(spell_level) = level_str.parse::<i32>()
                        && spell_level >= 0
                        && spell_level <= config.max_castable_level
                    {
                        let spell_id = SpellId(row_id as i32);
                        if let Some(details) =
                            self.get_spell_details_from_row(&spell_row, spell_id, game_data)
                        {
                            if is_mod_prefixed_name(&details.name) {
                                continue;
                            }
                            let is_domain_spell = domain_spell_map
                                .get(&config.class_id)
                                .is_some_and(|levels| {
                                    levels
                                        .get(&spell_level)
                                        .is_some_and(|spells| spells.contains(&spell_id))
                                });

                            known_spells.push(KnownSpellEntry {
                                level: spell_level,
                                spell_id,
                                name: details.name,
                                icon: details.icon,
                                school_name: details.school_name,
                                description: details.description,
                                class_id: config.class_id,
                                is_domain_spell,
                            });
                        }
                    }
                }
            }
        }

        // Process memorized spells
        for config in &class_configs {
            for spell_level in 0..=config.max_castable_level {
                let mem_spells = self.memorized_spells(config.class_id, spell_level);
                for mem in mem_spells {
                    if let Some(details) = self.get_spell_details(mem.spell_id, game_data) {
                        memorized_spells.push(MemorizedSpellEntry {
                            level: spell_level,
                            spell_id: mem.spell_id,
                            name: details.name,
                            icon: details.icon,
                            school_name: details.school_name,
                            description: details.description,
                            class_id: config.class_id,
                            metamagic: i32::from(mem.meta_magic),
                            ready: mem.ready,
                        });
                    }
                }
            }
        }

        let metamagic_feats = self.collect_metamagic_feats(game_data);
        let spell_resistance = self.get_i32("SR").unwrap_or(0);

        let pending_spell_learning: Vec<PendingSpellLearning> = self
            .class_entries()
            .iter()
            .filter(|ce| self.is_spellcaster(ce.class_id, game_data))
            .filter_map(|ce| self.get_pending_spells_to_learn(ce.class_id, game_data))
            .collect();

        let ability_spells = self.get_ability_spells(game_data);

        SpellsState {
            spellcasting_classes,
            spell_summary: SpellSummary {
                caster_classes: caster_class_summaries,
                total_spell_levels,
                metamagic_feats,
                spell_resistance,
            },
            memorized_spells,
            known_spells,
            pending_spell_learning,
            ability_spells,
        }
    }

    pub fn get_spell_table_column_for_class(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Option<String> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id.0)?;

        let spell_col = class_data
            .get("spelltablecolumn")
            .and_then(|v| v.as_ref())
            .map(|s| s.trim().to_lowercase())
            .filter(|v| !v.is_empty() && *v != "****");

        if spell_col.is_some() {
            return spell_col;
        }

        // Fallback based on label
        let label = class_data.get("label").and_then(|v| v.as_ref())?;

        let l_lower = label.to_lowercase();
        Some(
            if l_lower.contains("wizard") || l_lower.contains("sorcerer") {
                "wiz_sorc".to_string()
            } else if l_lower.contains("cleric") || l_lower.contains("favored") {
                "cleric".to_string()
            } else if l_lower.contains("druid") || l_lower.contains("spirit") {
                "druid".to_string()
            } else if l_lower.contains("bard") {
                "bard".to_string()
            } else if l_lower.contains("paladin") {
                "paladin".to_string()
            } else if l_lower.contains("ranger") {
                "ranger".to_string()
            } else {
                l_lower
            },
        )
    }

    fn get_spell_details_from_row(
        &self,
        spell_row: &ahash::AHashMap<String, Option<String>>,
        spell_id: SpellId,
        game_data: &GameData,
    ) -> Option<SpellDetails> {
        let name = if let Some(name_raw) = spell_row.get("name").and_then(|v| v.as_ref()) {
            if let Ok(strref) = name_raw.parse::<i32>() {
                game_data
                    .get_string(strref)
                    .unwrap_or_else(|| "Unknown Spell".to_string())
            } else if !name_raw.is_empty() {
                name_raw.clone()
            } else {
                "Unknown Spell".to_string()
            }
        } else {
            "Unknown Spell".to_string()
        };

        let icon = spell_row
            .get("iconresref")
            .and_then(|v| v.as_ref())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| "io_unknown".to_string());

        let (school_id, school_name) = self.resolve_school(spell_row, game_data);

        let description = spell_row
            .get("spelldesc")
            .and_then(|v| v.as_ref())
            .and_then(|desc_raw| {
                if let Ok(strref) = desc_raw.parse::<i32>() {
                    game_data.get_string(strref).filter(|s| !s.is_empty())
                } else if !desc_raw.is_empty() {
                    Some(desc_raw.clone())
                } else {
                    None
                }
            });

        Some(SpellDetails {
            id: spell_id,
            name,
            icon,
            school_id,
            school_name,
            description,
            spell_range: spell_row
                .get("range")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            cast_time: spell_row
                .get("casttime")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            conjuration_time: spell_row
                .get("conjtime")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            components: spell_row
                .get("vs")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            target_type: spell_row
                .get("targettype")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
        })
    }

    fn collect_domain_spells(
        &self,
        game_data: &GameData,
    ) -> HashMap<ClassId, HashMap<i32, Vec<SpellId>>> {
        let mut result = HashMap::new();
        for class_entry in self.class_entries() {
            if self.is_divine_caster(class_entry.class_id, game_data) {
                let domain_spells = self.get_domain_spells(class_entry.class_id, game_data);
                if !domain_spells.is_empty() {
                    result.insert(class_entry.class_id, domain_spells);
                }
            }
        }
        result
    }

    fn collect_metamagic_feats(&self, game_data: &GameData) -> Vec<MetamagicFeat> {
        let mut result = Vec::new();
        let Some(feats_table) = game_data.get_table("feat") else {
            return result;
        };

        for feat_id in self.feat_ids() {
            let Some(feat_data) = feats_table.get_by_id(feat_id.0) else {
                continue;
            };

            let Some(bit_str) = feat_data.get("metamagicbit").and_then(|v| v.as_ref()) else {
                continue;
            };

            if bit_str.parse::<u8>().unwrap_or(0) == 0 {
                continue;
            }

            let name = feat_data
                .get("feat")
                .and_then(|v| v.as_ref())
                .and_then(|strref| strref.parse::<i32>().ok())
                .and_then(|strref| game_data.get_string(strref))
                .unwrap_or_else(|| format!("Feat {}", feat_id.0));

            let level_cost = feat_data
                .get("metamagiclevelcost")
                .or_else(|| feat_data.get("spelllevelcost"))
                .and_then(|v| v.as_ref())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);

            result.push(MetamagicFeat {
                id: feat_id.0,
                name,
                level_cost,
            });
        }

        result
    }

    /// Get domain spells for a specific class.
    ///
    /// Returns a HashMap mapping spell level (1-9) to a Vec of spell IDs.
    /// Domain spells are granted by domains selected for divine spellcasting classes.
    pub fn get_domain_spells(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> HashMap<i32, Vec<SpellId>> {
        let mut domain_spells: HashMap<i32, Vec<SpellId>> = HashMap::new();

        let Some(class_list) = self.get_list("ClassList") else {
            return domain_spells;
        };

        let Some(class_entry) = class_list.iter().find(|e| {
            gff_value_to_i32(e.get("Class").unwrap_or(&GffValue::Int(-1))) == Some(class_id.0)
        }) else {
            return domain_spells;
        };

        let domain1 = class_entry
            .get("Domain1")
            .and_then(gff_value_to_i32)
            .unwrap_or(-1);
        let domain2 = class_entry
            .get("Domain2")
            .and_then(gff_value_to_i32)
            .unwrap_or(-1);

        let Some(domains_table) = game_data.get_table("domains") else {
            return domain_spells;
        };

        for domain_id in [domain1, domain2] {
            if domain_id < 0 {
                continue;
            }

            let Some(domain_data) = domains_table.get_by_id(domain_id) else {
                continue;
            };

            for spell_level in 1..=MAX_SPELL_LEVEL {
                let field_name = format!("level_{spell_level}");
                if let Some(spell_id_str) = domain_data.get(&field_name).and_then(|v| v.as_ref())
                    && let Ok(spell_id) = spell_id_str.parse::<i32>()
                    && spell_id >= 0
                {
                    domain_spells
                        .entry(spell_level)
                        .or_default()
                        .push(SpellId(spell_id));
                }
            }
        }

        domain_spells
    }

    /// Get spells granted by a specific domain from domains.2da.
    /// Returns HashMap of spell_level -> Vec<SpellId>.
    fn get_spells_for_domain(
        &self,
        domain_id: DomainId,
        game_data: &GameData,
    ) -> HashMap<i32, Vec<SpellId>> {
        let mut domain_spells: HashMap<i32, Vec<SpellId>> = HashMap::new();

        let Some(domains_table) = game_data.get_table("domains") else {
            return domain_spells;
        };

        let Some(domain_data) = domains_table.get_by_id(domain_id.0) else {
            return domain_spells;
        };

        for spell_level in 1..=MAX_SPELL_LEVEL {
            let field_name = format!("level_{spell_level}");
            if let Some(spell_id_str) = domain_data.get(&field_name).and_then(|v| v.as_ref())
                && let Ok(spell_id) = spell_id_str.parse::<i32>()
                && spell_id >= 0
            {
                domain_spells
                    .entry(spell_level)
                    .or_default()
                    .push(SpellId(spell_id));
            }
        }

        domain_spells
    }

    /// Remove memorized spells that belong to a specific domain.
    /// Filters by: SpellDomain == 1 AND spell_id in domain_spell_ids.
    /// Call this BEFORE removing the domain feats.
    pub fn remove_domain_spells(
        &mut self,
        domain_id: DomainId,
        game_data: &GameData,
    ) -> Result<Vec<SpellId>, CharacterError> {
        let domain_spell_map = self.get_spells_for_domain(domain_id, game_data);
        if domain_spell_map.is_empty() {
            return Ok(Vec::new());
        }

        let spell_id_set: HashSet<i32> = domain_spell_map.values().flatten().map(|s| s.0).collect();

        let mut removed_spells = Vec::new();

        let Some(mut class_list) = self.get_list_owned("ClassList") else {
            return Ok(removed_spells);
        };

        for class_entry in &mut class_list {
            let class_id = gff_value_to_i32(class_entry.get("Class").unwrap_or(&GffValue::Int(-1)))
                .unwrap_or(-1);

            if !self.is_divine_caster(ClassId(class_id), game_data) {
                continue;
            }

            for spell_level in 1..=MAX_SPELL_LEVEL {
                let field_name = format!("MemorizedList{spell_level}");

                let Some(GffValue::ListOwned(mem_list)) = class_entry.get(&field_name) else {
                    continue;
                };

                let original_len = mem_list.len();
                let filtered: Vec<_> = mem_list
                    .iter()
                    .filter(|e| {
                        let spell_id =
                            gff_value_to_i32(e.get("Spell").unwrap_or(&GffValue::Int(-1)))
                                .unwrap_or(-1);
                        let is_domain =
                            gff_value_to_i32(e.get("SpellDomain").unwrap_or(&GffValue::Int(0)))
                                .is_some_and(|v| v == 1);

                        !(is_domain && spell_id_set.contains(&spell_id))
                    })
                    .cloned()
                    .collect();

                if filtered.len() < original_len {
                    for e in mem_list {
                        let spell_id =
                            gff_value_to_i32(e.get("Spell").unwrap_or(&GffValue::Int(-1)))
                                .unwrap_or(-1);
                        let is_domain =
                            gff_value_to_i32(e.get("SpellDomain").unwrap_or(&GffValue::Int(0)))
                                .is_some_and(|v| v == 1);

                        if is_domain && spell_id_set.contains(&spell_id) {
                            removed_spells.push(SpellId(spell_id));
                        }
                    }
                    class_entry.insert(field_name, GffValue::ListOwned(filtered));
                }
            }
        }

        self.set_list("ClassList", class_list);
        Ok(removed_spells)
    }

    /// Get detailed information about a specific spell.
    ///
    /// Returns None if the spell ID is not found in the spells table.
    pub fn get_spell_details(
        &self,
        spell_id: SpellId,
        game_data: &GameData,
    ) -> Option<SpellDetails> {
        let spells_table = game_data.get_table("spells")?;
        let spell_data = spells_table.get_by_id(spell_id.0)?;

        let name = if let Some(name_raw) = spell_data.get("name").and_then(|v| v.as_ref()) {
            if let Ok(strref) = name_raw.parse::<i32>() {
                game_data
                    .get_string(strref)
                    .unwrap_or_else(|| "Unknown Spell".to_string())
            } else if !name_raw.is_empty() {
                name_raw.clone()
            } else {
                "Unknown Spell".to_string()
            }
        } else {
            "Unknown Spell".to_string()
        };

        let icon = spell_data
            .get("iconresref")
            .and_then(|v| v.as_ref())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| "io_unknown".to_string());

        let (school_id, school_name) = self.resolve_school(&spell_data, game_data);

        let description = spell_data
            .get("spelldesc")
            .and_then(|v| v.as_ref())
            .and_then(|desc_raw| {
                if let Ok(strref) = desc_raw.parse::<i32>() {
                    game_data.get_string(strref).filter(|s| !s.is_empty())
                } else if !desc_raw.is_empty() {
                    Some(desc_raw.clone())
                } else {
                    None
                }
            });

        Some(SpellDetails {
            id: spell_id,
            name,
            icon,
            school_id,
            school_name,
            description,
            spell_range: spell_data
                .get("range")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            cast_time: spell_data
                .get("casttime")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            conjuration_time: spell_data
                .get("conjtime")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            components: spell_data
                .get("vs")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
            target_type: spell_data
                .get("targettype")
                .and_then(|v| v.as_ref())
                .map(std::string::ToString::to_string),
        })
    }

    pub fn get_spell_name(&self, spell_id: SpellId, game_data: &GameData) -> String {
        let Some(spells_table) = game_data.get_table("spells") else {
            return format!("#{}", spell_id.0);
        };
        let Some(spell_data) = spells_table.get_by_id(spell_id.0) else {
            return format!("#{}", spell_id.0);
        };
        if let Some(name_raw) = spell_data.get("name").and_then(|v| v.as_ref()) {
            if let Ok(strref) = name_raw.parse::<i32>() {
                if let Some(name) = game_data.get_string(strref)
                    && !name.is_empty()
                {
                    return name;
                }
            } else if !name_raw.is_empty() {
                return name_raw.clone();
            }
        }
        spell_data
            .get("label")
            .and_then(|v| v.as_ref())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| format!("#{}", spell_id.0))
    }

    /// Resolve the school of magic for a spell.
    ///
    /// Returns (school_id, school_name) tuple. The school field can be either
    /// a letter code (G, A, C, etc.) or a numeric ID.
    fn resolve_school(
        &self,
        spell_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> (Option<i32>, Option<String>) {
        let Some(school_raw) = spell_data.get("school").and_then(|v| v.as_ref()) else {
            return (None, None);
        };
        if school_raw.is_empty() || school_raw == "****" {
            return (None, None);
        }

        let school_id = if let Some(letter) = school_raw.chars().next() {
            SCHOOL_LETTER_MAP.get(&letter.to_ascii_uppercase()).copied()
        } else {
            school_raw.parse().ok()
        };

        let school_name = school_id.and_then(|id| {
            game_data
                .get_table("spellschools")
                .and_then(|t| t.get_by_id(id))
                .and_then(|row| {
                    row.get("label")
                        .and_then(|v| v.as_ref())
                        .map(std::string::ToString::to_string)
                })
        });

        (school_id, school_name)
    }

    /// Validate spell configuration for data corruption prevention.
    ///
    /// Returns a list of error messages for invalid spell IDs in known or memorized lists.
    /// An empty vector indicates valid configuration.
    pub fn validate_spells(&self, game_data: &GameData) -> Vec<String> {
        let mut errors = Vec::new();

        let Some(spells_table) = game_data.get_table("spells") else {
            return errors;
        };

        for class_entry in self.class_entries() {
            let class_id = class_entry.class_id;

            // Validate known spells
            for spell_level in 0..=MAX_SPELL_LEVEL {
                for spell_id in self.known_spells(class_id, spell_level) {
                    if spells_table.get_by_id(spell_id.0).is_none() {
                        errors.push(format!(
                            "Invalid spell ID {} in known spells (class {}, level {})",
                            spell_id.0, class_id.0, spell_level
                        ));
                    }
                }
            }

            // Validate memorized spells
            for spell_level in 0..=MAX_SPELL_LEVEL {
                for mem_spell in self.memorized_spells(class_id, spell_level) {
                    if spells_table.get_by_id(mem_spell.spell_id.0).is_none() {
                        errors.push(format!(
                            "Invalid spell ID {} in memorized spells (class {}, level {})",
                            mem_spell.spell_id.0, class_id.0, spell_level
                        ));
                    }
                }
            }
        }

        errors
    }

    /// Get all spells that a class can potentially learn or cast at a specific level.
    ///
    /// This scans the `spells.2da` for spells where the class column matches `spell_level`.
    /// Useful for "Available Spells" UI and for "AllSpellsKnown" classes (Cleric/Druid).
    pub fn get_spells_available_to_class(
        &self,
        class_id: ClassId,
        spell_level: i32,
        game_data: &GameData,
    ) -> Vec<SpellId> {
        let mut available = Vec::new();

        let Some(target_col) = self.get_spell_table_column_for_class(class_id, game_data) else {
            return available;
        };

        let Some(spells_table) = game_data.get_table("spells") else {
            return available;
        };

        for row_id in 0..spells_table.row_count() {
            let Ok(spell_row) = spells_table.get_row(row_id) else {
                continue;
            };

            if let Some(removed) = spell_row.get("removed").and_then(|v| v.as_ref())
                && removed == "1"
            {
                continue;
            }

            if let Some(level_str) = spell_row.get(&target_col).and_then(|v| v.as_ref())
                && let Ok(lvl) = level_str.parse::<i32>()
                && lvl == spell_level
            {
                available.push(SpellId(row_id as i32));
            }
        }

        available
    }

    /// Get effectively known spells for a class and level.
    ///
    /// If the class has `AllSpellsKnown=1` (e.g. Cleric), this returns ALL valid spells from `spells.2da`.
    /// Otherwise, it returns the explicit list from `KnownListN`.
    pub fn get_all_known_spells(
        &self,
        class_id: ClassId,
        spell_level: i32,
        game_data: &GameData,
    ) -> Vec<SpellId> {
        if self.uses_all_spells_known(class_id, game_data) {
            self.get_spells_available_to_class(class_id, spell_level, game_data)
        } else {
            self.known_spells(class_id, spell_level)
        }
    }

    /// Calculate the level cost of metamagic feats applied in the bitmask.
    ///
    /// Iterates through the character's feats to find which metamagic feats they possess,
    /// checks if the bitmask includes them, and sums their `MetamagicLevelCost` (or `SpellLevelCost`).
    pub fn calculate_metamagic_cost(&self, meta_magic_flags: u8, game_data: &GameData) -> i32 {
        if meta_magic_flags == 0 {
            return 0;
        }

        let feats_table = if let Some(t) = game_data.get_table("feat") {
            t
        } else {
            return 0;
        };

        let mut total_cost = 0;

        // Iterate all character feats
        for feat_id in self.feat_ids() {
            let Some(feat_data) = feats_table.get_by_id(feat_id.0) else {
                continue;
            };

            // Check if it's a metamagic feat by checking MetamagicBit
            let Some(bit_str) = feat_data.get("metamagicbit").and_then(|v| v.as_ref()) else {
                continue;
            };

            let Ok(bit_val) = bit_str.parse::<u8>() else {
                continue;
            };

            if bit_val == 0 {
                continue;
            }

            // Check if this metamagic bit is active in the flags
            if (meta_magic_flags & bit_val) != 0 {
                // Get cost
                // Try MetamagicLevelCost first, then SpellLevelCost
                let cost_str = feat_data
                    .get("metamagiclevelcost")
                    .and_then(|v| v.as_ref())
                    .or_else(|| feat_data.get("spelllevelcost").and_then(|v| v.as_ref()));

                if let Some(cost) = cost_str.and_then(|s| s.parse::<i32>().ok()) {
                    total_cost += cost;
                }
            }
        }

        total_cost
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SpellcastingClass {
    pub index: i32,
    pub class_id: ClassId,
    pub class_name: String,
    pub class_level: i32,
    pub caster_level: i32,
    pub spell_type: String,
    pub can_edit_spells: bool,
    /// Spells-known cap per spell level from SpellKnownTable (e.g. cls_spkn_sorc).
    /// Empty for prepared casters and AllSpellsKnown classes (no cap applies).
    pub expected_spells_known: HashMap<i32, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CasterClassSummary {
    pub id: ClassId,
    pub name: String,
    pub total_slots: i32,
    pub max_spell_level: i32,
    pub slots_by_level: HashMap<i32, i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MetamagicFeat {
    pub id: i32,
    pub name: String,
    pub level_cost: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct SpellSummary {
    pub caster_classes: Vec<CasterClassSummary>,
    pub total_spell_levels: i32,
    pub metamagic_feats: Vec<MetamagicFeat>,
    pub spell_resistance: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassSpellInfoInternal {
    pub class_list_index: usize,
    pub class_id: ClassId,
    pub caster_level: i32,
    pub is_prepared: bool,
    pub slots: Vec<i32>,
    pub bonus_slots: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpellDetails {
    pub id: SpellId,
    pub name: String,
    pub icon: String,
    pub school_id: Option<i32>,
    pub school_name: Option<String>,
    pub description: Option<String>,
    pub spell_range: Option<String>,
    pub cast_time: Option<String>,
    pub conjuration_time: Option<String>,
    pub components: Option<String>,
    pub target_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PendingSpellLearning {
    pub class_id: ClassId,
    pub class_name: String,
    pub caster_type: String,
    pub by_level: HashMap<i32, i32>,
    pub total: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AbilitySpellEntry {
    pub spell_id: i32,
    pub name: String,
    pub icon: String,
    pub description: Option<String>,
    pub school_name: Option<String>,
    pub innate_level: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct SpellsState {
    pub spellcasting_classes: Vec<SpellcastingClass>,
    pub spell_summary: SpellSummary,
    pub memorized_spells: Vec<MemorizedSpellEntry>,
    pub known_spells: Vec<KnownSpellEntry>,
    pub pending_spell_learning: Vec<PendingSpellLearning>,
    pub ability_spells: Vec<AbilitySpellEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::types::ClassId;

    fn create_test_character_with_wizard() -> Character {
        let mut fields = IndexMap::new();

        let mut wizard_class = IndexMap::new();
        wizard_class.insert("Class".to_string(), GffValue::Int(10));
        wizard_class.insert("ClassLevel".to_string(), GffValue::Short(5));

        let mut known0 = IndexMap::new();
        known0.insert("Spell".to_string(), GffValue::Word(0));
        let mut known1 = IndexMap::new();
        known1.insert("Spell".to_string(), GffValue::Word(1));
        wizard_class.insert(
            "KnownList0".to_string(),
            GffValue::ListOwned(vec![known0, known1]),
        );

        let mut mem0 = IndexMap::new();
        mem0.insert("Spell".to_string(), GffValue::Word(0));
        mem0.insert("SpellMetaMagicN2".to_string(), GffValue::Dword(0));
        mem0.insert("Ready".to_string(), GffValue::Byte(1));
        wizard_class.insert(
            "MemorizedList0".to_string(),
            GffValue::ListOwned(vec![mem0]),
        );

        fields.insert(
            "ClassList".to_string(),
            GffValue::ListOwned(vec![wizard_class]),
        );

        Character::from_gff(fields)
    }

    #[test]
    fn test_known_spells() {
        let character = create_test_character_with_wizard();
        let known = character.known_spells(ClassId(10), 0);
        assert_eq!(known.len(), 2);
        assert!(known.contains(&SpellId(0)));
        assert!(known.contains(&SpellId(1)));
    }

    #[test]
    fn test_known_spells_empty_level() {
        let character = create_test_character_with_wizard();
        let known = character.known_spells(ClassId(10), 1);
        assert_eq!(known.len(), 0);
    }

    #[test]
    fn test_has_known_spell() {
        let character = create_test_character_with_wizard();
        assert!(character.has_known_spell(ClassId(10), SpellId(0)));
        assert!(character.has_known_spell(ClassId(10), SpellId(1)));
        assert!(!character.has_known_spell(ClassId(10), SpellId(99)));
    }

    #[test]
    fn test_memorized_spells() {
        let character = create_test_character_with_wizard();
        let memorized = character.memorized_spells(ClassId(10), 0);
        assert_eq!(memorized.len(), 1);
        assert_eq!(memorized[0].spell_id, SpellId(0));
        assert_eq!(memorized[0].meta_magic, 0);
        assert!(memorized[0].ready);
    }

    #[test]
    fn test_add_known_spell() {
        let mut character = create_test_character_with_wizard();
        assert!(character.add_known_spell(0, 0, SpellId(2)).is_ok());

        let known = character.known_spells(ClassId(10), 0);
        assert_eq!(known.len(), 3);
        assert!(known.contains(&SpellId(2)));
    }

    #[test]
    fn test_add_known_spell_duplicate() {
        let mut character = create_test_character_with_wizard();
        let result = character.add_known_spell(0, 0, SpellId(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_known_spell() {
        let mut character = create_test_character_with_wizard();
        assert!(character.remove_known_spell(0, 0, SpellId(1)).is_ok());

        let known = character.known_spells(ClassId(10), 0);
        assert_eq!(known.len(), 1);
        assert!(!known.contains(&SpellId(1)));
    }

    #[test]
    fn test_remove_known_spell_not_found() {
        let mut character = create_test_character_with_wizard();
        let result = character.remove_known_spell(0, 0, SpellId(99));
        assert!(result.is_err());
    }

    #[test]
    fn test_add_memorized_spell() {
        let mut character = create_test_character_with_wizard();
        let spell = MemorizedSpellRaw {
            spell_id: SpellId(1),
            meta_magic: 2,
            ready: false,
            is_domain: false,
        };
        assert!(character.add_memorized_spell(0, 0, spell).is_ok());

        let memorized = character.memorized_spells(ClassId(10), 0);
        assert_eq!(memorized.len(), 2);
        assert_eq!(memorized[1].spell_id, SpellId(1));
        assert_eq!(memorized[1].meta_magic, 2);
        assert!(!memorized[1].ready);
    }

    #[test]
    fn test_clear_memorized_spells() {
        let mut character = create_test_character_with_wizard();
        assert!(character.clear_memorized_spells(0, 0).is_ok());

        let memorized = character.memorized_spells(ClassId(10), 0);
        assert_eq!(memorized.len(), 0);
    }

    #[test]
    fn test_clear_all_memorized_spells() {
        let mut character = create_test_character_with_wizard();
        assert!(character.clear_all_memorized_spells(0).is_ok());

        for level in 0..=MAX_SPELL_LEVEL {
            let memorized = character.memorized_spells(ClassId(10), level);
            assert_eq!(memorized.len(), 0);
        }
    }

    #[test]
    fn test_invalid_spell_level() {
        let mut character = create_test_character_with_wizard();
        let result = character.add_known_spell(0, 10, SpellId(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_class_index() {
        let mut character = create_test_character_with_wizard();
        let result = character.add_known_spell(5, 0, SpellId(0));
        assert!(result.is_err());
    }
}
