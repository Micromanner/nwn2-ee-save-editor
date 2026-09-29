//! Character class-related methods.
//!
//! Provides access to character class information including:
//! - Total level calculation
//! - Class entries and levels
//! - Multi-class support
//!
//! All methods are sync (no async). ClassList structure in GFF:
//! - ClassList: List of structs
//!   - Class: Int (class ID)
//!   - ClassLevel: Short (level in that class)

use ahash::AHashMap;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet};

use super::{Character, CharacterError};
use crate::character::feats::FeatSource;
use crate::character::gff_helpers::gff_value_to_i32;
use crate::character::identity::Alignment;
use crate::character::race::featstable_feats;
use crate::character::types::{
    AbilityIndex, ClassId, FeatId, MAX_CLASSES, MAX_TOTAL_LEVEL, SkillId,
};
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::{row_bool, row_int, row_str};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct ClassEntry {
    pub class_id: ClassId,
    pub level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LevelHistoryEntry {
    pub character_level: i32,
    pub class_id: ClassId,
    pub class_level: i32,
    pub hp_gained: i32,
    pub skill_points_remaining: i32,
    pub ability_increase: Option<AbilityIndex>,
    pub feats_gained: Vec<FeatId>,
    pub skills_gained: Vec<SkillRankEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillRankEntry {
    pub skill_id: SkillId,
    pub ranks: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ResolvedLevelHistoryEntry {
    pub character_level: i32,
    pub class_id: i32,
    pub class_name: String,
    pub class_level: i32,
    pub hp_gained: i32,
    pub skill_points_remaining: i32,
    pub ability_increase: Option<String>,
    pub feats_gained: Vec<ResolvedFeatEntry>,
    pub skills_gained: Vec<ResolvedSkillEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ResolvedFeatEntry {
    pub feat_id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ResolvedSkillEntry {
    pub skill_id: i32,
    pub name: String,
    pub ranks: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub enum BabType {
    Full,
    ThreeQuarter,
    Half,
}

impl BabType {
    /// Vanilla `AttackBonusTable`s: `CLS_ATK_1` full, `CLS_ATK_2` three quarter, `CLS_ATK_3` half.
    fn from_table(bab_table_name: &str) -> Self {
        match bab_table_name.trim().to_lowercase().as_str() {
            "cls_atk_1" => Self::Full,
            "cls_atk_3" => Self::Half,
            _ => Self::ThreeQuarter,
        }
    }

    fn progression_name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::ThreeQuarter => "three_quarter",
            Self::Half => "half",
        }
    }

    /// BAB at `level` when the class's attack bonus table is missing.
    fn fallback_bab(self, level: i32) -> i32 {
        match self {
            Self::Full => level,
            Self::ThreeQuarter => (level * 3) / 4,
            Self::Half => level / 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassInfo {
    pub id: ClassId,
    pub name: String,
    pub hit_die: i32,
    pub primary_ability: Option<AbilityIndex>,
    pub is_spellcaster: bool,
    pub bab_type: BabType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct XpProgress {
    pub current_xp: i32,
    pub current_level: i32,
    pub xp_for_current_level: i32,
    pub xp_for_next_level: i32,
    pub xp_remaining: i32,
    pub progress_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassSummaryEntry {
    pub class_id: ClassId,
    pub name: String,
    pub level: i32,
    pub hit_die: i32,
    pub base_attack_bonus: i32,
    pub fortitude_save: i32,
    pub reflex_save: i32,
    pub will_save: i32,
    pub skill_points_per_level: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LevelUpResult {
    pub class_id: ClassId,
    pub new_level: i32,
    pub hp_gained: i32,
    pub skill_points_gained: i32,
    pub general_feat_slots_gained: i32,
    pub bonus_feat_slots_gained: i32,
    pub ability_increase_gained: bool,
    pub new_spells_gained: bool,
    pub granted_feats: Vec<FeatId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassProgression {
    pub class_id: i32,
    pub class_name: String,
    pub basic_info: ClassBasicInfo,
    pub level_progression: Vec<LevelProgressionEntry>,
    pub max_level_shown: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassBasicInfo {
    pub hit_die: i32,
    pub skill_points_per_level: i32,
    pub bab_progression: String,
    pub save_progression: String,
    pub is_spellcaster: bool,
    pub spell_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct LevelProgressionEntry {
    pub level: i32,
    pub base_attack_bonus: i32,
    pub fortitude_save: i32,
    pub reflex_save: i32,
    pub will_save: i32,
    pub features: Vec<ClassFeature>,
    pub spell_slots: Option<SpellSlots>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassFeature {
    pub name: String,
    pub feature_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SpellSlots {
    pub level_0: i32,
    pub level_1: i32,
    pub level_2: i32,
    pub level_3: i32,
    pub level_4: i32,
    pub level_5: i32,
    pub level_6: i32,
    pub level_7: i32,
    pub level_8: i32,
    pub level_9: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassFeatInfo {
    pub feat_id: FeatId,
    pub list_type: i32,
    pub granted_on_level: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ClassesState {
    pub total_level: i32,
    pub entries: Vec<ClassSummaryEntry>,
    pub xp_progress: XpProgress,
    pub level_history: Vec<ResolvedLevelHistoryEntry>,
}

/// `classes.2da` alignment gate. `AlignRestrict` bits name alignments on the axes selected by
/// `AlignRstrctType` (0x1 law/chaos, 0x2 good/evil). The named alignments are prohibited, or
/// required when `InvertRestrict` is 1; a match on any selected axis counts.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct AlignmentRestriction {
    pub bits: i32,
    pub axes: i32,
    pub invert: bool,
}

impl AlignmentRestriction {
    pub const NEUTRAL: i32 = 0x01;
    pub const LAWFUL: i32 = 0x02;
    pub const CHAOTIC: i32 = 0x04;
    pub const GOOD: i32 = 0x08;
    pub const EVIL: i32 = 0x10;
    pub const AXIS_LAW_CHAOS: i32 = 0x1;
    pub const AXIS_GOOD_EVIL: i32 = 0x2;

    const LAW_CHAOS: [(i32, &'static str); 3] = [
        (Self::LAWFUL, "Lawful"),
        (Self::NEUTRAL, "Neutral"),
        (Self::CHAOTIC, "Chaotic"),
    ];
    const GOOD_EVIL: [(i32, &'static str); 3] = [
        (Self::GOOD, "Good"),
        (Self::NEUTRAL, "Neutral"),
        (Self::EVIL, "Evil"),
    ];

    /// `None` when the class has no alignment restriction.
    pub fn from_class_row(class_data: &AHashMap<String, Option<String>>) -> Option<Self> {
        let restriction = Self {
            bits: row_int(class_data, "alignrestrict", 0),
            axes: row_int(class_data, "alignrstrcttype", 0),
            invert: row_int(class_data, "invertrestrict", 0) == 1,
        };
        restriction.is_restricted().then_some(restriction)
    }

    fn is_restricted(&self) -> bool {
        self.bits != 0 && self.axes & (Self::AXIS_LAW_CHAOS | Self::AXIS_GOOD_EVIL) != 0
    }

    fn axis_names(&self, axis: i32, values: [(i32, &'static str); 3]) -> Vec<&'static str> {
        if self.axes & axis == 0 {
            return Vec::new();
        }
        values
            .into_iter()
            .filter(|(bit, _)| (self.bits & bit != 0) == self.invert)
            .map(|(_, name)| name)
            .collect()
    }

    pub fn decode_to_string(&self) -> Option<String> {
        if !self.is_restricted() {
            return None;
        }
        let law_chaos = self.axis_names(Self::AXIS_LAW_CHAOS, Self::LAW_CHAOS);
        let good_evil = self.axis_names(Self::AXIS_GOOD_EVIL, Self::GOOD_EVIL);
        if self.invert {
            let mut required: Vec<&str> = law_chaos;
            for name in good_evil {
                if !required.contains(&name) {
                    required.push(name);
                }
            }
            return Some(required.join(" or "));
        }
        let parts: Vec<String> = [law_chaos, good_evil]
            .into_iter()
            .filter(|names| !names.is_empty())
            .map(|names| names.join(" or "))
            .collect();
        Some(parts.join(" "))
    }

    pub fn check_alignment(&self, alignment: &Alignment) -> bool {
        if !self.is_restricted() {
            return true;
        }
        let law_chaos = if alignment.is_lawful() {
            Self::LAWFUL
        } else if alignment.is_chaotic() {
            Self::CHAOTIC
        } else {
            Self::NEUTRAL
        };
        let good_evil = if alignment.is_good() {
            Self::GOOD
        } else if alignment.is_evil() {
            Self::EVIL
        } else {
            Self::NEUTRAL
        };
        let matched = (self.axes & Self::AXIS_LAW_CHAOS != 0 && self.bits & law_chaos != 0)
            || (self.axes & Self::AXIS_GOOD_EVIL != 0 && self.bits & good_evil != 0);
        matched == self.invert
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct PrestigeRequirements {
    pub base_attack_bonus: Option<i32>,
    pub skills: Vec<(String, i32)>,
    pub feats: Vec<String>,
    pub alignment: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct PrestigeClassValidation {
    pub can_take: bool,
    pub missing_requirements: Vec<String>,
    pub requirements: PrestigeRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrestigeClassOption {
    pub id: i32,
    pub name: String,
    pub label: String,
    pub can_take: bool,
    pub reason: String,
    pub requirements: PrestigeRequirements,
}

/// A single prestige-class prerequisite in evaluable form, parsed from
/// classes.2da columns and cls_pres_* rows.
#[derive(Debug, Clone)]
enum PrereqCheck {
    Bab(i32),
    /// Met when the character has at least one of the listed feats.
    AnyFeat(Vec<FeatId>),
    Skill(SkillId, i32),
    ArcaneCasterLevel(i32),
    DivineCasterLevel(i32),
    ArcaneSpellLevel(i32),
    DivineSpellLevel(i32),
    AnySpellLevel(i32),
    /// Met when any listed class is at or above the given level.
    AnyClassLevel(Vec<ClassId>, i32),
    /// (save type: 1=Fort, 2=Ref, 3=Will), minimum value.
    BaseSave(i32, i32),
    Alignment(AlignmentRestriction),
}

#[derive(Debug, Clone)]
pub struct PrestigePrereqStatus {
    pub label: String,
    pub met: bool,
    pub current_value: Option<String>,
}

impl Default for XpProgress {
    fn default() -> Self {
        Self {
            current_xp: 0,
            current_level: 1,
            xp_for_current_level: 0,
            xp_for_next_level: 1000,
            xp_remaining: 1000,
            progress_percent: 0.0,
        }
    }
}

impl Character {
    /// Get the total character level (sum of all class levels).
    pub fn total_level(&self) -> i32 {
        self.class_entries().iter().map(|entry| entry.level).sum()
    }

    /// Get all class entries for this character.
    pub fn class_entries(&self) -> Vec<ClassEntry> {
        let Some(class_list) = self.get_list("ClassList") else {
            return vec![];
        };

        class_list
            .iter()
            .filter_map(|entry| {
                let class_id = entry.get("Class").and_then(gff_value_to_i32)?;
                let class_level = entry.get("ClassLevel").and_then(gff_value_to_i32)?;

                Some(ClassEntry {
                    class_id: ClassId(class_id),
                    level: class_level,
                })
            })
            .collect()
    }

    /// Get the level in a specific class (0 if character doesn't have this class).
    pub fn class_level(&self, class_id: ClassId) -> i32 {
        self.class_entries()
            .iter()
            .find(|e| e.class_id == class_id)
            .map_or(0, |e| e.level)
    }

    /// Get the number of different classes the character has.
    pub fn class_count(&self) -> usize {
        self.class_entries().len()
    }

    /// Get all domains possessed by the character (from Cleric or similar classes).
    /// Returns a list of Domain IDs.
    pub fn domains(&self) -> Vec<crate::character::types::DomainId> {
        let Some(class_list) = self.get_list("ClassList") else {
            return vec![];
        };

        let mut domains = Vec::new();

        for entry in class_list {
            // Check for Domain1 and Domain2 fields
            if let Some(d1) = entry.get("Domain1").and_then(gff_value_to_i32)
                && d1 > 0
            {
                domains.push(crate::character::types::DomainId(d1));
            }
            if let Some(d2) = entry.get("Domain2").and_then(gff_value_to_i32)
                && d2 > 0
            {
                domains.push(crate::character::types::DomainId(d2));
            }
        }
        domains
    }

    /// Check if the character has a specific class.
    pub fn has_class(&self, class_id: ClassId) -> bool {
        self.class_entries().iter().any(|e| e.class_id == class_id)
    }

    /// Set the level for an existing class.
    pub fn set_class_level(
        &mut self,
        class_id: ClassId,
        new_level: i32,
    ) -> Result<(), CharacterError> {
        if new_level < 1 {
            return Err(CharacterError::ValidationFailed {
                field: "ClassLevel",
                message: format!("Level must be at least 1, got {new_level}"),
            });
        }

        if new_level > MAX_TOTAL_LEVEL {
            return Err(CharacterError::OutOfRange {
                field: "ClassLevel",
                value: new_level,
                min: 1,
                max: MAX_TOTAL_LEVEL,
            });
        }

        let mut class_list = self
            .get_list_owned("ClassList")
            .ok_or(CharacterError::FieldMissing { field: "ClassList" })?;

        let mut found = false;
        for entry in &mut class_list {
            let entry_class_id = entry.get("Class").and_then(gff_value_to_i32).unwrap_or(-1);
            if entry_class_id == class_id.0 {
                entry.insert("ClassLevel".to_string(), GffValue::Short(new_level as i16));
                found = true;
                break;
            }
        }

        if !found {
            return Err(CharacterError::NotFound {
                entity: "Class",
                id: class_id.0,
            });
        }

        self.set_list("ClassList", class_list);
        Ok(())
    }

    /// Add a new class to the character.
    pub fn add_class_entry(&mut self, class_id: ClassId, level: i32) -> Result<(), CharacterError> {
        if level < 1 {
            return Err(CharacterError::ValidationFailed {
                field: "ClassLevel",
                message: format!("Level must be at least 1, got {level}"),
            });
        }

        if self.has_class(class_id) {
            return Err(CharacterError::AlreadyExists {
                entity: "Class",
                id: class_id.0,
            });
        }

        let current_class_count = self.class_count();
        if current_class_count >= MAX_CLASSES {
            return Err(CharacterError::ValidationFailed {
                field: "ClassList",
                message: format!("Maximum {MAX_CLASSES} classes allowed"),
            });
        }

        let mut class_list = self.get_list_owned("ClassList").unwrap_or_default();

        let mut new_entry = IndexMap::new();
        new_entry.insert("Class".to_string(), GffValue::Int(class_id.0));
        new_entry.insert("ClassLevel".to_string(), GffValue::Short(level as i16));
        class_list.push(new_entry);

        self.set_list("ClassList", class_list);
        Ok(())
    }

    pub fn get_class_feats_for_level(
        &self,
        class_id: ClassId,
        level: i32,
        game_data: &GameData,
    ) -> Vec<ClassFeatInfo> {
        let mut feats_for_level = Vec::new();

        let Some(classes_table) = game_data.get_table("classes") else {
            return feats_for_level;
        };

        let Some(class_row) = classes_table.get_by_id(class_id.0) else {
            return feats_for_level;
        };

        let feats_table_name = row_str(&class_row, "featstable");

        let Some(table_name) = feats_table_name else {
            return feats_for_level;
        };

        let Some(feat_table) = game_data.get_table(&table_name.to_lowercase()) else {
            return feats_for_level;
        };
        let feats = game_data.get_table("feat");

        for row_idx in 0..feat_table.row_count() {
            let Ok(row) = feat_table.get_row(row_idx) else {
                continue;
            };

            let granted_level = row_int(&row, "grantedonlevel", -1);

            if granted_level != level {
                continue;
            }

            let feat_id = row_int(&row, "featindex", -1);

            let removed = feats
                .and_then(|t| t.get_by_id(feat_id))
                .is_some_and(|row| row_bool(&row, "removed", false));
            if feat_id < 0 || removed {
                continue;
            }

            let list_type = row_int(&row, "list", 3);

            feats_for_level.push(ClassFeatInfo {
                feat_id: FeatId(feat_id),
                list_type,
                granted_on_level: granted_level,
            });
        }

        feats_for_level
    }

    /// Automatic (`List 3`) feats a class grants on the class levels in `levels`.
    fn automatic_class_feat_ids(
        &self,
        class_id: ClassId,
        levels: std::ops::RangeInclusive<i32>,
        game_data: &GameData,
    ) -> HashSet<i32> {
        levels
            .flat_map(|level| self.get_class_feats_for_level(class_id, level, game_data))
            .filter(|feat| feat.list_type == AUTOMATIC_FEAT_LIST)
            .map(|feat| feat.feat_id.0)
            .collect()
    }

    pub fn get_preserved_feat_ids(&self, game_data: &GameData) -> HashSet<i32> {
        let mut preserved = HashSet::new();

        let race_id = self.race_id().0;
        preserved.extend(
            self.get_racial_feats_for_race(race_id, game_data)
                .into_iter()
                .map(|feat| feat.0),
        );

        if let Some(subrace) = self
            .subrace_index()
            .and_then(|idx| self.get_subrace_data_by_index(idx, game_data))
        {
            preserved.extend(
                featstable_feats(game_data, subrace.feats_table.as_deref())
                    .into_iter()
                    .map(|feat| feat.0),
            );
        }

        let feat_entries = self.feat_entries();
        for entry in &feat_entries {
            if (4000..=4999).contains(&entry.feat_id.0) {
                preserved.insert(entry.feat_id.0);
            }
        }

        for entry in &feat_entries {
            if matches!(entry.source, FeatSource::Race | FeatSource::Background) {
                preserved.insert(entry.feat_id.0);
            }
        }

        if let Some(feat_table) = game_data.get_table("feat") {
            for entry in &feat_entries {
                if let Some(feat_row) = feat_table.get_by_id(entry.feat_id.0) {
                    let cat = feat_row
                        .get("toolscategories")
                        .and_then(|v| v.as_deref())
                        .and_then(|s| s.trim().parse::<i32>().ok());
                    if let Some(cat) = cat {
                        if matches!(cat, 10 | 11 | 12 | 14) {
                            preserved.insert(entry.feat_id.0);
                        }
                        continue;
                    }

                    let feat_cat = feat_row
                        .get("featcategory")
                        .and_then(|v| v.as_deref())
                        .map(str::trim)
                        .unwrap_or_default();
                    if feat_cat.contains("BACKGROUND")
                        || feat_cat.contains("HISTORY")
                        || feat_cat.contains("HERITAGE")
                        || feat_cat.contains("RACIAL")
                    {
                        preserved.insert(entry.feat_id.0);
                    }
                }
            }
        }

        preserved
    }

    /// Remove a class entirely from the character.
    /// This is a batch operation that processes all levels in one pass for performance.
    /// Protected feats (racial, background, domain) are preserved.
    pub fn remove_class(
        &mut self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let current_class_count = self.class_count();
        if current_class_count <= 1 {
            return Err(CharacterError::ValidationFailed {
                field: "ClassList",
                message: "Cannot remove last class".to_string(),
            });
        }

        let current_level = self.class_level(class_id);
        if current_level == 0 {
            return Err(CharacterError::NotFound {
                entity: "Class",
                id: class_id.0,
            });
        }

        // Get preserved feat IDs ONCE before modifying anything
        let preserved_feats = self.get_preserved_feat_ids(game_data);

        // Load all lists ONCE
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        let mut class_list = self.get_list_owned("ClassList").unwrap_or_default();
        let mut char_feat_list = self.get_list_owned("FeatList").unwrap_or_default();
        let mut char_skill_list = self.get_list_owned("SkillList").unwrap_or_default();

        // Captured here for sync_mclass_levupin (called near end of fn).
        let removed_class_idx = class_list
            .iter()
            .position(|c| c.get("Class").and_then(gff_value_to_i32) == Some(class_id.0));

        // Find all level history entries for this class
        let mut indices_to_remove: Vec<usize> = Vec::new();
        let mut removed_hit_dice = 0i32;
        let old_total_level = self.total_level();
        let old_con_mod = self.get_effective_ability_modifier(AbilityIndex::CON, game_data);
        let mut total_sp_reduction = 0i32;
        let mut feats_to_remove: HashSet<i32> = HashSet::new();
        let mut feats_to_keep: HashSet<i32> = HashSet::new();
        let mut ability_reductions: Vec<u8> = Vec::new();
        let mut skill_ranks_to_remove: HashMap<usize, i32> = HashMap::new();
        let granted_per_level = self.skill_points_granted_per_level(game_data);

        // Collect info from all entries for this class
        for (idx, entry) in lvl_stat_list.iter().enumerate() {
            let entry_class_id = entry.get("LvlStatClass").and_then(gff_value_to_i32);
            if entry_class_id == Some(class_id.0) {
                indices_to_remove.push(idx);

                removed_hit_dice += entry
                    .get("LvlStatHitDie")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                total_sp_reduction += self.unspent_skill_points_of_level(
                    granted_per_level.get(idx).copied().unwrap_or(0),
                    entry,
                    game_data,
                );

                // Ability increase to revert
                if let Some(ability_idx) = entry.get("LvlStatAbility").and_then(gff_value_to_i32)
                    && ability_idx != 255
                {
                    ability_reductions.push(ability_idx as u8);
                }

                // Chosen feats to remove
                if let Some(feat_list) =
                    super::gff_helpers::extract_list_from_map(entry, "FeatList")
                {
                    for feat_entry in feat_list {
                        if let Some(feat_id) = feat_entry.get("Feat").and_then(gff_value_to_i32) {
                            feats_to_remove.insert(feat_id);
                        }
                    }
                }

                // Skill ranks to remove
                if let Some(skills) = super::gff_helpers::extract_list_from_map(entry, "SkillList")
                {
                    for (skill_idx, skill_entry) in skills.iter().enumerate() {
                        let ranks = skill_entry
                            .get("Rank")
                            .and_then(gff_value_to_i32)
                            .unwrap_or(0);
                        if ranks > 0 {
                            *skill_ranks_to_remove.entry(skill_idx).or_insert(0) += ranks;
                        }
                    }
                }
            } else {
                // Collect chosen feats from OTHER classes to KEEP
                if let Some(feat_list) =
                    super::gff_helpers::extract_list_from_map(entry, "FeatList")
                {
                    for feat_entry in feat_list {
                        if let Some(feat_id) = feat_entry.get("Feat").and_then(gff_value_to_i32) {
                            feats_to_keep.insert(feat_id);
                        }
                    }
                }
            }
        }

        feats_to_remove.extend(self.automatic_class_feat_ids(
            class_id,
            1..=current_level,
            game_data,
        ));
        if old_total_level - current_level <= EPIC_LEVEL_THRESHOLD {
            feats_to_remove.insert(FEAT_EPIC_CHARACTER.0);
        }
        for entry in self.class_entries() {
            if entry.class_id != class_id {
                feats_to_keep.extend(self.automatic_class_feat_ids(
                    entry.class_id,
                    1..=entry.level,
                    game_data,
                ));
            }
        }

        // Subtract feats_to_keep and preserved_feats
        feats_to_remove.retain(|f| !feats_to_keep.contains(f) && !preserved_feats.contains(f));

        let surviving_grants: i32 = granted_per_level
            .iter()
            .enumerate()
            .filter(|(idx, _)| !indices_to_remove.contains(idx))
            .map(|(_, granted)| granted)
            .sum();

        // Remove LvlStatList entries in reverse order to maintain indices
        for idx in indices_to_remove.into_iter().rev() {
            lvl_stat_list.remove(idx);
        }
        ability_reductions.extend(realign_ability_increases(&mut lvl_stat_list));

        // Remove feats (respecting preserved)
        char_feat_list.retain(|f| {
            let feat_id = f.get("Feat").and_then(gff_value_to_i32).unwrap_or(-1);
            !feats_to_remove.contains(&feat_id)
        });

        remove_feats_from_level_history(&mut lvl_stat_list, &feats_to_remove);

        // Remove skill ranks
        for (skill_idx, ranks_to_remove) in skill_ranks_to_remove {
            if let Some(skill_entry) = char_skill_list.get_mut(skill_idx) {
                let current = skill_entry
                    .get("Rank")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                skill_entry.insert(
                    "Rank".to_string(),
                    GffValue::Byte((current - ranks_to_remove).max(0) as u8),
                );
            }
        }

        // Remove class from ClassList (index captured above)
        if let Some(idx) = removed_class_idx {
            class_list.remove(idx);
        }

        // Revert ability increases
        for ability_idx in ability_reductions {
            if let Some(index) = AbilityIndex::from_index(ability_idx) {
                let current_val = self.base_ability(index);
                let _ = self.set_ability(index, (current_val - 1).max(3));
            }
        }

        self.revert_level_hit_points(
            removed_hit_dice,
            current_level,
            old_total_level,
            old_con_mod,
            game_data,
        );

        // Update skill points
        let current_sp = self.get_available_skill_points();
        self.set_available_skill_points((current_sp - total_sp_reduction).max(0));

        // Set all lists back ONCE
        self.set_list("LvlStatList", lvl_stat_list);
        self.set_list("ClassList", class_list);
        self.set_list("FeatList", char_feat_list);
        self.set_list("SkillList", char_skill_list);
        self.settle_shifted_skill_grants(surviving_grants, game_data);

        if let Some(idx) = removed_class_idx {
            self.sync_mclass_levupin(idx);
        }

        self.recalculate_stats(game_data)?;
        Ok(())
    }

    /// Keep the top-level `MClassLevUpIn` byte in sync after removing the
    /// class entry at `removed_idx`. The game uses this byte as an index into
    /// `ClassList` when the player clicks Level Up; if it points past the
    /// last entry the engine dereferences NULL and crashes nwn2.exe with an
    /// access violation.
    fn sync_mclass_levupin(&mut self, removed_idx: usize) {
        let class_count = self.class_count();
        if class_count == 0 {
            self.set_byte("MClassLevUpIn", 0);
            return;
        }
        let current = self.get_i32("MClassLevUpIn").unwrap_or(0).max(0) as usize;
        let adjusted = match current.cmp(&removed_idx) {
            std::cmp::Ordering::Greater => current - 1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Less => current,
        };
        let clamped = adjusted.min(class_count - 1);
        self.set_byte("MClassLevUpIn", clamped as u8);
    }

    pub fn level_history(&self) -> Vec<LevelHistoryEntry> {
        let Some(lvl_stat_list) = self.get_list("LvlStatList") else {
            return vec![];
        };

        let mut class_level_counts: HashMap<i32, i32> = HashMap::new();
        let mut result = Vec::with_capacity(lvl_stat_list.len());

        for (idx, entry) in lvl_stat_list.iter().enumerate() {
            let character_level = (idx + 1) as i32;

            let class_id = entry
                .get("LvlStatClass")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            let class_lvl = class_level_counts.entry(class_id).or_insert(0);
            *class_lvl += 1;
            let class_level = *class_lvl;

            let hp_gained = entry
                .get("LvlStatHitDie")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            let feats_gained = Self::extract_feat_list(entry, "FeatList");
            let skills_gained = Self::extract_skill_list(entry);

            let skill_points_remaining = entry
                .get("SkillPoints")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            let ability_raw = entry
                .get("LvlStatAbility")
                .and_then(gff_value_to_i32)
                .unwrap_or(255);
            let ability_increase = if ability_raw < 6 {
                AbilityIndex::from_index(ability_raw as u8)
            } else {
                None
            };

            result.push(LevelHistoryEntry {
                character_level,
                class_id: ClassId(class_id),
                class_level,
                hp_gained,
                skill_points_remaining,
                ability_increase,
                feats_gained,
                skills_gained,
            });
        }

        result
    }

    pub fn level_history_resolved(&self, game_data: &GameData) -> Vec<ResolvedLevelHistoryEntry> {
        let history = self.level_history();
        let ability_names = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
        let con_mod = self.get_effective_ability_modifier(AbilityIndex::CON, game_data);

        history
            .into_iter()
            .map(|entry| {
                let class_name = self.get_class_name(entry.class_id, game_data);

                let ability_increase = entry.ability_increase.map(|idx| {
                    ability_names
                        .get(idx.0 as usize)
                        .map(|s| (*s).to_string())
                        .unwrap_or_else(|| format!("Ability {}", idx.0))
                });

                let feats_gained: Vec<ResolvedFeatEntry> = entry
                    .feats_gained
                    .iter()
                    .map(|&feat_id| ResolvedFeatEntry {
                        feat_id: feat_id.0,
                        name: self.get_feat_name(feat_id, game_data),
                    })
                    .collect();

                let skills_gained: Vec<ResolvedSkillEntry> = entry
                    .skills_gained
                    .iter()
                    .map(|skill| ResolvedSkillEntry {
                        skill_id: skill.skill_id.0,
                        name: self.get_skill_name(skill.skill_id, game_data),
                        ranks: skill.ranks,
                    })
                    .collect();

                ResolvedLevelHistoryEntry {
                    character_level: entry.character_level,
                    class_id: entry.class_id.0,
                    class_name,
                    class_level: entry.class_level,
                    hp_gained: (entry.hp_gained + con_mod).max(1),
                    skill_points_remaining: entry.skill_points_remaining,
                    ability_increase,
                    feats_gained,
                    skills_gained,
                }
            })
            .collect()
    }

    /// Get class information from game data tables.
    ///
    /// Looks up class in classes.2da and resolves name from TLK.
    pub fn get_class_info(&self, class_id: ClassId, game_data: &GameData) -> Option<ClassInfo> {
        let classes_table = game_data.get_table("classes")?;
        let class_data = classes_table.get_by_id(class_id.0)?;

        let name = Self::resolve_class_name(&class_data, game_data);

        let hit_die = row_int(&class_data, "hitdie", 6);

        let primary_ability = class_data
            .get("primaryabil")
            .and_then(|v| v.as_deref())
            .and_then(|s| AbilityIndex::from_index(s.trim().parse::<u8>().ok()?));

        let is_spellcaster = row_bool(&class_data, "spellcaster", false);

        let bab_table_name = row_str(&class_data, "attackbonustable").unwrap_or_default();
        let bab_type = BabType::from_table(&bab_table_name);

        Some(ClassInfo {
            id: class_id,
            name,
            hit_die,
            primary_ability,
            is_spellcaster,
            bab_type,
        })
    }

    pub fn get_hit_die(&self, class_id: ClassId, game_data: &GameData) -> i32 {
        let Some(classes_table) = game_data.get_table("classes") else {
            return 6;
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return 6;
        };

        row_int(&class_data, "hitdie", 6)
    }

    pub fn calculate_xp_for_level(&self, level: i32, game_data: &GameData) -> i32 {
        if level < 1 {
            return 0;
        }

        let Some(exp_table) = game_data.get_table("exptable") else {
            return 0;
        };

        let row_index = (level - 1).max(0) as usize;
        exp_table
            .get_cell(row_index, "XP")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0)
    }

    pub fn get_xp_progress(&self, game_data: &GameData) -> XpProgress {
        let current_xp = self.experience();
        let current_level = self.total_level();

        let xp_for_current_level = self.calculate_xp_for_level(current_level, game_data);
        let xp_for_next_level = self.calculate_xp_for_level(current_level + 1, game_data);

        let xp_remaining = (xp_for_next_level - current_xp).max(0);

        let progress_percent = if xp_for_next_level > xp_for_current_level {
            let range = xp_for_next_level - xp_for_current_level;
            if range > 0 {
                let progress = (current_xp - xp_for_current_level).max(0);
                ((progress as f32 / range as f32) * 100.0).min(100.0)
            } else {
                100.0
            }
        } else {
            100.0
        };

        XpProgress {
            current_xp,
            current_level,
            xp_for_current_level,
            xp_for_next_level,
            xp_remaining,
            progress_percent,
        }
    }

    pub fn get_class_summary(&self, game_data: &GameData) -> Vec<ClassSummaryEntry> {
        let entries = self.class_entries();
        let mut summary = Vec::with_capacity(entries.len());

        for entry in entries {
            let name = self
                .get_class_info(entry.class_id, game_data)
                .map_or_else(|| format!("Class {}", entry.class_id.0), |info| info.name);

            let hit_die = self.get_hit_die(entry.class_id, game_data);

            let (base_attack_bonus, fortitude_save, reflex_save, will_save, skill_points_per_level) =
                Self::get_class_stats_at_level(entry.class_id, entry.level, game_data);

            summary.push(ClassSummaryEntry {
                class_id: entry.class_id,
                name,
                level: entry.level,
                hit_die,
                base_attack_bonus,
                fortitude_save,
                reflex_save,
                will_save,
                skill_points_per_level,
            });
        }

        summary
    }

    fn get_class_stats_at_level(
        class_id: ClassId,
        level: i32,
        game_data: &GameData,
    ) -> (i32, i32, i32, i32, i32) {
        let classes_table = match game_data.get_table("classes") {
            Some(t) => t,
            None => return (0, 0, 0, 0, 2),
        };

        let class_data = match classes_table.get_by_id(class_id.0) {
            Some(d) => d,
            None => return (0, 0, 0, 0, 2),
        };

        let skill_points = row_int(&class_data, "skillpointbase", 2);

        let bab_table_name = row_str(&class_data, "attackbonustable").unwrap_or_default();

        let save_table_name = row_str(&class_data, "savingthrowtable").unwrap_or_default();

        let bab_table = game_data.get_table(&bab_table_name.to_lowercase());
        let save_table = game_data.get_table(&save_table_name.to_lowercase());

        let row_idx = (level - 1).max(0) as usize;

        let bab_type = BabType::from_table(&bab_table_name);

        let base_attack_bonus = bab_table
            .as_ref()
            .and_then(|t| t.get_cell(row_idx, "BAB").ok().flatten())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or_else(|| bab_type.fallback_bab(level));

        let (fortitude_save, reflex_save, will_save) = save_table.as_ref().map_or((0, 0, 0), |t| {
            let fort = t
                .get_cell(row_idx, "FortSave")
                .ok()
                .flatten()
                .or_else(|| t.get_cell(row_idx, "Fort").ok().flatten())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            let ref_save = t
                .get_cell(row_idx, "RefSave")
                .ok()
                .flatten()
                .or_else(|| t.get_cell(row_idx, "Ref").ok().flatten())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            let will = t
                .get_cell(row_idx, "WillSave")
                .ok()
                .flatten()
                .or_else(|| t.get_cell(row_idx, "Will").ok().flatten())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            (fort, ref_save, will)
        });

        (
            base_attack_bonus,
            fortitude_save,
            reflex_save,
            will_save,
            skill_points,
        )
    }

    pub fn get_classes_state(&self, game_data: &GameData) -> ClassesState {
        ClassesState {
            total_level: self.total_level(),
            entries: self.get_class_summary(game_data),
            xp_progress: self.get_xp_progress(game_data),
            level_history: self.level_history_resolved(game_data),
        }
    }

    fn resolve_class_name(
        class_data: &AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        class_data
            .get("name")
            .and_then(|v| v.as_deref())
            .and_then(|s| s.trim().parse::<i32>().ok())
            .and_then(|strref| game_data.get_string(strref))
            .filter(|name| !name.is_empty())
            .or_else(|| row_str(class_data, "label"))
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn extract_feat_list(entry: &IndexMap<String, GffValue<'static>>, field: &str) -> Vec<FeatId> {
        let list = super::gff_helpers::extract_list_from_map(entry, field).unwrap_or_default();
        list.iter()
            .filter_map(|f| f.get("Feat").and_then(gff_value_to_i32).map(FeatId))
            .collect()
    }

    fn extract_skill_list(entry: &IndexMap<String, GffValue<'static>>) -> Vec<SkillRankEntry> {
        let list =
            super::gff_helpers::extract_list_from_map(entry, "SkillList").unwrap_or_default();

        list.iter()
            .enumerate()
            .filter_map(|(idx, s)| {
                let ranks = s.get("Rank").and_then(gff_value_to_i32)?;
                if ranks > 0 {
                    Some(SkillRankEntry {
                        skill_id: SkillId(idx as i32),
                        ranks,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_class_name(&self, class_id: ClassId, game_data: &GameData) -> String {
        self.get_class_info(class_id, game_data)
            .map_or_else(|| format!("Class {}", class_id.0), |info| info.name)
    }

    pub fn is_prestige_class(&self, class_id: ClassId, game_data: &GameData) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        row_bool(&class_data, "isprestige", false)
    }

    pub fn get_prestige_requirements(
        class_data: &AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> PrestigeRequirements {
        let name_of = |table: &str, id: i32, name_column: &str| -> String {
            game_data
                .get_table(table)
                .and_then(|t| t.get_by_id(id))
                .and_then(|row| row.get(name_column).and_then(|v| v.clone()))
                .and_then(|strref| strref.trim().parse::<i32>().ok())
                .and_then(|strref| game_data.get_string(strref))
                .unwrap_or_else(|| id.to_string())
        };
        let push_unique = |list: &mut Vec<String>, label: String| {
            if !list.contains(&label) {
                list.push(label);
            }
        };

        let mut requirements = PrestigeRequirements::default();
        for check in Self::collect_prestige_prereq_checks(class_data, game_data) {
            match check {
                PrereqCheck::Bab(bab) => requirements.base_attack_bonus = Some(bab),
                PrereqCheck::Skill(skill_id, ranks) => {
                    let name = name_of("skills", skill_id.0, "name");
                    if !requirements.skills.iter().any(|(n, _)| *n == name) {
                        requirements.skills.push((name, ranks));
                    }
                }
                PrereqCheck::AnyFeat(ids) => {
                    let names: Vec<String> =
                        ids.iter().map(|id| name_of("feat", id.0, "feat")).collect();
                    push_unique(&mut requirements.feats, names.join(" or "));
                }
                PrereqCheck::ArcaneCasterLevel(level) => requirements
                    .skills
                    .push((format!("Arcane Caster Level {level}"), level)),
                PrereqCheck::DivineCasterLevel(level) => requirements
                    .skills
                    .push((format!("Divine Caster Level {level}"), level)),
                PrereqCheck::ArcaneSpellLevel(level) => push_unique(
                    &mut requirements.feats,
                    format!("Able to cast level {level} arcane spells"),
                ),
                PrereqCheck::DivineSpellLevel(level) => push_unique(
                    &mut requirements.feats,
                    format!("Able to cast level {level} divine spells"),
                ),
                PrereqCheck::AnySpellLevel(level) => push_unique(
                    &mut requirements.feats,
                    format!("Able to cast level {level} spells"),
                ),
                PrereqCheck::AnyClassLevel(ids, level) => {
                    let names: Vec<String> = ids
                        .iter()
                        .map(|id| name_of("classes", id.0, "name"))
                        .collect();
                    push_unique(
                        &mut requirements.feats,
                        format!("{} Level {level}", names.join(" or ")),
                    );
                }
                PrereqCheck::BaseSave(save_type, value) => {
                    let save_name = match save_type {
                        1 => "Fortitude",
                        2 => "Reflex",
                        3 => "Will",
                        _ => "Base Save",
                    };
                    push_unique(
                        &mut requirements.feats,
                        format!("{save_name} Save +{value}"),
                    );
                }
                PrereqCheck::Alignment(restriction) => {
                    requirements.alignment = restriction.decode_to_string();
                }
            }
        }
        requirements
    }

    /// Parse a class's prestige prerequisites into evaluable checks.
    fn collect_prestige_prereq_checks(
        class_data: &AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> Vec<PrereqCheck> {
        let mut checks = Vec::new();
        let mut min_bab: Option<i32> = None;

        let bab = row_int(class_data, "minattackbonus", 0);
        if bab > 0 {
            min_bab = Some(bab);
        }

        if let Some(skill_str) = row_str(class_data, "reqskill") {
            let min_ranks = row_int(class_data, "reqskillminranks", 0);
            if min_ranks > 0
                && let Ok(skill_id) = skill_str.trim().parse::<i32>()
            {
                checks.push(PrereqCheck::Skill(SkillId(skill_id), min_ranks));
            }
        }

        if let Some(feat_str) = row_str(class_data, "prereqfeat1")
            && let Ok(feat_id) = feat_str.trim().parse::<i32>()
        {
            checks.push(PrereqCheck::AnyFeat(vec![FeatId(feat_id)]));
        }

        if let Some(restriction) = AlignmentRestriction::from_class_row(class_data) {
            checks.push(PrereqCheck::Alignment(restriction));
        }

        if let Some(prereq_table_name) = row_str(class_data, "prereqtable")
            && let Some(prereq_table) = game_data.get_table(&prereq_table_name.to_lowercase())
        {
            let mut rows_parsed: Vec<(String, String, String)> = Vec::new();
            for row_idx in 0..prereq_table.row_count() {
                let Ok(row) = prereq_table.get_row(row_idx) else {
                    continue;
                };
                rows_parsed.push((
                    row_str(&row, "reqtype").unwrap_or_default().to_uppercase(),
                    row_str(&row, "reqparam1").unwrap_or_default(),
                    row_str(&row, "reqparam2").unwrap_or_default(),
                ));
            }

            let mut i = 0;
            while i < rows_parsed.len() {
                let (ref req_type, ref param1, ref param2) = rows_parsed[i];
                match req_type.as_str() {
                    "FEAT" => {
                        if let Ok(id) = param1.trim().parse::<i32>() {
                            checks.push(PrereqCheck::AnyFeat(vec![FeatId(id)]));
                        }
                    }
                    "FEATOR" => {
                        let mut ids = Vec::new();
                        if let Ok(id) = param1.trim().parse::<i32>() {
                            ids.push(FeatId(id));
                        }
                        while i + 1 < rows_parsed.len() && rows_parsed[i + 1].0 == "FEATOR" {
                            i += 1;
                            if let Ok(id) = rows_parsed[i].1.trim().parse::<i32>() {
                                ids.push(FeatId(id));
                            }
                        }
                        if !ids.is_empty() {
                            checks.push(PrereqCheck::AnyFeat(ids));
                        }
                    }
                    "SKILL" => {
                        if let Ok(id) = param1.trim().parse::<i32>() {
                            let min_ranks = param2.trim().parse::<i32>().unwrap_or(1);
                            checks.push(PrereqCheck::Skill(SkillId(id), min_ranks));
                        }
                    }
                    "BAB" => {
                        if let Ok(bab) = param1.trim().parse::<i32>() {
                            min_bab = Some(min_bab.unwrap_or(0).max(bab));
                        }
                    }
                    "ARCANE" => checks.push(PrereqCheck::ArcaneCasterLevel(
                        param1.trim().parse().unwrap_or(1),
                    )),
                    "DIVINE" => checks.push(PrereqCheck::DivineCasterLevel(
                        param1.trim().parse().unwrap_or(1),
                    )),
                    "ARCSPELL" => checks.push(PrereqCheck::ArcaneSpellLevel(
                        param1.trim().parse().unwrap_or(1),
                    )),
                    "DIVSPELL" => checks.push(PrereqCheck::DivineSpellLevel(
                        param1.trim().parse().unwrap_or(1),
                    )),
                    "SPELLLVL" => checks.push(PrereqCheck::AnySpellLevel(
                        param1.trim().parse().unwrap_or(1),
                    )),
                    "CLASSLEVEL" => {
                        if let Ok(id) = param1.trim().parse::<i32>() {
                            let level = param2.trim().parse::<i32>().unwrap_or(1);
                            checks.push(PrereqCheck::AnyClassLevel(vec![ClassId(id)], level));
                        }
                    }
                    "CLASSOR" => {
                        let level = param2.trim().parse::<i32>().unwrap_or(1);
                        let mut ids = Vec::new();
                        if let Ok(id) = param1.trim().parse::<i32>() {
                            ids.push(ClassId(id));
                        }
                        while i + 1 < rows_parsed.len() && rows_parsed[i + 1].0 == "CLASSOR" {
                            i += 1;
                            if let Ok(id) = rows_parsed[i].1.trim().parse::<i32>() {
                                ids.push(ClassId(id));
                            }
                        }
                        if !ids.is_empty() {
                            checks.push(PrereqCheck::AnyClassLevel(ids, level));
                        }
                    }
                    "SAVE" => {
                        let value = param1.trim().parse::<i32>().unwrap_or(0);
                        let save_type = param2.trim().parse::<i32>().unwrap_or(0);
                        checks.push(PrereqCheck::BaseSave(save_type, value));
                    }
                    "VAR" => {}
                    other => {
                        tracing::debug!("Skipping unknown prereqtable reqtype: {other}");
                    }
                }
                i += 1;
            }
        }

        if let Some(bab) = min_bab {
            checks.push(PrereqCheck::Bab(bab));
        }

        checks
    }

    /// Caster levels prestige classes add to the base class `class_id`: every prestige level
    /// flagged in its `BonusSpellcasterLevelTable` counts for the base class whose
    /// spellcasting feat (`BonusCasterFeatByClassMap`) the character took.
    pub(crate) fn prestige_bonus_caster_levels(&self, class_id: i32, game_data: &GameData) -> i32 {
        let Some(classes_table) = game_data.get_table("classes") else {
            return 0;
        };
        let mut bonus = 0;
        for entry in self.class_entries() {
            let Some(prestige_row) = classes_table.get_by_id(entry.class_id.0) else {
                continue;
            };
            let (Some(level_table), Some(feat_map)) = (
                row_str(&prestige_row, "bonusspellcasterleveltable")
                    .and_then(|name| game_data.get_table(&name.to_lowercase())),
                row_str(&prestige_row, "bonuscasterfeatbyclassmap")
                    .and_then(|name| game_data.get_table(&name.to_lowercase())),
            ) else {
                continue;
            };
            let cell_int = |table: &crate::loaders::types::LoadedTable, row: usize, col: &str| {
                table
                    .get_cell(row, col)
                    .ok()
                    .flatten()
                    .and_then(|v| v.trim().parse::<i32>().ok())
            };
            let took_feat = (0..feat_map.row_count()).any(|row| {
                cell_int(feat_map, row, "ClassId") == Some(class_id)
                    && cell_int(feat_map, row, "SpellcasterFeat")
                        .is_some_and(|feat| feat >= 0 && self.has_feat(FeatId(feat)))
            });
            if !took_feat {
                continue;
            }
            let levels = (entry.level.max(0) as usize).min(level_table.row_count());
            bonus += (0..levels)
                .filter(|&row| cell_int(level_table, row, "GrantsBonusSpellcasterLevel") == Some(1))
                .count() as i32;
        }
        bonus
    }

    /// Highest caster level among the character's spellcasting classes of the
    /// requested type (true = arcane, false = divine, None = any).
    pub(crate) fn caster_level(&self, want_arcane: Option<bool>, game_data: &GameData) -> i32 {
        self.casting_classes(want_arcane, game_data)
            .map(|class_id| self.get_caster_level(class_id, game_data))
            .max()
            .unwrap_or(0)
    }

    /// Highest spell level the character can cast, from each casting class's
    /// spell gain table at its current caster level.
    pub(crate) fn max_castable_spell_level(
        &self,
        want_arcane: Option<bool>,
        game_data: &GameData,
    ) -> i32 {
        self.casting_classes(want_arcane, game_data)
            .filter_map(|class_id| self.max_spell_level(class_id, game_data))
            .max()
            .unwrap_or(0)
    }

    fn casting_classes<'a>(
        &'a self,
        want_arcane: Option<bool>,
        game_data: &'a GameData,
    ) -> impl Iterator<Item = ClassId> + 'a {
        let classes_table = game_data.get_table("classes");
        self.class_entries()
            .into_iter()
            .map(|entry| entry.class_id)
            .filter(move |class_id| {
                let Some(class_row) = classes_table.and_then(|t| t.get_by_id(class_id.0)) else {
                    return false;
                };
                match want_arcane {
                    Some(true) => row_bool(&class_row, "hasarcane", false),
                    Some(false) => row_bool(&class_row, "hasdivine", false),
                    None => true,
                }
            })
    }

    fn evaluate_prereq_check(
        &self,
        check: &PrereqCheck,
        game_data: &GameData,
    ) -> PrestigePrereqStatus {
        match check {
            PrereqCheck::Bab(min) => {
                let current = self.calculate_bab(game_data);
                PrestigePrereqStatus {
                    label: format!("Base Attack Bonus +{min}"),
                    met: current >= *min,
                    current_value: Some(format!("+{current}")),
                }
            }
            PrereqCheck::AnyFeat(ids) => {
                let names: Vec<String> = ids
                    .iter()
                    .map(|id| self.get_feat_name(*id, game_data))
                    .collect();
                PrestigePrereqStatus {
                    label: names.join(" or "),
                    met: ids.iter().any(|id| self.has_feat(*id)),
                    current_value: None,
                }
            }
            PrereqCheck::Skill(skill_id, min_ranks) => {
                let current = self.skill_rank(*skill_id);
                PrestigePrereqStatus {
                    label: format!(
                        "{} {min_ranks} ranks",
                        self.get_skill_name(*skill_id, game_data)
                    ),
                    met: current >= *min_ranks,
                    current_value: Some(format!("{current} ranks")),
                }
            }
            PrereqCheck::ArcaneCasterLevel(min) => {
                let current = self.caster_level(Some(true), game_data);
                PrestigePrereqStatus {
                    label: format!("Arcane Caster Level {min}"),
                    met: current >= *min,
                    current_value: Some(format!("level {current}")),
                }
            }
            PrereqCheck::DivineCasterLevel(min) => {
                let current = self.caster_level(Some(false), game_data);
                PrestigePrereqStatus {
                    label: format!("Divine Caster Level {min}"),
                    met: current >= *min,
                    current_value: Some(format!("level {current}")),
                }
            }
            PrereqCheck::ArcaneSpellLevel(min) => {
                let current = self.max_castable_spell_level(Some(true), game_data);
                PrestigePrereqStatus {
                    label: format!("Able to cast level {min} arcane spells"),
                    met: current >= *min,
                    current_value: Some(format!("can cast level {current}")),
                }
            }
            PrereqCheck::DivineSpellLevel(min) => {
                let current = self.max_castable_spell_level(Some(false), game_data);
                PrestigePrereqStatus {
                    label: format!("Able to cast level {min} divine spells"),
                    met: current >= *min,
                    current_value: Some(format!("can cast level {current}")),
                }
            }
            PrereqCheck::AnySpellLevel(min) => {
                let current = self.max_castable_spell_level(None, game_data);
                PrestigePrereqStatus {
                    label: format!("Able to cast level {min} spells"),
                    met: current >= *min,
                    current_value: Some(format!("can cast level {current}")),
                }
            }
            PrereqCheck::AnyClassLevel(ids, min_level) => {
                let names: Vec<String> = ids
                    .iter()
                    .map(|id| self.get_class_name(*id, game_data))
                    .collect();
                PrestigePrereqStatus {
                    label: format!("{} Level {min_level}", names.join(" or ")),
                    met: ids.iter().any(|id| self.class_level(*id) >= *min_level),
                    current_value: None,
                }
            }
            PrereqCheck::BaseSave(save_type, min) => {
                let saves = self.calculate_base_saves(game_data);
                let (name, current) = match save_type {
                    1 => ("Fortitude", saves.fortitude),
                    2 => ("Reflex", saves.reflex),
                    3 => ("Will", saves.will),
                    _ => ("Base", saves.fortitude.max(saves.reflex).max(saves.will)),
                };
                PrestigePrereqStatus {
                    label: format!("{name} Save +{min}"),
                    met: current >= *min,
                    current_value: Some(format!("+{current}")),
                }
            }
            PrereqCheck::Alignment(restriction) => {
                let alignment = self.alignment();
                PrestigePrereqStatus {
                    label: format!(
                        "Alignment: {}",
                        restriction
                            .decode_to_string()
                            .unwrap_or_else(|| "Any".to_string())
                    ),
                    met: restriction.check_alignment(&alignment),
                    current_value: Some(alignment.alignment_string()),
                }
            }
        }
    }

    /// Evaluate every prerequisite of a prestige class against this character.
    pub fn prestige_prerequisite_status(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Vec<PrestigePrereqStatus> {
        let Some(classes_table) = game_data.get_table("classes") else {
            return vec![];
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return vec![];
        };
        Self::collect_prestige_prereq_checks(&class_data, game_data)
            .iter()
            .map(|check| self.evaluate_prereq_check(check, game_data))
            .collect()
    }

    pub fn validate_prestige_class_requirements(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> PrestigeClassValidation {
        let Some(classes_table) = game_data.get_table("classes") else {
            return PrestigeClassValidation::default();
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return PrestigeClassValidation::default();
        };

        let requirements = Self::get_prestige_requirements(&class_data, game_data);
        let missing: Vec<String> = self
            .prestige_prerequisite_status(class_id, game_data)
            .into_iter()
            .filter(|status| !status.met)
            .map(|status| match status.current_value {
                Some(current) => format!("{} (have {current})", status.label),
                None => status.label,
            })
            .collect();

        PrestigeClassValidation {
            can_take: missing.is_empty(),
            missing_requirements: missing,
            requirements,
        }
    }

    pub fn get_prestige_class_options(&self, game_data: &GameData) -> Vec<PrestigeClassOption> {
        let Some(classes_table) = game_data.get_table("classes") else {
            return vec![];
        };

        let mut options = Vec::new();

        for row_idx in 0..classes_table.row_count() {
            let Some(row) = classes_table.get_row(row_idx).ok() else {
                continue;
            };

            let is_prestige = row_bool(&row, "isprestige", false);

            let max_level = row_int(&row, "maxlevel", 0);

            if !is_prestige && max_level <= 0 {
                continue;
            }

            let label = row_str(&row, "label").unwrap_or_default();
            if label.is_empty() || label.to_lowercase().contains("padding") {
                continue;
            }

            let class_id = ClassId(row_idx as i32);
            let validation = self.validate_prestige_class_requirements(class_id, game_data);

            let name = row
                .get("name")
                .and_then(|v| v.as_deref())
                .and_then(|s| s.trim().parse::<i32>().ok())
                .and_then(|strref| game_data.get_string(strref))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| label.clone());

            let reason = if validation.can_take {
                "All requirements met".to_string()
            } else {
                validation.missing_requirements.join("; ")
            };

            options.push(PrestigeClassOption {
                id: row_idx as i32,
                name,
                label,
                can_take: validation.can_take,
                reason,
                requirements: validation.requirements,
            });
        }

        options.sort_by_key(|a| a.name.to_lowercase());
        options
    }

    pub fn level_up(
        &mut self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Result<LevelUpResult, CharacterError> {
        let classes_table = game_data
            .get_table("classes")
            .ok_or_else(|| CharacterError::TableNotFound("classes".to_string()))?;
        let class_data = classes_table
            .get_by_id(class_id.0)
            .ok_or(CharacterError::NotFound {
                entity: "Class",
                id: class_id.0,
            })?;

        let current_total_level = self.total_level();
        let new_total_level = current_total_level + 1;

        if new_total_level > MAX_TOTAL_LEVEL {
            return Err(CharacterError::ValidationFailed {
                field: "TotalLevel",
                message: format!("Cannot exceed max level {MAX_TOTAL_LEVEL}"),
            });
        }

        let max_lvl = row_int(&class_data, "maxlevel", 0);
        if max_lvl > 0 {
            let current_class_level = self.class_level(class_id);
            if current_class_level >= max_lvl {
                return Err(CharacterError::ValidationFailed {
                    field: "ClassLevel",
                    message: format!("Cannot exceed max level {max_lvl} for class {class_id:?}"),
                });
            }
        }

        if self.class_level(class_id) == 0 && row_bool(&class_data, "isprestige", false) {
            let validation = self.validate_prestige_class_requirements(class_id, game_data);
            if !validation.can_take {
                return Err(CharacterError::ValidationFailed {
                    field: "PrestigeRequirements",
                    message: format!(
                        "Prestige class requirements not met: {}",
                        validation.missing_requirements.join("; ")
                    ),
                });
            }
        }

        let mut class_list = self.get_list_owned("ClassList").unwrap_or_default();
        let mut class_found = false;
        let mut class_level_gained = 1;

        for entry in &mut class_list {
            let entry_class = entry.get("Class").and_then(gff_value_to_i32).unwrap_or(-1);
            if entry_class == class_id.0 {
                let cur = entry
                    .get("ClassLevel")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                class_level_gained = cur + 1;
                entry.insert(
                    "ClassLevel".to_string(),
                    GffValue::Short(class_level_gained as i16),
                );
                class_found = true;
                break;
            }
        }

        if !class_found {
            if class_list.len() >= MAX_CLASSES {
                return Err(CharacterError::ValidationFailed {
                    field: "ClassList",
                    message: format!("Max classes {MAX_CLASSES} reached"),
                });
            }
            let mut new_entry = IndexMap::new();
            new_entry.insert("Class".to_string(), GffValue::Int(class_id.0));
            new_entry.insert("ClassLevel".to_string(), GffValue::Short(1));
            class_list.push(new_entry);
            class_level_gained = 1;
        }

        self.set_list("ClassList", class_list);

        let xp_req = self.calculate_xp_for_level(new_total_level, game_data);
        if self.experience() < xp_req {
            self.set_experience(xp_req)?;
        }

        let hit_die = row_int(&class_data, "hitdie", 6);
        let con_mod = self.get_effective_ability_modifier(AbilityIndex::CON, game_data);

        let hp_gained = std::cmp::max(1, hit_die + con_mod);

        let current_max_hp = self.max_hp();
        let current_base_hp = self.base_hp();
        let current_hp = self.current_hp();

        self.set_max_hp(current_max_hp + hp_gained);
        self.set_base_hp(current_base_hp + hit_die);
        self.set_current_hp(current_hp + hp_gained);

        // 5. Skill Points
        let int_mod = self.get_effective_ability_modifier(AbilityIndex::INT, game_data);
        let race_bonus = self.racial_skill_point_bonus();

        let skill_base = row_int(&class_data, "skillpointbase", 2);

        let mut sp_gained = std::cmp::max(1, skill_base + int_mod + race_bonus);
        if new_total_level == 1 {
            sp_gained *= 4;
        }

        let current_sp = self.get_available_skill_points();
        self.set_available_skill_points(current_sp + sp_gained);

        // 6. Feats
        let slots_now = 1
            + (std::cmp::min(new_total_level, 20) / 3)
            + ((std::cmp::max(new_total_level - 20, 0) + 1) / 2);
        let prev_level = new_total_level - 1;
        let slots_prev = if prev_level > 0 {
            1 + (std::cmp::min(prev_level, 20) / 3) + ((std::cmp::max(prev_level - 20, 0) + 1) / 2)
        } else {
            0
        };
        let general_slots_gained = slots_now - slots_prev;

        let bonus_now = self.get_bonus_feats_for_class(class_id, class_level_gained, game_data);
        let bonus_prev = if class_level_gained > 1 {
            self.get_bonus_feats_for_class(class_id, class_level_gained - 1, game_data)
        } else {
            0
        };
        let bonus_slots_gained = bonus_now - bonus_prev;

        // 7. Ability Increase
        let ability_increase_gained = (new_total_level % 4) == 0;

        // 8. Spellcasting
        let is_spellcaster = row_bool(&class_data, "spellcaster", false);

        // 9. Update LvlStatList History
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        let mut new_hist = IndexMap::new();
        new_hist.insert("LvlStatClass".to_string(), GffValue::Byte(class_id.0 as u8));
        // Store only the die roll: the engine (and our revert logic) adds CON dynamically.
        new_hist.insert("LvlStatHitDie".to_string(), GffValue::Byte(hit_die as u8));
        // The game records the pool remainder after this level-up, not the grant.
        new_hist.insert(
            "SkillPoints".to_string(),
            GffValue::Word(self.get_available_skill_points().max(0) as u16),
        );
        new_hist.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
        new_hist.insert(
            "EpicLevel".to_string(),
            GffValue::Byte(u8::from(new_total_level > EPIC_LEVEL_THRESHOLD)),
        );
        new_hist.insert("FeatList".to_string(), GffValue::ListOwned(vec![]));
        let skill_count = self.get_list("SkillList").map_or(0, |skills| skills.len());
        let empty_ranks = (0..skill_count)
            .map(|_| IndexMap::from([("Rank".to_string(), GffValue::Byte(0))]))
            .collect();
        new_hist.insert("SkillList".to_string(), GffValue::ListOwned(empty_ranks));

        lvl_stat_list.push(new_hist);
        self.set_list("LvlStatList", lvl_stat_list);

        // 10. Grant automatic feats: like the engine, also the missing ones of earlier class
        // levels, plus Epic Character on reaching level 21.
        let mut automatic: Vec<FeatId> = self
            .automatic_class_feat_ids(class_id, 1..=class_level_gained, game_data)
            .into_iter()
            .map(FeatId)
            .collect();
        automatic.sort_unstable_by_key(|feat| feat.0);
        if new_total_level > EPIC_LEVEL_THRESHOLD {
            automatic.push(FEAT_EPIC_CHARACTER);
        }
        let mut granted_feats = Vec::new();
        for feat_id in automatic {
            if !self.has_feat(feat_id)
                && self
                    .add_feat_with_source(feat_id, FeatSource::Class)
                    .is_ok()
            {
                granted_feats.push(feat_id);
            }
        }

        // 11. Recalculate global stats
        self.recalculate_stats(game_data)?;

        Ok(LevelUpResult {
            class_id,
            new_level: new_total_level,
            hp_gained,
            skill_points_gained: sp_gained,
            general_feat_slots_gained: general_slots_gained,
            bonus_feat_slots_gained: bonus_slots_gained,
            ability_increase_gained,
            new_spells_gained: is_spellcaster,
            granted_feats,
        })
    }

    /// Revert the last level for the specified class.
    /// Protected feats (racial, background, domain) are preserved.
    pub fn level_down(
        &mut self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        if self.total_level() <= 1 {
            return Err(CharacterError::ValidationFailed {
                field: "ClassList",
                message: "Cannot remove the last level".to_string(),
            });
        }
        self.level_down_internal(class_id, game_data)?;
        self.recalculate_stats(game_data)?;
        Ok(())
    }

    fn level_down_internal(
        &mut self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let current_level = self.class_level(class_id);
        // Get preserved feat IDs before any modifications
        let preserved_feats = self.get_preserved_feat_ids(game_data);
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        if lvl_stat_list.is_empty() {
            return Err(CharacterError::ValidationFailed {
                field: "LvlStatList",
                message: "No level history to revert".to_string(),
            });
        }

        // Find the last entry for this class
        let mut found_idx = None;
        for (i, entry) in lvl_stat_list.iter().enumerate().rev() {
            if entry.get("LvlStatClass").and_then(gff_value_to_i32) == Some(class_id.0) {
                found_idx = Some(i);
                break;
            }
        }

        let Some(idx) = found_idx else {
            return Err(CharacterError::NotFound {
                entity: "Level history entry for class",
                id: class_id.0,
            });
        };

        let entry = lvl_stat_list.remove(idx);
        let old_total_level = self.total_level();
        let old_con_mod = self.get_effective_ability_modifier(AbilityIndex::CON, game_data);
        let granted_per_level = self.skill_points_granted_per_level(game_data);
        let granted = granted_per_level.get(idx).copied().unwrap_or(0);
        let surviving_grants = granted_per_level.iter().sum::<i32>() - granted;
        let sp_to_revert = self.unspent_skill_points_of_level(granted, &entry, game_data);

        // 1. Revert Ability Increase (Must happen before HP to match Python/Engine logic)
        if let Some(ability_idx) = entry.get("LvlStatAbility").and_then(gff_value_to_i32)
            && ability_idx != 255
            && let Some(index) = AbilityIndex::from_index(ability_idx as u8)
        {
            let current_val = self.base_ability(index);
            let _ = self.set_ability(index, (current_val - 1).max(3));
        }
        for ability_idx in realign_ability_increases(&mut lvl_stat_list) {
            if let Some(index) = AbilityIndex::from_index(ability_idx) {
                let current_val = self.base_ability(index);
                let _ = self.set_ability(index, (current_val - 1).max(3));
            }
        }

        // 2. Revert HP
        let hp_roll = entry
            .get("LvlStatHitDie")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        self.revert_level_hit_points(hp_roll, 1, old_total_level, old_con_mod, game_data);

        // 3. Revert Skill Points
        let current_sp = self.get_available_skill_points();
        self.set_available_skill_points((current_sp - sp_to_revert).max(0));

        // 4. Refund Skill Ranks (Direct edit to avoid history side-effect)
        if let Some(skills) = super::gff_helpers::extract_list_from_map(&entry, "SkillList") {
            let mut char_skill_list = self.get_list_owned("SkillList").unwrap_or_default();
            for (skill_idx, skill_entry) in skills.iter().enumerate() {
                let ranks = skill_entry
                    .get("Rank")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                if ranks > 0
                    && let Some(char_skill) = char_skill_list.get_mut(skill_idx)
                {
                    let current_ranks = char_skill
                        .get("Rank")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(0);
                    char_skill.insert(
                        "Rank".to_string(),
                        GffValue::Byte((current_ranks - ranks).max(0) as u8),
                    );
                }
            }
            self.set_list("SkillList", char_skill_list);
        }

        // 5. Remove Feats (Direct edit to avoid history side-effect)
        let mut feats_to_remove: HashSet<i32> = HashSet::new();
        let mut feats_to_keep: HashSet<i32> = HashSet::new();

        // 5a. Collect chosen feats from the removed level
        if let Some(feats) = super::gff_helpers::extract_list_from_map(&entry, "FeatList") {
            for feat_entry in feats {
                if let Some(feat_id) = feat_entry.get("Feat").and_then(gff_value_to_i32) {
                    feats_to_remove.insert(feat_id);
                }
            }
        }

        // 5b. Add auto feats granted at THIS specific level
        feats_to_remove.extend(self.automatic_class_feat_ids(
            class_id,
            current_level..=current_level,
            game_data,
        ));
        if old_total_level == EPIC_LEVEL_THRESHOLD + 1 {
            feats_to_remove.insert(FEAT_EPIC_CHARACTER.0);
        }

        // 5c. Collect chosen feats to KEEP from surviving LvlStatList entries
        for stat_entry in &lvl_stat_list {
            if let Some(feat_list) =
                super::gff_helpers::extract_list_from_map(stat_entry, "FeatList")
            {
                for feat_entry in feat_list {
                    if let Some(feat_id) = feat_entry.get("Feat").and_then(gff_value_to_i32) {
                        feats_to_keep.insert(feat_id);
                    }
                }
            }
        }

        // 5d. Add auto feats to keep from remaining levels of THIS class and ALL levels of OTHER classes
        for class_entry in self.class_entries() {
            let max_level = if class_entry.class_id == class_id {
                current_level - 1
            } else {
                class_entry.level
            };
            feats_to_keep.extend(self.automatic_class_feat_ids(
                class_entry.class_id,
                1..=max_level,
                game_data,
            ));
        }

        feats_to_remove.retain(|f| !feats_to_keep.contains(f) && !preserved_feats.contains(f));

        let mut char_feat_list = self.get_list_owned("FeatList").unwrap_or_default();
        char_feat_list.retain(|f| {
            let feat_id = f.get("Feat").and_then(gff_value_to_i32).unwrap_or(-1);
            !feats_to_remove.contains(&feat_id)
        });
        self.set_list("FeatList", char_feat_list);

        remove_feats_from_level_history(&mut lvl_stat_list, &feats_to_remove);

        // 6. Undo Spell Selections (Direct edit to avoid history side-effect)
        if let Some(mut class_list) = self.get_list_owned("ClassList") {
            for i in 0..10 {
                let list_key = format!("KnownList{i}");
                if let Some(spells) = super::gff_helpers::extract_list_from_map(&entry, &list_key) {
                    for spell_entry in spells {
                        if let Some(spell_id) = spell_entry.get("Spell").and_then(gff_value_to_i32)
                        {
                            for class_entry in &mut class_list {
                                if class_entry.get("Class").and_then(gff_value_to_i32)
                                    == Some(class_id.0)
                                    && let Some(mut known_list) =
                                        super::gff_helpers::extract_list_from_map(
                                            class_entry,
                                            &list_key,
                                        )
                                {
                                    known_list.retain(|s| {
                                        s.get("Spell").and_then(gff_value_to_i32) != Some(spell_id)
                                    });
                                    class_entry
                                        .insert(list_key.clone(), GffValue::ListOwned(known_list));
                                }
                            }
                        }
                    }
                }
            }
            self.set_list("ClassList", class_list);
        }

        // 7. Update ClassList
        let mut class_list = self.get_list_owned("ClassList").unwrap_or_default();
        let mut removed_idx: Option<usize> = None;
        for (i, entry) in class_list.iter_mut().enumerate() {
            if entry.get("Class").and_then(gff_value_to_i32) == Some(class_id.0) {
                let current_level = entry
                    .get("ClassLevel")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                if current_level > 1 {
                    entry.insert(
                        "ClassLevel".to_string(),
                        GffValue::Short((current_level - 1) as i16),
                    );
                } else {
                    removed_idx = Some(i);
                }
                break;
            }
        }
        if let Some(i) = removed_idx {
            class_list.remove(i);
        }

        self.set_list("ClassList", class_list);
        self.set_list("LvlStatList", lvl_stat_list);
        self.settle_shifted_skill_grants(surviving_grants, game_data);

        if let Some(idx) = removed_idx {
            self.sync_mclass_levupin(idx);
        }

        Ok(())
    }

    /// Sync a feat change to the level history: an added feat goes to the current level,
    /// a removed one leaves every level that recorded it.
    pub fn record_feat_change(&mut self, feat_id: FeatId, added: bool) {
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        if lvl_stat_list.is_empty() {
            return;
        }

        if !added {
            remove_feats_from_level_history(&mut lvl_stat_list, &HashSet::from([feat_id.0]));
            self.set_list("LvlStatList", lvl_stat_list);
            return;
        }

        let last_idx = lvl_stat_list.len() - 1;
        let entry = &mut lvl_stat_list[last_idx];
        let mut feat_list =
            super::gff_helpers::extract_list_from_map(entry, "FeatList").unwrap_or_default();
        if !feat_list
            .iter()
            .any(|f| f.get("Feat").and_then(gff_value_to_i32) == Some(feat_id.0))
        {
            let mut new_feat = IndexMap::new();
            new_feat.insert("Feat".to_string(), GffValue::Word(feat_id.0 as u16));
            feat_list.push(new_feat);
        }
        entry.insert("FeatList".to_string(), GffValue::ListOwned(feat_list));
        self.set_list("LvlStatList", lvl_stat_list);
    }

    /// Class taken at a `LvlStatList` index.
    pub fn level_class(&self, level_idx: usize) -> Option<ClassId> {
        self.get_list("LvlStatList")?
            .get(level_idx)?
            .get("LvlStatClass")
            .and_then(gff_value_to_i32)
            .filter(|id| *id != -1)
            .map(ClassId)
    }

    /// How `count` ranks of a skill come back out of the level history:
    /// `(level index, ranks)` most recent level first, then `(None, rest)`
    /// for ranks no level recorded.
    pub fn skill_rank_take_back_plan(
        &self,
        skill_id: SkillId,
        count: i32,
    ) -> Vec<(Option<usize>, i32)> {
        let mut remaining = count.max(0);
        let mut plan = Vec::new();

        if let Some(levels) = self.get_list("LvlStatList") {
            for (idx, entry) in levels.iter().enumerate().rev() {
                if remaining == 0 {
                    break;
                }
                let bought = super::gff_helpers::extract_list_from_map(entry, "SkillList")
                    .and_then(|skills| {
                        skills
                            .get(skill_id.0 as usize)?
                            .get("Rank")
                            .and_then(gff_value_to_i32)
                    })
                    .unwrap_or(0);
                let take = bought.min(remaining);
                if take > 0 {
                    plan.push((Some(idx), take));
                    remaining -= take;
                }
            }
        }

        if remaining > 0 {
            plan.push((None, remaining));
        }
        plan
    }

    /// Sync a skill rank change to the level history.
    ///
    /// Added ranks are recorded at the current (last) level. Removed ranks
    /// are taken back most recent level first, so a later delevel refunds
    /// the cost of the class that bought them; ranks no level recorded
    /// (toolset creation ranks, injected ranks) leave the history untouched.
    pub fn record_skill_change(&mut self, skill_id: SkillId, rank_delta: i32) {
        if rank_delta == 0 {
            return;
        }

        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        if lvl_stat_list.is_empty() {
            return;
        }

        let skill_idx = skill_id.0 as usize;
        if rank_delta > 0 {
            let last_idx = lvl_stat_list.len() - 1;
            Self::adjust_level_skill_rank(&mut lvl_stat_list[last_idx], skill_idx, rank_delta);
        } else {
            for (level_idx, ranks) in self.skill_rank_take_back_plan(skill_id, -rank_delta) {
                if let Some(level_idx) = level_idx {
                    Self::adjust_level_skill_rank(&mut lvl_stat_list[level_idx], skill_idx, -ranks);
                }
            }
        }

        self.set_list("LvlStatList", lvl_stat_list);
    }

    fn adjust_level_skill_rank(
        entry: &mut IndexMap<String, GffValue<'static>>,
        skill_idx: usize,
        delta: i32,
    ) {
        let mut skill_list =
            super::gff_helpers::extract_list_from_map(entry, "SkillList").unwrap_or_default();

        while skill_list.len() <= skill_idx {
            let mut empty_skill = IndexMap::new();
            empty_skill.insert("Rank".to_string(), GffValue::Byte(0));
            skill_list.push(empty_skill);
        }

        let skill_entry = &mut skill_list[skill_idx];
        let current_rank = skill_entry
            .get("Rank")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        skill_entry.insert(
            "Rank".to_string(),
            GffValue::Byte((current_rank + delta).max(0) as u8),
        );

        entry.insert("SkillList".to_string(), GffValue::ListOwned(skill_list));
    }

    /// Record an ability score increase in the first unspent level-up slot.
    /// A slot counts as unspent when `LvlStatAbility` is absent or not a valid
    /// ability index, mirroring `get_level_up_ability_history`; the editor
    /// writes 255, but toolset-made histories may omit the field entirely.
    pub fn record_ability_change(&mut self, ability_index: AbilityIndex) {
        use crate::character::types::ABILITY_INCREASE_INTERVAL;
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        if lvl_stat_list.is_empty() {
            return;
        }

        let mut changed = false;
        for (idx, entry) in lvl_stat_list.iter_mut().enumerate() {
            let char_level = (idx + 1) as i32;
            let spent = matches!(
                entry.get("LvlStatAbility").and_then(gff_value_to_i32),
                Some(0..=5)
            );
            if char_level % ABILITY_INCREASE_INTERVAL == 0 && !spent {
                entry.insert(
                    "LvlStatAbility".to_string(),
                    GffValue::Byte(ability_index.0),
                );
                changed = true;
                break;
            }
        }

        if changed {
            self.set_list("LvlStatList", lvl_stat_list);
        }
    }

    /// Free the most recent level-up slot spent on `ability_index`, the
    /// inverse of `record_ability_change`.
    pub fn release_ability_change(&mut self, ability_index: AbilityIndex) {
        use crate::character::types::ABILITY_INCREASE_INTERVAL;
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        let slot = lvl_stat_list
            .iter()
            .enumerate()
            .rev()
            .find_map(|(idx, entry)| {
                let char_level = (idx + 1) as i32;
                let spent_on_ability = entry.get("LvlStatAbility").and_then(gff_value_to_i32)
                    == Some(i32::from(ability_index.0));
                (char_level % ABILITY_INCREASE_INTERVAL == 0 && spent_on_ability).then_some(idx)
            });

        if let Some(idx) = slot {
            lvl_stat_list[idx].insert("LvlStatAbility".to_string(), GffValue::Byte(255));
            self.set_list("LvlStatList", lvl_stat_list);
        }
    }

    /// Record a learned or forgotten spell in the most recent level of its class, where
    /// delevelling that class takes it back. A spell learned and then forgotten (or the
    /// reverse) cancels the earlier entry instead of landing in both lists.
    pub(crate) fn record_spell_change(
        &mut self,
        class_id: ClassId,
        level: i32,
        spell_id: u16,
        added: bool,
    ) {
        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        let Some(entry) = lvl_stat_list
            .iter_mut()
            .rev()
            .find(|e| e.get("LvlStatClass").and_then(gff_value_to_i32) == Some(class_id.0))
        else {
            return;
        };

        let known_key = format!("KnownList{level}");
        let removed_key = format!("KnownRemoveList{level}");
        let (list_key, opposite_key) = if added {
            (known_key, removed_key)
        } else {
            (removed_key, known_key)
        };
        let is_spell = |s: &IndexMap<String, GffValue<'static>>| {
            s.get("Spell").and_then(gff_value_to_i32) == Some(i32::from(spell_id))
        };

        let mut opposite =
            super::gff_helpers::extract_list_from_map(entry, &opposite_key).unwrap_or_default();
        if opposite.iter().any(is_spell) {
            opposite.retain(|s| !is_spell(s));
            entry.insert(opposite_key, GffValue::ListOwned(opposite));
        } else {
            let mut spell_list =
                super::gff_helpers::extract_list_from_map(entry, &list_key).unwrap_or_default();
            if !spell_list.iter().any(is_spell) {
                let mut new_spell = IndexMap::new();
                new_spell.insert("Spell".to_string(), GffValue::Word(spell_id));
                spell_list.push(new_spell);
            }
            entry.insert(list_key, GffValue::ListOwned(spell_list));
        }
        self.set_list("LvlStatList", lvl_stat_list);
    }

    /// After levels leave the history, the remaining ones can earn a different skill point
    /// grant (a new level 1 earns four times the points, INT increases may have moved); the
    /// unspent pool follows the difference to `grants_before` of those levels.
    fn settle_shifted_skill_grants(&mut self, grants_before: i32, game_data: &GameData) {
        let grants_after: i32 = self.skill_points_granted_per_level(game_data).iter().sum();
        let pool = self.get_available_skill_points();
        self.set_available_skill_points((pool + grants_after - grants_before).max(0));
    }

    /// Take back the hit points of removed levels. `HitPoints` is the plain hit-die sum;
    /// the CON modifier counts on every level, so a CON increase reverted with the levels
    /// also takes back its bonus on the levels that remain.
    fn revert_level_hit_points(
        &mut self,
        hit_dice: i32,
        levels_removed: i32,
        old_total_level: i32,
        old_con_mod: i32,
        game_data: &GameData,
    ) {
        let new_con_mod = self.get_effective_ability_modifier(AbilityIndex::CON, game_data);
        let new_total_level = old_total_level - levels_removed;
        let reduction = hit_dice + old_total_level * old_con_mod - new_total_level * new_con_mod;

        let max_hp = (self.max_hp() - reduction).max(1);
        self.set_max_hp(max_hp);
        self.set_current_hp((self.current_hp() - reduction).clamp(1, max_hp));
        self.set_base_hp((self.base_hp() - hit_dice).max(1));
    }

    /// Recalculate the stored `BaseAttackBonus`. Base saves are derived, never stored:
    /// `fortbonus/refbonus/willbonus` hold the misc bonus.
    pub fn recalculate_stats(&mut self, game_data: &GameData) -> Result<(), CharacterError> {
        let bab = self.calculate_bab(game_data);
        self.set_base_attack_bonus(bab);
        Ok(())
    }

    /// Skill points the engine granted at each `LvlStatList` level.
    ///
    /// Class `SkillPointBase` + racial bonus + INT modifier (minimum 1),
    /// quadrupled at level 1. The engine grants points with the INT modifier
    /// in effect at each level-up and never retroactively, so the level-up
    /// ability increases recorded in `LvlStatAbility` are replayed: the
    /// current INT (stored score + racial adjustment) is rolled back by the number of INT increases and
    /// re-applied level by level (an increase counts from the level it was
    /// taken). Entries without a class grant 0.
    pub fn skill_points_granted_per_level(&self, game_data: &GameData) -> Vec<i32> {
        let Some(lvl_stat_list) = self.get_list("LvlStatList") else {
            return Vec::new();
        };

        let race_bonus = self.racial_skill_point_bonus();

        let is_int_increase = |entry: &IndexMap<String, GffValue>| {
            entry.get("LvlStatAbility").and_then(gff_value_to_i32)
                == Some(i32::from(AbilityIndex::INT.0))
        };
        let int_increases = lvl_stat_list.iter().filter(|e| is_int_increase(e)).count() as i32;
        let mut int_score = self.get_effective_abilities(game_data).int - int_increases;

        let classes_table = game_data.get_table("classes");

        lvl_stat_list
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                if is_int_increase(entry) {
                    int_score += 1;
                }
                let int_mod = crate::character::types::calculate_modifier(int_score);

                let Some(class_id) = entry
                    .get("LvlStatClass")
                    .and_then(gff_value_to_i32)
                    .filter(|id| *id != -1)
                else {
                    return 0;
                };

                let class_base = classes_table
                    .as_ref()
                    .and_then(|t| t.get_by_id(class_id))
                    .map_or(2, |c_data| row_int(&c_data, "skillpointbase", 2));

                let points = std::cmp::max(1, class_base + int_mod + race_bonus);
                if idx == 0 { points * 4 } else { points }
            })
            .collect()
    }

    /// Skill points the character earned over their level history, per class.
    pub fn get_theoretical_skill_points(&self, game_data: &GameData) -> HashMap<ClassId, i32> {
        let mut class_points = HashMap::new();
        let Some(lvl_stat_list) = self.get_list("LvlStatList") else {
            return class_points;
        };

        for (entry, points) in lvl_stat_list
            .iter()
            .zip(self.skill_points_granted_per_level(game_data))
        {
            if let Some(class_id) = entry.get("LvlStatClass").and_then(gff_value_to_i32) {
                *class_points.entry(ClassId(class_id)).or_insert(0) += points;
            }
        }

        class_points
    }

    /// Unspent skill points a level-history entry contributed to the pool:
    /// what the level granted minus the cost of the ranks it recorded, costed
    /// against the class taken at that level the way the engine does.
    ///
    /// Removing the level takes exactly this much back from the pool; the
    /// ranks themselves are refunded separately. A level whose recorded ranks
    /// cost more than it granted (injected ranks) contributed nothing, never
    /// a negative amount, so the overdraw must not be credited to the pool.
    /// `LvlStatList.SkillPoints` cannot be used here: it is the pool
    /// remainder after that level-up, not the grant.
    fn unspent_skill_points_of_level(
        &self,
        granted: i32,
        entry: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> i32 {
        let has_able_learner = self.has_feat(FeatId(super::skills::ABLE_LEARNER_FEAT_ID));
        let class_id = ClassId(
            entry
                .get("LvlStatClass")
                .and_then(gff_value_to_i32)
                .unwrap_or(-1),
        );
        let spent: i32 = Self::extract_skill_list(entry)
            .iter()
            .map(|s| {
                self.skill_cost_for_class(
                    class_id,
                    s.skill_id,
                    s.ranks,
                    has_able_learner,
                    game_data,
                )
            })
            .sum();
        (granted - spent).max(0)
    }
}

pub fn get_class_progression(
    class_id: i32,
    max_level: i32,
    game_data: &GameData,
) -> Option<ClassProgression> {
    let classes_table = game_data.get_table("classes")?;
    let class_data = classes_table.get_by_id(class_id)?;

    let class_name = class_data
        .get("name")
        .and_then(|v| v.clone())
        .and_then(|s| s.parse::<i32>().ok())
        .and_then(|strref| game_data.get_string(strref))
        .or_else(|| row_str(&class_data, "label"))
        .unwrap_or_else(|| format!("Class {class_id}"));

    let hit_die = row_int(&class_data, "hitdie", 6);

    let skill_points = row_int(&class_data, "skillpointbase", 2);

    let bab_table_name = row_str(&class_data, "attackbonustable").unwrap_or_default();

    let bab_type = BabType::from_table(&bab_table_name);

    let save_table_name = row_str(&class_data, "savingthrowtable").unwrap_or_default();

    let is_spellcaster = row_bool(&class_data, "spellcaster", false);

    let arcane_flag = row_bool(&class_data, "hasarcane", false);

    let spell_type = if !is_spellcaster {
        "none".to_string()
    } else if arcane_flag {
        "arcane".to_string()
    } else {
        "divine".to_string()
    };

    let spell_table_name = row_str(&class_data, "spellgaintable");

    let mut level_progression = Vec::with_capacity(max_level as usize);

    let bab_table = game_data.get_table(&bab_table_name.to_lowercase());
    let save_table = game_data.get_table(&save_table_name.to_lowercase());
    let spell_table = spell_table_name
        .as_ref()
        .and_then(|name| game_data.get_table(&name.to_lowercase()));

    for level in 1..=max_level {
        let row_idx = (level - 1) as usize;

        let base_attack_bonus = bab_table
            .as_ref()
            .and_then(|t| t.get_cell(row_idx, "BAB").ok().flatten())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or_else(|| bab_type.fallback_bab(level));

        let (fortitude_save, reflex_save, will_save) = save_table.as_ref().map_or((0, 0, 0), |t| {
            let fort = t
                .get_cell(row_idx, "FortSave")
                .ok()
                .flatten()
                .or_else(|| t.get_cell(row_idx, "Fort").ok().flatten())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            let ref_save = t
                .get_cell(row_idx, "RefSave")
                .ok()
                .flatten()
                .or_else(|| t.get_cell(row_idx, "Ref").ok().flatten())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            let will = t
                .get_cell(row_idx, "WillSave")
                .ok()
                .flatten()
                .or_else(|| t.get_cell(row_idx, "Will").ok().flatten())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            (fort, ref_save, will)
        });

        let spell_slots = if is_spellcaster {
            spell_table.as_ref().map(|t| SpellSlots {
                level_0: t
                    .get_cell(row_idx, "SpellLevel0")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_1: t
                    .get_cell(row_idx, "SpellLevel1")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_2: t
                    .get_cell(row_idx, "SpellLevel2")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_3: t
                    .get_cell(row_idx, "SpellLevel3")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_4: t
                    .get_cell(row_idx, "SpellLevel4")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_5: t
                    .get_cell(row_idx, "SpellLevel5")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_6: t
                    .get_cell(row_idx, "SpellLevel6")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_7: t
                    .get_cell(row_idx, "SpellLevel7")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_8: t
                    .get_cell(row_idx, "SpellLevel8")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
                level_9: t
                    .get_cell(row_idx, "SpellLevel9")
                    .ok()
                    .flatten()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0),
            })
        } else {
            None
        };

        level_progression.push(LevelProgressionEntry {
            level,
            base_attack_bonus,
            fortitude_save,
            reflex_save,
            will_save,
            features: Vec::new(),
            spell_slots,
        });
    }

    Some(ClassProgression {
        class_id,
        class_name,
        basic_info: ClassBasicInfo {
            hit_die,
            skill_points_per_level: skill_points,
            bab_progression: bab_type.progression_name().to_string(),
            save_progression: save_table_name,
            is_spellcaster,
            spell_type,
        },
        level_progression,
        max_level_shown: max_level,
    })
}

/// Levels above this are epic: the engine writes `EpicLevel = 1` and grants Epic Character.
pub(crate) const EPIC_LEVEL_THRESHOLD: i32 = 20;

/// `FEAT_EPIC_CHARACTER`: granted by the engine itself on reaching level 21 (no class feat
/// table lists it; verified against an in-game level-up 20 -> 21, 2026-09-26).
const FEAT_EPIC_CHARACTER: FeatId = FeatId(1001);

/// `cls_feat_*` `List` value of feats the class grants on `GrantedOnLevel`; the other
/// lists only make the feat selectable from that level on.
const AUTOMATIC_FEAT_LIST: i32 = 3;

/// After levels were taken out of the middle of the history, moves ability increases that
/// now sit on a level without one onto the free levels that grant one, in order. Increases
/// already on such a level stay. Returns the increases no level is left for.
fn realign_ability_increases(lvl_stat_list: &mut [IndexMap<String, GffValue<'static>>]) -> Vec<u8> {
    use crate::character::types::ABILITY_INCREASE_INTERVAL;

    let grants_increase = |idx: usize| (idx as i32 + 1) % ABILITY_INCREASE_INTERVAL == 0;
    let increase_at = |entry: &IndexMap<String, GffValue<'static>>| {
        entry
            .get("LvlStatAbility")
            .and_then(gff_value_to_i32)
            .filter(|ability| (0..=5).contains(ability))
    };

    let mut misplaced = Vec::new();
    for (idx, entry) in lvl_stat_list.iter_mut().enumerate() {
        if !grants_increase(idx)
            && let Some(ability) = increase_at(entry)
        {
            misplaced.push(ability as u8);
            entry.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
        }
    }

    let mut pending = misplaced.into_iter();
    for (idx, entry) in lvl_stat_list.iter_mut().enumerate() {
        if grants_increase(idx)
            && increase_at(entry).is_none()
            && let Some(ability) = pending.next()
        {
            entry.insert("LvlStatAbility".to_string(), GffValue::Byte(ability));
        }
    }
    pending.collect()
}

fn remove_feats_from_level_history(
    lvl_stat_list: &mut [IndexMap<String, GffValue<'static>>],
    feats: &HashSet<i32>,
) {
    for stat_entry in lvl_stat_list {
        if let Some(mut feat_list) =
            super::gff_helpers::extract_list_from_map(stat_entry, "FeatList")
        {
            feat_list.retain(|f| {
                let feat_id = f.get("Feat").and_then(gff_value_to_i32).unwrap_or(-1);
                !feats.contains(&feat_id)
            });
            stat_entry.insert("FeatList".to_string(), GffValue::ListOwned(feat_list));
        }
    }
}

impl Character {
    /// Swap levels of one class for another.
    ///
    /// This is a destructive operation that:
    /// 1. Removes ALL levels of the old class (reverting all gains).
    /// 2. Adds the new class at level 1.
    /// 3. Returns the new class ID and level (which will be 1).
    ///
    /// Protected feats (racial, background, domain) are preserved.
    pub fn swap_class(
        &mut self,
        old_class_id: ClassId,
        new_class_id: ClassId,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let current_level = self.class_level(old_class_id);
        if current_level == 0 {
            return Err(CharacterError::NotFound {
                entity: "Class",
                id: old_class_id.0,
            });
        }

        self.transactional(|c| {
            for _ in 0..current_level {
                c.level_down_internal(old_class_id, game_data)?;
            }
            c.level_up(new_class_id, game_data)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restriction(bits: i32, axes: i32, invert: bool) -> AlignmentRestriction {
        AlignmentRestriction { bits, axes, invert }
    }

    const LG: Alignment = Alignment {
        law_chaos: 85,
        good_evil: 85,
    };
    const LE: Alignment = Alignment {
        law_chaos: 85,
        good_evil: 15,
    };
    const NN: Alignment = Alignment {
        law_chaos: 50,
        good_evil: 50,
    };
    const NE: Alignment = Alignment {
        law_chaos: 50,
        good_evil: 15,
    };
    const CG: Alignment = Alignment {
        law_chaos: 15,
        good_evil: 85,
    };
    const CN: Alignment = Alignment {
        law_chaos: 15,
        good_evil: 50,
    };

    fn level_entry(ability: Option<u8>, feats: &[u16]) -> IndexMap<String, GffValue<'static>> {
        let mut entry = IndexMap::new();
        entry.insert(
            "LvlStatAbility".to_string(),
            GffValue::Byte(ability.unwrap_or(255)),
        );
        let feats = feats
            .iter()
            .map(|id| IndexMap::from([("Feat".to_string(), GffValue::Word(*id))]))
            .collect();
        entry.insert("FeatList".to_string(), GffValue::ListOwned(feats));
        entry
    }

    fn recorded_increases(lvl_stat_list: &[IndexMap<String, GffValue<'static>>]) -> Vec<i32> {
        lvl_stat_list
            .iter()
            .map(|e| e.get("LvlStatAbility").and_then(gff_value_to_i32).unwrap())
            .collect()
    }

    #[test]
    fn realign_moves_misplaced_increases_onto_increase_levels() {
        // Fighter 2 / Wizard 6 with STR at 4 and INT at 8, Fighter removed: the six Wizard
        // levels hold the increases at levels 2 and 6, but only level 4 grants one.
        let mut history: Vec<_> = (0..6).map(|_| level_entry(None, &[])).collect();
        history[1] = level_entry(Some(0), &[]);
        history[5] = level_entry(Some(3), &[]);

        let excess = realign_ability_increases(&mut history);

        assert_eq!(
            recorded_increases(&history),
            vec![255, 255, 255, 0, 255, 255]
        );
        assert_eq!(excess, vec![3]);
    }

    #[test]
    fn realign_keeps_increases_already_on_increase_levels() {
        let mut history: Vec<_> = (0..8).map(|_| level_entry(None, &[])).collect();
        history[7] = level_entry(Some(3), &[]);

        let excess = realign_ability_increases(&mut history);

        assert_eq!(recorded_increases(&history)[3], 255);
        assert_eq!(recorded_increases(&history)[7], 3);
        assert!(excess.is_empty());
    }

    #[test]
    fn record_feat_change_removes_feat_from_every_level() {
        let mut fields = IndexMap::new();
        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(vec![
                level_entry(None, &[8]),
                level_entry(None, &[44, 8]),
                level_entry(None, &[]),
            ]),
        );
        let mut character = Character::from_gff(fields);

        character.record_feat_change(FeatId(8), false);

        let history = character.get_list_owned("LvlStatList").unwrap();
        let feats: Vec<Vec<i32>> = history
            .iter()
            .map(|e| {
                Character::extract_feat_list(e, "FeatList")
                    .iter()
                    .map(|f| f.0)
                    .collect()
            })
            .collect();
        assert_eq!(feats, vec![vec![], vec![44], vec![]]);
    }

    #[test]
    fn alignment_restriction_follows_classes_2da_bits() {
        let blackguard = restriction(0x09, 0x2, false);
        assert!(blackguard.check_alignment(&LE) && blackguard.check_alignment(&NE));
        assert!(!blackguard.check_alignment(&LG) && !blackguard.check_alignment(&NN));
        assert_eq!(blackguard.decode_to_string().as_deref(), Some("Evil"));

        let harper = restriction(0x10, 0x2, false);
        assert!(harper.check_alignment(&NN) && !harper.check_alignment(&NE));

        let dwarven_defender = restriction(0x05, 0x1, false);
        assert!(dwarven_defender.check_alignment(&LE) && !dwarven_defender.check_alignment(&NN));
        assert_eq!(
            dwarven_defender.decode_to_string().as_deref(),
            Some("Lawful")
        );

        let arcane_trickster = restriction(0x02, 0x1, false);
        assert!(arcane_trickster.check_alignment(&NN) && !arcane_trickster.check_alignment(&LG));

        let paladin = restriction(0x15, 0x3, false);
        assert!(paladin.check_alignment(&LG));
        assert!(!paladin.check_alignment(&LE) && !paladin.check_alignment(&CG));
        assert_eq!(paladin.decode_to_string().as_deref(), Some("Lawful Good"));

        let druid = restriction(0x01, 0x3, true);
        assert!(druid.check_alignment(&NE) && druid.check_alignment(&CN));
        assert!(!druid.check_alignment(&LG));

        let warlock = restriction(0x14, 0x3, true);
        assert!(warlock.check_alignment(&CG) && warlock.check_alignment(&LE));
        assert!(!warlock.check_alignment(&LG) && !warlock.check_alignment(&NN));
        assert_eq!(
            warlock.decode_to_string().as_deref(),
            Some("Chaotic or Evil")
        );

        assert!(restriction(0x00, 0x1, false).check_alignment(&LG));
        assert!(restriction(0x00, 0x1, false).decode_to_string().is_none());
    }

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();

        let mut class1 = IndexMap::new();
        class1.insert("Class".to_string(), GffValue::Byte(0));
        class1.insert("ClassLevel".to_string(), GffValue::Short(5));

        let mut class2 = IndexMap::new();
        class2.insert("Class".to_string(), GffValue::Byte(3));
        class2.insert("ClassLevel".to_string(), GffValue::Short(3));

        fields.insert(
            "ClassList".to_string(),
            GffValue::ListOwned(vec![class1, class2]),
        );

        // Add LvlStatList for proper level_down testing
        let mut lvl_stat_list = Vec::new();
        for i in 0..8 {
            let mut entry = IndexMap::new();
            entry.insert(
                "LvlStatClass".to_string(),
                GffValue::Byte(if i < 5 { 0 } else { 3 }),
            );
            entry.insert("LvlStatHitDie".to_string(), GffValue::Byte(6));
            entry.insert("SkillPoints".to_string(), GffValue::Short(4));
            entry.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
            entry.insert("FeatList".to_string(), GffValue::ListOwned(vec![]));
            entry.insert("SkillList".to_string(), GffValue::ListOwned(vec![]));
            lvl_stat_list.push(entry);
        }
        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(lvl_stat_list),
        );

        // Add required ability scores for level_down
        fields.insert("Str".to_string(), GffValue::Byte(14));
        fields.insert("Dex".to_string(), GffValue::Byte(12));
        fields.insert("Con".to_string(), GffValue::Byte(14));
        fields.insert("Int".to_string(), GffValue::Byte(10));
        fields.insert("Wis".to_string(), GffValue::Byte(10));
        fields.insert("Cha".to_string(), GffValue::Byte(10));

        // Add HP fields
        fields.insert("MaxHitPoints".to_string(), GffValue::Short(60));
        fields.insert("CurrentHitPoints".to_string(), GffValue::Short(60));
        fields.insert("HitPoints".to_string(), GffValue::Short(60));

        // Add skill points
        fields.insert("SkillPoints".to_string(), GffValue::Short(0));

        Character::from_gff(fields)
    }

    fn create_mock_game_data() -> GameData {
        use crate::parsers::tlk::TLKParser;
        use std::sync::Arc;
        GameData::new(Arc::new(std::sync::RwLock::new(TLKParser::default())))
    }

    #[test]
    fn test_total_level() {
        let character = create_test_character();
        assert_eq!(character.total_level(), 8);
    }

    #[test]
    fn test_class_entries() {
        let character = create_test_character();
        let entries = character.class_entries();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].class_id.0, 0);
        assert_eq!(entries[0].level, 5);
        assert_eq!(entries[1].class_id.0, 3);
        assert_eq!(entries[1].level, 3);
    }

    #[test]
    fn test_class_level() {
        let character = create_test_character();
        assert_eq!(character.class_level(ClassId(0)), 5);
        assert_eq!(character.class_level(ClassId(3)), 3);
        assert_eq!(character.class_level(ClassId(99)), 0);
    }

    #[test]
    fn test_class_count() {
        let character = create_test_character();
        assert_eq!(character.class_count(), 2);
    }

    #[test]
    fn test_has_class() {
        let character = create_test_character();
        assert!(character.has_class(ClassId(0)));
        assert!(character.has_class(ClassId(3)));
        assert!(!character.has_class(ClassId(99)));
    }

    #[test]
    fn test_set_class_level() {
        let mut character = create_test_character();

        character.set_class_level(ClassId(0), 10).unwrap();
        assert_eq!(character.class_level(ClassId(0)), 10);

        character.set_class_level(ClassId(3), 7).unwrap();
        assert_eq!(character.class_level(ClassId(3)), 7);
    }

    #[test]
    fn test_set_class_level_validation() {
        let mut character = create_test_character();

        let result = character.set_class_level(ClassId(0), 0);
        assert!(result.is_err());

        let result = character.set_class_level(ClassId(99), 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_class_entry() {
        let mut character = create_test_character();

        character.add_class_entry(ClassId(5), 2).unwrap();
        assert_eq!(character.class_count(), 3);
        assert_eq!(character.class_level(ClassId(5)), 2);
    }

    #[test]
    fn test_add_class_entry_duplicate() {
        let mut character = create_test_character();

        let result = character.add_class_entry(ClassId(0), 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_class_entry_max_classes() {
        let mut character = create_test_character();

        character.add_class_entry(ClassId(5), 1).unwrap();
        assert_eq!(character.class_count(), 3);

        character.add_class_entry(ClassId(7), 1).unwrap();
        assert_eq!(character.class_count(), 4);

        let result = character.add_class_entry(ClassId(9), 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_class() {
        let mut character = create_test_character();
        let game_data = create_mock_game_data();

        character.remove_class(ClassId(3), &game_data).unwrap();
        assert_eq!(character.class_count(), 1);
        assert!(!character.has_class(ClassId(3)));
        assert!(character.has_class(ClassId(0)));
    }

    #[test]
    fn test_remove_class_clamps_mclass_levupin() {
        let mut character = create_test_character();
        character.set_byte("MClassLevUpIn", 1);
        let game_data = create_mock_game_data();

        character.remove_class(ClassId(3), &game_data).unwrap();

        assert_eq!(character.class_count(), 1);
        assert_eq!(character.get_i32("MClassLevUpIn"), Some(0));
    }

    #[test]
    fn test_level_down_to_zero_clamps_mclass_levupin() {
        // Same fix path via level_down_internal: dropping a class from
        // level 1 to 0 removes its ClassList entry.
        let mut character = create_test_character();
        character.set_byte("MClassLevUpIn", 1);
        let game_data = create_mock_game_data();

        character.level_down(ClassId(3), &game_data).unwrap();
        character.level_down(ClassId(3), &game_data).unwrap();
        character.level_down(ClassId(3), &game_data).unwrap();

        assert_eq!(character.class_count(), 1);
        assert_eq!(character.get_i32("MClassLevUpIn"), Some(0));
    }

    #[test]
    fn test_remove_class_preserves_mclass_levupin_when_unaffected() {
        let mut character = create_test_character();
        character.set_byte("MClassLevUpIn", 0);
        let game_data = create_mock_game_data();

        character.remove_class(ClassId(3), &game_data).unwrap();

        assert_eq!(character.get_i32("MClassLevUpIn"), Some(0));
    }

    #[test]
    fn test_remove_last_class() {
        let mut fields = IndexMap::new();
        let mut class1 = IndexMap::new();
        class1.insert("Class".to_string(), GffValue::Byte(0));
        class1.insert("ClassLevel".to_string(), GffValue::Short(5));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class1]));

        let mut character = Character::from_gff(fields);
        let game_data = create_mock_game_data();

        let result = character.remove_class(ClassId(0), &game_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_nonexistent_class() {
        let mut character = create_test_character();
        let game_data = create_mock_game_data();

        let result = character.remove_class(ClassId(99), &game_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_class_list() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);

        assert_eq!(character.total_level(), 0);
        assert_eq!(character.class_count(), 0);
        assert_eq!(character.class_entries().len(), 0);
        assert!(!character.has_class(ClassId(0)));
    }

    #[test]
    fn test_level_history() {
        let mut fields = IndexMap::new();

        let mut class1 = IndexMap::new();
        class1.insert("Class".to_string(), GffValue::Byte(0));
        class1.insert("ClassLevel".to_string(), GffValue::Short(5));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class1]));

        let mut lvl1 = IndexMap::new();
        lvl1.insert("LvlStatClass".to_string(), GffValue::Byte(0));
        lvl1.insert("LvlStatHitDie".to_string(), GffValue::Byte(10));
        lvl1.insert("SkillPoints".to_string(), GffValue::Short(0));
        lvl1.insert("LvlStatAbility".to_string(), GffValue::Byte(255));

        let mut lvl2 = IndexMap::new();
        lvl2.insert("LvlStatClass".to_string(), GffValue::Byte(0));
        lvl2.insert("LvlStatHitDie".to_string(), GffValue::Byte(6));
        lvl2.insert("SkillPoints".to_string(), GffValue::Short(0));
        lvl2.insert("LvlStatAbility".to_string(), GffValue::Byte(0));

        let mut feat1 = IndexMap::new();
        feat1.insert("Feat".to_string(), GffValue::Word(1));
        lvl2.insert("FeatList".to_string(), GffValue::ListOwned(vec![feat1]));

        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(vec![lvl1, lvl2]),
        );

        let character = Character::from_gff(fields);
        let history = character.level_history();

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].character_level, 1);
        assert_eq!(history[0].class_id.0, 0);
        assert_eq!(history[0].class_level, 1);
        assert_eq!(history[0].hp_gained, 10);
        assert!(history[0].ability_increase.is_none());
        assert_eq!(history[0].feats_gained.len(), 0);

        assert_eq!(history[1].character_level, 2);
        assert_eq!(history[1].class_id.0, 0);
        assert_eq!(history[1].class_level, 2);
        assert_eq!(history[1].hp_gained, 6);
        assert_eq!(history[1].ability_increase, Some(AbilityIndex::STR));
        assert_eq!(history[1].feats_gained.len(), 1);
        assert_eq!(history[1].feats_gained[0].0, 1);
    }

    #[test]
    fn test_level_history_empty() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);

        let history = character.level_history();
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_level_up_missing_table() {
        let mut character = create_test_character();
        let game_data = GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        )));

        // classes table missing in game_data
        let result = character.level_up(ClassId(0), &game_data);
        assert!(result.is_err());
        match result.unwrap_err() {
            CharacterError::TableNotFound(table) => assert_eq!(table, "classes"),
            _ => panic!("Expected TableNotFound error"),
        }
    }

    fn insert_mock_table(game_data: &mut GameData, name: &str, content: &str) {
        use crate::loaders::LoadedTable;
        use crate::parsers::tda::TDAParser;
        use std::sync::Arc;

        let mut parser = TDAParser::new();
        parser.parse_from_bytes(content.as_bytes()).unwrap();
        game_data.tables.insert(
            name.to_string(),
            LoadedTable::new(name.to_string(), Arc::new(parser)),
        );
    }

    fn create_game_data_with_prestige_classes() -> GameData {
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let mut game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));

        // Row 1: prestige requiring BAB +6; row 2: no requirements;
        // row 3: requires casting level 3 arcane spells; row 4: requires one of feats 10/11;
        // row 5: arcane caster base class
        let classes_content = "2DA V2.0

   Label          IsPrestige   MinAttackBonus   MaxLevel   HitDie   SkillPointBase   PreReqTable    HasArcane   SpellCaster   SpellGainTable
0  Fighter        0            ****             ****       10       2                ****           0           0             ****
1  HardPrestige   1            6                10         8        2                ****           0           0             ****
2  FreePrestige   1            ****             10         8        2                ****           0           0             ****
3  SpellPrestige  1            ****             10         6        2                CLS_PRES_ARC   0           0             ****
4  FeatPrestige   1            ****             10         8        2                CLS_PRES_OR    0           0             ****
5  Wizard         0            ****             ****       4        2                ****           1           1             CLS_SPGN_TWIZ
";
        insert_mock_table(&mut game_data, "classes", classes_content);

        insert_mock_table(
            &mut game_data,
            "cls_pres_arc",
            "2DA V2.0

   LABEL   ReqType    ReqParam1   ReqParam2
0  arc     ARCSPELL   3           ****
",
        );

        insert_mock_table(
            &mut game_data,
            "cls_pres_or",
            "2DA V2.0

   LABEL   ReqType   ReqParam1   ReqParam2
0  f1      FEATOR    10          ****
1  f2      FEATOR    11          ****
",
        );

        insert_mock_table(
            &mut game_data,
            "cls_spgn_twiz",
            "2DA V2.0

   Level   SpellLevel0   SpellLevel1   SpellLevel2   SpellLevel3
0  1       3             1             ****          ****
1  2       4             2             ****          ****
2  3       4             2             1             ****
3  4       4             3             2             ****
4  5       4             3             2             1
",
        );

        game_data
    }

    #[test]
    fn test_level_up_prestige_blocked_when_requirements_unmet() {
        // Test character is level 8 with no attack bonus tables loaded, so
        // calculate_bab falls back to level/2 per class = +3, below the +6 requirement.
        let mut character = create_test_character();
        let game_data = create_game_data_with_prestige_classes();

        let result = character.level_up(ClassId(1), &game_data);

        match result.unwrap_err() {
            CharacterError::ValidationFailed { field, message } => {
                assert_eq!(field, "PrestigeRequirements");
                assert!(message.contains("Base Attack Bonus"), "message: {message}");
            }
            other => panic!("Expected ValidationFailed, got {other:?}"),
        }
        assert!(!character.has_class(ClassId(1)));
        assert_eq!(character.total_level(), 8);
    }

    #[test]
    fn test_level_up_prestige_allowed_when_requirements_met() {
        let mut character = create_test_character();
        let game_data = create_game_data_with_prestige_classes();

        character.level_up(ClassId(2), &game_data).unwrap();

        assert_eq!(character.class_level(ClassId(2)), 1);
        assert_eq!(character.total_level(), 9);
    }

    #[test]
    fn test_prestige_feator_group_blocks_when_no_feat_owned() {
        let character = create_test_character();
        let game_data = create_game_data_with_prestige_classes();

        let validation = character.validate_prestige_class_requirements(ClassId(4), &game_data);

        assert!(!validation.can_take);
        assert!(!validation.missing_requirements.is_empty());
    }

    #[test]
    fn test_prestige_feator_group_met_with_one_feat() {
        let mut character = create_test_character();
        character.add_feat(FeatId(11)).unwrap();
        let game_data = create_game_data_with_prestige_classes();

        let validation = character.validate_prestige_class_requirements(ClassId(4), &game_data);

        assert!(
            validation.can_take,
            "missing: {:?}",
            validation.missing_requirements
        );
    }

    #[test]
    fn test_prestige_arcane_spell_requirement_blocks_noncaster() {
        let character = create_test_character();
        let game_data = create_game_data_with_prestige_classes();

        let validation = character.validate_prestige_class_requirements(ClassId(3), &game_data);

        assert!(!validation.can_take);
        assert!(
            validation
                .missing_requirements
                .iter()
                .any(|m| m.contains("arcane")),
            "missing: {:?}",
            validation.missing_requirements
        );
    }

    #[test]
    fn test_prestige_arcane_spell_requirement_met_by_caster() {
        let mut fields = IndexMap::new();
        let mut wizard = IndexMap::new();
        wizard.insert("Class".to_string(), GffValue::Byte(5));
        wizard.insert("ClassLevel".to_string(), GffValue::Short(5));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![wizard]));
        let character = Character::from_gff(fields);
        let game_data = create_game_data_with_prestige_classes();

        let validation = character.validate_prestige_class_requirements(ClassId(3), &game_data);

        assert!(
            validation.can_take,
            "missing: {:?}",
            validation.missing_requirements
        );
    }

    #[test]
    fn test_level_up_existing_prestige_class_skips_validation() {
        // A character that already legitimately has the prestige class must be
        // able to keep leveling it even if our recomputed requirements fail.
        let mut character = create_test_character();
        character.add_class_entry(ClassId(1), 1).unwrap();
        let game_data = create_game_data_with_prestige_classes();

        character.level_up(ClassId(1), &game_data).unwrap();

        assert_eq!(character.class_level(ClassId(1)), 2);
    }

    #[test]
    fn test_level_up_stores_die_roll_without_con_in_history() {
        // LvlStatHitDie must hold only the hit-die roll (game convention);
        // CON is applied dynamically by the engine and by our revert logic.
        let mut character = create_test_character(); // CON 14 -> +2
        let game_data = create_game_data_with_prestige_classes(); // Fighter d10

        character.level_up(ClassId(0), &game_data).unwrap();

        let history = character.level_history();
        let last = history.last().unwrap();
        assert_eq!(
            last.hp_gained, 10,
            "LvlStatHitDie must store the die roll only, not roll + CON"
        );
    }

    fn set_last_level_ranks(character: &mut Character, ranks: &[(usize, u8)]) {
        let mut history = character.get_list_owned("LvlStatList").unwrap();
        let entry = history.last_mut().unwrap();
        let mut list: Vec<IndexMap<String, GffValue>> = Vec::new();
        for (skill_idx, rank) in ranks {
            while list.len() <= *skill_idx {
                let mut skill = IndexMap::new();
                skill.insert("Rank".to_string(), GffValue::Byte(0));
                list.push(skill);
            }
            list[*skill_idx].insert("Rank".to_string(), GffValue::Byte(*rank));
        }
        entry.insert("SkillList".to_string(), GffValue::ListOwned(list));
        character.set_list("LvlStatList", history);
    }

    /// `LvlStatList.SkillPoints` is the pool remainder after each level-up;
    /// the history must report it as stored, not as a running sum.
    #[test]
    fn level_history_reports_stored_pool_remainder() {
        let character = create_test_character();
        let remaining: Vec<i32> = character
            .level_history()
            .iter()
            .map(|e| e.skill_points_remaining)
            .collect();
        assert_eq!(remaining, vec![4; 8]);
    }

    /// Fighter (SkillPointBase 2), INT 10, no racial bonus: 8 at level 1, then
    /// 2 per level. `FEAT_SKILLED` adds 1 per level (4 at level 1).
    #[test]
    fn skill_points_granted_per_level_applies_skilled_feat() {
        let mut character = create_test_character();
        let game_data = create_game_data_with_prestige_classes();

        assert_eq!(
            character.skill_points_granted_per_level(&game_data),
            vec![8, 2, 2, 2, 2, 2, 2, 2]
        );

        let mut feat = IndexMap::new();
        feat.insert(
            "Feat".to_string(),
            GffValue::Word(super::super::skills::SKILLED_FEAT_ID as u16),
        );
        character.set_list("FeatList", vec![feat]);
        assert_eq!(
            character.skill_points_granted_per_level(&game_data),
            vec![12, 3, 3, 3, 3, 3, 3, 3]
        );
    }

    /// A level whose 2 granted points were fully spent (1 cross-class rank
    /// costs 2) contributed nothing to the pool, so removing it leaves the
    /// pool alone; an unspent level takes its 2 points back.
    #[test]
    fn level_down_takes_back_only_the_unspent_points_of_that_level() {
        let game_data = create_game_data_with_prestige_classes();

        let mut spent = create_test_character();
        spent.set_available_skill_points(10);
        set_last_level_ranks(&mut spent, &[(0, 1)]);
        spent.level_down(ClassId(3), &game_data).unwrap();
        assert_eq!(spent.get_available_skill_points(), 10);
        assert_eq!(spent.skill_rank(SkillId(0)), 0, "rank refunded");

        let mut banked = create_test_character();
        banked.set_available_skill_points(10);
        banked.level_down(ClassId(3), &game_data).unwrap();
        assert_eq!(banked.get_available_skill_points(), 8);
    }

    /// A level that recorded more rank cost than it granted (injected ranks)
    /// must not credit the overdraw to the pool when removed.
    #[test]
    fn level_down_never_credits_overdrawn_ranks_to_the_pool() {
        let game_data = create_game_data_with_prestige_classes();
        let mut character = create_test_character();
        character.set_available_skill_points(10);
        set_last_level_ranks(&mut character, &[(0, 5), (1, 5)]);

        character.level_down(ClassId(3), &game_data).unwrap();

        assert_eq!(character.get_available_skill_points(), 10);
    }

    /// Removing a class takes back the unspent points of every one of its
    /// levels: three unspent levels at 2 points each.
    #[test]
    fn remove_class_takes_back_unspent_points_of_all_its_levels() {
        let game_data = create_game_data_with_prestige_classes();
        let mut character = create_test_character();
        character.set_available_skill_points(10);
        set_last_level_ranks(&mut character, &[(0, 1)]);

        character.remove_class(ClassId(3), &game_data).unwrap();

        assert_eq!(character.get_available_skill_points(), 10 - 2 - 2);
    }

    /// The history entry written by level_up carries the pool remainder, the
    /// way the game writes it, so a later level_down reads it consistently.
    #[test]
    fn level_up_records_pool_remainder_in_history() {
        let game_data = create_game_data_with_prestige_classes();
        let mut character = create_test_character();
        character.set_available_skill_points(5);

        character.level_up(ClassId(0), &game_data).unwrap();

        assert_eq!(character.get_available_skill_points(), 7);
        let last = character.level_history().pop().unwrap();
        assert_eq!(last.skill_points_remaining, 7);

        character.level_down(ClassId(0), &game_data).unwrap();
        assert_eq!(character.get_available_skill_points(), 5);
    }

    #[test]
    fn test_level_up_then_level_down_restores_max_hp() {
        let mut character = create_test_character(); // CON 14 -> +2, 60 max HP
        let game_data = create_game_data_with_prestige_classes(); // Fighter d10
        let max_hp_before = character.max_hp();

        character.level_up(ClassId(0), &game_data).unwrap();
        assert_eq!(character.max_hp(), max_hp_before + 12); // d10 + 2 CON

        character.level_down(ClassId(0), &game_data).unwrap();
        assert_eq!(
            character.max_hp(),
            max_hp_before,
            "level_down must subtract exactly what level_up added"
        );
    }

    fn create_character_with_history() -> Character {
        let mut fields = IndexMap::new();

        // Class list: Fighter(0) level 3
        let mut class1 = IndexMap::new();
        class1.insert("Class".to_string(), GffValue::Byte(0));
        class1.insert("ClassLevel".to_string(), GffValue::Short(3));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class1]));

        // Level history
        let mut lvl1 = IndexMap::new();
        lvl1.insert("LvlStatClass".to_string(), GffValue::Byte(0));
        lvl1.insert("LvlStatHitDie".to_string(), GffValue::Byte(10));
        lvl1.insert("SkillPoints".to_string(), GffValue::Short(8));
        lvl1.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
        lvl1.insert("FeatList".to_string(), GffValue::ListOwned(vec![]));
        lvl1.insert("SkillList".to_string(), GffValue::ListOwned(vec![]));

        let mut lvl2 = IndexMap::new();
        lvl2.insert("LvlStatClass".to_string(), GffValue::Byte(0));
        lvl2.insert("LvlStatHitDie".to_string(), GffValue::Byte(6));
        lvl2.insert("SkillPoints".to_string(), GffValue::Short(2));
        lvl2.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
        lvl2.insert("FeatList".to_string(), GffValue::ListOwned(vec![]));
        lvl2.insert("SkillList".to_string(), GffValue::ListOwned(vec![]));

        let mut lvl3 = IndexMap::new();
        lvl3.insert("LvlStatClass".to_string(), GffValue::Byte(0));
        lvl3.insert("LvlStatHitDie".to_string(), GffValue::Byte(6));
        lvl3.insert("SkillPoints".to_string(), GffValue::Short(2));
        lvl3.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
        lvl3.insert("FeatList".to_string(), GffValue::ListOwned(vec![]));
        lvl3.insert("SkillList".to_string(), GffValue::ListOwned(vec![]));

        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(vec![lvl1, lvl2, lvl3]),
        );

        // Abilities for CON modifier calculation
        fields.insert("Con".to_string(), GffValue::Byte(14)); // +2 modifier

        // HP fields
        fields.insert("HitPoints".to_string(), GffValue::Int(28));
        fields.insert("MaxHitPoints".to_string(), GffValue::Int(28));
        fields.insert("CurrentHitPoints".to_string(), GffValue::Int(28));

        Character::from_gff(fields)
    }

    #[test]
    fn test_level_up_does_not_write_root_class_field() {
        let mut character = create_character_with_history();
        let game_data = create_game_data_with_prestige_classes();
        assert_eq!(character.class_count(), 1);

        character.level_up(ClassId(0), &game_data).unwrap();

        assert!(
            !character.has_field("Class"),
            "root-level Class is not part of the game's creature schema"
        );
    }

    #[test]
    fn test_swap_class_nonexistent_old_class() {
        let mut character = create_character_with_history();
        let game_data = GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        )));

        // Trying to swap a class that doesn't exist
        let result = character.swap_class(ClassId(99), ClassId(1), &game_data);
        assert!(result.is_err());
        match result.unwrap_err() {
            CharacterError::NotFound { entity, id } => {
                assert_eq!(entity, "Class");
                assert_eq!(id, 99);
            }
            _ => panic!("Expected NotFound error"),
        }
    }
}
