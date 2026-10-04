use crate::character::gff_helpers::gff_value_to_i32;
use crate::character::types::{
    ABILITY_INCREASE_INTERVAL, ABILITY_MAX, ABILITY_MIN, AbilityIndex, AbilityModifiers,
    AbilityScores, HitPoints, calculate_modifier,
};
use crate::character::{Character, CharacterError};
use crate::loaders::GameData;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::services::item_property_decoder::{ItemBonuses, ItemPropertyDecoder};

// Point buy constants (NWN2 standard)
const POINT_BUY_COSTS: [i32; 11] = [0, 1, 2, 3, 4, 5, 6, 8, 10, 13, 16];
pub const POINT_BUY_BUDGET: i32 = 32;
pub const POINT_BUY_MIN: i32 = 8;
pub const POINT_BUY_MAX: i32 = 18;

fn point_buy_cost_for_score(score: i32) -> i32 {
    if score <= 8 {
        0
    } else if score >= 18 {
        16
    } else {
        POINT_BUY_COSTS[(score - 8) as usize]
    }
}

pub fn calculate_point_buy_cost(scores: &AbilityScores) -> i32 {
    point_buy_cost_for_score(scores.str_)
        + point_buy_cost_for_score(scores.dex)
        + point_buy_cost_for_score(scores.con)
        + point_buy_cost_for_score(scores.int)
        + point_buy_cost_for_score(scores.wis)
        + point_buy_cost_for_score(scores.cha)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
pub struct AbilityIncrease {
    pub level: i32,
    pub ability: AbilityIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, Default)]
pub struct AbilityPointsSummary {
    pub base_scores: AbilityScores,
    pub level_increases: Vec<AbilityIncrease>,
    pub expected_increases: i32,
    pub actual_increases: i32,
    pub available: i32,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type)]
pub struct PointBuyState {
    pub starting_scores: AbilityScores,
    pub point_buy_cost: i32,
    pub budget: i32,
    pub remaining: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AbilitiesState {
    pub base_scores: AbilityScores,
    pub effective_scores: AbilityScores,
    pub modifiers: AbilityModifiers,
    pub racial_modifiers: AbilityModifiers,
    pub equipment_modifiers: AbilityModifiers,
    pub hit_points: HitPoints,
    pub point_summary: AbilityPointsSummary,
    pub point_buy: PointBuyState,
}

impl Character {
    pub fn base_ability(&self, ability: AbilityIndex) -> i32 {
        self.get_byte(ability.gff_field())
            .map_or(10, |v| i32::from(v))
    }

