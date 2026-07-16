use crate::character::types::{AbilityIndex, ClassId};
use crate::character::{Character, CharacterError};
use crate::loaders::GameData;
use crate::utils::parsing::row_int;
use serde::{Deserialize, Serialize};
use specta::Type;

const NATURAL_AC_MIN: i32 = 0;
const NATURAL_AC_MAX: i32 = 255;

const CLASS_ID_BARBARIAN: i32 = 0;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct CombatStats {
    pub natural_ac: i32,
    pub damage_reduction: i32,
    pub spell_resistance: i32,
}

impl CombatStats {
    pub fn new(natural_ac: i32, damage_reduction: i32, spell_resistance: i32) -> Self {
        Self {
            natural_ac,
            damage_reduction,
            spell_resistance,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type)]
pub struct DamageBonuses {
    pub melee: i32,
    pub two_handed: i32,
    pub off_hand: i32,
    pub ranged: i32,
}

impl DamageBonuses {
    pub fn from_strength(str_mod: i32) -> Self {
        Self {
            melee: str_mod,
            two_handed: (str_mod * 3) / 2,
            off_hand: str_mod / 2,
            ranged: 0,
        }
    }
}

impl Character {
    pub fn calculate_bab(&self, game_data: &GameData) -> i32 {
        let class_entries = self.class_entries();
        if class_entries.is_empty() {
            return 0;
        }

        let mut total_bab = 0;
        let total_level: i32 = class_entries.iter().map(|e| e.level).sum();

        for entry in class_entries {
            let heroic_level = entry.level.min(20);
            let class_bab = self.get_class_bab_at_level(game_data, entry.class_id, heroic_level);
            total_bab += class_bab;
        }

        if total_level > 20 {
            let epic_levels = total_level - 20;
            let epic_bab = (epic_levels + 1) / 2;
            total_bab += epic_bab;
        }

        total_bab
    }

    fn get_class_bab_at_level(&self, game_data: &GameData, class_id: ClassId, level: i32) -> i32 {
        if level <= 0 {
            return 0;
        }

        let Some(classes_table) = game_data.get_table("classes") else {
            return level / 2;
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return level / 2;
        };

        let bab_table_name = class_data.get("attackbonustable").and_then(|s| s.as_ref());

        let Some(bab_table_name) = bab_table_name else {
            return level / 2;
        };

        let bab_table_lower = bab_table_name.to_lowercase();
        let Some(bab_table) = game_data.get_table(&bab_table_lower) else {
            return level / 2;
        };

        let row_index = (level - 1).clamp(0, 19) as usize;
        bab_table
            .get_cell(row_index, "BAB")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(level / 2)
    }

    pub fn get_attack_sequence(&self, game_data: &GameData) -> Vec<i32> {
        let mut bab = self.calculate_bab(game_data);
        let mut attacks = vec![bab];

        while bab > 5 {
            bab -= 5;
            attacks.push(bab);
        }

        attacks
    }

    pub fn calculate_base_ac(&self) -> i32 {
        10 + self.natural_ac()
    }

    pub fn calculate_initiative(&self, _game_data: &GameData) -> i32 {
        self.ability_modifier(AbilityIndex::DEX)
    }

    pub fn size_modifier(&self) -> i32 {
        self.size_category().ac_modifier_default()
    }

    pub fn get_melee_attack_bonus(&self, game_data: &GameData) -> i32 {
        let bab = self.calculate_bab(game_data);
        let str_mod = self.ability_modifier(AbilityIndex::STR);
        let size_mod = self.size_modifier();

        bab + str_mod + size_mod
    }

    pub fn get_ranged_attack_bonus(&self, game_data: &GameData) -> i32 {
        let bab = self.calculate_bab(game_data);
        let dex_mod = self.ability_modifier(AbilityIndex::DEX);
        let size_mod = self.size_modifier();

        bab + dex_mod + size_mod
    }

    pub fn get_damage_bonuses(&self) -> DamageBonuses {
        let str_mod = self.ability_modifier(AbilityIndex::STR);
        DamageBonuses::from_strength(str_mod)
    }

