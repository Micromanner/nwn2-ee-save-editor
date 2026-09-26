use crate::character::{Character, CharacterError};
use crate::loaders::GameData;
use serde::{Deserialize, Serialize};

const SAVE_MIN: i32 = -35;
const SAVE_MAX: i32 = 255;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct SaveBonuses {
    pub fortitude: i32,
    pub reflex: i32,
    pub will: i32,
}

impl SaveBonuses {
    pub fn new(fortitude: i32, reflex: i32, will: i32) -> Self {
        Self {
            fortitude,
            reflex,
            will,
        }
    }
}

impl Character {
    pub fn base_fortitude(&self) -> i32 {
        self.get_i32("fortbonus").unwrap_or(0)
    }

    pub fn base_reflex(&self) -> i32 {
        self.get_i32("refbonus").unwrap_or(0)
    }

    pub fn base_will(&self) -> i32 {
        self.get_i32("willbonus").unwrap_or(0)
    }

    pub fn set_fortitude(&mut self, value: i32) -> Result<(), CharacterError> {
        if !(SAVE_MIN..=SAVE_MAX).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: "fortbonus",
                value,
                min: SAVE_MIN,
                max: SAVE_MAX,
            });
        }
        self.set_i16("fortbonus", value as i16);
        Ok(())
    }

    pub fn set_reflex(&mut self, value: i32) -> Result<(), CharacterError> {
        if !(SAVE_MIN..=SAVE_MAX).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: "refbonus",
                value,
                min: SAVE_MIN,
                max: SAVE_MAX,
            });
        }
        self.set_i16("refbonus", value as i16);
        Ok(())
    }

    pub fn set_will(&mut self, value: i32) -> Result<(), CharacterError> {
        if !(SAVE_MIN..=SAVE_MAX).contains(&value) {
            return Err(CharacterError::OutOfRange {
                field: "willbonus",
                value,
                min: SAVE_MIN,
                max: SAVE_MAX,
            });
        }
        self.set_i16("willbonus", value as i16);
        Ok(())
    }

    pub fn save_bonuses(&self) -> SaveBonuses {
        SaveBonuses {
            fortitude: self.base_fortitude(),
            reflex: self.base_reflex(),
            will: self.base_will(),
        }
    }

    pub fn calculate_base_saves(&self, game_data: &GameData) -> SaveBonuses {
        let class_entries = self.class_entries();
        let mut fort = 0;
        let mut reflex = 0;
        let mut will = 0;

        for entry in class_entries {
            let (f, r, w) = self.get_class_saves_at_level(game_data, entry.class_id, entry.level);
            fort += f;
            reflex += r;
            will += w;
        }

        SaveBonuses::new(fort, reflex, will)
    }

    fn get_class_saves_at_level(
        &self,
        game_data: &GameData,
        class_id: crate::character::types::ClassId,
        level: i32,
    ) -> (i32, i32, i32) {
        if level <= 0 {
            return (0, 0, 0);
        }

        let Some(classes_table) = game_data.get_table("classes") else {
            return (0, 0, 0);
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return (0, 0, 0);
        };

        let save_table_name = class_data.get("savingthrowtable").and_then(|s| s.as_ref());

        let Some(save_table_name) = save_table_name else {
            return (0, 0, 0);
        };

        let save_table_lower = save_table_name.to_lowercase();
        let Some(save_table) = game_data.get_table(&save_table_lower) else {
            return (0, 0, 0);
        };

        let row_index = (level - 1) as usize;
        let fort = save_table
            .get_cell(row_index, "fortsave")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let reflex = save_table
            .get_cell(row_index, "refsave")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);
        let will = save_table
            .get_cell(row_index, "willsave")
            .ok()
            .flatten()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        (fort, reflex, will)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;
    use indexmap::IndexMap;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("fortbonus".to_string(), GffValue::Short(5));
        fields.insert("refbonus".to_string(), GffValue::Short(3));
        fields.insert("willbonus".to_string(), GffValue::Short(7));
        Character::from_gff(fields)
    }

    #[test]
    fn test_base_fortitude() {
        let character = create_test_character();
        assert_eq!(character.base_fortitude(), 5);
    }

    #[test]
    fn test_base_reflex() {
        let character = create_test_character();
        assert_eq!(character.base_reflex(), 3);
    }

    #[test]
    fn test_base_will() {
        let character = create_test_character();
        assert_eq!(character.base_will(), 7);
    }

    #[test]
    fn test_save_bonuses() {
        let character = create_test_character();
        let saves = character.save_bonuses();
        assert_eq!(saves.fortitude, 5);
        assert_eq!(saves.reflex, 3);
        assert_eq!(saves.will, 7);
    }

    #[test]
    fn test_set_fortitude() {
        let mut character = create_test_character();
        assert!(!character.is_modified());

        character.set_fortitude(10).unwrap();
        assert!(character.is_modified());
        assert_eq!(character.base_fortitude(), 10);
    }

    #[test]
    fn test_set_reflex() {
        let mut character = create_test_character();
        character.set_reflex(8).unwrap();
        assert_eq!(character.base_reflex(), 8);
    }

    #[test]
    fn test_set_will() {
        let mut character = create_test_character();
        character.set_will(12).unwrap();
        assert_eq!(character.base_will(), 12);
    }

    #[test]
    fn test_set_fortitude_out_of_range_low() {
        let mut character = create_test_character();
        let result = character.set_fortitude(-50);
        assert!(result.is_err());
        assert_eq!(character.base_fortitude(), 5);
    }

    #[test]
    fn test_set_fortitude_out_of_range_high() {
        let mut character = create_test_character();
        let result = character.set_fortitude(300);
        assert!(result.is_err());
        assert_eq!(character.base_fortitude(), 5);
    }

    #[test]
    fn test_set_saves_boundary_values() {
        let mut character = create_test_character();

        character.set_fortitude(SAVE_MIN).unwrap();
        assert_eq!(character.base_fortitude(), SAVE_MIN);

        character.set_reflex(SAVE_MAX).unwrap();
        assert_eq!(character.base_reflex(), SAVE_MAX);

        character.set_will(0).unwrap();
        assert_eq!(character.base_will(), 0);
    }

    #[test]
    fn test_save_bonuses_default() {
        let bonuses = SaveBonuses::default();
        assert_eq!(bonuses.fortitude, 0);
        assert_eq!(bonuses.reflex, 0);
        assert_eq!(bonuses.will, 0);
    }

    #[test]
    fn test_save_bonuses_new() {
        let bonuses = SaveBonuses::new(5, 3, 7);
        assert_eq!(bonuses.fortitude, 5);
        assert_eq!(bonuses.reflex, 3);
        assert_eq!(bonuses.will, 7);
    }

    #[test]
    fn test_save_bonuses_with_missing_fields() {
        let character = Character::from_gff(IndexMap::new());
        let saves = character.save_bonuses();
        assert_eq!(saves.fortitude, 0);
        assert_eq!(saves.reflex, 0);
        assert_eq!(saves.will, 0);
    }
}
