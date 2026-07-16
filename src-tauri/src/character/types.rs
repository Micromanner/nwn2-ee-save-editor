use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fmt;
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[repr(transparent)]
pub struct RaceId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[serde(transparent)]
#[repr(transparent)]
pub struct ClassId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[serde(transparent)]
#[repr(transparent)]
pub struct FeatId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[repr(transparent)]
pub struct SpellId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[repr(transparent)]
pub struct SkillId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[serde(transparent)]
#[repr(transparent)]
pub struct DomainId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[serde(transparent)]
#[repr(transparent)]
pub struct BackgroundId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[repr(transparent)]
pub struct ItemId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default, Type)]
#[repr(transparent)]
pub struct BaseItemId(pub i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[repr(transparent)]
pub struct AbilityIndex(pub u8);

impl AbilityIndex {
    pub const STR: Self = Self(0);
    pub const DEX: Self = Self(1);
    pub const CON: Self = Self(2);
    pub const INT: Self = Self(3);
    pub const WIS: Self = Self(4);
    pub const CHA: Self = Self(5);

    pub const fn all() -> [Self; 6] {
        [
            Self::STR,
            Self::DEX,
            Self::CON,
            Self::INT,
            Self::WIS,
            Self::CHA,
        ]
    }

    pub const fn gff_field(&self) -> &'static str {
        match self.0 {
            0 => "Str",
            1 => "Dex",
            2 => "Con",
            3 => "Int",
            4 => "Wis",
            5 => "Cha",
            _ => "Str",
        }
    }

    pub const fn name(&self) -> &'static str {
        match self.0 {
            0 => "Strength",
            1 => "Dexterity",
            2 => "Constitution",
            3 => "Intelligence",
            4 => "Wisdom",
            5 => "Charisma",
            _ => "Unknown",
        }
    }

    pub const fn short_name(&self) -> &'static str {
        match self.0 {
            0 => "STR",
            1 => "DEX",
            2 => "CON",
            3 => "INT",
            4 => "WIS",
            5 => "CHA",
            _ => "???",
        }
    }

    pub fn from_gff_field(field: &str) -> Option<Self> {
        match field.to_lowercase().as_str() {
            "str" | "strength" => Some(Self::STR),
            "dex" | "dexterity" => Some(Self::DEX),
            "con" | "constitution" => Some(Self::CON),
            "int" | "intelligence" => Some(Self::INT),
            "wis" | "wisdom" => Some(Self::WIS),
            "cha" | "charisma" => Some(Self::CHA),
            _ => None,
        }
    }

    pub fn from_index(index: u8) -> Option<Self> {
        if index < 6 { Some(Self(index)) } else { None }
    }
}

impl Default for AbilityIndex {
    fn default() -> Self {
        Self::STR
    }
}

impl fmt::Display for AbilityIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct AbilityScores {
    #[serde(rename = "Str")]
    pub str_: i32,
    #[serde(rename = "Dex")]
    pub dex: i32,
    #[serde(rename = "Con")]
    pub con: i32,
    #[serde(rename = "Int")]
    pub int: i32,
    #[serde(rename = "Wis")]
    pub wis: i32,
    #[serde(rename = "Cha")]
    pub cha: i32,
}

impl AbilityScores {
    pub fn new(str_: i32, dex: i32, con: i32, int: i32, wis: i32, cha: i32) -> Self {
        Self {
            str_,
            dex,
            con,
            int,
            wis,
            cha,
        }
    }

    pub fn get(&self, ability: AbilityIndex) -> i32 {
        match ability.0 {
            0 => self.str_,
            1 => self.dex,
            2 => self.con,
            3 => self.int,
            4 => self.wis,
            5 => self.cha,
            _ => 0,
        }
    }

    pub fn set(&mut self, ability: AbilityIndex, value: i32) {
        match ability.0 {
            0 => self.str_ = value,
            1 => self.dex = value,
            2 => self.con = value,
            3 => self.int = value,
            4 => self.wis = value,
            5 => self.cha = value,
            _ => {}
        }
    }

    pub fn to_map(&self) -> HashMap<String, i32> {
        let mut map = HashMap::new();
        map.insert("Str".to_string(), self.str_);
        map.insert("Dex".to_string(), self.dex);
        map.insert("Con".to_string(), self.con);
        map.insert("Int".to_string(), self.int);
        map.insert("Wis".to_string(), self.wis);
        map.insert("Cha".to_string(), self.cha);
        map
    }
}