    pub fn natural_ac(&self) -> i32 {
        self.get_i32("NaturalAC").unwrap_or(0)
    }

    pub fn set_natural_ac(&mut self, value: i32) -> Result<(), CharacterError> {
        if !(NATURAL_AC_MIN..=NATURAL_AC_MAX).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: "NaturalAC",
                value,
                min: NATURAL_AC_MIN,
                max: NATURAL_AC_MAX,
            });
        }
        self.set_u8("NaturalAC", value as u8);
        Ok(())
    }

    pub fn damage_reduction(&self) -> i32 {
        self.get_i32("DR").unwrap_or(0)
    }

    pub fn set_damage_reduction(&mut self, value: i32) -> Result<(), CharacterError> {
        if !(0..=255).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: "DR",
                value,
                min: 0,
                max: 255,
            });
        }
        self.set_u8("DR", value as u8);
        Ok(())
    }

    pub fn spell_resistance(&self) -> i32 {
        self.get_i32("SR").unwrap_or(0)
    }

    pub fn set_spell_resistance(&mut self, value: i32) -> Result<(), CharacterError> {
        if !(0..=255).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: "SR",
                value,
                min: 0,
                max: 255,
            });
        }
        self.set_u8("SR", value as u8);
        Ok(())
    }

    pub fn base_attack_bonus(&self) -> i32 {
        self.get_i32("BAB").unwrap_or(0)
    }

    pub fn combat_stats(&self) -> CombatStats {
        CombatStats {
            natural_ac: self.natural_ac(),
            damage_reduction: self.damage_reduction(),
            spell_resistance: self.spell_resistance(),
        }
    }

    pub fn set_base_attack_bonus(&mut self, bab: i32) {
        self.set_i32("BAB", bab);
    }

    pub fn current_hit_points(&self) -> i32 {
        self.get_i32("CurrentHitPoints").unwrap_or(0)
    }

    pub fn max_hit_points(&self) -> i32 {
        self.get_i32("MaxHitPoints").unwrap_or(0)
    }

    pub fn temp_hit_points(&self) -> i32 {
        self.get_i32("TempHitPoints").unwrap_or(0)
    }

    pub fn set_current_hit_points(&mut self, value: i32) {
        self.set_i32("CurrentHitPoints", value);
    }

    pub fn set_max_hit_points(&mut self, value: i32) {
        let clamped = value.max(1);
        self.set_i32("MaxHitPoints", clamped);
    }

    pub fn set_temp_hit_points(&mut self, value: i32) {
        self.set_i32("TempHitPoints", value.max(0));
    }

    pub fn get_class_level(&self, class_id: ClassId) -> i32 {
        self.class_entries()
            .iter()
            .find(|e| e.class_id == class_id)
            .map(|e| e.level)
            .unwrap_or(0)
    }

    pub fn get_barbarian_damage_reduction(&self, _game_data: &GameData) -> i32 {
        let barb_level = self.get_class_level(ClassId(CLASS_ID_BARBARIAN));
        if barb_level >= 7 {
            1 + (barb_level - 7) / 3
        } else {
            0
        }
    }

    pub fn get_racial_spell_resistance(&self, game_data: &GameData) -> i32 {
        let race_id = self.race_id();
        let Some(racial_table) = game_data.get_table("racialtypes") else {
            return 0;
        };
        let Some(race_data) = racial_table.get_by_id(race_id.0) else {
            return 0;
        };

        let sr = row_int(&race_data, "sr", -1);
        if sr >= 0 {
            sr
        } else {
            row_int(&race_data, "spellresistance", 0)
        }
    }

    pub fn get_total_spell_resistance(&self, game_data: &GameData) -> i32 {
        let base_sr = self.spell_resistance();
        let racial_sr = self.get_racial_spell_resistance(game_data);
        base_sr.max(racial_sr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;
    use indexmap::IndexMap;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("NaturalAC".to_string(), GffValue::Byte(2));
        fields.insert("DR".to_string(), GffValue::Byte(5));
        fields.insert("SR".to_string(), GffValue::Byte(15));
        Character::from_gff(fields)
    }

    #[test]
    fn test_natural_ac() {
        let character = create_test_character();
        assert_eq!(character.natural_ac(), 2);
    }

    #[test]
    fn test_damage_reduction() {
        let character = create_test_character();
        assert_eq!(character.damage_reduction(), 5);
    }

    #[test]
    fn test_spell_resistance() {
        let character = create_test_character();
        assert_eq!(character.spell_resistance(), 15);
    }

    #[test]
    fn test_combat_stats() {
        let character = create_test_character();
        let stats = character.combat_stats();
        assert_eq!(stats.natural_ac, 2);
        assert_eq!(stats.damage_reduction, 5);
        assert_eq!(stats.spell_resistance, 15);
    }

    #[test]
    fn test_set_natural_ac() {
        let mut character = create_test_character();
        assert!(!character.is_modified());

        character.set_natural_ac(10).unwrap();
        assert!(character.is_modified());
        assert_eq!(character.natural_ac(), 10);
    }

    #[test]
    fn test_set_damage_reduction() {
        let mut character = create_test_character();
        character.set_damage_reduction(8).unwrap();
        assert_eq!(character.damage_reduction(), 8);
    }

    #[test]
    fn test_set_spell_resistance() {
        let mut character = create_test_character();
        character.set_spell_resistance(20).unwrap();
        assert_eq!(character.spell_resistance(), 20);
    }

    #[test]
    fn test_set_natural_ac_out_of_range_low() {
        let mut character = create_test_character();
        let result = character.set_natural_ac(-5);
        assert!(result.is_err());
        assert_eq!(character.natural_ac(), 2);
    }

    #[test]
    fn test_set_natural_ac_out_of_range_high() {
        let mut character = create_test_character();
        let result = character.set_natural_ac(300);
        assert!(result.is_err());
        assert_eq!(character.natural_ac(), 2);
    }

    #[test]
    fn test_set_natural_ac_boundary_values() {
        let mut character = create_test_character();

        character.set_natural_ac(NATURAL_AC_MIN).unwrap();
        assert_eq!(character.natural_ac(), NATURAL_AC_MIN);

        character.set_natural_ac(NATURAL_AC_MAX).unwrap();
        assert_eq!(character.natural_ac(), NATURAL_AC_MAX);
    }

    #[test]
    fn test_set_damage_reduction_out_of_range() {
        let mut character = create_test_character();
        assert!(character.set_damage_reduction(-1).is_err());
        assert!(character.set_damage_reduction(256).is_err());
        assert_eq!(character.damage_reduction(), 5);
    }

    #[test]
    fn test_set_spell_resistance_out_of_range() {
        let mut character = create_test_character();
        assert!(character.set_spell_resistance(-1).is_err());
        assert!(character.set_spell_resistance(256).is_err());
        assert_eq!(character.spell_resistance(), 15);
    }

    #[test]
    fn test_combat_stats_default() {
        let stats = CombatStats::default();
        assert_eq!(stats.natural_ac, 0);
        assert_eq!(stats.damage_reduction, 0);
        assert_eq!(stats.spell_resistance, 0);
    }

    #[test]
    fn test_combat_stats_new() {
        let stats = CombatStats::new(5, 10, 15);
        assert_eq!(stats.natural_ac, 5);
        assert_eq!(stats.damage_reduction, 10);
        assert_eq!(stats.spell_resistance, 15);
    }

    #[test]
    fn test_combat_stats_with_missing_fields() {
        let character = Character::from_gff(IndexMap::new());
        let stats = character.combat_stats();
        assert_eq!(stats.natural_ac, 0);
        assert_eq!(stats.damage_reduction, 0);
        assert_eq!(stats.spell_resistance, 0);
    }

    #[test]
    fn test_base_attack_bonus_default() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.base_attack_bonus(), 0);
    }

    #[test]
    fn test_base_attack_bonus_stored() {
        let mut fields = IndexMap::new();
        fields.insert("BAB".to_string(), GffValue::Int(15));
        let character = Character::from_gff(fields);
        assert_eq!(character.base_attack_bonus(), 15);
    }

    #[test]
    fn test_damage_bonuses_from_strength() {
        let damage = DamageBonuses::from_strength(4);
        assert_eq!(damage.melee, 4);
        assert_eq!(damage.two_handed, 6);
        assert_eq!(damage.off_hand, 2);
        assert_eq!(damage.ranged, 0);
    }

    #[test]
    fn test_damage_bonuses_negative_strength() {
        let damage = DamageBonuses::from_strength(-2);
        assert_eq!(damage.melee, -2);
        assert_eq!(damage.two_handed, -3);
        assert_eq!(damage.off_hand, -1);
        assert_eq!(damage.ranged, 0);
    }

    #[test]
    fn test_get_damage_bonuses() {
        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(18));
        let character = Character::from_gff(fields);

        let damage = character.get_damage_bonuses();
        assert_eq!(damage.melee, 4);
        assert_eq!(damage.two_handed, 6);
        assert_eq!(damage.off_hand, 2);
        assert_eq!(damage.ranged, 0);
    }

    #[test]
    fn test_calculate_base_ac() {
        let mut fields = IndexMap::new();
        fields.insert("NaturalAC".to_string(), GffValue::Byte(3));
        let character = Character::from_gff(fields);

        assert_eq!(character.calculate_base_ac(), 13);
    }

    #[test]
    fn test_calculate_base_ac_no_natural() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.calculate_base_ac(), 10);
    }

    #[test]
    fn test_size_modifier() {
        let mut fields = IndexMap::new();

        // NWN2 creaturesize.2da: 1=Tiny(+2), 2=Small(+1), 3=Medium(0), 4=Large(-1), 5=Huge(-2)
        fields.insert("CreatureSize".to_string(), GffValue::Int(1));
        let character = Character::from_gff(fields.clone());
        assert_eq!(character.size_modifier(), 2);

        fields.insert("CreatureSize".to_string(), GffValue::Int(2));
        let character = Character::from_gff(fields.clone());
        assert_eq!(character.size_modifier(), 1);

        fields.insert("CreatureSize".to_string(), GffValue::Int(3));
        let character = Character::from_gff(fields.clone());
        assert_eq!(character.size_modifier(), 0);

        fields.insert("CreatureSize".to_string(), GffValue::Int(4));
        let character = Character::from_gff(fields.clone());
        assert_eq!(character.size_modifier(), -1);

        fields.insert("CreatureSize".to_string(), GffValue::Int(5));
        let character = Character::from_gff(fields.clone());
        assert_eq!(character.size_modifier(), -2);
    }

    #[test]
    fn test_hit_points() {
        let mut fields = IndexMap::new();
        fields.insert("CurrentHitPoints".to_string(), GffValue::Int(75));
        fields.insert("MaxHitPoints".to_string(), GffValue::Int(100));
        fields.insert("TempHitPoints".to_string(), GffValue::Int(10));
        let character = Character::from_gff(fields);

        assert_eq!(character.current_hit_points(), 75);
        assert_eq!(character.max_hit_points(), 100);
        assert_eq!(character.temp_hit_points(), 10);
    }

    #[test]
    fn test_set_hit_points() {
        let mut character = Character::from_gff(IndexMap::new());

        character.set_current_hit_points(50);
        assert_eq!(character.current_hit_points(), 50);

        character.set_max_hit_points(100);
        assert_eq!(character.max_hit_points(), 100);

        character.set_temp_hit_points(15);
        assert_eq!(character.temp_hit_points(), 15);
    }

    #[test]
    fn test_set_max_hit_points_min_clamp() {
        let mut character = Character::from_gff(IndexMap::new());
        character.set_max_hit_points(0);
        assert_eq!(character.max_hit_points(), 1);

        character.set_max_hit_points(-10);
        assert_eq!(character.max_hit_points(), 1);
    }

    #[test]
    fn test_set_temp_hit_points_clamp() {
        let mut character = Character::from_gff(IndexMap::new());
        character.set_temp_hit_points(-5);
        assert_eq!(character.temp_hit_points(), 0);
    }

    #[test]
    fn test_get_class_level_no_class() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.get_class_level(ClassId(0)), 0);
    }
}
