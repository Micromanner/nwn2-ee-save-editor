use crate::character::Character;
use crate::character::types::{AbilityIndex, ClassId, calculate_modifier};
use crate::loaders::GameData;
use serde::{Deserialize, Serialize};
use specta::Type;

const CLASS_ID_BARBARIAN: i32 = 0;
const CLASS_ID_MONK: i32 = 5;
const CLASS_ID_SACRED_FIST: i32 = 45;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ArmorClass {
    pub total: i32,
    pub touch: i32,
    pub flat_footed: i32,
    pub breakdown: ACBreakdown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ACBreakdown {
    pub base: i32,
    pub armor: i32,
    pub shield: i32,
    pub dex: i32,
    pub natural: i32,
    pub dodge: i32,
    pub deflection: i32,
    pub size: i32,
    pub misc: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AttackBonuses {
    #[serde(rename = "melee_attack_bonus")]
    pub melee: i32,
    #[serde(rename = "ranged_attack_bonus")]
    pub ranged: i32,
    #[serde(rename = "base_attack_bonus")]
    pub bab: i32,
    pub melee_breakdown: AttackBreakdown,
    pub ranged_breakdown: AttackBreakdown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct AttackBreakdown {
    pub base: i32,
    pub ability: i32,
    pub size: i32,
    pub equipment: i32,
    pub misc: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct Initiative {
    pub total: i32,
    pub dex: i32,
    pub feat: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CombatManeuverBonus {
    pub total: i32,
    pub bab: i32,
    pub str_mod: i32,
    pub size_mod: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct MovementSpeed {
    pub base: i32,
    pub current: i32,
    pub armor_penalty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DamageReduction {
    pub amount: i32,
    pub bypass: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NaturalArmorChange {
    pub old_value: i32,
    pub new_value: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct CombatSummary {
    pub armor_class: ArmorClass,
    pub attack_bonuses: AttackBonuses,
    pub initiative: Initiative,
    #[serde(rename = "base_attack_bonus")]
    pub bab: i32,
    pub attack_sequence: Vec<i32>,
    pub damage_bonuses: super::DamageBonuses,
    pub cmb: CombatManeuverBonus,
    pub movement: MovementSpeed,
    pub damage_reductions: Vec<DamageReduction>,
}

impl Character {
    pub fn get_combat_summary(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> CombatSummary {
        let bab = self.calculate_bab(game_data);
        let attack_sequence = self.get_attack_sequence(game_data);
        let damage_bonuses = self.get_damage_bonuses();

        let armor_class = self.get_armor_class(game_data, decoder);
        let attack_bonuses = self.get_attack_bonuses(game_data, decoder);
        let initiative = self.get_initiative_breakdown(game_data, decoder);
        let cmb = self.get_combat_maneuver_bonus(game_data, decoder);
        let movement = self.get_movement_speed(game_data);
        let damage_reductions = self.get_damage_reductions(game_data, decoder);

        CombatSummary {
            armor_class,
            attack_bonuses,
            initiative,
            bab,
            attack_sequence,
            damage_bonuses,
            cmb,
            movement,
            damage_reductions,
        }
    }

    pub fn get_damage_reductions(
        &self,
        game_data: &GameData,
        _decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> Vec<DamageReduction> {
        let mut reductions = Vec::new();

        let base_dr = self.damage_reduction();
        if base_dr > 0 {
            reductions.push(DamageReduction {
                amount: base_dr,
                bypass: "None".to_string(),
                source: "Base".to_string(),
            });
        }

        let barb_dr = self.get_barbarian_damage_reduction(game_data);
        if barb_dr > 0 {
            reductions.push(DamageReduction {
                amount: barb_dr,
                bypass: "-".to_string(),
                source: "Barbarian class".to_string(),
            });
        }

        reductions
    }

    pub fn get_armor_class(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> ArmorClass {
        let item_bonuses = self.get_equipment_bonuses(game_data, decoder);
        let dex_mod =
            calculate_modifier(self.base_ability(AbilityIndex::DEX) + item_bonuses.dex_bonus);
        let size_mod = self.size_modifier();
        let natural_ac = self.natural_ac();
        let feat_ac = self.get_feat_ac_bonuses(game_data);
        let class_ac = self.get_class_ac_bonus(game_data, item_bonuses.ac_shield_bonus > 0);

        let max_dex = self.get_equipped_armor_max_dex(game_data);
        let capped_dex = dex_mod.min(max_dex);

        let breakdown = ACBreakdown {
            base: 10,
            armor: item_bonuses.ac_armor_bonus,
            shield: item_bonuses.ac_shield_bonus,
            dex: capped_dex,
            natural: natural_ac + item_bonuses.ac_natural_bonus,
            dodge: item_bonuses.ac_dodge_bonus,
            deflection: item_bonuses.ac_deflection_bonus + item_bonuses.ac_bonus,
            size: size_mod,
            misc: feat_ac + class_ac,
        };

        let total = breakdown.base
            + breakdown.armor
            + breakdown.shield
            + breakdown.dex
            + breakdown.natural
            + breakdown.dodge
            + breakdown.deflection
            + breakdown.size
            + breakdown.misc;

        let touch = breakdown.base
            + breakdown.dex
            + breakdown.dodge
            + breakdown.deflection
            + breakdown.size
            + breakdown.misc;

        let flat_footed = breakdown.base
            + breakdown.armor
            + breakdown.shield
            + breakdown.natural
            + breakdown.deflection
            + breakdown.size
            + breakdown.misc;

        ArmorClass {
            total,
            touch,
            flat_footed,
            breakdown,
        }
    }

    /// Monk and Sacred Fist grant an AC bonus when unarmored and unshielded.
    /// Monk: WIS mod (if positive) + floor(monk_level / 5), unarmored only.
    /// Sacred Fist: +1 at lvl 1, +2 at lvl 5, +3 at lvl 10, light-or-no armor.
    fn get_class_ac_bonus(&self, game_data: &GameData, shield_equipped: bool) -> i32 {
        if shield_equipped {
            return 0;
        }
        let armor_rank = self.get_equipped_armor_rank(game_data);
        let unarmored = armor_rank == "None";
        let light_or_none = unarmored || armor_rank == "Light";

        let mut bonus = 0;

        let monk_level = self.get_class_level(ClassId(CLASS_ID_MONK));
        if monk_level > 0 && unarmored {
            let wis_mod = calculate_modifier(self.base_ability(AbilityIndex::WIS));
            bonus += wis_mod.max(0) + (monk_level / 5);
        }

        let sfist_level = self.get_class_level(ClassId(CLASS_ID_SACRED_FIST));
        if sfist_level > 0 && light_or_none {
            bonus += if sfist_level >= 10 {
                3
            } else if sfist_level >= 5 {
                2
            } else {
                1
            };
        }

        bonus
    }

    pub fn get_attack_bonuses(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> AttackBonuses {
        let bab = self.calculate_bab(game_data);
        let item_bonuses = self.get_equipment_bonuses(game_data, decoder);
        let str_mod =
            calculate_modifier(self.base_ability(AbilityIndex::STR) + item_bonuses.str_bonus);
        let dex_mod =
            calculate_modifier(self.base_ability(AbilityIndex::DEX) + item_bonuses.dex_bonus);
        let size_mod = self.size_modifier();

        let melee_breakdown = AttackBreakdown {
            base: bab,
            ability: str_mod,
            size: size_mod,
            equipment: item_bonuses.attack_bonus,
            misc: 0,
        };

        let ranged_breakdown = AttackBreakdown {
            base: bab,
            ability: dex_mod,
            size: size_mod,
            equipment: item_bonuses.attack_bonus,
            misc: 0,
        };

        let melee = melee_breakdown.base
            + melee_breakdown.ability
            + melee_breakdown.size
            + melee_breakdown.equipment
            + melee_breakdown.misc;

        let ranged = ranged_breakdown.base
            + ranged_breakdown.ability
            + ranged_breakdown.size
            + ranged_breakdown.equipment
            + ranged_breakdown.misc;

        AttackBonuses {
            melee,
            ranged,
            bab,
            melee_breakdown,
            ranged_breakdown,
        }
    }

    pub fn get_initiative_breakdown(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> Initiative {
        let item_bonuses = self.get_equipment_bonuses(game_data, decoder);
        let dex_mod =
            calculate_modifier(self.base_ability(AbilityIndex::DEX) + item_bonuses.dex_bonus);
        let feat_bonus = self.get_feat_initiative_bonus(game_data);

        Initiative {
            total: dex_mod + feat_bonus,
            dex: dex_mod,
            feat: feat_bonus,
        }
    }

    pub fn get_combat_maneuver_bonus(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> CombatManeuverBonus {
        let bab = self.calculate_bab(game_data);
        let item_bonuses = self.get_equipment_bonuses(game_data, decoder);
        let str_mod =
            calculate_modifier(self.base_ability(AbilityIndex::STR) + item_bonuses.str_bonus);
        let size_mod = self.size_modifier();

        CombatManeuverBonus {
            total: bab + str_mod - size_mod,
            bab,
            str_mod,
            size_mod,
        }
    }

    pub fn get_movement_speed(&self, game_data: &GameData) -> MovementSpeed {
        let base = self.get_i32("MovementRate").unwrap_or(30);
        let armor_rank = self.get_equipped_armor_rank(game_data);

        let armor_penalty = armor_rank == "Medium" || armor_rank == "Heavy";
        let mut current = if armor_penalty {
            (base as f32 * 0.75) as i32
        } else {
            base
        };

        let barb_level = self.get_class_level(ClassId(CLASS_ID_BARBARIAN));
        if barb_level > 0 {
            current += 10;
        }

        let monk_level = self.get_class_level(ClassId(CLASS_ID_MONK));
        if monk_level > 0 {
            current += (monk_level / 3) * 10;
        }

        MovementSpeed {
            base,
            current,
            armor_penalty,
        }
    }

    pub fn get_damage_reduction_list(&self, _game_data: &GameData) -> Vec<DamageReduction> {
        let dr = self.damage_reduction();
        if dr > 0 {
            vec![DamageReduction {
                amount: dr,
                bypass: "Magic".to_string(),
                source: "Base".to_string(),
            }]
        } else {
            vec![]
        }
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
        fields.insert("Str".to_string(), GffValue::Byte(16));
        fields.insert("Dex".to_string(), GffValue::Byte(14));
        fields.insert("NaturalAC".to_string(), GffValue::Byte(2));
        fields.insert("CreatureSize".to_string(), GffValue::Int(3)); // 3 = Medium in NWN2
        Character::from_gff(fields)
    }

    fn create_test_game_data() -> GameData {
        GameData::new(Arc::new(RwLock::new(TLKParser::default())))
    }

    fn create_test_decoder() -> crate::services::item_property_decoder::ItemPropertyDecoder {
        use crate::config::nwn2_paths::NWN2Paths;
        use crate::services::resource_manager::ResourceManager;
        use tauri::async_runtime::RwLock;
        let paths = Arc::new(RwLock::new(NWN2Paths::default()));
        let rm = Arc::new(RwLock::new(ResourceManager::new(paths)));
        crate::services::item_property_decoder::ItemPropertyDecoder::new(rm)
    }

    #[test]
    fn test_armor_class_calculation() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let decoder = create_test_decoder();

        let ac = character.get_armor_class(&game_data, &decoder);

        assert_eq!(ac.breakdown.base, 10);
        assert_eq!(ac.breakdown.dex, 2);
        assert_eq!(ac.breakdown.natural, 2);
        assert_eq!(ac.breakdown.size, 0);
    }

    #[test]
    fn test_attack_bonuses() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let decoder = create_test_decoder();

        let attacks = character.get_attack_bonuses(&game_data, &decoder);

        assert_eq!(attacks.melee_breakdown.ability, 3);
        assert_eq!(attacks.ranged_breakdown.ability, 2);
    }

    #[test]
    fn test_initiative_breakdown() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let decoder = create_test_decoder();

        let init = character.get_initiative_breakdown(&game_data, &decoder);

        assert_eq!(init.dex, 2);
        assert_eq!(init.feat, 0);
        assert_eq!(init.total, 2);
    }

    #[test]
    fn test_combat_summary() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let decoder = create_test_decoder();

        let summary = character.get_combat_summary(&game_data, &decoder);

        assert!(summary.armor_class.total >= 10);
        assert_eq!(summary.bab, 0);
        assert_eq!(summary.attack_sequence.len(), 1);
    }
}