impl Index<AbilityIndex> for AbilityScores {
    type Output = i32;

    fn index(&self, index: AbilityIndex) -> &Self::Output {
        match index.0 {
            0 => &self.str_,
            1 => &self.dex,
            2 => &self.con,
            3 => &self.int,
            4 => &self.wis,
            5 => &self.cha,
            _ => &self.str_,
        }
    }
}

impl IndexMut<AbilityIndex> for AbilityScores {
    fn index_mut(&mut self, index: AbilityIndex) -> &mut Self::Output {
        match index.0 {
            0 => &mut self.str_,
            1 => &mut self.dex,
            2 => &mut self.con,
            3 => &mut self.int,
            4 => &mut self.wis,
            5 => &mut self.cha,
            _ => &mut self.str_,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct AbilityModifiers {
    #[serde(rename = "Str")]
    pub str_mod: i32,
    #[serde(rename = "Dex")]
    pub dex_mod: i32,
    #[serde(rename = "Con")]
    pub con_mod: i32,
    #[serde(rename = "Int")]
    pub int_mod: i32,
    #[serde(rename = "Wis")]
    pub wis_mod: i32,
    #[serde(rename = "Cha")]
    pub cha_mod: i32,
}

impl AbilityModifiers {
    pub fn new(
        str_mod: i32,
        dex_mod: i32,
        con_mod: i32,
        int_mod: i32,
        wis_mod: i32,
        cha_mod: i32,
    ) -> Self {
        Self {
            str_mod,
            dex_mod,
            con_mod,
            int_mod,
            wis_mod,
            cha_mod,
        }
    }

    pub fn from_scores(scores: &AbilityScores) -> Self {
        Self {
            str_mod: calculate_modifier(scores.str_),
            dex_mod: calculate_modifier(scores.dex),
            con_mod: calculate_modifier(scores.con),
            int_mod: calculate_modifier(scores.int),
            wis_mod: calculate_modifier(scores.wis),
            cha_mod: calculate_modifier(scores.cha),
        }
    }

    pub fn get(&self, ability: AbilityIndex) -> i32 {
        match ability.0 {
            0 => self.str_mod,
            1 => self.dex_mod,
            2 => self.con_mod,
            3 => self.int_mod,
            4 => self.wis_mod,
            5 => self.cha_mod,
            _ => 0,
        }
    }

    pub fn set(&mut self, ability: AbilityIndex, value: i32) {
        match ability.0 {
            0 => self.str_mod = value,
            1 => self.dex_mod = value,
            2 => self.con_mod = value,
            3 => self.int_mod = value,
            4 => self.wis_mod = value,
            5 => self.cha_mod = value,
            _ => {}
        }
    }

    pub fn add(&mut self, other: &AbilityModifiers) {
        self.str_mod += other.str_mod;
        self.dex_mod += other.dex_mod;
        self.con_mod += other.con_mod;
        self.int_mod += other.int_mod;
        self.wis_mod += other.wis_mod;
        self.cha_mod += other.cha_mod;
    }
}

impl Index<AbilityIndex> for AbilityModifiers {
    type Output = i32;

    fn index(&self, index: AbilityIndex) -> &Self::Output {
        match index.0 {
            0 => &self.str_mod,
            1 => &self.dex_mod,
            2 => &self.con_mod,
            3 => &self.int_mod,
            4 => &self.wis_mod,
            5 => &self.cha_mod,
            _ => &self.str_mod,
        }
    }
}

impl IndexMut<AbilityIndex> for AbilityModifiers {
    fn index_mut(&mut self, index: AbilityIndex) -> &mut Self::Output {
        match index.0 {
            0 => &mut self.str_mod,
            1 => &mut self.dex_mod,
            2 => &mut self.con_mod,
            3 => &mut self.int_mod,
            4 => &mut self.wis_mod,
            5 => &mut self.cha_mod,
            _ => &mut self.str_mod,
        }
    }
}

pub fn calculate_modifier(score: i32) -> i32 {
    (score - 10).div_euclid(2)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Type)]
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

    pub fn add(&mut self, other: &SaveBonuses) {
        self.fortitude += other.fortitude;
        self.reflex += other.reflex;
        self.will += other.will;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct HitPoints {
    pub current: i32,
    pub max: i32,
    pub temp: i32,
}

impl HitPoints {
    pub fn new(current: i32, max: i32, temp: i32) -> Self {
        Self { current, max, temp }
    }

    pub fn effective_current(&self) -> i32 {
        self.current + self.temp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            valid: false,
            errors: vec![msg.into()],
            warnings: vec![],
        }
    }

    pub fn warning(msg: impl Into<String>) -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![msg.into()],
        }
    }

