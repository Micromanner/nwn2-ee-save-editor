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

use super::{Character, CharacterError, ClassId};
use crate::character::gff_helpers::gff_value_to_i32;
use crate::character::types::{AbilityIndex, FeatId, SkillId};
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
    /// Class skill of the class being levelled (1 point per rank); the rank
    /// cap uses any class instead (`Character::is_class_skill`).
    pub is_class_skill: bool,
    pub untrained: bool,
    pub armor_check_penalty: bool,
    pub feat_bonus: i32,
    pub item_bonus: i32,
}

impl Character {
    /// Check if a skill is a class skill for any of the character's classes.
    ///
    /// This is what the rank cap uses (NWN2 follows the 3.5 multiclass rule:
    /// a skill any class lists may reach level + 3). Rank cost is judged
    /// against the class being levelled instead (`is_levelling_class_skill`).
    pub fn is_class_skill(&self, skill_id: SkillId, game_data: &GameData) -> bool {
        self.class_entries()
            .iter()
            .any(|entry| self.is_class_skill_for(entry.class_id, skill_id, game_data))
    }

    /// Class a skill purchase is costed against: the class taken at the most
    /// recent level, where the editor records rank changes. Characters
    /// without a level history fall back to their last `ClassList` entry.
    pub fn levelling_class(&self) -> Option<ClassId> {
        self.get_list("LvlStatList")
            .and_then(|levels| levels.last())
            .and_then(|entry| entry.get("LvlStatClass"))
            .and_then(gff_value_to_i32)
            .filter(|id| *id != -1)
            .map(ClassId)
            .or_else(|| self.class_entries().last().map(|entry| entry.class_id))
    }

