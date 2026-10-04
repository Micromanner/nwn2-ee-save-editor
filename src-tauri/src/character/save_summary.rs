use crate::character::Character;
use crate::character::types::{FeatId, SaveBonuses, calculate_modifier};
use crate::loaders::GameData;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub enum SaveType {
    Fortitude,
    Reflex,
    Will,
}

impl SaveType {
    pub fn gff_field(&self) -> &'static str {
        match self {
            Self::Fortitude => "fortbonus",
            Self::Reflex => "refbonus",
            Self::Will => "willbonus",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Fortitude => "Fortitude",
            Self::Reflex => "Reflex",
            Self::Will => "Will",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct SavingThrows {
    pub fortitude: SaveBreakdown,
    pub reflex: SaveBreakdown,
    pub will: SaveBreakdown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct SaveBreakdown {
    pub total: i32,
    pub base: i32,
    pub ability: i32,
    pub equipment: i32,
    pub feat: i32,
    pub class_bonus: i32,
    pub misc: i32,
}

impl SaveBreakdown {
    pub fn calculate(
        base: i32,
        ability: i32,
        equipment: i32,
        feat: i32,
        class_bonus: i32,
        misc: i32,
    ) -> Self {
        Self {
            total: base + ability + equipment + feat + class_bonus + misc,
            base,
            ability,
            equipment,
            feat,
            class_bonus,
            misc,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct SaveSummary {
    pub fortitude: i32,
    pub reflex: i32,
    pub will: i32,
    pub saves: SavingThrows,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveChange {
    pub save_type: SaveType,
    pub old_misc: i32,
    pub new_misc: i32,
}

impl Character {
    pub fn get_save_summary(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> SaveSummary {
        let saves = self.get_saving_throws(game_data, decoder);

        SaveSummary {
            fortitude: saves.fortitude.total,
            reflex: saves.reflex.total,
            will: saves.will.total,
            saves,
        }
    }

    pub fn get_saving_throws(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> SavingThrows {
        // Base Saves: Derived exclusively from Class Levels (Heroic + Epic)
        let base_saves = self.calculate_base_saves(game_data);

        // Misc/Magic Bonuses: Stored in the GFF fields (fortbonus, refbonus, willbonus)
        let misc_bonuses = self.save_bonuses();

        let feat_bonuses = self.get_feat_save_bonuses(game_data);
        let item_bonuses = self.get_equipment_bonuses(game_data, decoder);
        let total_abilities = self.total_abilities_with_items(game_data, &item_bonuses);
        let class_bonuses = self.get_class_save_bonuses(total_abilities.cha);

        let con_mod = calculate_modifier(total_abilities.con);
        let dex_mod = calculate_modifier(total_abilities.dex);
        let wis_mod = calculate_modifier(total_abilities.wis);

        let fortitude = SaveBreakdown::calculate(
            base_saves.fortitude,
            con_mod,
            item_bonuses.fortitude_bonus,
            feat_bonuses.fortitude,
            class_bonuses.fortitude,
            misc_bonuses.fortitude,
        );

        let reflex = SaveBreakdown::calculate(
            base_saves.reflex,
            dex_mod,
            item_bonuses.reflex_bonus,
            feat_bonuses.reflex,
            class_bonuses.reflex,
            misc_bonuses.reflex,
        );

        let will = SaveBreakdown::calculate(
            base_saves.will,
            wis_mod,
            item_bonuses.will_bonus,
            feat_bonuses.will,
            class_bonuses.will,
            misc_bonuses.will,
        );

        SavingThrows {
            fortitude,
            reflex,
            will,
        }
    }

    /// Paladin Divine Grace and Blackguard Dark Blessing each add the positive CHA
    /// modifier to every save; with both feats the engine adds it twice.
    fn get_class_save_bonuses(&self, total_cha: i32) -> SaveBonuses {
        const FEAT_DIVINE_GRACE: FeatId = FeatId(217);
        const FEAT_DARK_BLESSING: FeatId = FeatId(473);

        let feats = [FEAT_DIVINE_GRACE, FEAT_DARK_BLESSING]
            .into_iter()
            .filter(|&feat| self.has_feat(feat))
            .count() as i32;
        let bonus = feats * calculate_modifier(total_cha).max(0);
        SaveBonuses::new(bonus, bonus, bonus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;
    use crate::parsers::tlk::TLKParser;
    use indexmap::IndexMap;
    use std::sync::{Arc, RwLock};

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("fortbonus".to_string(), GffValue::Short(5));
        fields.insert("refbonus".to_string(), GffValue::Short(3));
        fields.insert("willbonus".to_string(), GffValue::Short(7));
        fields.insert("Con".to_string(), GffValue::Byte(14));
        fields.insert("Dex".to_string(), GffValue::Byte(12));
        fields.insert("Wis".to_string(), GffValue::Byte(16));
        Character::from_gff(fields).expect("character from gff")
    }

    fn create_test_game_data() -> GameData {
        GameData::new(Arc::new(RwLock::new(TLKParser::default())))
    }

    fn create_test_decoder() -> crate::services::item_property_decoder::ItemPropertyDecoder {
        crate::services::item_property_decoder::ItemPropertyDecoder::default()
    }

    #[test]
    fn test_save_breakdown_calculate() {
        let breakdown = SaveBreakdown::calculate(5, 2, 1, 2, 2, 0);
        assert_eq!(breakdown.total, 12);
        assert_eq!(breakdown.base, 5);
        assert_eq!(breakdown.ability, 2);
    }

    #[test]
    fn test_divine_grace_and_dark_blessing_stack() {
        let mut character = create_test_character();
        assert_eq!(character.get_class_save_bonuses(18).fortitude, 0);

        character.add_feat(FeatId(217)).unwrap();
        assert_eq!(character.get_class_save_bonuses(18).will, 4);
        assert_eq!(character.get_class_save_bonuses(8).will, 0);

        character.add_feat(FeatId(473)).unwrap();
        assert_eq!(character.get_class_save_bonuses(18).reflex, 8);
    }

    #[test]
    fn test_save_type_fields() {
        assert_eq!(SaveType::Fortitude.gff_field(), "fortbonus");
        assert_eq!(SaveType::Reflex.gff_field(), "refbonus");
        assert_eq!(SaveType::Will.gff_field(), "willbonus");
    }

    #[test]
    fn test_get_saving_throws() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let decoder = create_test_decoder();

        let throws = character.get_saving_throws(&game_data, &decoder);

        assert_eq!(throws.fortitude.base, 0);
        assert_eq!(throws.fortitude.misc, 5); // from fortbonus GFF field
        assert_eq!(throws.fortitude.ability, 2);

        assert_eq!(throws.reflex.base, 0);
        assert_eq!(throws.reflex.misc, 3); // from refbonus GFF field
        assert_eq!(throws.reflex.ability, 1);

        assert_eq!(throws.will.base, 0);
        assert_eq!(throws.will.misc, 7); // from willbonus GFF field
        assert_eq!(throws.will.ability, 3);
    }

    #[test]
    fn test_get_save_summary() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let decoder = create_test_decoder();

        let summary = character.get_save_summary(&game_data, &decoder);

        assert!(summary.fortitude >= 2); // 0 misc + 2 ability
        assert!(summary.reflex >= 1); // 0 misc + 1 ability
        assert!(summary.will >= 3); // 0 misc + 3 ability
    }
}
