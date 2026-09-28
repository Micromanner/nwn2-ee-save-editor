use crate::character::types::ClassId;
use crate::character::{Character, CharacterError};
use crate::loaders::GameData;

const NATURAL_AC_MIN: i32 = 0;
const NATURAL_AC_MAX: i32 = 255;

impl Character {
    pub fn calculate_bab(&self, game_data: &GameData) -> i32 {
        let class_entries = self.class_entries();
        if class_entries.is_empty() {
            return 0;
        }

        class_entries
            .iter()
            .map(|entry| self.get_class_bab_at_level(game_data, entry.class_id, entry.level))
            .sum()
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

        let row_index = (level - 1) as usize;
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

    pub fn size_modifier(&self) -> i32 {
        self.size_category().ac_modifier_default()
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

    pub fn base_attack_bonus(&self) -> i32 {
        self.get_i32("BaseAttackBonus").unwrap_or(0)
    }

    pub fn set_base_attack_bonus(&mut self, bab: i32) {
        self.set_byte("BaseAttackBonus", bab.clamp(0, i32::from(u8::MAX)) as u8);
    }

    pub fn get_class_level(&self, class_id: ClassId) -> i32 {
        self.class_entries()
            .iter()
            .find(|e| e.class_id == class_id)
            .map(|e| e.level)
            .unwrap_or(0)
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
        Character::from_gff(fields)
    }

    #[test]
    fn test_natural_ac() {
        let character = create_test_character();
        assert_eq!(character.natural_ac(), 2);
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
    fn test_base_attack_bonus_default() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.base_attack_bonus(), 0);
    }

    #[test]
    fn test_base_attack_bonus_stored() {
        let mut fields = IndexMap::new();
        fields.insert("BaseAttackBonus".to_string(), GffValue::Byte(15));
        let character = Character::from_gff(fields);
        assert_eq!(character.base_attack_bonus(), 15);
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
    fn test_get_class_level_no_class() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.get_class_level(ClassId(0)), 0);
    }
}
