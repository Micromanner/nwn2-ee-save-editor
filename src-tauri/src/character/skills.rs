//! Character skill-related methods.
//!
//! Provides access to character skill information including:
//! - Skill ranks and totals
//! - Setting skill ranks
//! - Total skill points spent
//! - Class skill detection
//! - Skill modifiers and bonuses
//! - Max ranks calculation
//!
//! All methods are sync (no async). SkillList structure in GFF:
//! - SkillList: List of structs
//!   - Rank: Byte (skill rank, i8 range: -128 to 127)
//! - Index in list corresponds to skill ID from skills.2da

use std::collections::HashMap;

use super::{Character, CharacterError};
use crate::character::gff_helpers::gff_value_to_i32;
use crate::character::types::{AbilityIndex, AbilityModifiers, SkillId};
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::services::item_property_decoder::ItemPropertyDecoder;
use crate::utils::parsing::{row_bool, row_int};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::warn;

const MAX_SKILL_RANK: i32 = 127;
pub const ABLE_LEARNER_FEAT_ID: i32 = 406;
/// `FEAT_SKILLED` from `race_feat_human.2da`: the engine grants +1 skill point
/// per level (x4 at level 1) to whoever has it. NWN2 has no race skill-point
/// column; this feat is the only source of the human bonus.
pub const SKILLED_FEAT_ID: i32 = 1773;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillPointsSummary {
    pub total_points: i32,
    pub available_points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SkillSummaryEntry {
    pub skill_id: SkillId,
    pub name: String,
    pub ranks: i32,
    pub max_ranks: i32,
    pub modifier: i32,
    pub total: i32,
    pub ability: String,
    pub is_class_skill: bool,
    pub untrained: bool,
    pub armor_check_penalty: bool,
    pub feat_bonus: i32,
    pub item_bonus: i32,
}

impl Character {
    /// Check if a skill is a class skill for this character.
    ///
    /// True when any of the character's classes lists it as a class skill.
    /// Note the engine costs a rank against the class being levelled
    /// (`is_class_skill_for`), not this union.
    pub fn is_class_skill(&self, skill_id: SkillId, game_data: &GameData) -> bool {
        self.class_entries()
            .iter()
            .any(|entry| self.is_class_skill_for(entry.class_id, skill_id, game_data))
    }

    /// Check if a skill is a class skill of one class (`classes.SkillsTable`).
    pub fn is_class_skill_for(
        &self,
        class_id: super::ClassId,
        skill_id: SkillId,
        game_data: &GameData,
    ) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };
        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        let skills_table_name = class_data
            .get("skillstable")
            .and_then(|opt| opt.as_deref())
            .unwrap_or("");
        if skills_table_name.is_empty() {
            return false;
        }

        let Some(class_skills_table) = game_data.get_table(&skills_table_name.to_lowercase())
        else {
            return false;
        };

        // The table is a list of skills, not indexed by SkillId.
        (0..class_skills_table.row_count())
            .filter_map(|i| class_skills_table.get_by_id(i as i32))
            .find(|row| {
                row.get("skillindex")
                    .and_then(|v| v.as_deref())
                    .and_then(|v| v.parse::<i32>().ok())
                    == Some(skill_id.0)
            })
            .is_some_and(|row| row_bool(&row, "classskill", false))
    }

    /// Cost of `ranks` bought while levelling `class_id`: 1 per rank for that
    /// class's skills or with Able Learner, 2 otherwise.
    pub fn skill_cost_for_class(
        &self,
        class_id: super::ClassId,
        skill_id: SkillId,
        ranks: i32,
        has_able_learner: bool,
        game_data: &GameData,
    ) -> i32 {
        if ranks <= 0 {
            return 0;
        }
        if has_able_learner || self.is_class_skill_for(class_id, skill_id, game_data) {
            ranks
        } else {
            ranks * 2
        }
    }

    /// Get the maximum skill ranks allowed for a skill.
    ///
    /// Class skills: `total_level + 3`
    /// Cross-class skills: `(total_level + 3) / 2`
    pub fn get_max_skill_ranks(&self, skill_id: SkillId, game_data: &GameData) -> i32 {
        let total_level = self.total_level();
        if self.is_class_skill(skill_id, game_data) {
            total_level + 3
        } else {
            i32::midpoint(total_level, 3)
        }
    }

    /// Extra skill points per level from racial feats (`FEAT_SKILLED`).
    pub fn racial_skill_point_bonus(&self) -> i32 {
        i32::from(self.has_feat(super::FeatId(SKILLED_FEAT_ID)))
    }

    /// Calculate the cost in skill points for a given number of ranks.
    ///
    /// - Class skill OR Able Learner feat: 1 point per rank
    /// - Cross-class skill: 2 points per rank
    pub fn calculate_skill_cost(
        &self,
        skill_id: SkillId,
        ranks: i32,
        has_able_learner: bool,
        game_data: &GameData,
    ) -> i32 {
        if ranks <= 0 {
            return 0;
        }

        let is_class_skill = self.is_class_skill(skill_id, game_data);
        if is_class_skill || has_able_learner {
            ranks
        } else {
            ranks * 2
        }
    }

    /// Calculate the total skill modifier for a skill.
    ///
    /// Formula: `ranks + ability_modifier`
    /// If decoder is provided, uses total ability scores (base + race + equipment).
    /// Otherwise uses base ability scores.
    pub fn calculate_skill_modifier(
        &self,
        skill_id: SkillId,
        game_data: &GameData,
        decoder: Option<&ItemPropertyDecoder>,
    ) -> i32 {
        let ranks = self.skill_rank(skill_id);
        let key_ability = self.get_skill_key_ability(skill_id, game_data);

        // Base ability modifier
        let mut ability_mod = self.ability_modifier(key_ability);
        let mut item_skill_bonus = 0;

        if let Some(decoder) = decoder {
            // Get ability modifiers WITH equipment
            let total_mods = self.get_total_ability_modifiers(game_data, decoder);
            ability_mod = total_mods.get(key_ability);

            // Get specific skill modifiers from equipment (properties 52, etc.)
            let bonuses = self.get_equipment_bonuses(game_data, decoder);

            // Skill properties are keyed by "Skill_<ID>"
            let skill_key = format!("Skill_{}", skill_id.0);
            item_skill_bonus = bonuses.skill_bonuses.get(&skill_key).copied().unwrap_or(0);
        }

        ranks + ability_mod + item_skill_bonus
    }

    /// Calculate skill modifier using pre-calculated ability modifiers.
    /// Useful for batch operations to avoid recalculating abilities for every skill.
    pub fn calculate_skill_modifier_with_mods(
        &self,
        skill_id: SkillId,
        game_data: &GameData,
        ability_modifiers: &AbilityModifiers,
    ) -> i32 {
        let ranks = self.skill_rank(skill_id);
        let key_ability = self.get_skill_key_ability(skill_id, game_data);
        ranks + ability_modifiers.get(key_ability)
    }

    /// Get the key ability for a skill from the skills table.
    fn get_skill_key_ability(&self, skill_id: SkillId, game_data: &GameData) -> AbilityIndex {
        let Some(skills_table) = game_data.get_table("skills") else {
            return AbilityIndex::STR;
        };

        let Some(skill_data) = skills_table.get_by_id(skill_id.0) else {
            return AbilityIndex::STR;
        };

        let key_ability_str = skill_data
            .get("keyability")
            .and_then(|opt| opt.as_deref())
            .unwrap_or("");

        AbilityIndex::from_gff_field(key_ability_str).unwrap_or(AbilityIndex::STR)
    }

    /// Get a summary of all skills for this character.
    ///
    /// Returns a list of all valid skills from the skills table with:
    /// - Current ranks
    /// - Maximum ranks allowed
    /// - Total modifier (ranks + ability mod + equipment if decoder provided)
    /// - Whether it's a class skill
    /// - Whether it can be used untrained
    pub fn get_skill_summary(
        &self,
        game_data: &GameData,
        decoder: Option<&ItemPropertyDecoder>,
    ) -> Vec<SkillSummaryEntry> {
        let Some(skills_table) = game_data.get_table("skills") else {
            return Vec::new();
        };

        let (ability_modifiers, item_bonuses) = if let Some(d) = decoder {
            (
                Some(self.get_total_ability_modifiers(game_data, d)),
                Some(self.get_equipment_bonuses(game_data, d)),
            )
        } else {
            (None, None)
        };

        let feat_skill_bonuses = self.get_feat_skill_bonuses(game_data);

        let row_count = skills_table.row_count();
        let mut entries = Vec::with_capacity(row_count);

        for skill_id in 0..row_count {
            let Some(skill_data) = skills_table.get_by_id(skill_id as i32) else {
                continue;
            };

            let label = skill_data
                .get("label")
                .and_then(|opt| opt.as_deref())
                .unwrap_or("");

            if label.starts_with("****") || label.starts_with("DEL_") || label.contains("DELETED") {
                continue;
            }

            if row_bool(&skill_data, "removed", false) {
                continue;
            }

            let name = self.resolve_skill_name(skill_id as i32, &skill_data, game_data);
            let skill_id = SkillId(skill_id as i32);
            let ranks = self.skill_rank(skill_id);
            let max_ranks = self.get_max_skill_ranks(skill_id, game_data);

            let key_ability = self.get_skill_key_ability(skill_id, game_data);
            let ability_name = key_ability.short_name().to_string();

            let feat_bonus = Self::lookup_feat_skill_bonus(&feat_skill_bonuses, label, &name);
            if feat_bonus != 0 {
                tracing::debug!(
                    "[skill_summary] Skill '{}' (label='{}') got feat_bonus={}",
                    name,
                    label,
                    feat_bonus
                );
            }

            let (ability_mod, item_skill_bonus) =
                if let (Some(mods), Some(bonuses)) = (&ability_modifiers, &item_bonuses) {
                    let ability_mod = mods.get(key_ability);
                    let skill_key = format!("Skill_{}", skill_id.0);
                    let item_bonus = bonuses.skill_bonuses.get(&skill_key).copied().unwrap_or(0);
                    (ability_mod, item_bonus)
                } else {
                    (self.ability_modifier(key_ability), 0)
                };

            let untrained = row_int(&skill_data, "untrained", 1) > 0;

            let mut synergy_bonus = 0;
            if skill_id.0 == 29 && self.skill_rank(SkillId(14)) >= 5 {
                // Search -> Survival synergy
                synergy_bonus += 2;
            } else if skill_id.0 == 2 && self.skill_rank(SkillId(15)) >= 5 {
                // Set Trap -> Disable Device synergy
                synergy_bonus += 2;
            } else if skill_id.0 == 15 && self.skill_rank(SkillId(2)) >= 5 {
                // Disable Device -> Set Trap synergy
                synergy_bonus += 2;
            }

            let total = if !untrained && ranks == 0 {
                0
            } else {
                ranks + ability_mod + item_skill_bonus + feat_bonus + synergy_bonus
            };

            let is_class_skill = self.is_class_skill(skill_id, game_data);

            let armor_check_penalty = row_bool(&skill_data, "armorcheckpenalty", false);

            entries.push(SkillSummaryEntry {
                skill_id,
                name,
                ranks,
                max_ranks,
                modifier: ability_mod,
                total,
                ability: ability_name,
                is_class_skill,
                untrained,
                armor_check_penalty,
                feat_bonus,
                item_bonus: item_skill_bonus,
            });
        }

        entries
    }

    fn lookup_feat_skill_bonus(
        feat_bonuses: &HashMap<String, i32>,
        label: &str,
        name: &str,
    ) -> i32 {
        if feat_bonuses.is_empty() {
            return 0;
        }

        let normalized_label = label.trim().to_uppercase().replace([' ', '-', '_'], "");

        if let Some(&bonus) = feat_bonuses.get(&normalized_label) {
            return bonus;
        }

        let normalized_name = name.trim().to_uppercase().replace([' ', '-', '_'], "");

        if normalized_name != normalized_label
            && let Some(&bonus) = feat_bonuses.get(&normalized_name)
        {
            return bonus;
        }

        0
    }

    /// Calculate skill points gained for a single level.
    pub fn calculate_skill_points_for_level(
        &self,
        class_id: u8,
        int_modifier: i32,
        is_first_level: bool,
        game_data: &GameData,
    ) -> Result<i32, CharacterError> {
        let classes_table = game_data
            .get_table("classes")
            .ok_or_else(|| CharacterError::TableNotFound("classes.2da".to_string()))?;

        let class_data = classes_table
            .get_by_id(i32::from(class_id))
            .ok_or_else(|| CharacterError::ValidationFailed {
                field: "Class",
                message: format!("Class ID {class_id} not found"),
            })?;

        let base_points = class_data
            .get("skillpointbase")
            .and_then(|s| s.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(|| CharacterError::ValidationFailed {
                field: "SkillPointBase",
                message: format!("Class {class_id} missing SkillPointBase"),
            })?;

        let racial_bonus = self.racial_skill_point_bonus();

        let mut points = base_points + int_modifier + racial_bonus;
        // Minimum 1 point per level, even with negative modifiers
        points = points.max(1);

        if is_first_level {
            points *= 4;
            // Minimum 4 points at level 1
            points = points.max(4);
        }

        Ok(points)
    }

    /// Validate skill state (checking for negative ranks).
    pub fn validate_skills(&self) -> Vec<String> {
        let mut errors = Vec::new();

        let Some(skill_list) = self.get_list("SkillList") else {
            return errors;
        };

        for (idx, entry) in skill_list.iter().enumerate() {
            if let Some(rank) = entry.get("Rank").and_then(gff_value_to_i32)
                && rank < 0
            {
                errors.push(format!("Skill {idx}: Negative rank ({rank}) is invalid"));
            }
        }

        errors
    }

    pub fn get_skill_name(&self, skill_id: SkillId, game_data: &GameData) -> String {
        let Some(skills_table) = game_data.get_table("skills") else {
            return format!("Skill {}", skill_id.0);
        };

        let Some(skill_data) = skills_table.get_by_id(skill_id.0) else {
            return format!("Skill {}", skill_id.0);
        };

        self.resolve_skill_name(skill_id.0, &skill_data, game_data)
    }

    /// Resolve the skill name from strref or label.
    fn resolve_skill_name(
        &self,
        skill_id: i32,
        skill_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        let name_strref = skill_data
            .get("name")
            .and_then(|opt| opt.as_deref())
            .and_then(|s| s.parse::<i32>().ok());

        if let Some(strref) = name_strref
            && let Some(name) = game_data.get_string(strref)
            && !name.is_empty()
        {
            return name;
        }

        skill_data
            .get("label")
            .and_then(|opt| opt.as_deref())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| format!("Skill_{skill_id}"))
    }

    /// Get the rank in a specific skill (0 if not found or no ranks).
    pub fn skill_rank(&self, skill_id: SkillId) -> i32 {
        let Some(skill_list) = self.get_list("SkillList") else {
            return 0;
        };

        if skill_id.0 < 0 || skill_id.0 as usize >= skill_list.len() {
            return 0;
        }

        skill_list
            .get(skill_id.0 as usize)
            .and_then(|entry| entry.get("Rank"))
            .and_then(gff_value_to_i32)
            .unwrap_or(0)
    }

    /// Get all skills with non-zero ranks.
    pub fn skill_ranks(&self) -> Vec<super::SkillRankEntry> {
        let Some(skill_list) = self.get_list("SkillList") else {
            return vec![];
        };

        skill_list
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let rank = entry.get("Rank").and_then(gff_value_to_i32)?;
                if rank > 0 {
                    Some(super::SkillRankEntry {
                        skill_id: SkillId(idx as i32),
                        ranks: rank,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Set the rank for a specific skill.
    ///
    /// Creates missing skill list entries if needed.
    /// Does NOT manage skill points or validate against max ranks.
    pub fn set_skill_rank(&mut self, skill_id: SkillId, rank: i32) -> Result<(), CharacterError> {
        let old_rank = self.skill_rank(skill_id);
        let delta = rank - old_rank;

        if skill_id.0 < 0 {
            return Err(CharacterError::ValidationFailed {
                field: "SkillId",
                message: format!("Skill ID cannot be negative: {}", skill_id.0),
            });
        }

        if rank < 0 {
            return Err(CharacterError::ValidationFailed {
                field: "Rank",
                message: format!("Skill rank cannot be negative: {rank}"),
            });
        }

        if rank > MAX_SKILL_RANK {
            return Err(CharacterError::OutOfRange {
                field: "Rank",
                value: rank,
                min: 0,
                max: MAX_SKILL_RANK,
            });
        }

        let mut skill_list = self.get_list_owned("SkillList").unwrap_or_default();

        while skill_list.len() <= skill_id.0 as usize {
            let mut empty = IndexMap::new();
            empty.insert("Rank".to_string(), GffValue::Byte(0));
            skill_list.push(empty);
        }

        let mut updated = IndexMap::new();
        updated.insert("Rank".to_string(), GffValue::Byte(rank as u8));
        skill_list[skill_id.0 as usize] = updated;

        self.set_list("SkillList", skill_list);

        if delta != 0 {
            self.record_skill_change(skill_id, delta);
        }

        Ok(())
    }

    pub fn total_skill_points_spent(&self) -> i32 {
        let Some(skill_list) = self.get_list("SkillList") else {
            return 0;
        };

        skill_list
            .iter()
            .filter_map(|entry| entry.get("Rank").and_then(gff_value_to_i32))
            .sum()
    }

    /// Get the number of unspent skill points from the GFF field.
    pub fn get_available_skill_points(&self) -> i32 {
        self.gff
            .get("SkillPoints")
            .and_then(gff_value_to_i32)
            .unwrap_or(0)
    }

    /// Set the number of unspent skill points.
    pub fn set_available_skill_points(&mut self, points: i32) {
        let val = if points > 65535 {
            GffValue::Int(points) // Should be WORD usually but safe to handle larger
        } else {
            GffValue::Word(points as u16)
        };
        self.gff.insert("SkillPoints".to_string(), val);
        self.modified = true;
    }

    /// Set skill rank AND deduct skill points from available pool.
    ///
    /// This is the "economy-aware" version of `set_skill_rank`.
    /// - Calculates net cost (new_rank cost - current_rank cost)
    /// - Deducts from available skill points
    /// - Records change in level history
    ///
    /// Returns the net cost on success, or error if validation fails.
    pub fn set_skill_rank_with_cost(
        &mut self,
        skill_id: SkillId,
        new_rank: i32,
        game_data: &GameData,
    ) -> Result<i32, CharacterError> {
        let current_rank = self.skill_rank(skill_id);
        let has_able_learner = self.has_feat(super::FeatId(ABLE_LEARNER_FEAT_ID));

        let current_cost =
            self.calculate_skill_cost(skill_id, current_rank, has_able_learner, game_data);
        let new_cost = self.calculate_skill_cost(skill_id, new_rank, has_able_learner, game_data);
        let net_cost = new_cost - current_cost;

        let available = self.get_available_skill_points();
        if net_cost > available {
            return Err(CharacterError::ValidationFailed {
                field: "SkillPoints",
                message: format!("Insufficient skill points: need {net_cost}, have {available}"),
            });
        }

        self.set_skill_rank(skill_id, new_rank)?;
        self.set_available_skill_points(available - net_cost);

        Ok(net_cost)
    }

    /// Reset all skill ranks to 0 and restore the character's earned point pool.
    ///
    /// With every rank cleared, the unspent pool must equal the points the
    /// character actually earned across their level history, so it is
    /// recomputed from class/race/INT rather than refunding the cost of the
    /// ranks currently present. Refunding would mint points out of nothing for
    /// ranks another editor injected without paying for them.
    ///
    /// Falls back to a cost refund when there is no level history to derive a
    /// budget from, since assigning zero would silently destroy the pool.
    ///
    /// Returns the resulting number of available skill points.
    pub fn reset_all_skills(&mut self, game_data: &GameData) -> i32 {
        let earned: i32 = self.get_theoretical_skill_points(game_data).values().sum();

        let available = if earned > 0 {
            earned
        } else {
            warn!("No level history to derive a skill point budget from; refunding spent points");
            self.get_available_skill_points() + self.calculate_total_spent_with_costs(game_data)
        };

        let mut skill_list = self.get_list_owned("SkillList").unwrap_or_default();
        for entry in &mut skill_list {
            entry.insert("Rank".to_string(), GffValue::Byte(0));
        }
        self.set_list("SkillList", skill_list);

        self.set_available_skill_points(available);

        available
    }

    /// Calculate total skill points spent, factoring in class/cross-class costs.
    ///
    /// Unlike `total_skill_points_spent` which just sums ranks, this method
    /// calculates actual point cost: 1 per rank for class skills, 2 for cross-class.
    pub fn calculate_total_spent_with_costs(&self, game_data: &GameData) -> i32 {
        let Some(skill_list) = self.get_list("SkillList") else {
            return 0;
        };

        let has_able_learner = self.has_feat(super::FeatId(ABLE_LEARNER_FEAT_ID));

        skill_list
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let ranks = entry.get("Rank").and_then(gff_value_to_i32).unwrap_or(0);
                if ranks > 0 {
                    self.calculate_skill_cost(
                        SkillId(idx as i32),
                        ranks,
                        has_able_learner,
                        game_data,
                    )
                } else {
                    0
                }
            })
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();

        let mut skill0 = IndexMap::new();
        skill0.insert("Rank".to_string(), GffValue::Byte(5));
        let mut skill1 = IndexMap::new();
        skill1.insert("Rank".to_string(), GffValue::Byte(3));
        let mut skill2 = IndexMap::new();
        skill2.insert("Rank".to_string(), GffValue::Byte(0));
        let mut skill3 = IndexMap::new();
        skill3.insert("Rank".to_string(), GffValue::Byte(10));

        fields.insert(
            "SkillList".to_string(),
            GffValue::ListOwned(vec![skill0, skill1, skill2, skill3]),
        );

        Character::from_gff(fields)
    }

    #[test]
    fn test_skill_rank() {
        let character = create_test_character();
        assert_eq!(character.skill_rank(SkillId(0)), 5);
        assert_eq!(character.skill_rank(SkillId(1)), 3);
        assert_eq!(character.skill_rank(SkillId(2)), 0);
        assert_eq!(character.skill_rank(SkillId(3)), 10);
    }

    #[test]
    fn test_skill_rank_out_of_bounds() {
        let character = create_test_character();
        assert_eq!(character.skill_rank(SkillId(99)), 0);
        assert_eq!(character.skill_rank(SkillId(-1)), 0);
    }

    #[test]
    fn test_skill_rank_empty_list() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        assert_eq!(character.skill_rank(SkillId(0)), 0);
    }

    #[test]
    fn test_skill_ranks() {
        let character = create_test_character();
        let ranks = character.skill_ranks();

        assert_eq!(ranks.len(), 3);
        assert_eq!(ranks[0].skill_id.0, 0);
        assert_eq!(ranks[0].ranks, 5);
        assert_eq!(ranks[1].skill_id.0, 1);
        assert_eq!(ranks[1].ranks, 3);
        assert_eq!(ranks[2].skill_id.0, 3);
        assert_eq!(ranks[2].ranks, 10);
    }

    #[test]
    fn test_skill_ranks_empty_list() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        let ranks = character.skill_ranks();
        assert_eq!(ranks.len(), 0);
    }

    #[test]
    fn test_skill_ranks_all_zero() {
        let mut fields = IndexMap::new();
        let mut skill0 = IndexMap::new();
        skill0.insert("Rank".to_string(), GffValue::Byte(0));
        let mut skill1 = IndexMap::new();
        skill1.insert("Rank".to_string(), GffValue::Byte(0));

        fields.insert(
            "SkillList".to_string(),
            GffValue::ListOwned(vec![skill0, skill1]),
        );

        let character = Character::from_gff(fields);
        let ranks = character.skill_ranks();
        assert_eq!(ranks.len(), 0);
    }

    #[test]
    fn test_set_skill_rank() {
        let mut character = create_test_character();

        character.set_skill_rank(SkillId(0), 8).unwrap();
        assert_eq!(character.skill_rank(SkillId(0)), 8);

        character.set_skill_rank(SkillId(1), 0).unwrap();
        assert_eq!(character.skill_rank(SkillId(1)), 0);
    }

    #[test]
    fn test_set_skill_rank_creates_entries() {
        let mut fields = IndexMap::new();
        fields.insert("SkillList".to_string(), GffValue::ListOwned(vec![]));

        let mut character = Character::from_gff(fields);

        character.set_skill_rank(SkillId(5), 7).unwrap();
        assert_eq!(character.skill_rank(SkillId(5)), 7);
        assert_eq!(character.skill_rank(SkillId(0)), 0);
        assert_eq!(character.skill_rank(SkillId(4)), 0);
    }

    #[test]
    fn test_set_skill_rank_negative_rank() {
        let mut character = create_test_character();

        let result = character.set_skill_rank(SkillId(0), -5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterError::ValidationFailed { .. }
        ));
    }

    #[test]
    fn test_set_skill_rank_negative_id() {
        let mut character = create_test_character();

        let result = character.set_skill_rank(SkillId(-1), 5);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterError::ValidationFailed { .. }
        ));
    }

    #[test]
    fn test_set_skill_rank_max_value() {
        let mut character = create_test_character();

        character
            .set_skill_rank(SkillId(0), MAX_SKILL_RANK)
            .unwrap();
        assert_eq!(character.skill_rank(SkillId(0)), MAX_SKILL_RANK);

        let result = character.set_skill_rank(SkillId(0), MAX_SKILL_RANK + 1);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CharacterError::OutOfRange { .. }
        ));
    }

    #[test]
    fn test_total_skill_points_spent() {
        let character = create_test_character();
        assert_eq!(character.total_skill_points_spent(), 18);
    }

    #[test]
    fn test_total_skill_points_spent_empty() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        assert_eq!(character.total_skill_points_spent(), 0);
    }

    #[test]
    fn test_total_skill_points_spent_all_zero() {
        let mut fields = IndexMap::new();
        let mut skill0 = IndexMap::new();
        skill0.insert("Rank".to_string(), GffValue::Byte(0));
        let mut skill1 = IndexMap::new();
        skill1.insert("Rank".to_string(), GffValue::Byte(0));

        fields.insert(
            "SkillList".to_string(),
            GffValue::ListOwned(vec![skill0, skill1]),
        );

        let character = Character::from_gff(fields);
        assert_eq!(character.total_skill_points_spent(), 0);
    }

    #[test]
    fn test_set_skill_rank_marks_modified() {
        let mut character = create_test_character();
        assert!(!character.is_modified());

        character.set_skill_rank(SkillId(0), 10).unwrap();
        assert!(character.is_modified());
    }

    #[test]
    fn test_calculate_skill_cost_zero_ranks() {
        let character = create_test_character_with_class();
        let game_data = create_mock_game_data();

        let cost = character.calculate_skill_cost(SkillId(0), 0, false, &game_data);
        assert_eq!(cost, 0);
    }

    #[test]
    fn test_calculate_skill_cost_with_able_learner() {
        let character = create_test_character_with_class();
        let game_data = create_mock_game_data();

        let cost = character.calculate_skill_cost(SkillId(10), 5, true, &game_data);
        assert_eq!(cost, 5);
    }

    #[test]
    fn test_is_class_skill_no_classes() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        let game_data = create_mock_game_data();

        assert!(!character.is_class_skill(SkillId(0), &game_data));
    }

    #[test]
    fn test_is_class_skill_no_game_data() {
        let character = create_test_character_with_class();
        let game_data = create_mock_game_data();

        let result = character.is_class_skill(SkillId(0), &game_data);
        assert!(!result);
    }

    #[test]
    fn test_get_skill_summary_empty_no_tables() {
        let character = create_test_character_with_class();
        let game_data = create_mock_game_data();

        let summary = character.get_skill_summary(&game_data, None);
        assert_eq!(summary.len(), 0);
    }

    fn create_test_character_with_class() -> Character {
        let mut fields = IndexMap::new();

        fields.insert("Str".to_string(), GffValue::Byte(16));
        fields.insert("Dex".to_string(), GffValue::Byte(14));
        fields.insert("Con".to_string(), GffValue::Byte(12));
        fields.insert("Int".to_string(), GffValue::Byte(10));
        fields.insert("Wis".to_string(), GffValue::Byte(8));
        fields.insert("Cha".to_string(), GffValue::Byte(14));

        let mut class1 = IndexMap::new();
        class1.insert("Class".to_string(), GffValue::Byte(0));
        class1.insert("ClassLevel".to_string(), GffValue::Short(5));

        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class1]));

        let mut skill0 = IndexMap::new();
        skill0.insert("Rank".to_string(), GffValue::Byte(5));
        let mut skill1 = IndexMap::new();
        skill1.insert("Rank".to_string(), GffValue::Byte(3));

        fields.insert(
            "SkillList".to_string(),
            GffValue::ListOwned(vec![skill0, skill1]),
        );

        Character::from_gff(fields)
    }

    fn create_mock_game_data() -> crate::loaders::GameData {
        use crate::loaders::GameData;
        use crate::parsers::tlk::TLKParser;
        use std::sync::Arc;

        GameData::new(Arc::new(std::sync::RwLock::new(TLKParser::default())))
    }

    #[test]
    fn test_skill_modifier_with_equipment_bonus() {
        use crate::loaders::LoadedTable;
        use crate::parsers::tda::TDAParser;
        use crate::services::item_property_decoder::ItemPropertyDecoder;

        let mut fields = IndexMap::new();
        // Base Dex 10 (+0)
        fields.insert("Str".to_string(), GffValue::Byte(10));
        fields.insert("Dex".to_string(), GffValue::Byte(10));
        fields.insert("Con".to_string(), GffValue::Byte(10));
        fields.insert("Int".to_string(), GffValue::Byte(10));
        fields.insert("Wis".to_string(), GffValue::Byte(10));
        fields.insert("Cha".to_string(), GffValue::Byte(10));

        // Skill 0: Rank 5
        let mut skill0 = IndexMap::new();
        skill0.insert("Rank".to_string(), GffValue::Byte(5));
        fields.insert("SkillList".to_string(), GffValue::ListOwned(vec![skill0]));

        // Equip Item: +4 Dex -> Total Dex 14 (+2)
        let mut props = Vec::new();
        let mut prop = IndexMap::new();
        prop.insert("PropertyName".to_string(), GffValue::Word(0)); // Ability Bonus
        prop.insert("Subtype".to_string(), GffValue::Word(1)); // Dex
        prop.insert("CostValue".to_string(), GffValue::Byte(4)); // +4
        props.push(prop);

        let mut item_struct = IndexMap::new();
        item_struct.insert("__struct_id__".to_string(), GffValue::Dword(16)); // Right Hand
        item_struct.insert("PropertiesList".to_string(), GffValue::ListOwned(props));
        fields.insert(
            "Equip_ItemList".to_string(),
            GffValue::ListOwned(vec![item_struct]),
        );

        let character = Character::from_gff(fields);

        let mut game_data = create_mock_game_data();

        // Mock skills.2da
        // Row 0: Label=MoveSilently, KeyAbility=Dex
        let mut tda_parser = TDAParser::new();
        tda_parser.add_column("label");
        tda_parser.add_column("keyability");

        use ahash::AHashMap;
        let mut row0 = AHashMap::new();
        row0.insert("label".to_string(), Some("MoveSilently".to_string()));
        row0.insert("keyability".to_string(), Some("Dex".to_string()));
        tda_parser.add_row(row0);

        game_data.tables.insert(
            "skills".to_string(),
            LoadedTable::new("skills.2da".to_string(), std::sync::Arc::new(tda_parser)),
        );

        let paths =
            std::sync::Arc::new(tokio::sync::RwLock::new(crate::config::NWN2Paths::default()));
        let rm = std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::services::resource_manager::ResourceManager::new(paths),
        ));
        let mut decoder = ItemPropertyDecoder::new(rm);
        use std::collections::HashMap;
        let abilities = HashMap::from([
            (0, "Str".to_string()),
            (1, "Dex".to_string()),
            (2, "Con".to_string()),
            (3, "Int".to_string()),
            (4, "Wis".to_string()),
            (5, "Cha".to_string()),
        ]);
        decoder.set_2da_tables(
            abilities,
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        );

        // Test WITHOUT decoder (Base Dex 10 -> Mod +0)
        // Skill Mod = Rank 5 + Mod 0 = 5
        let mod_no_equip = character.calculate_skill_modifier(SkillId(0), &game_data, None);
        assert_eq!(
            mod_no_equip, 5,
            "Without equipment, modifier should be rank + base_ability_mod"
        );

        // Test WITH decoder (Total Dex 14 -> Mod +2)
        // Skill Mod = Rank 5 + Mod 2 = 7
        let mod_with_equip =
            character.calculate_skill_modifier(SkillId(0), &game_data, Some(&decoder));
        assert_eq!(
            mod_with_equip, 7,
            "With equipment, modifier should be rank + total_ability_mod"
        );
    }

    /// Level 2 rogue (skillpointbase 8) with Int 14 (+2):
    /// level 1 grants (8 + 2) * 4 = 40, level 2 grants 10, for 50 earned.
    fn create_level2_rogue_with_injected_ranks() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("Int".to_string(), GffValue::Byte(14));
        fields.insert("Race".to_string(), GffValue::Byte(0));
        fields.insert("SkillPoints".to_string(), GffValue::Word(0));

        let mut class1 = IndexMap::new();
        class1.insert("Class".to_string(), GffValue::Byte(0));
        class1.insert("ClassLevel".to_string(), GffValue::Short(2));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class1]));

        let mut lvl_stats = Vec::new();
        for _ in 0..2 {
            let mut entry = IndexMap::new();
            entry.insert("LvlStatClass".to_string(), GffValue::Byte(0));
            lvl_stats.push(entry);
        }
        fields.insert("LvlStatList".to_string(), GffValue::ListOwned(lvl_stats));

        // Ranks another editor injected without paying from the pool.
        let injected = (0..4)
            .map(|_| {
                let mut skill = IndexMap::new();
                skill.insert("Rank".to_string(), GffValue::Byte(40));
                skill
            })
            .collect();
        fields.insert("SkillList".to_string(), GffValue::ListOwned(injected));

        Character::from_gff(fields)
    }

    fn game_data_with_rogue_class() -> crate::loaders::GameData {
        use crate::loaders::LoadedTable;
        use crate::parsers::tda::TDAParser;

        let mut game_data = create_mock_game_data();

        let mut parser = TDAParser::new();
        parser
            .parse_from_bytes(
                b"2DA V2.0

   Label    SkillPointBase
0  Rogue    8
",
            )
            .unwrap();

        game_data.tables.insert(
            "classes".to_string(),
            LoadedTable::new("classes.2da".to_string(), std::sync::Arc::new(parser)),
        );

        game_data
    }

    #[test]
    fn test_cost_aware_total_exceeds_rank_sum_for_cross_class() {
        let character = create_test_character();
        let game_data = create_mock_game_data();

        // Ranks 5 + 3 + 0 + 10 = 18. With no cls_skill table every skill is
        // cross-class, so the real cost is double the rank sum. Callers
        // reporting points spent must use the cost-aware total, not the ranks.
        assert_eq!(character.total_skill_points_spent(), 18);
        assert_eq!(character.calculate_total_spent_with_costs(&game_data), 36);
    }

    #[test]
    fn test_reset_all_skills_uses_earned_budget_not_refund() {
        let mut character = create_level2_rogue_with_injected_ranks();
        let game_data = game_data_with_rogue_class();

        let available = character.reset_all_skills(&game_data);

        // Refunding the 160 injected ranks would have minted at least 160
        // points onto a level 2 character; only the 50 earned are legitimate.
        assert_eq!(available, 50);
        assert_eq!(character.get_available_skill_points(), 50);
    }

    /// Level 4 rogue whose INT went 13 -> 14 with the level 4 ability
    /// increase: levels 1-3 were granted at +1 INT ((8 + 1) * 4 + 9 + 9),
    /// level 4 at +2 (10), for 64 earned. Applying today's +2 to every
    /// level would claim 70.
    #[test]
    fn test_reset_all_skills_replays_int_increases_from_level_history() {
        let mut character = create_level2_rogue_with_injected_ranks();
        let mut lvl_stats = character.get_list_owned("LvlStatList").unwrap();
        for level in 3..=4 {
            let mut entry = IndexMap::new();
            entry.insert("LvlStatClass".to_string(), GffValue::Byte(0));
            if level == 4 {
                entry.insert("LvlStatAbility".to_string(), GffValue::Byte(3));
            }
            lvl_stats.push(entry);
        }
        character.set_list("LvlStatList", lvl_stats);
        let game_data = game_data_with_rogue_class();

        assert_eq!(character.reset_all_skills(&game_data), 64);
    }

    #[test]
    fn test_reset_all_skills_clears_every_rank() {
        let mut character = create_level2_rogue_with_injected_ranks();
        let game_data = game_data_with_rogue_class();

        character.reset_all_skills(&game_data);

        for skill_id in 0..4 {
            assert_eq!(character.skill_rank(SkillId(skill_id)), 0);
        }
    }

    #[test]
    fn test_reset_all_skills_is_idempotent() {
        let mut character = create_level2_rogue_with_injected_ranks();
        let game_data = game_data_with_rogue_class();

        let first = character.reset_all_skills(&game_data);
        let second = character.reset_all_skills(&game_data);

        assert_eq!(first, second, "Repeated resets must not accumulate points");
    }

    #[test]
    fn test_reset_all_skills_without_level_history_preserves_points() {
        let mut character = create_test_character();
        character.set_available_skill_points(7);
        let game_data = game_data_with_rogue_class();

        // No LvlStatList means no derivable budget; the pool must not be zeroed.
        let available = character.reset_all_skills(&game_data);

        assert!(
            available >= 7,
            "Fallback must not destroy existing points, got {available}"
        );
    }
}
