use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use super::gff_helpers::{gff_value_to_i32, gff_value_to_u32};
use super::types::{AbilityIndex, AbilityScores, ClassId, SpellId, calculate_modifier};
use super::{Character, CharacterError};
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::services::item_property_decoder::ItemPropertyDecoder;
use crate::utils::parsing::{row_bool, row_str};

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
    pub meta_magic: u32,
    pub ready: bool,
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
    pub metamagic: u32,
    pub ready: bool,
}

/// Bonus spell slots at a spell level from the total casting ability score
/// (stored + racial + items): `1 + (mod - level) / 4` once the modifier reaches the
/// spell level. Cantrips get none.
pub fn bonus_spell_slots(spell_level: i32, casting_score: i32) -> i32 {
    let modifier = calculate_modifier(casting_score);
    if spell_level > 0 && modifier >= spell_level {
        1 + (modifier - spell_level) / 4
    } else {
        0
    }
}

/// Whether a casting ability score allows casting spells of `spell_level` at all: the
/// engine grants no slots below 10 + spell level.
fn can_cast_spell_level(spell_level: i32, casting_score: i32) -> bool {
    casting_score >= 10 + spell_level
}

fn entry_class_id(entry: &IndexMap<String, GffValue<'static>>) -> ClassId {
    ClassId(entry.get("Class").and_then(gff_value_to_i32).unwrap_or(-1))
}