    pub fn set_ability(&mut self, ability: AbilityIndex, value: i32) -> Result<(), CharacterError> {
        if !(ABILITY_MIN..=ABILITY_MAX).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: ability.gff_field(),
                value,
                min: ABILITY_MIN,
                max: ABILITY_MAX,
            });
        }

        self.set_byte(ability.gff_field(), value as u8);
        Ok(())
    }

    /// Set an ability score and trigger cascading effects (like HP recalculation for Constitution).
    pub fn set_ability_with_cascades(
        &mut self,
        ability: AbilityIndex,
        value: i32,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let old_value = self.base_ability(ability);

        // 1. Set the raw value first
        self.set_ability(ability, value)?;

        // 2. Handle Cascades
        // For now, only Constitution changes trigger persistent updates (HP)
        if ability == AbilityIndex::CON {
            self.recalculate_hit_points(old_value, value);
        }

        // 3. One level-up slot per point raised, freed again per point lowered. An INT
        // point on a level-up slot changes that level's skill point grant, like in-game.
        let raise = value > old_value;
        let steps: Vec<i32> = if raise {
            (old_value..value).collect()
        } else {
            (value + 1..=old_value).rev().collect()
        };
        for from in steps {
            let settles_skills = ability == AbilityIndex::INT;
            let grants_before = if settles_skills {
                self.set_ability(ability, from)?;
                let grants: i32 = self.skill_points_granted_per_level(game_data).iter().sum();
                self.set_ability(ability, if raise { from + 1 } else { from - 1 })?;
                grants
            } else {
                0
            };
            let slot_changed = if raise {
                self.record_ability_change(ability)
            } else {
                self.release_ability_change(ability)
            };
            if !slot_changed {
                break;
            }
            if settles_skills {
                self.settle_shifted_skill_grants(grants_before, game_data);
            }
        }
        self.set_ability(ability, value)
    }

    pub(crate) fn recalculate_hit_points(&mut self, old_con: i32, new_con: i32) {
        let old_mod = calculate_modifier(old_con);
        let new_mod = calculate_modifier(new_con);

        if old_mod == new_mod {
            return;
        }

        let total_level = self.total_level();
        if total_level == 0 {
            return;
        }

        let mod_diff = new_mod - old_mod;
        let hp_change = total_level * mod_diff;

        if hp_change == 0 {
            return;
        }

        let current_hp = self.current_hp();
        let max_hp = self.max_hp();
        let new_max_hp = (max_hp + hp_change).max(1);
        let new_current_hp = (current_hp + hp_change).clamp(1, new_max_hp);

        self.set_max_hp(new_max_hp);
        self.set_current_hp(new_current_hp);
    }

    pub fn base_scores(&self) -> AbilityScores {
        AbilityScores::new(
            self.base_ability(AbilityIndex::STR),
            self.base_ability(AbilityIndex::DEX),
            self.base_ability(AbilityIndex::CON),
            self.base_ability(AbilityIndex::INT),
            self.base_ability(AbilityIndex::WIS),
            self.base_ability(AbilityIndex::CHA),
        )
    }

    pub fn ability_modifier(&self, ability: AbilityIndex) -> i32 {
        let score = self.base_ability(ability);
        calculate_modifier(score)
    }

    pub fn ability_modifiers(&self) -> AbilityModifiers {
        let scores = self.base_scores();
        AbilityModifiers::from_scores(&scores)
    }

    pub fn current_hp(&self) -> i32 {
        self.get_i32("CurrentHitPoints").unwrap_or(0)
    }

    pub fn max_hp(&self) -> i32 {
        self.get_i32("MaxHitPoints").unwrap_or(0)
    }

    pub fn base_hp(&self) -> i32 {
        self.get_i32("HitPoints").unwrap_or(0)
    }

    pub fn temp_hp(&self) -> i32 {
        self.get_i32("TempHitPoints").unwrap_or(0)
    }

    pub fn set_current_hp(&mut self, hp: i32) {
        self.set_i32("CurrentHitPoints", hp);
    }

    pub fn set_max_hp(&mut self, hp: i32) {
        self.set_i32("MaxHitPoints", hp);
    }

    pub fn set_base_hp(&mut self, hp: i32) {
        self.set_i32("HitPoints", hp);
    }

    pub fn set_temp_hp(&mut self, hp: i32) {
        self.set_i32("TempHitPoints", hp);
    }

    pub fn hit_points(&self) -> HitPoints {
        HitPoints::new(self.current_hp(), self.max_hp(), self.temp_hp())
    }

    pub fn get_effective_abilities(&self, game_data: &GameData) -> AbilityScores {
        let mut scores = self.base_scores();
        let racial_mods = self.get_racial_ability_modifiers(game_data);

        for ability in AbilityIndex::all() {
            let current = scores.get(ability);
            scores.set(ability, current + racial_mods.get(ability));
        }

        scores
    }

    pub fn get_racial_ability_modifiers(&self, game_data: &GameData) -> AbilityModifiers {
        // Delegate to get_racial_modifier_deltas in race.rs which handles both
        // base races (from racialtypes.2da) and subraces (from racialsubtypes.2da)
        self.get_racial_modifier_deltas(game_data)
    }

    pub fn get_total_abilities(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> AbilityScores {
        let equip_bonuses = self.get_equipment_bonuses(game_data, decoder);
        self.total_abilities_with_items(game_data, &equip_bonuses)
    }

    /// Stored score + racial adjustment + item bonuses. Stored `.bic` scores
    /// exclude the racial adjustment, so every derived value must start here.
    pub fn total_abilities_with_items(
        &self,
        game_data: &GameData,
        equip_bonuses: &ItemBonuses,
    ) -> AbilityScores {
        let mut scores = self.get_effective_abilities(game_data);

        scores.str_ += equip_bonuses.str_bonus;
        scores.dex += equip_bonuses.dex_bonus;
        scores.con += equip_bonuses.con_bonus;
        scores.int += equip_bonuses.int_bonus;
        scores.wis += equip_bonuses.wis_bonus;
        scores.cha += equip_bonuses.cha_bonus;

        scores
    }

    pub fn get_total_ability_modifiers(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> AbilityModifiers {
        let scores = self.get_total_abilities(game_data, decoder);
        AbilityModifiers::from_scores(&scores)
    }

    /// Modifier of the stored score plus the racial adjustment: what the
    /// engine grants level-up hit points and skill points with.
    pub fn get_effective_ability_modifier(
        &self,
        ability: AbilityIndex,
        game_data: &GameData,
    ) -> i32 {
        let scores = self.get_effective_abilities(game_data);
        calculate_modifier(scores.get(ability))
    }

    pub fn get_level_up_ability_history(&self) -> Vec<AbilityIncrease> {
        let Some(lvl_stat_list) = self.get_list("LvlStatList") else {
            return vec![];
        };

        let mut history = Vec::new();

        for (idx, entry) in lvl_stat_list.iter().enumerate() {
            let char_level = (idx + 1) as i32;

            if char_level % ABILITY_INCREASE_INTERVAL != 0 {
                continue;
            }

            if let Some(ability_value) = entry.get("LvlStatAbility") {
                let ability_index = gff_value_to_i32(ability_value).unwrap_or(-1);
                if (0..6).contains(&ability_index)
                    && let Some(ability) = AbilityIndex::from_index(ability_index as u8)
                {
                    history.push(AbilityIncrease {
                        level: char_level,
                        ability,
                    });
                }
            }
        }

        history
    }

    /// Level-up ability points are tracked per `LvlStatList` entry, so a
    /// character without a level history (e.g. Bishop in the OC, level 7 with
    /// no entries) has no slot to spend into and reports 0 expected/available
    /// rather than a phantom point.
    pub fn get_ability_points_summary(&self) -> AbilityPointsSummary {
        let base_scores = self.base_scores();
        let level_increases = self.get_level_up_ability_history();
        let history_levels = self
            .get_list("LvlStatList")
            .map_or(0, |list| list.len() as i32)
            .min(self.total_level());
        let expected_increases = history_levels / ABILITY_INCREASE_INTERVAL;
        let actual_increases = level_increases.len() as i32;
        let available = expected_increases - actual_increases;

        AbilityPointsSummary {
            base_scores,
            level_increases,
            expected_increases,
            actual_increases,
            available,
        }
    }

    pub fn get_starting_ability_scores(&self) -> AbilityScores {
        let base = self.base_scores();
        let history = self.get_level_up_ability_history();

        let mut increases = [0i32; 6];
        for inc in &history {
            increases[inc.ability.0 as usize] += 1;
        }

        AbilityScores {
            str_: base.str_ - increases[0],
            dex: base.dex - increases[1],
            con: base.con - increases[2],
            int: base.int - increases[3],
            wis: base.wis - increases[4],
            cha: base.cha - increases[5],
        }
    }

    pub fn get_point_buy_state(&self) -> PointBuyState {
        let starting_scores = self.get_starting_ability_scores();
        let point_buy_cost = calculate_point_buy_cost(&starting_scores);

        PointBuyState {
            starting_scores,
            point_buy_cost,
            budget: POINT_BUY_BUDGET,
            remaining: POINT_BUY_BUDGET - point_buy_cost,
        }
    }

    pub fn get_abilities_state(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> AbilitiesState {
        let base_scores = self.base_scores();
        let effective_scores = self.get_effective_abilities(game_data);
        let racial_modifiers = self.get_racial_ability_modifiers(game_data);

        let equip_bonuses = self.get_equipment_bonuses(game_data, decoder);
        let equipment_modifiers = AbilityModifiers::new(
            equip_bonuses.str_bonus,
            equip_bonuses.dex_bonus,
            equip_bonuses.con_bonus,
            equip_bonuses.int_bonus,
            equip_bonuses.wis_bonus,
            equip_bonuses.cha_bonus,
        );

        let modifiers = AbilityModifiers::from_scores(
            &self.total_abilities_with_items(game_data, &equip_bonuses),
        );

        AbilitiesState {
            base_scores,
            effective_scores,
            modifiers,
            racial_modifiers,
            equipment_modifiers,
            hit_points: self.hit_points(),
            point_summary: self.get_ability_points_summary(),
            point_buy: self.get_point_buy_state(),
        }
    }

    pub fn clear_ability_level_up_history(&mut self) -> Result<(), CharacterError> {
        use crate::parsers::gff::GffValue;

        if let Some(list) = self.get_list_mut("LvlStatList") {
            for entry in list
                .iter_mut()
                .skip(ABILITY_INCREASE_INTERVAL as usize - 1)
                .step_by(ABILITY_INCREASE_INTERVAL as usize)
            {
                entry.insert("LvlStatAbility".to_string(), GffValue::Byte(255));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;
    use indexmap::IndexMap;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(16));
        fields.insert("Dex".to_string(), GffValue::Byte(14));
        fields.insert("Con".to_string(), GffValue::Byte(12));
        fields.insert("Int".to_string(), GffValue::Byte(10));
        fields.insert("Wis".to_string(), GffValue::Byte(8));
        fields.insert("Cha".to_string(), GffValue::Byte(14));
        fields.insert("CurrentHitPoints".to_string(), GffValue::Int(50));
        fields.insert("MaxHitPoints".to_string(), GffValue::Int(100));
        fields.insert("HitPoints".to_string(), GffValue::Int(80));
        fields.insert("TempHitPoints".to_string(), GffValue::Int(5));
        Character::from_gff(fields).expect("character from gff")
    }

    #[test]
    fn test_base_ability() {
        let character = create_test_character();
        assert_eq!(character.base_ability(AbilityIndex::STR), 16);
        assert_eq!(character.base_ability(AbilityIndex::DEX), 14);
        assert_eq!(character.base_ability(AbilityIndex::CON), 12);
        assert_eq!(character.base_ability(AbilityIndex::INT), 10);
        assert_eq!(character.base_ability(AbilityIndex::WIS), 8);
        assert_eq!(character.base_ability(AbilityIndex::CHA), 14);
    }

    #[test]
    fn test_base_scores() {
        let character = create_test_character();
        let scores = character.base_scores();
        assert_eq!(scores.str_, 16);
        assert_eq!(scores.dex, 14);
        assert_eq!(scores.con, 12);
        assert_eq!(scores.int, 10);
        assert_eq!(scores.wis, 8);
        assert_eq!(scores.cha, 14);
    }

    #[test]
    fn test_ability_modifier() {
        let character = create_test_character();
        assert_eq!(character.ability_modifier(AbilityIndex::STR), 3);
        assert_eq!(character.ability_modifier(AbilityIndex::DEX), 2);
        assert_eq!(character.ability_modifier(AbilityIndex::CON), 1);
        assert_eq!(character.ability_modifier(AbilityIndex::INT), 0);
        assert_eq!(character.ability_modifier(AbilityIndex::WIS), -1);
        assert_eq!(character.ability_modifier(AbilityIndex::CHA), 2);
    }

    #[test]
    fn test_ability_modifiers() {
        let character = create_test_character();
        let mods = character.ability_modifiers();
        assert_eq!(mods.str_mod, 3);
        assert_eq!(mods.dex_mod, 2);
        assert_eq!(mods.con_mod, 1);
        assert_eq!(mods.int_mod, 0);
        assert_eq!(mods.wis_mod, -1);
        assert_eq!(mods.cha_mod, 2);
    }

    #[test]
    fn test_set_ability_valid() {
        let mut character = create_test_character();
        let result = character.set_ability(AbilityIndex::STR, 18);
        assert!(result.is_ok());
        assert_eq!(character.base_ability(AbilityIndex::STR), 18);
    }

    #[test]
    fn test_set_ability_too_low() {
        let mut character = create_test_character();
        let result = character.set_ability(AbilityIndex::STR, 2);
        assert!(result.is_err());
        match result {
            Err(CharacterError::OutOfRange {
                field,
                value,
                min,
                max,
            }) => {
                assert_eq!(field, "Str");
                assert_eq!(value, 2);
                assert_eq!(min, 3);
                assert_eq!(max, 50);
            }
            _ => panic!("Expected OutOfRange error"),
        }
    }

    #[test]
    fn test_set_ability_too_high() {
        let mut character = create_test_character();
        let result = character.set_ability(AbilityIndex::STR, 51);
        assert!(result.is_err());
    }

    #[test]
    fn test_hit_points_getters() {
        let character = create_test_character();
        assert_eq!(character.current_hp(), 50);
        assert_eq!(character.max_hp(), 100);
        assert_eq!(character.base_hp(), 80);
        assert_eq!(character.temp_hp(), 5);
    }

    #[test]
    fn test_hit_points_setters() {
        let mut character = create_test_character();

        character.set_current_hp(75);
        assert_eq!(character.current_hp(), 75);

        character.set_max_hp(120);
        assert_eq!(character.max_hp(), 120);

        character.set_base_hp(90);
        assert_eq!(character.base_hp(), 90);

        character.set_temp_hp(10);
        assert_eq!(character.temp_hp(), 10);
    }

    #[test]
    fn test_hit_points_struct() {
        let character = create_test_character();
        let hp = character.hit_points();
        assert_eq!(hp.current, 50);
        assert_eq!(hp.max, 100);
        assert_eq!(hp.temp, 5);
        assert_eq!(hp.effective_current(), 55);
    }

    #[test]
    fn test_base_ability_missing_field() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields).expect("character from gff");
        assert_eq!(character.base_ability(AbilityIndex::STR), 10);
    }

    #[test]
    fn test_ability_at_boundaries() {
        let mut character = create_test_character();

        character.set_ability(AbilityIndex::STR, 3).unwrap();
        assert_eq!(character.base_ability(AbilityIndex::STR), 3);
        assert_eq!(character.ability_modifier(AbilityIndex::STR), -4);

        character.set_ability(AbilityIndex::STR, 50).unwrap();
        assert_eq!(character.base_ability(AbilityIndex::STR), 50);
        assert_eq!(character.ability_modifier(AbilityIndex::STR), 20);
    }

    #[test]
    fn test_get_level_up_ability_history_empty() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields).expect("character from gff");
        let history = character.get_level_up_ability_history();
        assert!(history.is_empty());
    }

    #[test]
    fn test_get_level_up_ability_history() {
        let mut fields = IndexMap::new();

        let mut lvl_stat_list = Vec::new();
        for i in 0..8 {
            let mut entry = IndexMap::new();
            if i == 3 {
                entry.insert("LvlStatAbility".to_string(), GffValue::Byte(0));
            } else if i == 7 {
                entry.insert("LvlStatAbility".to_string(), GffValue::Byte(1));
            }
            lvl_stat_list.push(entry);
        }

        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(lvl_stat_list),
        );
        let character = Character::from_gff(fields).expect("character from gff");

        let history = character.get_level_up_ability_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].level, 4);
        assert_eq!(history[0].ability, AbilityIndex::STR);
        assert_eq!(history[1].level, 8);
        assert_eq!(history[1].ability, AbilityIndex::DEX);
    }

    #[test]
    fn test_get_ability_points_summary() {
        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(16));
        fields.insert("Dex".to_string(), GffValue::Byte(14));
        fields.insert("Con".to_string(), GffValue::Byte(12));
        fields.insert("Int".to_string(), GffValue::Byte(10));
        fields.insert("Wis".to_string(), GffValue::Byte(8));
        fields.insert("Cha".to_string(), GffValue::Byte(14));

        let mut class_list = Vec::new();
        let mut class_entry = IndexMap::new();
        class_entry.insert("Class".to_string(), GffValue::Byte(0));
        class_entry.insert("ClassLevel".to_string(), GffValue::Short(8));
        class_list.push(class_entry);
        fields.insert("ClassList".to_string(), GffValue::ListOwned(class_list));

        let mut lvl_stat_list = Vec::new();
        for i in 0..8 {
            let mut entry = IndexMap::new();
            if i == 3 {
                entry.insert("LvlStatAbility".to_string(), GffValue::Byte(0));
            } else if i == 7 {
                entry.insert("LvlStatAbility".to_string(), GffValue::Byte(1));
            }
            lvl_stat_list.push(entry);
        }
        fields.insert(
            "LvlStatList".to_string(),
            GffValue::ListOwned(lvl_stat_list),
        );

        let character = Character::from_gff(fields).expect("character from gff");
        let summary = character.get_ability_points_summary();

        assert_eq!(summary.expected_increases, 2);
        assert_eq!(summary.actual_increases, 2);
        assert_eq!(summary.level_increases.len(), 2);
        assert_eq!(summary.base_scores.str_, 16);
    }

    #[test]
    fn test_con_change_updates_hp() {
        let mut fields = IndexMap::new();
        fields.insert("Con".to_string(), GffValue::Byte(14));

        let mut class_list = Vec::new();
        let mut class_entry = IndexMap::new();
        class_entry.insert("Class".to_string(), GffValue::Byte(0));
        class_entry.insert("ClassLevel".to_string(), GffValue::Short(10));
        class_list.push(class_entry);
        fields.insert("ClassList".to_string(), GffValue::ListOwned(class_list));

        fields.insert("MaxHitPoints".to_string(), GffValue::Int(100));
        fields.insert("CurrentHitPoints".to_string(), GffValue::Int(80));
        fields.insert("HitPoints".to_string(), GffValue::Int(100));

        let mut character = Character::from_gff(fields).expect("character from gff");

        let game_data = crate::loaders::GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        )));

        let result = character.set_ability_with_cascades(AbilityIndex::CON, 16, &game_data);

        assert!(result.is_ok());
        assert_eq!(character.base_ability(AbilityIndex::CON), 16);
        assert_eq!(character.max_hp(), 110);
        assert_eq!(character.base_hp(), 100);
        assert_eq!(character.current_hp(), 90);

        let result = character.set_ability_with_cascades(AbilityIndex::CON, 12, &game_data);

        assert!(result.is_ok());
        assert_eq!(character.base_ability(AbilityIndex::CON), 12);
        assert_eq!(character.max_hp(), 90);
        assert_eq!(character.current_hp(), 70);
    }

    #[test]
    fn test_get_total_abilities_with_equipment() {
        use crate::services::item_property_decoder::ItemPropertyDecoder;

        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(14));
        fields.insert("Dex".to_string(), GffValue::Byte(12));
        fields.insert("Con".to_string(), GffValue::Byte(10));
        fields.insert("Int".to_string(), GffValue::Byte(10));
        fields.insert("Wis".to_string(), GffValue::Byte(10));
        fields.insert("Cha".to_string(), GffValue::Byte(10));

        // Create item with +2 STR
        let mut props = Vec::new();
        let mut prop = IndexMap::new();
        prop.insert("PropertyName".to_string(), GffValue::Word(0)); // Ability Bonus (0)
        prop.insert("Subtype".to_string(), GffValue::Word(0)); // Str (0)
        prop.insert("CostValue".to_string(), GffValue::Byte(2)); // +2
        props.push(prop);

        let mut item_struct = IndexMap::new();
        item_struct.insert("__struct_id__".to_string(), GffValue::Dword(16)); // Right Hand (0x10)
        item_struct.insert("BaseItem".to_string(), GffValue::Int(0)); // Shortsword or generic
        item_struct.insert("PropertiesList".to_string(), GffValue::ListOwned(props));

        fields.insert(
            "Equip_ItemList".to_string(),
            GffValue::ListOwned(vec![item_struct]),
        );

        let character = Character::from_gff(fields).expect("character from gff");

        let game_data = crate::loaders::GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        )));

        let decoder = ItemPropertyDecoder::default();

        let total = character.get_total_abilities(&game_data, &decoder);
        assert_eq!(total.str_, 16); // 14 + 2
        assert_eq!(total.dex, 12);
    }

    fn character_with_levels(level4_ability: Option<GffValue<'static>>) -> Character {
        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(16));
        let mut class = IndexMap::new();
        class.insert("Class".to_string(), GffValue::Byte(0));
        class.insert("ClassLevel".to_string(), GffValue::Short(5));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class]));
        let levels = (1..=5)
            .map(|lvl| {
                let mut entry = IndexMap::new();
                entry.insert("LvlStatClass".to_string(), GffValue::Byte(0));
                if lvl == 4
                    && let Some(v) = level4_ability.clone()
                {
                    entry.insert("LvlStatAbility".to_string(), v);
                }
                entry
            })
            .collect();
        fields.insert("LvlStatList".to_string(), GffValue::ListOwned(levels));
        Character::from_gff(fields).expect("character from gff")
    }

    #[test]
    fn no_level_history_means_no_points() {
        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(16));
        let mut class = IndexMap::new();
        class.insert("Class".to_string(), GffValue::Byte(0));
        class.insert("ClassLevel".to_string(), GffValue::Short(7));
        fields.insert("ClassList".to_string(), GffValue::ListOwned(vec![class]));
        let character = Character::from_gff(fields).expect("character from gff");
        let summary = character.get_ability_points_summary();
        assert_eq!(summary.expected_increases, 0);
        assert_eq!(summary.available, 0);
    }

    #[test]
    fn increase_fills_unspent_slot_written_as_255() {
        let mut character = character_with_levels(Some(GffValue::Byte(255)));
        assert_eq!(character.get_ability_points_summary().available, 1);
        character.record_ability_change(AbilityIndex::STR);
        let summary = character.get_ability_points_summary();
        assert_eq!(summary.available, 0);
        assert_eq!(summary.level_increases[0].ability, AbilityIndex::STR);
    }

    #[test]
    fn increase_fills_unspent_slot_with_absent_field() {
        let mut character = character_with_levels(None);
        assert_eq!(character.get_ability_points_summary().available, 1);
        character.record_ability_change(AbilityIndex::DEX);
        let summary = character.get_ability_points_summary();
        assert_eq!(summary.available, 0);
        assert_eq!(summary.level_increases[0].ability, AbilityIndex::DEX);
    }

    #[test]
    fn increase_does_not_overwrite_spent_slot() {
        let mut character = character_with_levels(Some(GffValue::Byte(2)));
        assert_eq!(character.get_ability_points_summary().available, 0);
        character.record_ability_change(AbilityIndex::STR);
        let summary = character.get_ability_points_summary();
        assert_eq!(summary.level_increases[0].ability, AbilityIndex::CON);
    }

    fn character_with_eight_levels() -> Character {
        let mut character = character_with_levels(None);
        let mut levels = character.get_list_owned("LvlStatList").unwrap();
        levels.extend(levels.clone().into_iter().take(3));
        character.set_list("LvlStatList", levels);
        let mut classes = character.get_list_owned("ClassList").unwrap();
        classes[0].insert("ClassLevel".to_string(), GffValue::Short(8));
        character.set_list("ClassList", classes);
        character
    }

    #[test]
    fn each_raised_point_spends_a_slot_and_each_lowered_point_frees_one() {
        let mut character = character_with_eight_levels();
        let game_data = crate::loaders::GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        )));

        character
            .set_ability_with_cascades(AbilityIndex::STR, 18, &game_data)
            .unwrap();
        assert_eq!(character.get_ability_points_summary().available, 0);
        assert_eq!(character.get_starting_ability_scores().str_, 16);

        character
            .set_ability_with_cascades(AbilityIndex::STR, 17, &game_data)
            .unwrap();
        let summary = character.get_ability_points_summary();
        assert_eq!(summary.available, 1);
        assert_eq!(summary.level_increases[0].level, 4);
    }

    #[test]
    fn int_raised_on_a_level_up_slot_grants_its_skill_points() {
        let mut character = character_with_eight_levels();
        character.set_ability(AbilityIndex::INT, 13).unwrap();
        character.set_available_skill_points(0);
        let game_data = crate::loaders::GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        )));

        character
            .set_ability_with_cascades(AbilityIndex::INT, 14, &game_data)
            .unwrap();
        assert_eq!(character.get_available_skill_points(), 5);

        character
            .set_ability_with_cascades(AbilityIndex::INT, 13, &game_data)
            .unwrap();
        assert_eq!(character.get_available_skill_points(), 0);
    }

    #[test]
    fn clearing_history_marks_only_ability_levels() {
        let mut character = character_with_eight_levels();
        character.clear_ability_level_up_history().unwrap();

        let levels = character.get_list("LvlStatList").unwrap();
        for (idx, level) in levels.iter().enumerate() {
            let expected = ((idx + 1) % 4 == 0).then_some(255);
            assert_eq!(
                level.get("LvlStatAbility").and_then(gff_value_to_i32),
                expected,
                "level {}",
                idx + 1
            );
        }
    }
}