    /// Whether the class being levelled lists the skill, which decides the
    /// 1 or 2 point rank price.
    pub fn is_levelling_class_skill(&self, skill_id: SkillId, game_data: &GameData) -> bool {
        self.levelling_class()
            .is_some_and(|class_id| self.is_class_skill_for(class_id, skill_id, game_data))
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

    /// Cost of `ranks` bought now, against the class being levelled
    /// (`skill_cost_for_class`). Without any class every skill is cross-class.
    pub fn calculate_skill_cost(
        &self,
        skill_id: SkillId,
        ranks: i32,
        has_able_learner: bool,
        game_data: &GameData,
    ) -> i32 {
        let class_id = self.levelling_class().unwrap_or(ClassId(-1));
        self.skill_cost_for_class(class_id, skill_id, ranks, has_able_learner, game_data)
    }

    /// Net skill points a rank change costs (negative = refund).
    ///
    /// Added ranks are bought by the class being levelled. Removed ranks are
    /// handed back most recent level first and refund what the class taken at
    /// that level paid, mirroring `record_skill_change`; ranks no level
    /// recorded refund at the levelling class's price.
    pub fn skill_change_cost(&self, skill_id: SkillId, new_rank: i32, game_data: &GameData) -> i32 {
        let has_able_learner = self.has_feat(FeatId(ABLE_LEARNER_FEAT_ID));
        let delta = new_rank - self.skill_rank(skill_id);
        if delta >= 0 {
            return self.calculate_skill_cost(skill_id, delta, has_able_learner, game_data);
        }

        let levelling = self.levelling_class().unwrap_or(ClassId(-1));
        let refund: i32 = self
            .skill_rank_take_back_plan(skill_id, -delta)
            .into_iter()
            .map(|(level_idx, ranks)| {
                let class_id = level_idx
                    .and_then(|idx| self.level_class(idx))
                    .unwrap_or(levelling);
                self.skill_cost_for_class(class_id, skill_id, ranks, has_able_learner, game_data)
            })
            .sum();
        -refund
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
            if skill_id.0 == 2 && self.skill_rank(SkillId(15)) >= 5 {
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

            let is_class_skill = self.is_levelling_class_skill(skill_id, game_data);

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

    /// Get the number of unspent skill points from the GFF field.
    pub fn get_available_skill_points(&self) -> i32 {
        self.gff
            .get("SkillPoints")
            .and_then(gff_value_to_i32)
            .unwrap_or(0)
    }

    /// Set the number of unspent skill points.
    pub fn set_available_skill_points(&mut self, points: i32) {
        let points = points.clamp(0, i32::from(u16::MAX)) as u16;
        self.gff
            .insert("SkillPoints".to_string(), GffValue::Word(points));
        self.modified = true;
    }

    /// Set a skill rank and pay for it from the unspent pool.
    ///
    /// Buying more than the pool holds is allowed and leaves it at 0. A refund
    /// pays such an overdraw back first: it never raises the pool above what
    /// the level history earned minus what the remaining ranks cost, so buying
    /// and removing ranks cannot mint points. Without a level history there is
    /// no budget to check against and refunds are paid in full.
    ///
    /// Returns the net cost, negative for a refund.
    pub fn change_skill_rank(
        &mut self,
        skill_id: SkillId,
        new_rank: i32,
        game_data: &GameData,
    ) -> Result<i32, CharacterError> {
        let net_cost = self.skill_change_cost(skill_id, new_rank, game_data);
        let available = self.get_available_skill_points();

        self.set_skill_rank(skill_id, new_rank)?;

        let pool = if net_cost > 0 {
            (available - net_cost).max(0)
        } else {
            let refunded = available - net_cost;
            let earned: i32 = self.get_theoretical_skill_points(game_data).values().sum();
            if earned > 0 {
                let unspent = earned - self.calculate_total_spent_with_costs(game_data);
                refunded.min(available.max(unspent))
            } else {
                refunded
            }
        };
        self.set_available_skill_points(pool);

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

        let mut lvl_stat_list = self.get_list_owned("LvlStatList").unwrap_or_default();
        for level in &mut lvl_stat_list {
            if let Some(mut level_skills) =
                super::gff_helpers::extract_list_from_map(level, "SkillList")
            {
                for entry in &mut level_skills {
                    entry.insert("Rank".to_string(), GffValue::Byte(0));
                }
                level.insert("SkillList".to_string(), GffValue::ListOwned(level_skills));
            }
        }
        if !lvl_stat_list.is_empty() {
            self.set_list("LvlStatList", lvl_stat_list);
        }

        self.set_available_skill_points(available);

        available
    }

    /// Skill points the current ranks cost.
    ///
    /// Ranks the level history records are costed against the class taken at
    /// that level, oldest first and never more than the skill currently has;
    /// ranks it does not cover (toolset creation ranks, injected ranks)
    /// against the class being levelled.
    pub fn calculate_total_spent_with_costs(&self, game_data: &GameData) -> i32 {
        let has_able_learner = self.has_feat(FeatId(ABLE_LEARNER_FEAT_ID));
        let levelling = self.levelling_class().unwrap_or(ClassId(-1));

        let mut recorded: HashMap<SkillId, i32> = HashMap::new();
        let mut spent = 0;
        for level in self.level_history() {
            for skill in &level.skills_gained {
                let recorded_ranks = recorded.entry(skill.skill_id).or_insert(0);
                let ranks = skill
                    .ranks
                    .min(self.skill_rank(skill.skill_id) - *recorded_ranks);
                if ranks <= 0 {
                    continue;
                }
                spent += self.skill_cost_for_class(
                    level.class_id,
                    skill.skill_id,
                    ranks,
                    has_able_learner,
                    game_data,
                );
                *recorded_ranks += ranks;
            }
        }

        for skill in self.skill_ranks() {
            let uncovered = skill.ranks - recorded.get(&skill.skill_id).copied().unwrap_or(0);
            if uncovered > 0 {
                spent += self.skill_cost_for_class(
                    levelling,
                    skill.skill_id,
                    uncovered,
                    has_able_learner,
                    game_data,
                );
            }
        }
        spent
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
        let mod_no_equip = character.get_skill_summary(&game_data, None)[0].total;
        assert_eq!(
            mod_no_equip, 5,
            "Without equipment, modifier should be rank + base_ability_mod"
        );

        // Test WITH decoder (Total Dex 14 -> Mod +2)
        // Skill Mod = Rank 5 + Mod 2 = 7
        let mod_with_equip = character.get_skill_summary(&game_data, Some(&decoder))[0].total;
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

    const SORCERER: i32 = 9;
    const FIGHTER: i32 = 4;
    const LORE: SkillId = SkillId(7);
    const PARRY: SkillId = SkillId(10);

    fn parse_2da(text: &[u8]) -> crate::loaders::LoadedTable {
        use crate::parsers::tda::TDAParser;
        let mut parser = TDAParser::new();
        parser.parse_from_bytes(text).unwrap();
        crate::loaders::LoadedTable::new("test.2da".to_string(), std::sync::Arc::new(parser))
    }

    /// Sorcerer lists Lore, Fighter lists Parry.
    fn game_data_with_sorcerer_and_fighter() -> crate::loaders::GameData {
        let mut game_data = create_mock_game_data();
        game_data.tables.insert(
            "classes".to_string(),
            parse_2da(
                b"2DA V2.0

   Label     SkillPointBase  SkillsTable
0  Barbarian 4               ****
1  Bard      6               ****
2  Cleric    2               ****
3  Druid     4               ****
4  Fighter   2               cls_skill_fight
5  Monk      4               ****
6  Paladin   2               ****
7  Ranger    6               ****
8  Rogue     8               ****
9  Sorcerer  2               cls_skill_sorc
",
            ),
        );
        game_data.tables.insert(
            "cls_skill_fight".to_string(),
            parse_2da(
                b"2DA V2.0

   SkillLabel  SkillIndex  ClassSkill
0  Parry       10          1
1  Lore        7           0
",
            ),
        );
        game_data.tables.insert(
            "cls_skill_sorc".to_string(),
            parse_2da(
                b"2DA V2.0

   SkillLabel  SkillIndex  ClassSkill
0  Lore        7           1
",
            ),
        );
        game_data
    }

    fn level_entry(
        class_id: i32,
        bought: &[(SkillId, i32)],
    ) -> IndexMap<String, GffValue<'static>> {
        let mut skills: Vec<IndexMap<String, GffValue<'static>>> = Vec::new();
        for (skill_id, ranks) in bought {
            while skills.len() <= skill_id.0 as usize {
                let mut empty = IndexMap::new();
                empty.insert("Rank".to_string(), GffValue::Byte(0));
                skills.push(empty);
            }
            skills[skill_id.0 as usize].insert("Rank".to_string(), GffValue::Byte(*ranks as u8));
        }
        let mut entry = IndexMap::new();
        entry.insert("LvlStatClass".to_string(), GffValue::Byte(class_id as u8));
        entry.insert("SkillList".to_string(), GffValue::ListOwned(skills));
        entry
    }

    /// Sorcerer 1 / Fighter 1 who bought 3 Lore as a Sorcerer, then 1 Lore
    /// and 2 Parry as a Fighter; the Fighter level is the one being levelled.
    fn create_sorcerer_then_fighter() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("SkillPoints".to_string(), GffValue::Word(10));

        let mut sorcerer = IndexMap::new();
        sorcerer.insert("Class".to_string(), GffValue::Byte(SORCERER as u8));
        sorcerer.insert("ClassLevel".to_string(), GffValue::Short(1));
        let mut fighter = IndexMap::new();
        fighter.insert("Class".to_string(), GffValue::Byte(FIGHTER as u8));
        fighter.insert("ClassLevel".to_string(), GffValue::Short(1));
        fields.insert(
            "ClassList".to_string(),
            GffValue::ListOwned(vec![sorcerer, fighter]),
        );

        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(vec![
                level_entry(SORCERER, &[(LORE, 3)]),
                level_entry(FIGHTER, &[(LORE, 1), (PARRY, 2)]),
            ]),
        );

        let totals = level_entry(0, &[(LORE, 4), (PARRY, 2)]);
        fields.insert("SkillList".to_string(), totals["SkillList"].clone());

        Character::from_gff(fields)
    }

    #[test]
    fn test_skill_cost_follows_levelling_class_not_union() {
        let character = create_sorcerer_then_fighter();
        let game_data = game_data_with_sorcerer_and_fighter();

        assert_eq!(character.levelling_class(), Some(ClassId(FIGHTER)));
        // Lore is a class skill of a class the character has, so the cap is
        // the full one, but a Fighter pays the cross-class price for it.
        assert!(character.is_class_skill(LORE, &game_data));
        assert!(!character.is_levelling_class_skill(LORE, &game_data));
        assert!(character.is_levelling_class_skill(PARRY, &game_data));
        assert_eq!(
            character.calculate_skill_cost(LORE, 1, false, &game_data),
            2
        );
        assert_eq!(
            character.calculate_skill_cost(PARRY, 1, false, &game_data),
            1
        );
        assert_eq!(character.skill_change_cost(LORE, 5, &game_data), 2);
        assert_eq!(
            character.get_max_skill_ranks(LORE, &game_data),
            character.total_level() + 3
        );
    }

    #[test]
    fn test_removed_ranks_refund_the_class_that_bought_them() {
        let mut character = create_sorcerer_then_fighter();
        let game_data = game_data_with_sorcerer_and_fighter();

        // Lore 4 -> 2: the Fighter rank (2 points) and one Sorcerer rank (1).
        assert_eq!(character.skill_change_cost(LORE, 2, &game_data), -3);
        assert_eq!(
            character.skill_rank_take_back_plan(LORE, 2),
            vec![(Some(1), 1), (Some(0), 1)]
        );

        character.set_skill_rank(LORE, 2).unwrap();
        let history = character.level_history();
        assert_eq!(history[0].skills_gained[0].ranks, 2);
        assert!(history[1].skills_gained.iter().all(|s| s.skill_id != LORE));

        // Ranks no level recorded refund at the levelling class's price and
        // never drive a level's record negative.
        assert_eq!(character.skill_change_cost(LORE, 0, &game_data), -2);
        let mut skill_list = character.get_list_owned("SkillList").unwrap();
        skill_list[LORE.0 as usize].insert("Rank".to_string(), GffValue::Byte(5));
        character.set_list("SkillList", skill_list);
        assert_eq!(
            character.skill_rank_take_back_plan(LORE, 5),
            vec![(Some(0), 2), (None, 3)]
        );
        character.set_skill_rank(LORE, 0).unwrap();
        assert!(character.level_history()[0].skills_gained.is_empty());
    }

    #[test]
    fn test_total_spent_costs_each_level_against_its_class() {
        let mut character = create_sorcerer_then_fighter();
        let game_data = game_data_with_sorcerer_and_fighter();

        // 3 Lore at 1 (Sorcerer) + 1 Lore at 2 (Fighter) + 2 Parry at 1.
        assert_eq!(character.calculate_total_spent_with_costs(&game_data), 7);

        // A rank the history does not cover is priced for the levelling class.
        let mut skill_list = character.get_list_owned("SkillList").unwrap();
        skill_list[LORE.0 as usize].insert("Rank".to_string(), GffValue::Byte(5));
        character.set_list("SkillList", skill_list);
        assert_eq!(character.calculate_total_spent_with_costs(&game_data), 9);
    }

    #[test]
    fn test_total_spent_caps_recorded_ranks_at_current_rank() {
        let mut character = create_sorcerer_then_fighter();
        let game_data = game_data_with_sorcerer_and_fighter();

        // History records 4 Lore, the skill has 1: only the oldest Sorcerer
        // rank is paid for, plus 2 Parry.
        let mut skill_list = character.get_list_owned("SkillList").unwrap();
        skill_list[LORE.0 as usize].insert("Rank".to_string(), GffValue::Byte(1));
        character.set_list("SkillList", skill_list);
        assert_eq!(character.calculate_total_spent_with_costs(&game_data), 3);
    }

    #[test]
    fn test_reset_all_skills_clears_level_history_ranks() {
        let mut character = create_sorcerer_then_fighter();
        let game_data = game_data_with_sorcerer_and_fighter();

        character.reset_all_skills(&game_data);

        assert!(
            character
                .level_history()
                .iter()
                .all(|level| level.skills_gained.is_empty())
        );
        assert_eq!(character.calculate_total_spent_with_costs(&game_data), 0);
    }

    #[test]
    fn test_overdraw_refund_does_not_mint_points() {
        let mut character = create_sorcerer_then_fighter();
        let game_data = game_data_with_sorcerer_and_fighter();
        let earned: i32 = character
            .get_theoretical_skill_points(&game_data)
            .values()
            .sum();
        let unspent = earned - character.calculate_total_spent_with_costs(&game_data);
        character.set_available_skill_points(unspent);

        // Parry is a Fighter class skill: 8 more ranks than the pool pays for.
        let bought = unspent + 8;
        assert_eq!(
            character
                .change_skill_rank(PARRY, 2 + bought, &game_data)
                .unwrap(),
            bought
        );
        assert_eq!(character.get_available_skill_points(), 0);

        assert_eq!(
            character.change_skill_rank(PARRY, 2, &game_data).unwrap(),
            -bought
        );
        assert_eq!(character.get_available_skill_points(), unspent);
    }
}