    pub fn add_error(&mut self, msg: impl Into<String>) {
        self.valid = false;
        self.errors.push(msg.into());
    }

    pub fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }

    pub fn merge(&mut self, other: ValidationResult) {
        if !other.valid {
            self.valid = false;
        }
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::ok()
    }
}

pub const ABILITY_MIN: i32 = 3;
pub const ABILITY_MAX: i32 = 50;
pub const NATURAL_AC_MIN: i32 = 0;
pub const NATURAL_AC_MAX: i32 = 255;
pub const SAVE_BONUS_MIN: i32 = -35;
pub const SAVE_BONUS_MAX: i32 = 255;
pub const ALIGNMENT_MIN: i32 = 0;
pub const ALIGNMENT_MAX: i32 = 100;
/// Values at or below this are Evil/Chaotic.
pub const ALIGNMENT_EVIL_THRESHOLD: i32 = 30;
/// Values at or above this are Good/Lawful.
pub const ALIGNMENT_GOOD_THRESHOLD: i32 = 70;

pub const MAX_TOTAL_LEVEL: i32 = 60;
pub const MAX_CLASSES: usize = 4;
pub const HEROIC_LEVEL_CAP: i32 = 20;
pub const ABILITY_INCREASE_INTERVAL: i32 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_index_gff_field() {
        assert_eq!(AbilityIndex::STR.gff_field(), "Str");
        assert_eq!(AbilityIndex::DEX.gff_field(), "Dex");
        assert_eq!(AbilityIndex::CON.gff_field(), "Con");
        assert_eq!(AbilityIndex::INT.gff_field(), "Int");
        assert_eq!(AbilityIndex::WIS.gff_field(), "Wis");
        assert_eq!(AbilityIndex::CHA.gff_field(), "Cha");
    }

    #[test]
    fn test_ability_index_from_gff_field() {
        assert_eq!(AbilityIndex::from_gff_field("str"), Some(AbilityIndex::STR));
        assert_eq!(
            AbilityIndex::from_gff_field("Strength"),
            Some(AbilityIndex::STR)
        );
        assert_eq!(AbilityIndex::from_gff_field("invalid"), None);
    }

    #[test]
    fn test_calculate_modifier() {
        assert_eq!(calculate_modifier(10), 0);
        assert_eq!(calculate_modifier(11), 0);
        assert_eq!(calculate_modifier(12), 1);
        assert_eq!(calculate_modifier(16), 3);
        assert_eq!(calculate_modifier(18), 4);
        assert_eq!(calculate_modifier(8), -1);
        assert_eq!(calculate_modifier(6), -2);
        assert_eq!(calculate_modifier(1), -5);
    }

    #[test]
    fn test_ability_scores_index() {
        let scores = AbilityScores::new(16, 14, 12, 10, 8, 10);
        assert_eq!(scores[AbilityIndex::STR], 16);
        assert_eq!(scores[AbilityIndex::DEX], 14);
        assert_eq!(scores[AbilityIndex::WIS], 8);
    }

    #[test]
    fn test_ability_modifiers_from_scores() {
        let scores = AbilityScores::new(16, 14, 12, 10, 8, 10);
        let mods = AbilityModifiers::from_scores(&scores);
        assert_eq!(mods.str_mod, 3);
        assert_eq!(mods.dex_mod, 2);
        assert_eq!(mods.con_mod, 1);
        assert_eq!(mods.int_mod, 0);
        assert_eq!(mods.wis_mod, -1);
        assert_eq!(mods.cha_mod, 0);
    }

    #[test]
    fn test_validation_result_merge() {
        let mut result = ValidationResult::ok();
        assert!(result.is_valid());

        result.add_warning("Some warning");
        assert!(result.is_valid());

        result.add_error("Some error");
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_save_bonuses_add() {
        let mut a = SaveBonuses::new(1, 2, 3);
        let b = SaveBonuses::new(4, 5, 6);
        a.add(&b);
        assert_eq!(a.fortitude, 5);
        assert_eq!(a.reflex, 7);
        assert_eq!(a.will, 9);
    }

    #[test]
    fn test_hit_points_effective() {
        let hp = HitPoints::new(50, 100, 10);
        assert_eq!(hp.effective_current(), 60);
    }
}