/// `Spell` fields are Words; ids outside that range cannot be stored.
fn spell_word(spell_id: SpellId) -> Result<u16, CharacterError> {
    u16::try_from(spell_id.0).map_err(|_| CharacterError::OutOfRange {
        field: "Spell",
        value: spell_id.0,
        min: 0,
        max: i32::from(u16::MAX),
    })
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
                    .and_then(gff_value_to_u32)
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

                Some(MemorizedSpellRaw {
                    spell_id: SpellId(spell_id),
                    meta_magic,
                    ready,
                })
            })
            .collect()
    }

    /// ClassList for a spell edit on the class at `class_index`, once the spell level and
    /// the index are known to be valid.
    fn class_list_for_spell_edit(
        &self,
        class_index: usize,
        spell_level: i32,
    ) -> Result<Vec<IndexMap<String, GffValue<'static>>>, CharacterError> {
        if !(0..=MAX_SPELL_LEVEL).contains(&spell_level) {
            return Err(CharacterError::InvalidOperation(format!(
                "Spell level {spell_level} out of range [0, {MAX_SPELL_LEVEL}]"
            )));
        }
        let Some(class_list) = self.get_list_owned("ClassList") else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };
        if class_index >= class_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "Class index {} out of range [0, {})",
                class_index,
                class_list.len()
            )));
        }
        Ok(class_list)
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
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let mut class_list = self.class_list_for_spell_edit(class_index, spell_level)?;
        let spell_word = spell_word(spell_id)?;

        let class_entry = &mut class_list[class_index];
        let class_id = entry_class_id(class_entry);
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
        new_spell.insert("Spell".to_string(), GffValue::Word(spell_word));
        known_list.push(new_spell);

        class_entry.insert(field_name, GffValue::ListOwned(known_list));
        self.set_list("ClassList", class_list);

        self.record_spell_change(class_id, spell_level, spell_word, true, game_data);

        Ok(())
    }

    /// Remove a known spell from a class at a specific spell level, together with
    /// the class's prepared copies of it at that level.
    ///
    /// `class_index` is the index into ClassList (0, 1, or 2 for most characters).
    /// Returns error if class_index is out of bounds or spell not found.
    pub fn remove_known_spell(
        &mut self,
        class_index: usize,
        spell_level: i32,
        spell_id: SpellId,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let mut class_list = self.class_list_for_spell_edit(class_index, spell_level)?;
        let spell_word = spell_word(spell_id)?;

        let class_entry = &mut class_list[class_index];
        let class_id = entry_class_id(class_entry);
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

        let memorized_field = format!("MemorizedList{spell_level}");
        if let Some(GffValue::ListOwned(mem_list)) = class_entry.get_mut(&memorized_field) {
            mem_list.retain(|e| e.get("Spell").and_then(gff_value_to_i32) != Some(spell_id.0));
        }
        self.set_list("ClassList", class_list);

        self.record_spell_change(class_id, spell_level, spell_word, false, game_data);

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

    /// Base spell slots per spell level (0-9) from the class's spell gain table at its
    /// caster level. `None` = no access (`****`), `Some(0)` = level unlocked with bonus
    /// slots only. The cleric table already includes the domain slot.
    pub fn calculate_spell_slots(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Vec<Option<i32>> {
        self.spell_slots_at(
            class_id,
            self.get_caster_level(class_id, game_data),
            game_data,
        )
    }

    fn spell_slots_at(
        &self,
        class_id: ClassId,
        caster_level: i32,
        game_data: &GameData,
    ) -> Vec<Option<i32>> {
        let Some(row) = Self::caster_level_row(class_id, "spellgaintable", caster_level, game_data)
        else {
            return vec![None; 10];
        };
        (0..=MAX_SPELL_LEVEL)
            .map(|spell_level| {
                row.get(&format!("spelllevel{spell_level}"))
                    .and_then(|v| v.as_ref())
                    .and_then(|v| v.parse::<i32>().ok())
                    .map(|count| count.max(0))
            })
            .collect()
    }

    /// Row of the class's progression table named in `table_column` (`SpellGainTable`,
    /// `SpellKnownTable`) at `caster_level`, clamped to the table's last row.
    fn caster_level_row(
        class_id: ClassId,
        table_column: &str,
        caster_level: i32,
        game_data: &GameData,
    ) -> Option<ahash::AHashMap<String, Option<String>>> {
        if caster_level <= 0 {
            return None;
        }
        let table_name = row_str(
            &game_data.get_table("classes")?.get_by_id(class_id.0)?,
            table_column,
        )?;
        let table = game_data.get_table(&table_name.to_lowercase())?;
        let last_row = table.row_count().checked_sub(1)?;
        table
            .get_row((caster_level as usize - 1).min(last_row))
            .ok()
    }

    /// Highest spell level the class has access to at its caster level.
    pub fn max_spell_level(&self, class_id: ClassId, game_data: &GameData) -> Option<i32> {
        self.calculate_spell_slots(class_id, game_data)
            .iter()
            .rposition(Option::is_some)
            .map(|level| level as i32)
    }

    /// Spellcasting ability of a class (`classes.2da` `SpellAbil`).
    pub fn get_casting_ability(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Option<AbilityIndex> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id.0)?;
        AbilityIndex::from_gff_field(&row_str(&class_data, "spellabil")?)
    }

    /// Caster level of a spellcasting class: its class level plus the levels prestige
    /// classes add to it. 0 for classes that do not cast (`SpellCaster = 0`).
    pub fn get_caster_level(&self, class_id: ClassId, game_data: &GameData) -> i32 {
        if !self.is_spellcaster(class_id, game_data) {
            return 0;
        }
        let class_level = self.class_level(class_id);
        if class_level == 0 {
            return 0;
        }
        class_level + self.prestige_bonus_caster_levels(class_id.0, game_data)
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
    /// Returns HashMap of spell_level -> expected count at the class's caster level.
    fn get_expected_spells_known_from_table(
        &self,
        class_id: ClassId,
        caster_level: i32,
        game_data: &GameData,
    ) -> HashMap<i32, i32> {
        let mut expected: HashMap<i32, i32> = HashMap::new();

        let Some(row_data) =
            Self::caster_level_row(class_id, "spellknowntable", caster_level, game_data)
        else {
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

        // Only classes whose spell list the editor can offer (the All Spells tab reads
        // the same columns). Warlock invocations live in the `warlock` column and the
        // game stores them in one KnownList regardless of grade, so neither the
        // per-level count nor the picker applies.
        let column = self.get_spell_table_column_for_class(class_id, game_data)?;
        if !Self::learnable_spell_columns(game_data).contains_key(&column) {
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
        let caster_level = self.get_caster_level(class_id, game_data);
        let expected = self.get_expected_spells_known_from_table(class_id, caster_level, game_data);
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
    fn get_spell_summary_internal(
        &self,
        game_data: &GameData,
        abilities: &AbilityScores,
    ) -> Vec<ClassSpellInfoInternal> {
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
            let casting_score = self
                .get_casting_ability(class_id, game_data)
                .map(|ability| abilities.get(ability));
            let mut base_slots = self.spell_slots_at(class_id, caster_level, game_data);
            if let Some(score) = casting_score {
                for (spell_level, slots) in (0..).zip(base_slots.iter_mut()) {
                    if !can_cast_spell_level(spell_level, score) {
                        *slots = slots.map(|_| 0);
                    }
                }
            }
            let bonus_slots = (0..=MAX_SPELL_LEVEL)
                .map(|spell_level| {
                    casting_score.map_or(0, |score| bonus_spell_slots(spell_level, score))
                })
                .collect();

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

    /// Columns in `spells.2da` that gate which class spell lists the editor treats as
    /// learnable, keyed by column name with the base caster classes (`SpellCaster`
    /// = 1) that share it: Wizard and Sorcerer both resolve to `wiz_sorc`, Favored
    /// Soul to `cleric`, Spirit Shaman to `druid`. Kept are only columns that
    /// actually exist in the loaded `spells.2da` (a HAK class bringing its own
    /// `SpellTableColumn` and column qualifies automatically). `warlock` is
    /// deliberately excluded: invocations are written to `KnownList4` regardless of
    /// grade, so the editor neither lists nor counts them.
    pub fn learnable_spell_columns(game_data: &GameData) -> IndexMap<String, Vec<ClassId>> {
        let mut by_column: IndexMap<String, Vec<ClassId>> = IndexMap::new();

        let Some(classes_table) = game_data.get_table("classes") else {
            return by_column;
        };
        let Some(spells_table) = game_data.get_table("spells") else {
            return by_column;
        };

        for row_id in 0..classes_table.row_count() {
            let Ok(class_data) = classes_table.get_row(row_id) else {
                continue;
            };
            let is_caster = class_data
                .get("spellcaster")
                .and_then(|v| v.as_ref())
                .is_some_and(|v| v == "1");
            if !is_caster {
                continue;
            }
            let Some(column) = Self::spell_table_column_from_row(&class_data) else {
                continue;
            };
            if column == "warlock" || spells_table.find_column_index(&column).is_none() {
                continue;
            }

            by_column
                .entry(column)
                .or_default()
                .push(ClassId(row_id as i32));
        }

        by_column
    }

    /// Learnable spell-table columns paired with a display label built from
    /// `classes.2da` names via TLK. Classes sharing a column (e.g. Wizard and
    /// Sorcerer both using `wiz_sorc`) are joined with "/".
    pub fn learnable_spell_class_labels(&self, game_data: &GameData) -> Vec<(String, String)> {
        Self::learnable_spell_columns(game_data)
            .into_iter()
            .map(|(column, class_ids)| {
                let label = class_ids
                    .iter()
                    .map(|&class_id| self.get_class_name(class_id, game_data))
                    .collect::<Vec<_>>()
                    .join("/");
                (column, label)
            })
            .collect()
    }

    fn is_on_learnable_caster_list(
        spell_row: &ahash::AHashMap<String, Option<String>>,
        learnable_columns: &IndexMap<String, Vec<ClassId>>,
    ) -> bool {
        learnable_columns.keys().any(|col| {
            spell_row
                .get(col.as_str())
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

        let learnable_columns = Self::learnable_spell_columns(game_data);

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
            let on_caster_list = Self::is_on_learnable_caster_list(&spell_row, &learnable_columns);

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

    pub fn get_spells_state(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> SpellsState {
        let abilities = self.get_total_abilities(game_data, decoder);
        let internal_classes = self.get_spell_summary_internal(game_data, &abilities);

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
                let Some(base) = class_info.slots.get(level as usize).copied().flatten() else {
                    continue;
                };
                max_spell_level = level;
                let slot_count = base
                    + class_info
                        .bonus_slots
                        .get(level as usize)
                        .copied()
                        .unwrap_or(0);
                if slot_count > 0 {
                    total_slots += slot_count;
                    slots_by_level.insert(level, slot_count);
                }
            }
            total_spell_levels += max_spell_level;

            let expected_spells_known = if !class_info.is_prepared && !all_spells_known {
                self.get_expected_spells_known_from_table(
                    class_info.class_id,
                    class_info.caster_level,
                    game_data,
                )
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
            for spell_level in 0..=MAX_SPELL_LEVEL {
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
                            metamagic: mem.meta_magic,
                            ready: mem.ready,
                        });
                    }
                }
            }
        }

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
        Self::spell_table_column_from_row(&class_data)
    }

    /// Spell level of `spell_id` on the class list entry at `class_index`, read from that
    /// class's `spells.2da` column. `None` when the class has no list entry, no spell column,
    /// or does not offer the spell.
    pub fn spell_level_for_class_index(
        &self,
        class_index: usize,
        spell_id: SpellId,
        game_data: &GameData,
    ) -> Option<i32> {
        let class_id = self.class_entries().get(class_index)?.class_id;
        let column = self.get_spell_table_column_for_class(class_id, game_data)?;
        let spell_row = game_data.get_table("spells")?.get_by_id(spell_id.0)?;
        Self::spell_level_in_row(&spell_row, &column.to_lowercase())
    }

    /// Level of a `spells.2da` row in one class column; `None` for `****` or no access.
    pub(crate) fn spell_level_in_row(
        spell_row: &ahash::AHashMap<String, Option<String>>,
        column: &str,
    ) -> Option<i32> {
        spell_row
            .get(column)
            .and_then(|v| v.as_ref())
            .and_then(|v| v.parse::<i32>().ok())
            .filter(|level| *level >= 0)
    }

    /// `spells.2da` column for a `classes.2da` row: its `SpellTableColumn` when the
    /// table carries one (vanilla does not; some mods add it), else derived from
    /// the class label the way the shipped classes map to columns.
    fn spell_table_column_from_row(
        class_data: &ahash::AHashMap<String, Option<String>>,
    ) -> Option<String> {
        let spell_col = class_data
            .get("spelltablecolumn")
            .and_then(|v| v.as_ref())
            .map(|s| s.trim().to_lowercase())
            .filter(|v| !v.is_empty() && *v != "****");

        if spell_col.is_some() {
            return spell_col;
        }

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

    /// Get detailed information about a specific spell.
    ///
    /// Returns None if the spell ID is not found in the spells table.
    pub fn get_spell_details(
        &self,
        spell_id: SpellId,
        game_data: &GameData,
    ) -> Option<SpellDetails> {
        let spell_data = game_data.get_table("spells")?.get_by_id(spell_id.0)?;
        self.get_spell_details_from_row(&spell_data, spell_id, game_data)
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

            if Self::spell_level_in_row(&spell_row, &target_col) == Some(spell_level) {
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

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct SpellSummary {
    pub caster_classes: Vec<CasterClassSummary>,
    pub total_spell_levels: i32,
    pub spell_resistance: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassSpellInfoInternal {
    pub class_list_index: usize,
    pub class_id: ClassId,
    pub caster_level: i32,
    pub is_prepared: bool,
    pub slots: Vec<Option<i32>>,
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

    fn empty_game_data() -> GameData {
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};
        GameData::new(Arc::new(RwLock::new(TLKParser::default())))
    }

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
        assert!(
            character
                .add_known_spell(0, 0, SpellId(2), &empty_game_data())
                .is_ok()
        );

        let known = character.known_spells(ClassId(10), 0);
        assert_eq!(known.len(), 3);
        assert!(known.contains(&SpellId(2)));
    }

    #[test]
    fn test_add_known_spell_duplicate() {
        let mut character = create_test_character_with_wizard();
        let result = character.add_known_spell(0, 0, SpellId(0), &empty_game_data());
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_known_spell() {
        let mut character = create_test_character_with_wizard();
        assert!(
            character
                .remove_known_spell(0, 0, SpellId(1), &empty_game_data())
                .is_ok()
        );

        let known = character.known_spells(ClassId(10), 0);
        assert_eq!(known.len(), 1);
        assert!(!known.contains(&SpellId(1)));
    }

    #[test]
    fn test_remove_known_spell_not_found() {
        let mut character = create_test_character_with_wizard();
        let result = character.remove_known_spell(0, 0, SpellId(99), &empty_game_data());
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_spell_level() {
        let mut character = create_test_character_with_wizard();
        let result = character.add_known_spell(0, 10, SpellId(0), &empty_game_data());
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_class_index() {
        let mut character = create_test_character_with_wizard();
        let result = character.add_known_spell(5, 0, SpellId(0), &empty_game_data());
        assert!(result.is_err());
    }

    #[test]
    fn test_spell_ids_outside_word_range_are_rejected() {
        let mut character = create_test_character_with_wizard();
        assert!(
            character
                .add_known_spell(0, 1, SpellId(-1), &empty_game_data())
                .is_err()
        );
        assert!(
            character
                .add_known_spell(0, 1, SpellId(70_000), &empty_game_data())
                .is_err()
        );
        assert!(character.known_spells(ClassId(10), 1).is_empty());
    }

    #[test]
    fn test_remove_known_spell_drops_prepared_copies() {
        let mut character = create_test_character_with_wizard();
        character
            .remove_known_spell(0, 0, SpellId(0), &empty_game_data())
            .unwrap();
        assert!(character.memorized_spells(ClassId(10), 0).is_empty());
    }

    #[test]
    fn test_spell_changes_land_on_the_latest_level_of_their_class() {
        let mut character = create_test_character_with_wizard();
        let level = |class_id: u8| {
            let mut entry = IndexMap::new();
            entry.insert("LvlStatClass".to_string(), GffValue::Byte(class_id));
            entry
        };
        character.set_list("LvlStatList", vec![level(10), level(4)]);
        let spell_list = |character: &Character, index: usize, field: &str| {
            character.get_list_owned("LvlStatList").unwrap()[index]
                .get(field)
                .map(|v| match v {
                    GffValue::ListOwned(list) => list.len(),
                    _ => 0,
                })
        };

        character
            .add_known_spell(0, 1, SpellId(5), &empty_game_data())
            .unwrap();
        assert_eq!(spell_list(&character, 0, "KnownList1"), Some(1));
        assert_eq!(spell_list(&character, 1, "KnownList1"), None);

        character
            .remove_known_spell(0, 1, SpellId(5), &empty_game_data())
            .unwrap();
        assert_eq!(spell_list(&character, 0, "KnownList1"), Some(0));
        assert_eq!(spell_list(&character, 0, "KnownRemoveList1"), None);
    }

    #[test]
    fn test_can_cast_spell_level() {
        assert!(can_cast_spell_level(0, 10));
        assert!(!can_cast_spell_level(0, 9));
        assert!(can_cast_spell_level(3, 13));
        assert!(!can_cast_spell_level(3, 12));
    }

    #[test]
    fn test_bonus_spell_slots() {
        assert_eq!(bonus_spell_slots(0, 30), 0);
        assert_eq!(bonus_spell_slots(1, 11), 0);
        assert_eq!(bonus_spell_slots(1, 12), 1);
        assert_eq!(bonus_spell_slots(4, 18), 1);
        assert_eq!(bonus_spell_slots(1, 20), 2);
    }
}
