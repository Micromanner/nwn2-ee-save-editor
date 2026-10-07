use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use tracing::{debug, trace};

use super::classes::EPIC_LEVEL_THRESHOLD;
use super::gff_helpers::gff_value_to_i32;
use super::types::{
    ABILITY_MAX, ABILITY_MIN, AbilityIndex, AbilityScores, BackgroundId, ClassId, DomainId, FeatId,
    SaveBonuses, SkillId,
};
use super::{Character, CharacterError};
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::{row_bool, row_int, row_str};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Type)]
#[repr(transparent)]
pub struct FeatType(pub i32);

impl Serialize for FeatType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i32(self.0)
    }
}

impl<'de> Deserialize<'de> for FeatType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = i32::deserialize(deserializer)?;
        Ok(FeatType(value))
    }
}

impl FeatType {
    pub const GENERAL: Self = Self(1);
    pub const PROFICIENCY: Self = Self(2);
    pub const SKILL_SAVE: Self = Self(4);
    pub const METAMAGIC: Self = Self(8);
    pub const DIVINE: Self = Self(16);
    pub const EPIC: Self = Self(32);
    pub const CLASS: Self = Self(64);
    pub const BACKGROUND: Self = Self(128);
    pub const SPELLCASTING: Self = Self(256);
    pub const HISTORY: Self = Self(512);
    pub const HERITAGE: Self = Self(1024);
    pub const ITEM_CREATION: Self = Self(2048);
    pub const RACIAL: Self = Self(4096);
    pub const DOMAIN: Self = Self(8192);

    pub fn contains(&self, flag: FeatType) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn from_string(s: &str) -> Self {
        let upper = s.to_uppercase();
        let value = match upper.as_str() {
            "GENERAL" | "GENERAL_FT_CAT" | "TEAMWORK" | "TEAMWORK_FT_CAT" => 1,
            "PROFICIENCY" | "COMBAT" | "PROFICIENCY_FT_CAT" | "COMBAT_FT_CAT" => 2,
            "SKILLNSAVE" | "SKILL" | "SAVE" | "SKILLNSAVE_FT_CAT" | "SKILL_FT_CAT" => 4,
            "METAMAGIC" | "METAMAGIC_FT_CAT" => 8,
            "DIVINE" | "SPECIAL" | "DIVINE_FT_CAT" | "SPECIAL_FT_CAT" => 16,
            "EPIC" | "EPIC_FT_CAT" => 32,
            "CLASSABILITY" | "CLASS" | "CLASSABILITY_FT_CAT" | "CLASS_FT_CAT" => 64,
            "BACKGROUND" | "BACKGROUND_FT_CAT" => 128,
            "SPELLCASTING" | "SPELLCASTING_FT_CAT" => 256,
            "HISTORY" | "HISTORY_FT_CAT" => 512,
            "HERITAGE" | "HERITAGE_FT_CAT" => 1024,
            "ITEMCREATION" | "ITEM" | "ITEMCREATION_FT_CAT" => 2048,
            "RACIALABILITY" | "RACIAL" | "RACIALABILITY_FT_CAT" | "RACIAL_FT_CAT" => 4096,
            "DOMAIN" | "DOMAIN_FT_CAT" => 8192,
            _ => {
                if upper.contains("PROFICIENCY") || upper.contains("COMBAT") {
                    2
                } else if upper.contains("EPIC") {
                    32
                } else if upper.contains("METAMAGIC") {
                    8
                } else if upper.contains("CLASS") {
                    64
                } else if upper.contains("SKILL") || upper.contains("SAVE") {
                    4
                } else if upper.contains("DIVINE") || upper.contains("SPECIAL") {
                    16
                } else if upper.contains("SPELL") {
                    256
                } else if upper.contains("RACIAL") {
                    4096
                } else if upper.contains("ITEM") {
                    2048
                } else if upper.contains("BACKGROUND") {
                    128
                } else if upper.contains("HISTORY") {
                    512
                } else if upper.contains("HERITAGE") {
                    1024
                } else if upper.contains("DOMAIN") {
                    8192
                } else if upper.contains("GENERAL") || upper.contains("TEAMWORK") {
                    1
                } else {
                    match s.parse::<i32>() {
                        Ok(n) if (1..=13).contains(&n) => Self::from_category_int(n).0,
                        Ok(n) => n,
                        Err(_) => 0,
                    }
                }
            }
        };
        Self(value)
    }

    /// Maps NWN2 sequential TOOLCATEGORIES integer IDs to bitmask values.
    fn from_category_int(id: i32) -> Self {
        match id {
            1 => Self::GENERAL,
            2 => Self::PROFICIENCY,
            3 => Self::METAMAGIC,
            4 => Self::DIVINE,
            5 => Self::CLASS,
            6 => Self::EPIC,
            7 => Self::ITEM_CREATION,
            8 => Self::SPELLCASTING,
            9 => Self::BACKGROUND,
            10 => Self::HISTORY,
            11 => Self::HERITAGE,
            12 => Self::RACIAL,
            13 => Self::SKILL_SAVE,
            _ => Self(0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Type)]
pub enum FeatCategory {
    #[default]
    Unknown,
    General,
    Proficiency,
    SkillSave,
    Metamagic,
    Divine,
    Epic,
    Class,
    Background,
    Spellcasting,
    History,
    Heritage,
    ItemCreation,
    Racial,
    Domain,
}

impl FeatCategory {
    pub fn from_feat_type(feat_type: FeatType, is_domain: bool) -> Self {
        if is_domain {
            return Self::Domain;
        }

        if feat_type.contains(FeatType::EPIC) {
            Self::Epic
        } else if feat_type.contains(FeatType::BACKGROUND) {
            Self::Background
        } else if feat_type.contains(FeatType::HISTORY) {
            Self::History
        } else if feat_type.contains(FeatType::HERITAGE) {
            Self::Heritage
        } else if feat_type.contains(FeatType::RACIAL) {
            Self::Racial
        } else if feat_type.contains(FeatType::CLASS) {
            Self::Class
        } else if feat_type.contains(FeatType::METAMAGIC) {
            Self::Metamagic
        } else if feat_type.contains(FeatType::DIVINE) {
            Self::Divine
        } else if feat_type.contains(FeatType::ITEM_CREATION) {
            Self::ItemCreation
        } else if feat_type.contains(FeatType::SPELLCASTING) {
            Self::Spellcasting
        } else if feat_type.contains(FeatType::SKILL_SAVE) {
            Self::SkillSave
        } else if feat_type.contains(FeatType::PROFICIENCY) {
            Self::Proficiency
        } else if feat_type.contains(FeatType::GENERAL) {
            Self::General
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FeatInfo {
    pub id: FeatId,
    pub label: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    #[serde(rename = "type")]
    pub feat_type: FeatType,
    pub category: FeatCategory,
    #[serde(rename = "protected")]
    pub is_protected: bool,
    #[serde(rename = "custom")]
    pub is_custom: bool,
    pub has_feat: bool,
    pub prerequisites: FeatPrerequisites,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct FeatPrerequisites {
    pub feats: Vec<PrereqFeat>,
    /// `OrReqFeat0-5`: any one of them is enough.
    pub any_feats: Vec<PrereqFeat>,
    pub abilities: Vec<PrereqAbility>,
    pub bab: Option<PrereqValue>,
    pub level: Option<PrereqValue>,
    pub class_level: Option<PrereqClassLevel>,
    /// `MaxLevel`: met while the total level is at most `required`.
    pub max_level: Option<PrereqValue>,
    pub epic: Option<PrereqValue>,
    pub base_fortitude: Option<PrereqValue>,
    pub caster_level: Option<PrereqValue>,
    pub spell_level: Option<PrereqValue>,
    pub skills: Vec<PrereqSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrereqClassLevel {
    pub class_name: String,
    #[serde(flatten)]
    pub level: PrereqValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrereqValue {
    pub required: i32,
    pub current: i32,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrereqSkill {
    pub skill: String,
    pub required: i32,
    pub current: i32,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrereqFeat {
    pub id: FeatId,
    pub name: String,
    pub met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PrereqAbility {
    pub ability: String,
    pub required: i32,
    pub current: i32,
    pub met: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct FeatSummary {
    pub total: i32,
    pub class_feats: Vec<FeatInfo>,
    pub general_feats: Vec<FeatInfo>,
    pub custom_feats: Vec<FeatInfo>,
    pub background_feats: Vec<FeatInfo>,
    pub domain_feats: Vec<FeatInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DomainInfo {
    pub id: DomainId,
    pub name: String,
    pub description: String,
    pub granted_feat: Option<FeatId>,
    pub is_castable: bool,
    pub epithet_feat: Option<FeatId>,
    pub has_domain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Type)]
pub enum FeatSource {
    #[default]
    Unknown,
    Manual,
    Class,
    Race,
    Domain,
    Level,
    Background,
}

impl FeatSource {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "manual" => Self::Manual,
            "class" => Self::Class,
            "race" | "racial" => Self::Race,
            "domain" => Self::Domain,
            "level" => Self::Level,
            "background" => Self::Background,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Manual => "manual",
            Self::Class => "class",
            Self::Race => "race",
            Self::Domain => "domain",
            Self::Level => "level",
            Self::Background => "background",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FeatEntry {
    pub feat_id: FeatId,
    pub source: FeatSource,
    pub uses: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct FeatSlots {
    pub total_general_slots: i32,
    pub total_bonus_slots: i32,
    pub total_slots: i32,
    pub filled_slots: i32,
    pub open_slots: i32,
    pub open_general_slots: i32,
    pub open_bonus_slots: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct FeatAddResult {
    pub feat_id: FeatId,
    pub auto_added_feats: Vec<AutoAddedFeat>,
    pub auto_modified_abilities: Vec<AbilityChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AutoAddedFeat {
    pub feat_id: FeatId,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AbilityChange {
    pub ability: String,
    pub old_value: i32,
    pub new_value: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct FeatsState {
    pub summary: FeatSummary,
    pub feat_slots: FeatSlots,
    pub domains: Vec<DomainInfo>,
}

enum SaveType {
    Universal,
    Fortitude,
    Reflex,
    Will,
    FortitudeAndWill,
}

static SAVE_PATTERNS: LazyLock<Vec<(Regex, SaveType)>> = LazyLock::new(|| {
    vec![
        (
            Regex::new(r"([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+all\s+(?:saving\s+throws|saves)")
                .unwrap(),
            SaveType::Universal,
        ),
        (
            Regex::new(r"([+-]\d+)\s+(?:to\s+)?all\s+(?:saving\s+throws|saves)").unwrap(),
            SaveType::Universal,
        ),
        (
            Regex::new(
                r"([+-]\d+)\s+(?:bonus\s+)?(?:to|on)\s+Fortitude\s+and\s+Will\s+(?:saving\s+throws|saves?)",
            )
            .unwrap(),
            SaveType::FortitudeAndWill,
        ),
        (
            Regex::new(
                r"([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+(?:all\s+)?Fortitude\s+(?:saving\s+throws|saves?)",
            )
            .unwrap(),
            SaveType::Fortitude,
        ),
        (
            Regex::new(r"([+-]\d+)\s+Fortitude\s+Save").unwrap(),
            SaveType::Fortitude,
        ),
        (
            Regex::new(
                r"([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+(?:all\s+)?Reflex\s+(?:saving\s+throws|saves?)",
            )
            .unwrap(),
            SaveType::Reflex,
        ),
        (
            Regex::new(r"([+-]\d+)\s+Reflex\s+Save").unwrap(),
            SaveType::Reflex,
        ),
        (
            Regex::new(
                r"([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+(?:all\s+)?Will\s+(?:saving\s+throws|saves?)",
            )
            .unwrap(),
            SaveType::Will,
        ),
        (
            Regex::new(r"([+-]\d+)\s+(?:to|on)\s+all\s+Will\s+(?:saving\s+throws|saves?)").unwrap(),
            SaveType::Will,
        ),
        (
            Regex::new(r"([+-]\d+)\s+Will\s+Save").unwrap(),
            SaveType::Will,
        ),
        (
            Regex::new(r"([+-]\d+)\s+racial\s+(?:\w+\s+)?bonus\s+on\s+all\s+saving\s+throws").unwrap(),
            SaveType::Universal,
        ),
    ]
});

static AC_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\+(\d+)\s+(?:\w+\s+)?bonus\s+to\s+(?:Armor\s+Class|AC)").unwrap(),
        Regex::new(r"\+(\d+)\s+(?:to\s+)?AC(?:\s|\.|\,)").unwrap(),
        Regex::new(r"\+(\d+)\s+AC\s+bonus").unwrap(),
    ]
});

static AC_DODGE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\+(\d+)").unwrap());

static INITIATIVE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"\+(\d+)\s+(?:\w+\s+)?bonus\s+to\s+initiative").unwrap(),
        Regex::new(r"\+(\d+)\s+(?:to\s+)?initiative").unwrap(),
    ]
});

/// Matches NWN2 EE epithet feat labels like `FEAT_EPITHET_STRENGTH_DOMAIN` whose
/// domain row in `domains.2da` left `epithetfeat` blank. Captures the domain label.
static DOMAIN_EPITHET_LABEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?i)FEAT_EPITHET_(.+?)_DOMAIN$").expect("Invalid domain epithet regex")
});

const SAVE_CONDITIONAL_KEYWORDS: &[&str] = &[
    "against",
    "vs ",
    "versus",
    "to avoid",
    "made to",
    "while raging",
    "while in a rage",
    "while in a frenzy",
    "during rage",
    "during frenzy",
    "when raging",
];

/// Keywords in feat descriptions indicating bonuses require a temporary active state.
const ACTIVATION_CONDITIONAL_KEYWORDS: &[&str] = &[
    "while raging",
    "while in a rage",
    "while in a frenzy",
    "during rage",
    "during frenzy",
    "when raging",
    "while in defensive stance",
    "during defensive stance",
    "while frenzied",
    "when frenzied",
];
const AC_CONDITIONAL_KEYWORDS: &[&str] = &[
    "against",
    "vs ",
    "versus",
    "when ",
    "while ",
    "if ",
    "when wielding",
    "when wearing",
    "when using",
    "when fighting",
];

/// Determines if a feat's bonuses are conditional (only apply in specific situations).
/// Uses description keywords to detect conditional bonuses. Works with modded feats.
/// Note: IsActive+SPELLID is NOT used as a standalone filter because permanent feats
/// like Blessed of Waukeen also use spells to apply their effects.
fn is_conditional_feat(
    _feat_data: &ahash::AHashMap<String, Option<String>>,
    description_lower: &str,
    context_keywords: &[&str],
) -> bool {
    if ACTIVATION_CONDITIONAL_KEYWORDS
        .iter()
        .any(|kw| description_lower.contains(kw))
    {
        return true;
    }

    context_keywords
        .iter()
        .any(|kw| description_lower.contains(kw))
}

/// `feat.2da` minimum score columns in `AbilityIndex::all()` order.
const ABILITY_MIN_COLUMNS: [&str; 6] = ["minstr", "mindex", "mincon", "minint", "minwis", "mincha"];

const OR_REQ_FEAT_COLUMNS: [&str; 6] = [
    "orreqfeat0",
    "orreqfeat1",
    "orreqfeat2",
    "orreqfeat3",
    "orreqfeat4",
    "orreqfeat5",
];

/// Epithet feat of every domain, including those `domains.2da` leaves blank.
pub fn domain_epithet_feats(game_data: &GameData) -> HashSet<i32> {
    let Some(domains_table) = game_data.get_table("domains") else {
        return HashSet::new();
    };

    (0..domains_table.row_count())
        .filter_map(|row_id| domains_table.get_by_id(row_id as i32))
        .filter_map(|domain_data| Character::resolve_domain_epithet_feat(&domain_data, game_data))
        .map(|feat_id| feat_id.0)
        .collect()
}

/// Obsidian never added a DOMAIN_FT_CAT; all domain abilities/epithets use
/// CLASSABILITY_FT_CAT. We bucket a feat as Domain when any of:
/// (a) the character acquired it via FeatSource::Domain, (b) it's an epithet
/// referenced in domains.2da, or (c) its label names a domain while being a
/// class ability — this catches orphan epithets that feat.2da defines but
/// domains.2da never wires up.
fn compute_is_domain(
    feat_type: FeatType,
    label: &str,
    from_domain_source: bool,
    is_epithet: bool,
) -> bool {
    if is_epithet {
        return true;
    }
    if !feat_type.contains(FeatType::CLASS) {
        return false;
    }
    from_domain_source || label.to_uppercase().contains("DOMAIN")
}

/// Character values that feat prerequisites compare against, computed on first use and
/// shared by every feat of one listing.
#[derive(Default)]
struct PrereqCache {
    abilities: OnceCell<AbilityScores>,
    bab: OnceCell<i32>,
    base_fortitude: OnceCell<i32>,
    caster_level: OnceCell<i32>,
    max_spell_level: OnceCell<i32>,
}

/// Lookups shared by every `FeatInfo` of one listing, so domain and ownership checks
/// do not rescan the 2DAs per feat.
pub struct FeatListContext {
    epithet_feats: HashSet<i32>,
    feat_sources: HashMap<FeatId, FeatSource>,
    cache: PrereqCache,
}

impl FeatListContext {
    pub fn new(character: &Character, game_data: &GameData) -> Self {
        Self {
            epithet_feats: domain_epithet_feats(game_data),
            feat_sources: character
                .feat_entries()
                .into_iter()
                .map(|e| (e.feat_id, e.source))
                .collect(),
            cache: PrereqCache::default(),
        }
    }
}

impl Character {
    // ========== BASIC FEAT ACCESS ==========

    pub fn feat_ids(&self) -> Vec<FeatId> {
        self.feat_entries()
            .into_iter()
            .map(|entry| entry.feat_id)
            .collect()
    }

    pub fn feat_entries(&self) -> Vec<FeatEntry> {
        let Some(feat_list) = self.get_list("FeatList") else {
            return Vec::new();
        };

        feat_list
            .iter()
            .filter_map(|entry| {
                let feat_id = entry.get("Feat").and_then(gff_value_to_i32)?;
                let source = entry
                    .get("Source")
                    .and_then(|v| match v {
                        GffValue::String(s) => Some(FeatSource::parse(s)),
                        GffValue::ResRef(s) => Some(FeatSource::parse(s)),
                        _ => None,
                    })
                    .unwrap_or(FeatSource::Unknown);
                let uses = entry.get("Uses").and_then(gff_value_to_i32);

                Some(FeatEntry {
                    feat_id: FeatId(feat_id),
                    source,
                    uses,
                })
            })
            .collect()
    }

    pub fn has_feat(&self, feat_id: FeatId) -> bool {
        let Some(feat_list) = self.get_list("FeatList") else {
            return false;
        };

        feat_list.iter().any(|entry| {
            entry
                .get("Feat")
                .and_then(gff_value_to_i32)
                .is_some_and(|id| id == feat_id.0)
        })
    }

    pub fn feat_count(&self) -> usize {
        self.get_list("FeatList").map_or(0, std::vec::Vec::len)
    }

    pub fn add_feat(&mut self, feat_id: FeatId) -> Result<(), CharacterError> {
        self.add_feat_with_source(feat_id, FeatSource::Manual)
    }

    pub fn add_feat_with_source(
        &mut self,
        feat_id: FeatId,
        source: FeatSource,
    ) -> Result<(), CharacterError> {
        if self.has_feat(feat_id) {
            return Err(CharacterError::FeatAlreadyExists(feat_id.0));
        }

        let mut feat_list = self.get_list_owned("FeatList").unwrap_or_default();

        let mut new_entry = IndexMap::new();
        new_entry.insert("__struct_id__".to_string(), GffValue::Dword(1));
        new_entry.insert("Feat".to_string(), GffValue::Word(feat_id.0 as u16));

        if source != FeatSource::Unknown {
            new_entry.insert(
                "Source".to_string(),
                GffValue::String(source.as_str().into()),
            );
        }

        feat_list.push(new_entry);
        self.set_list("FeatList", feat_list);

        self.record_feat_change(feat_id, true);

        Ok(())
    }

    /// Add a feat together with its missing `PREREQFEAT1/2` chain, raising stored ability
    /// scores just enough that the effective (stored + racial) score meets every minimum.
    /// The whole change is planned before anything is written, so an error leaves the
    /// character untouched.
    pub fn add_feat_with_prerequisites(
        &mut self,
        feat_id: FeatId,
        source: FeatSource,
        game_data: &GameData,
    ) -> Result<FeatAddResult, CharacterError> {
        let mut result = FeatAddResult {
            feat_id,
            auto_added_feats: vec![],
            auto_modified_abilities: vec![],
        };

        if self.has_feat(feat_id) {
            return Ok(result);
        }

        let feats_table = game_data
            .get_table("feat")
            .ok_or_else(|| CharacterError::TableNotFound("feat".to_string()))?;

        let mut visited = HashSet::new();
        let mut to_add = Vec::new();
        let mut ability_minimums = [0; 6];
        let mut stack = vec![(feat_id, false)];
        while let Some((current, children_done)) = stack.pop() {
            if children_done {
                to_add.push(current);
                continue;
            }
            if !visited.insert(current) {
                continue;
            }
            let feat_data = feats_table
                .get_by_id(current.0)
                .ok_or(CharacterError::NotFound {
                    entity: "Feat",
                    id: current.0,
                })?;

            for (ability, column) in AbilityIndex::all().into_iter().zip(ABILITY_MIN_COLUMNS) {
                let minimum = &mut ability_minimums[ability.0 as usize];
                *minimum = (*minimum).max(row_int(&feat_data, column, 0));
            }

            stack.push((current, true));
            for column in ["prereqfeat2", "prereqfeat1"] {
                let prereq = FeatId(row_int(&feat_data, column, -1));
                if prereq.0 >= 0 && !self.has_feat(prereq) && !visited.contains(&prereq) {
                    stack.push((prereq, false));
                }
            }
        }

        let effective = self.get_effective_abilities(game_data);
        let raises: Vec<(AbilityIndex, i32, i32)> = AbilityIndex::all()
            .into_iter()
            .filter_map(|ability| {
                let missing = ability_minimums[ability.0 as usize] - effective.get(ability);
                let old_value = self.base_ability(ability);
                (missing > 0).then_some((ability, old_value, old_value + missing))
            })
            .collect();
        if let Some(&(ability, _, value)) =
            raises.iter().find(|&&(_, _, value)| value > ABILITY_MAX)
        {
            return Err(CharacterError::OutOfRange {
                field: ability.gff_field(),
                value,
                min: ABILITY_MIN,
                max: ABILITY_MAX,
            });
        }

        for (ability, old_value, new_value) in raises {
            self.set_ability_with_cascades(ability, new_value, game_data)?;
            result.auto_modified_abilities.push(AbilityChange {
                ability: ability.gff_field().to_string(),
                old_value,
                new_value,
            });
        }

        for added in to_add {
            if added == feat_id {
                self.add_feat_with_source(added, source)?;
            } else {
                self.add_feat_with_source(added, FeatSource::Manual)?;
                result.auto_added_feats.push(AutoAddedFeat {
                    feat_id: added,
                    label: self.get_feat_name(added, game_data),
                });
            }
        }

        Ok(result)
    }

    pub fn remove_feat(&mut self, feat_id: FeatId) -> Result<(), CharacterError> {
        if !self.has_feat(feat_id) {
            return Err(CharacterError::FeatNotFound(feat_id.0));
        }

        let mut feat_list = self.get_list_owned("FeatList").unwrap_or_default();

        feat_list.retain(|entry| {
            entry
                .get("Feat")
                .and_then(gff_value_to_i32)
                .map_or(true, |id| id != feat_id.0)
        });

        self.set_list("FeatList", feat_list);

        self.record_feat_change(feat_id, false);

        Ok(())
    }

    // ========== FEAT SOURCE TRACKING ==========

    pub fn feat_source(&self, feat_id: FeatId) -> Option<FeatSource> {
        let entries = self.feat_entries();
        entries
            .iter()
            .find(|e| e.feat_id == feat_id)
            .map(|e| e.source)
    }

    // ========== DOMAIN METHODS ==========

    pub fn get_character_domains(&self) -> Vec<DomainId> {
        let mut domains = Vec::new();

        let Some(class_list) = self.get_list("ClassList") else {
            trace!("get_character_domains: No ClassList found");
            return domains;
        };

        trace!(
            "get_character_domains: Found {} class entries",
            class_list.len()
        );

        for (idx, class_entry) in class_list.iter().enumerate() {
            let domain1_raw = class_entry.get("Domain1");
            let domain2_raw = class_entry.get("Domain2");
            trace!(
                "get_character_domains: Class {} - Domain1 raw: {:?}, Domain2 raw: {:?}",
                idx, domain1_raw, domain2_raw
            );

            if let Some(domain1) = class_entry.get("Domain1").and_then(gff_value_to_i32)
                && domain1 >= 0
            {
                trace!("get_character_domains: Adding Domain1 = {}", domain1);
                domains.push(DomainId(domain1));
            }
            if let Some(domain2) = class_entry.get("Domain2").and_then(gff_value_to_i32)
                && domain2 >= 0
            {
                trace!("get_character_domains: Adding Domain2 = {}", domain2);
                domains.push(DomainId(domain2));
            }
        }

        trace!("get_character_domains: Total domains found: {:?}", domains);
        domains
    }

    /// Resolves the epithet feat for a domain, falling back to a `feat.2da` label
    /// search when `domains.2da` left `epithetfeat` blank (Strength, Death, Evil,
    /// Healing, Protection, Sun, Fury).
    fn resolve_domain_epithet_feat(
        domain_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> Option<FeatId> {
        if let Some(feat_id) = domain_data
            .get("epithetfeat")
            .and_then(|s| s.as_ref()?.parse::<i32>().ok())
            .filter(|&id| id >= 0)
        {
            return Some(FeatId(feat_id));
        }

        let domain_label = row_str(domain_data, "label")?;
        let feat_table = game_data.get_table("feat")?;
        let expected = format!("FEAT_EPITHET_{}_DOMAIN", domain_label.to_ascii_uppercase());
        (0..feat_table.row_count())
            .find(|&row| {
                feat_table
                    .get_cell(row, "label")
                    .ok()
                    .flatten()
                    .is_some_and(|label| label.eq_ignore_ascii_case(&expected))
            })
            .map(|row| FeatId(row as i32))
    }

    /// The `grantedfeat` and epithet feat of a `domains.2da` row.
    fn domain_feats(
        domain_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> (Option<FeatId>, Option<FeatId>) {
        let granted_feat = domain_data
            .get("grantedfeat")
            .and_then(|s| s.as_ref()?.parse::<i32>().ok())
            .filter(|&id| id >= 0)
            .map(FeatId);
        (
            granted_feat,
            Self::resolve_domain_epithet_feat(domain_data, game_data),
        )
    }

    pub fn add_domain(
        &mut self,
        domain_id: DomainId,
        game_data: &GameData,
    ) -> Result<Vec<FeatId>, CharacterError> {
        let Some(domains_table) = game_data.get_table("domains") else {
            return Err(CharacterError::TableNotFound("domains".to_string()));
        };

        let Some(domain_data) = domains_table.get_by_id(domain_id.0) else {
            return Err(CharacterError::NotFound {
                entity: "Domain",
                id: domain_id.0,
            });
        };

        self.assign_domain_slot(domain_id, game_data)?;

        let (granted_feat, epithet_feat) = Self::domain_feats(&domain_data, game_data);

        let mut added_feats = Vec::new();
        for feat_id in [granted_feat, epithet_feat].into_iter().flatten() {
            if !self.has_feat(feat_id) {
                self.add_feat_with_source(feat_id, FeatSource::Domain)?;
                added_feats.push(feat_id);
            }
        }

        Ok(added_feats)
    }

    pub fn remove_domain(
        &mut self,
        domain_id: DomainId,
        game_data: &GameData,
    ) -> Result<Vec<FeatId>, CharacterError> {
        let Some(domains_table) = game_data.get_table("domains") else {
            return Err(CharacterError::TableNotFound("domains".to_string()));
        };

        let Some(domain_data) = domains_table.get_by_id(domain_id.0) else {
            return Err(CharacterError::NotFound {
                entity: "Domain",
                id: domain_id.0,
            });
        };

        self.clear_domain_slot(domain_id);

        let (granted_feat, epithet_feat) = Self::domain_feats(&domain_data, game_data);

        let mut removed_feats = Vec::new();
        for feat_id in [granted_feat, epithet_feat].into_iter().flatten() {
            if self.has_feat(feat_id) {
                self.remove_feat(feat_id)?;
                removed_feats.push(feat_id);
            }
        }

        Ok(removed_feats)
    }

    /// Domain slots (`Domain1`, `Domain2`) are stored as GFF `Byte` per NWN2 schema;
    /// the engine reads them from the divine-caster class entry. When no divine caster is
    /// present we fall back to the first class entry - the granted feats still land in
    /// `FeatList`, though the engine may ignore the slot itself.
    fn assign_domain_slot(
        &mut self,
        domain_id: DomainId,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let class_list = self
            .get_list("ClassList")
            .filter(|list| !list.is_empty())
            .ok_or(CharacterError::FieldMissing { field: "ClassList" })?;
        let idx = class_list
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                e.get("Class")
                    .and_then(gff_value_to_i32)
                    .map(|cid| (i, cid))
            })
            .find(|&(_, cid)| self.is_divine_caster(ClassId(cid), game_data))
            .map(|(i, _)| i)
            .unwrap_or(0);

        let is_stale = |field: &str| {
            let domain = class_list[idx]
                .get(field)
                .and_then(gff_value_to_i32)
                .unwrap_or(-1);
            domain >= 0 && self.domain_slot_is_stale(domain, game_data)
        };
        let stale1 = is_stale("Domain1");
        let stale2 = is_stale("Domain2");

        let Some(entry) = self
            .get_list_mut("ClassList")
            .and_then(|list| list.get_mut(idx))
        else {
            return Err(CharacterError::FieldMissing { field: "ClassList" });
        };

        if stale1 {
            entry.shift_remove("Domain1");
        }
        if stale2 {
            entry.shift_remove("Domain2");
        }

        let d1 = entry
            .get("Domain1")
            .and_then(gff_value_to_i32)
            .unwrap_or(-1);
        let d2 = entry
            .get("Domain2")
            .and_then(gff_value_to_i32)
            .unwrap_or(-1);

        if d1 == domain_id.0 || d2 == domain_id.0 {
            return Ok(());
        }

        let value = GffValue::Byte(domain_id.0 as u8);
        if d1 < 0 {
            entry.insert("Domain1".to_string(), value);
        } else if d2 < 0 {
            entry.insert("Domain2".to_string(), value);
        } else {
            return Err(CharacterError::ValidationFailed {
                field: "Domain",
                message: format!("Both domain slots are occupied (Domain1={d1}, Domain2={d2})"),
            });
        }

        Ok(())
    }

    /// A slot is "stale" when it points to a domain whose epithet feat is no longer
    /// in `FeatList`. This happens when an earlier editor version removed the feat
    /// without clearing `Domain1`/`Domain2`, leaving the slot orphaned.
    fn domain_slot_is_stale(&self, slot_domain_id: i32, game_data: &GameData) -> bool {
        let Some(domains_table) = game_data.get_table("domains") else {
            return false;
        };
        let Some(domain_data) = domains_table.get_by_id(slot_domain_id) else {
            return true;
        };
        let Some(epithet) = Self::resolve_domain_epithet_feat(&domain_data, game_data) else {
            return false;
        };
        !self.has_feat(epithet)
    }

    fn clear_domain_slot(&mut self, domain_id: DomainId) {
        let Some(class_list) = self.get_list_mut("ClassList") else {
            return;
        };

        for entry in class_list.iter_mut() {
            if entry.get("Domain1").and_then(gff_value_to_i32) == Some(domain_id.0) {
                entry.shift_remove("Domain1");
            }
            if entry.get("Domain2").and_then(gff_value_to_i32) == Some(domain_id.0) {
                entry.shift_remove("Domain2");
            }
        }
    }

    /// If feat_id is the epithet feat for a domain, return that domain's ID. Only epithet feats
    /// uniquely identify a domain - granted/castable feats are frequently general-purpose feats
    /// (e.g. Iron Will granted by LAW, Improved Initiative granted by Time) that would otherwise
    /// hijack plain feat adds/removes.
    ///
    /// Several stock domains (Strength, Death, Evil, Healing, Protection, Sun, Fury) leave the
    /// `epithetfeat` column blank even though `feat.2da` ships matching `FEAT_EPITHET_<NAME>_DOMAIN`
    /// rows. We fall back to a label match so the UI feat-remove path still routes through
    /// `remove_domain` and clears `Domain1`/`Domain2`.
    pub fn find_domain_for_feat(&self, feat_id: FeatId, game_data: &GameData) -> Option<DomainId> {
        let domains_table = game_data.get_table("domains")?;
        for row_id in 0..domains_table.row_count() {
            let Some(domain_data) = domains_table.get_by_id(row_id as i32) else {
                continue;
            };
            let epithet = row_int(&domain_data, "epithetfeat", -1);
            if epithet >= 0 && epithet == feat_id.0 {
                return Some(DomainId(row_id as i32));
            }
        }

        let feat_table = game_data.get_table("feat")?;
        let feat_data = feat_table.get_by_id(feat_id.0)?;
        let feat_label = feat_data.get("label").and_then(|s| s.as_ref())?;
        let captures = DOMAIN_EPITHET_LABEL_PATTERN.captures(feat_label)?;
        let target_label = captures.get(1)?.as_str().to_ascii_uppercase();

        for row_id in 0..domains_table.row_count() {
            let Some(domain_data) = domains_table.get_by_id(row_id as i32) else {
                continue;
            };
            if row_int(&domain_data, "epithetfeat", -1) >= 0 {
                continue;
            }
            let Some(domain_label) = row_str(&domain_data, "label") else {
                continue;
            };
            if domain_label.eq_ignore_ascii_case(&target_label) {
                return Some(DomainId(row_id as i32));
            }
        }

        None
    }

    // ========== BACKGROUND METHODS ==========

    const CHAR_BACKGROUND_FIELD: &'static str = "CharBackground";

    /// Feat columns in `backgrounds.2da` whose values tie a row to the character's feat list.
    /// Column lookups are lowercased by the 2DA loader, so these are the stored keys.
    /// `MasterFeatGained` is a `masterfeats.2da` row, not a feat.
    const BACKGROUND_FEAT_COLUMNS: [&'static str; 2] = ["displayfeat", "featgained"];

    /// If `feat_id` appears in any non-removed background row, return that background's ID.
    pub fn find_background_for_feat(
        &self,
        feat_id: FeatId,
        game_data: &GameData,
    ) -> Option<BackgroundId> {
        let bg_table = game_data.get_table("backgrounds")?;
        for row_id in 0..bg_table.row_count() {
            let Some(row) = bg_table.get_by_id(row_id as i32) else {
                continue;
            };
            if row_bool(&row, "removed", false) {
                continue;
            }
            for field in Self::BACKGROUND_FEAT_COLUMNS {
                let owned = row
                    .get(field)
                    .and_then(|s| s.as_ref()?.parse::<i32>().ok())
                    .filter(|&id| id >= 0);
                if owned == Some(feat_id.0) {
                    return Some(BackgroundId(row_id as i32));
                }
            }
        }
        None
    }

    /// Return the feat IDs (DisplayFeat, FeatGained) declared on a background row.
    fn background_feats(
        background_id: BackgroundId,
        game_data: &GameData,
    ) -> Result<Vec<FeatId>, CharacterError> {
        let bg_table = game_data
            .get_table("backgrounds")
            .ok_or_else(|| CharacterError::TableNotFound("backgrounds".to_string()))?;

        let row = bg_table
            .get_by_id(background_id.0)
            .ok_or(CharacterError::NotFound {
                entity: "Background",
                id: background_id.0,
            })?;

        let feats = Self::BACKGROUND_FEAT_COLUMNS
            .iter()
            .filter_map(|field| {
                row.get(*field)
                    .and_then(|s| s.as_ref()?.parse::<i32>().ok())
                    .filter(|&id| id >= 0)
                    .map(FeatId)
            })
            .collect();

        Ok(feats)
    }

    /// Currently selected background (row in `backgrounds.2da`), if any.
    /// Row 0 is the "None" sentinel, so it returns `None` in that case.
    pub fn get_character_background(&self) -> Option<BackgroundId> {
        let id = self.get_i32(Self::CHAR_BACKGROUND_FIELD)?;
        if id > 0 { Some(BackgroundId(id)) } else { None }
    }

    /// Set `CharBackground` and add any feats the background grants, replacing the
    /// previous background's feats. `CharBackground` is `Dword` on the character root
    /// per NWN2-EE schema.
    pub fn add_background(
        &mut self,
        background_id: BackgroundId,
        game_data: &GameData,
    ) -> Result<Vec<FeatId>, CharacterError> {
        let feats = Self::background_feats(background_id, game_data)?;

        if self
            .get_character_background()
            .is_some_and(|current| current != background_id)
        {
            self.remove_background(game_data)?;
        }

        self.set_u32(Self::CHAR_BACKGROUND_FIELD, background_id.0 as u32);

        let mut added = Vec::new();
        for feat_id in feats {
            if !self.has_feat(feat_id) {
                self.add_feat_with_source(feat_id, FeatSource::Background)?;
                added.push(feat_id);
            }
        }

        Ok(added)
    }

    /// Clear `CharBackground` (to 0) and remove the current background's feats from the feat list.
    pub fn remove_background(
        &mut self,
        game_data: &GameData,
    ) -> Result<Vec<FeatId>, CharacterError> {
        let Some(background_id) = self.get_character_background() else {
            return Ok(Vec::new());
        };

        let feats = Self::background_feats(background_id, game_data)?;

        self.set_u32(Self::CHAR_BACKGROUND_FIELD, 0);

        let mut removed = Vec::new();
        for feat_id in feats {
            if self.has_feat(feat_id) && self.remove_feat(feat_id).is_ok() {
                removed.push(feat_id);
            }
        }

        Ok(removed)
    }

    // ========== FEAT SLOTS CALCULATION (Blueprint Method) ==========

    const QUICK_TO_MASTER_FEAT_ID: i32 = 258;
    /// Feat categories the game grants without a feat slot. Background feats (Luck of
    /// Heroes, Spellcasting Prodigy) take the level-1 slot; history feats come with the
    /// `backgrounds.2da` choice.
    const AUTO_GRANTED_FEAT_TYPES: [FeatType; 2] = [FeatType::HISTORY, FeatType::RACIAL];

    /// Calculate feat slots using the Blueprint Method - analyzes level history for accuracy.
    pub fn get_feat_slots(&self, game_data: &GameData) -> FeatSlots {
        let level_history = self.level_history();

        if level_history.is_empty() {
            return FeatSlots::default();
        }

        let mut open_general_slots = 0;
        let mut open_bonus_slots = 0;
        let mut filled_general_slots = 0;
        let mut filled_bonus_slots = 0;

        let mut class_level_tracker: std::collections::HashMap<i32, i32> =
            std::collections::HashMap::new();
        let mut class_feat_table_cache: std::collections::HashMap<
            i32,
            std::collections::HashMap<i32, i32>,
        > = std::collections::HashMap::new();

        let mut leftover_selectable: i32 = 0;
        let granted_feats = self.feats_granted_without_slot(game_data);
        let feats_table = game_data.get_table("feat");
        let racial_bonus = self.get_racial_bonus_feats();

        for (total_level_idx, level_entry) in level_history.iter().enumerate() {
            let total_level = (total_level_idx + 1) as i32;
            let class_id = level_entry.class_id.0;

            *class_level_tracker.entry(class_id).or_insert(0) += 1;
            let class_level = class_level_tracker[&class_id];

            let level_feat_ids: Vec<i32> = level_entry.feats_gained.iter().map(|f| f.0).collect();

            class_feat_table_cache
                .entry(class_id)
                .or_insert_with(|| self.load_class_feat_table(ClassId(class_id), game_data));
            let class_feat_table = &class_feat_table_cache[&class_id];

            let mut selectable_feats: Vec<i32> = Vec::new();
            for feat_id in &level_feat_ids {
                if class_feat_table.get(feat_id).copied() == Some(3)
                    || granted_feats.contains(feat_id)
                {
                    continue;
                }

                if let Some(feat_data) = feats_table.and_then(|t| t.get_by_id(*feat_id)) {
                    let feat_type = Self::parse_feat_type(&feat_data, "");
                    if Self::AUTO_GRANTED_FEAT_TYPES
                        .iter()
                        .any(|&t| feat_type.contains(t))
                    {
                        continue;
                    }
                }

                selectable_feats.push(*feat_id);
            }

            let has_general_slot = if total_level <= 20 {
                total_level == 1 || total_level % 3 == 0
            } else {
                total_level % 2 != 0
            };
            let general_slots =
                i32::from(has_general_slot) + if total_level == 1 { racial_bonus } else { 0 };

            for _ in 0..general_slots {
                if !selectable_feats.is_empty() {
                    filled_general_slots += 1;
                    selectable_feats.remove(0);
                } else {
                    open_general_slots += 1;
                }
            }

            let has_bonus_slot =
                self.check_bonus_feat_slot(ClassId(class_id), class_level, game_data);

            if has_bonus_slot {
                if !selectable_feats.is_empty() {
                    filled_bonus_slots += 1;
                    selectable_feats.remove(0);
                } else {
                    open_bonus_slots += 1;
                }
            }

            leftover_selectable += selectable_feats.len() as i32;
        }

        // Feats added via editor may land on a level without a slot.
        // Use leftover selectable feats to fill open slots from other levels.
        while leftover_selectable > 0 && open_general_slots > 0 {
            open_general_slots -= 1;
            filled_general_slots += 1;
            leftover_selectable -= 1;
        }
        while leftover_selectable > 0 && open_bonus_slots > 0 {
            open_bonus_slots -= 1;
            filled_bonus_slots += 1;
            leftover_selectable -= 1;
        }

        let total_general_slots = filled_general_slots + open_general_slots;
        let total_bonus_slots = filled_bonus_slots + open_bonus_slots;
        let total_slots = total_general_slots + total_bonus_slots;

        let open_slots = open_general_slots + open_bonus_slots;
        let filled_slots = filled_general_slots + filled_bonus_slots;

        FeatSlots {
            total_general_slots,
            total_bonus_slots,
            total_slots,
            filled_slots,
            open_slots,
            open_general_slots,
            open_bonus_slots,
        }
    }

    fn load_class_feat_table(
        &self,
        class_id: ClassId,
        game_data: &GameData,
    ) -> std::collections::HashMap<i32, i32> {
        let mut feat_table = std::collections::HashMap::new();

        let Some(classes_table) = game_data.get_table("classes") else {
            return feat_table;
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return feat_table;
        };

        let feats_table_name_opt = class_data.get("featstable").and_then(|s| s.as_ref());

        let Some(feats_table_name) = feats_table_name_opt else {
            return feat_table;
        };

        if feats_table_name.starts_with("****") {
            return feat_table;
        }

        let Some(table) = game_data.get_table(&feats_table_name.to_lowercase()) else {
            return feat_table;
        };

        for row_id in 0..table.row_count() {
            let Some(row) = table.get_by_id(row_id as i32) else {
                continue;
            };

            let feat_id = row_int(&row, "featindex", -1);
            let list_type = row_int(&row, "list", 0);

            if feat_id >= 0 {
                feat_table.insert(feat_id, list_type);
            }
        }

        feat_table
    }

    fn check_bonus_feat_slot(
        &self,
        class_id: ClassId,
        class_level: i32,
        game_data: &GameData,
    ) -> bool {
        let Some(classes_table) = game_data.get_table("classes") else {
            return false;
        };

        let Some(class_data) = classes_table.get_by_id(class_id.0) else {
            return false;
        };

        let bonus_table_name_opt = class_data.get("bonusfeatstable").and_then(|s| s.as_ref());

        let Some(bonus_table_name) = bonus_table_name_opt else {
            return false;
        };

        if bonus_table_name.starts_with("****") {
            return false;
        }

        let Some(bonus_table) = game_data.get_table(&bonus_table_name.to_lowercase()) else {
            return false;
        };

        let level_idx = class_level - 1;
        if level_idx < 0 {
            return false;
        }

        let Some(level_data) = bonus_table.get_by_id(level_idx) else {
            return false;
        };

        row_int(&level_data, "bonus", 0) > 0
    }

    fn get_racial_bonus_feats(&self) -> i32 {
        i32::from(self.has_feat(FeatId(Self::QUICK_TO_MASTER_FEAT_ID)))
    }

    /// Feats the character's race, subrace and domains grant outside the feat slots.
    fn feats_granted_without_slot(&self, game_data: &GameData) -> HashSet<i32> {
        let mut granted: HashSet<i32> = self
            .get_racial_feats_for_race(self.race_id().0, game_data)
            .into_iter()
            .map(|f| f.0)
            .collect();

        if let Some(subrace) = self
            .subrace_index()
            .and_then(|index| self.get_subrace_data_by_index(index, game_data))
        {
            granted.extend(
                super::race::featstable_feats(game_data, subrace.feats_table.as_deref())
                    .into_iter()
                    .map(|f| f.0),
            );
        }

        if let Some(domains_table) = game_data.get_table("domains") {
            for domain_id in self.get_character_domains() {
                let Some(domain_data) = domains_table.get_by_id(domain_id.0) else {
                    continue;
                };
                let (granted_feat, epithet_feat) = Self::domain_feats(&domain_data, game_data);
                granted.extend(
                    [granted_feat, epithet_feat]
                        .into_iter()
                        .flatten()
                        .map(|f| f.0),
                );
            }
        }

        granted
    }

    pub fn get_bonus_feats_for_class(
        &self,
        class_id: ClassId,
        level: i32,
        game_data: &GameData,
    ) -> i32 {
        let mut count = 0;
        for lvl in 1..=level {
            if self.check_bonus_feat_slot(class_id, lvl, game_data) {
                count += 1;
            }
        }
        count
    }

    // ========== PREREQUISITE VALIDATION ==========

    pub fn get_feat_name(&self, feat_id: FeatId, game_data: &GameData) -> String {
        let Some(feats_table) = game_data.get_table("feat") else {
            return format!("Feat {}", feat_id.0);
        };

        let Some(feat_data) = feats_table.get_by_id(feat_id.0) else {
            return format!("Feat {}", feat_id.0);
        };

        let name_strref = feat_data
            .get("feat")
            .and_then(|v| v.clone())
            .or_else(|| feat_data.get("name").and_then(|v| v.clone()))
            .and_then(|s| s.parse::<i32>().ok());

        if let Some(strref) = name_strref
            && let Some(name) = game_data.get_string(strref)
            && !name.is_empty()
        {
            return name;
        }

        row_str(&feat_data, "label").unwrap_or_else(|| format!("Feat {}", feat_id.0))
    }

    // ========== BONUS CALCULATION FROM FEAT DESCRIPTIONS ==========

    pub fn get_feat_save_bonuses(&self, game_data: &GameData) -> SaveBonuses {
        let mut bonuses = SaveBonuses::default();
        let feat_entries = self.feat_entries();

        for entry in &feat_entries {
            let Some(feats_table) = game_data.get_table("feat") else {
                continue;
            };

            let Some(feat_data) = feats_table.get_by_id(entry.feat_id.0) else {
                continue;
            };

            let description = Self::resolve_feat_description_english(&feat_data, game_data);
            let description_lower = description.to_lowercase();

            if is_conditional_feat(&feat_data, &description_lower, SAVE_CONDITIONAL_KEYWORDS) {
                continue;
            }

            let mut found_fort = false;
            let mut found_ref = false;
            let mut found_will = false;

            for (pattern, save_type) in SAVE_PATTERNS.iter() {
                if let Some(captures) = pattern.captures(&description)
                    && let Some(bonus_str) = captures.get(1)
                    && let Ok(bonus_value) = bonus_str.as_str().parse::<i32>()
                {
                    match save_type {
                        SaveType::Universal => {
                            if !found_fort {
                                bonuses.fortitude += bonus_value;
                                found_fort = true;
                            }
                            if !found_ref {
                                bonuses.reflex += bonus_value;
                                found_ref = true;
                            }
                            if !found_will {
                                bonuses.will += bonus_value;
                                found_will = true;
                            }
                        }
                        SaveType::Fortitude => {
                            if !found_fort {
                                bonuses.fortitude += bonus_value;
                                found_fort = true;
                            }
                        }
                        SaveType::Reflex => {
                            if !found_ref {
                                bonuses.reflex += bonus_value;
                                found_ref = true;
                            }
                        }
                        SaveType::Will => {
                            if !found_will {
                                bonuses.will += bonus_value;
                                found_will = true;
                            }
                        }
                        SaveType::FortitudeAndWill => {
                            if !found_fort {
                                bonuses.fortitude += bonus_value;
                                found_fort = true;
                            }
                            if !found_will {
                                bonuses.will += bonus_value;
                                found_will = true;
                            }
                        }
                    }
                }
            }
        }

        bonuses
    }

    pub fn get_feat_ac_bonuses(&self, game_data: &GameData) -> i32 {
        let mut total_ac = 0;
        let feat_entries = self.feat_entries();

        for entry in &feat_entries {
            let Some(feats_table) = game_data.get_table("feat") else {
                continue;
            };

            let Some(feat_data) = feats_table.get_by_id(entry.feat_id.0) else {
                continue;
            };

            let label = feat_data
                .get("label")
                .and_then(|s| s.as_ref().map(std::string::ToString::to_string))
                .unwrap_or_default()
                .to_lowercase();

            let description = Self::resolve_feat_description_english(&feat_data, game_data);
            let description_lower = description.to_lowercase();

            if is_conditional_feat(&feat_data, &description_lower, AC_CONDITIONAL_KEYWORDS) {
                continue;
            }

            if label.contains("dodge") || label.contains("mobility") {
                if let Some(captures) = AC_DODGE_PATTERN.captures(&description) {
                    if let Some(bonus_str) = captures.get(1)
                        && let Ok(bonus_value) = bonus_str.as_str().parse::<i32>()
                    {
                        total_ac += bonus_value;
                        continue;
                    }
                } else if label.contains("dodge") {
                    total_ac += 1;
                    continue;
                }
            }

            for pattern in AC_PATTERNS.iter() {
                if let Some(captures) = pattern.captures(&description)
                    && let Some(bonus_str) = captures.get(1)
                    && let Ok(bonus_value) = bonus_str.as_str().parse::<i32>()
                {
                    total_ac += bonus_value;
                    break;
                }
            }
        }

        total_ac
    }

    pub fn get_feat_initiative_bonus(&self, game_data: &GameData) -> i32 {
        let mut bonus = 0;
        let feat_entries = self.feat_entries();

        for entry in &feat_entries {
            let Some(feats_table) = game_data.get_table("feat") else {
                continue;
            };

            let Some(feat_data) = feats_table.get_by_id(entry.feat_id.0) else {
                continue;
            };

            let label = feat_data
                .get("label")
                .and_then(|s| s.as_ref().map(std::string::ToString::to_string))
                .unwrap_or_default()
                .replace(['_', ' '], "")
                .to_lowercase();

            if label.contains("improvedinitiative") {
                bonus += 4;
                continue;
            }

            let description = Self::resolve_feat_description_english(&feat_data, game_data);

            for pattern in INITIATIVE_PATTERNS.iter() {
                if let Some(captures) = pattern.captures(&description)
                    && let Some(bonus_str) = captures.get(1)
                    && let Ok(bonus_value) = bonus_str.as_str().parse::<i32>()
                {
                    bonus += bonus_value;
                    break;
                }
            }
        }

        bonus
    }

    pub fn get_feat_skill_bonuses(
        &self,
        game_data: &GameData,
    ) -> std::collections::HashMap<String, i32> {
        use std::collections::HashMap;

        static SKILL_BONUS_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
            vec![
                Regex::new(r"(?i)\+(\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+([A-Za-z][A-Za-z\s]+?)\s+(?:checks|skill)")
                    .expect("Invalid skill bonus regex 1"),
                Regex::new(r"(?i)\+(\d+)\s+(?:to|on)\s+([A-Za-z][A-Za-z\s]+?)\s+(?:checks|skill)")
                    .expect("Invalid skill bonus regex 2"),
                Regex::new(r"(?i)\+(\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+all\s+([A-Za-z][A-Za-z\s]+?)\s+checks")
                    .expect("Invalid skill bonus regex 3"),
            ]
        });

        static DUAL_SKILL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?i)\+(\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+([A-Za-z][A-Za-z\s]+?)\s+and\s+([A-Za-z][A-Za-z\s]+?)\s+(?:checks|skills?)")
                .expect("Invalid dual skill regex")
        });

        static EFFECTS_ENTRY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"([+-]\d+)\s+([A-Za-z][A-Za-z\s]*[A-Za-z])")
                .expect("Invalid effects entry regex")
        });

        static SKILL_CONDITIONAL_KEYWORDS: &[&str] = &[
            "against", "vs ", "versus", "when ", "while ", "if ", "to avoid",
        ];

        let known_skills = Self::build_known_skill_names(game_data);

        let mut skill_bonuses: HashMap<String, i32> = HashMap::new();
        let feat_entries = self.feat_entries();

        for entry in &feat_entries {
            let Some(feats_table) = game_data.get_table("feat") else {
                continue;
            };

            let Some(feat_data) = feats_table.get_by_id(entry.feat_id.0) else {
                continue;
            };

            let description = Self::resolve_feat_description_english(&feat_data, game_data);
            let description_lower = description.to_lowercase();

            let feat_label = feat_data
                .get("label")
                .and_then(|s| s.as_ref().map(String::as_str))
                .unwrap_or("unknown");

            // Explicit `Effects:` line is authoritative. Scope the conditional
            // check to the effects text so narrative prose words ("when", "if",
            // etc.) don't suppress unconditional bonuses — e.g. TheFlirt's prose
            // says "...when you try to act tough" but its effects are flat.
            if let Some(effects_start) = description_lower.find("effects:") {
                let effects_text = &description[effects_start..];
                let effects_lower = effects_text.to_lowercase();

                if is_conditional_feat(&feat_data, &effects_lower, SKILL_CONDITIONAL_KEYWORDS) {
                    debug!(
                        "[feat_skill] Skipping conditional effects feat '{}'",
                        feat_label
                    );
                    continue;
                }

                debug!(
                    "[feat_skill] Feat '{}' has effects line: {:?}",
                    feat_label, effects_text
                );

                for cap in EFFECTS_ENTRY_PATTERN.captures_iter(effects_text) {
                    let Some(value_str) = cap.get(1) else {
                        continue;
                    };
                    let Some(name_match) = cap.get(2) else {
                        continue;
                    };
                    let Ok(bonus_value) = value_str.as_str().parse::<i32>() else {
                        continue;
                    };

                    let normalized = Self::normalize_skill_name(name_match.as_str());
                    if known_skills.contains(&normalized) {
                        *skill_bonuses.entry(normalized).or_insert(0) += bonus_value;
                    }
                }
                continue;
            }

            if is_conditional_feat(&feat_data, &description_lower, SKILL_CONDITIONAL_KEYWORDS) {
                debug!("[feat_skill] Skipping conditional feat '{}'", feat_label);
                continue;
            }

            if let Some(captures) = DUAL_SKILL_PATTERN.captures(&description)
                && let (Some(bonus_str), Some(skill1), Some(skill2)) =
                    (captures.get(1), captures.get(2), captures.get(3))
                && let Ok(bonus_value) = bonus_str.as_str().parse::<i32>()
            {
                let skill1_name = Self::normalize_skill_name(skill1.as_str());
                let skill2_name = Self::normalize_skill_name(skill2.as_str());
                *skill_bonuses.entry(skill1_name).or_insert(0) += bonus_value;
                *skill_bonuses.entry(skill2_name).or_insert(0) += bonus_value;
                continue;
            }

            for pattern in SKILL_BONUS_PATTERNS.iter() {
                if let Some(captures) = pattern.captures(&description)
                    && let (Some(bonus_str), Some(skill_match)) = (captures.get(1), captures.get(2))
                    && let Ok(bonus_value) = bonus_str.as_str().parse::<i32>()
                {
                    let skill_name = Self::normalize_skill_name(skill_match.as_str());
                    *skill_bonuses.entry(skill_name).or_insert(0) += bonus_value;
                    break;
                }
            }
        }

        debug!("[feat_skill] Final skill_bonuses: {:?}", skill_bonuses);
        skill_bonuses
    }

    fn build_known_skill_names(game_data: &GameData) -> HashSet<String> {
        let mut names = HashSet::new();
        let Some(skills_table) = game_data.get_table("skills") else {
            return names;
        };

        for i in 0..skills_table.row_count() {
            let Some(skill_data) = skills_table.get_by_id(i as i32) else {
                continue;
            };

            let label = skill_data
                .get("label")
                .and_then(|opt| opt.as_deref())
                .unwrap_or("");

            if label.starts_with("****") || label.starts_with("DEL_") || label.contains("DELETED") {
                continue;
            }

            names.insert(Self::normalize_skill_name(label));
        }

        names
    }

    fn normalize_skill_name(name: &str) -> String {
        let mut normalized = name.trim().to_uppercase().replace([' ', '-', '_'], "");
        if normalized.ends_with("TRAPS") {
            normalized = normalized.trim_end_matches('S').to_string();
        }
        normalized
    }

    fn resolve_feat_description(
        feat_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        Self::feat_description_with(feat_data, |strref| game_data.get_string(strref))
    }

    /// The description as English text: the bonus parsers match English wording
    /// whatever the editor language.
    fn resolve_feat_description_english(
        feat_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        Self::feat_description_with(feat_data, |strref| game_data.get_english_string(strref))
    }

    fn feat_description_with(
        feat_data: &ahash::AHashMap<String, Option<String>>,
        resolve: impl Fn(i32) -> Option<String>,
    ) -> String {
        row_str(feat_data, "description")
            .and_then(|s| s.parse::<i32>().ok())
            .and_then(resolve)
            .map(|desc| crate::utils::strip_markup(&desc))
            .unwrap_or_default()
    }

    // ========== FEAT INFO AND METADATA ==========

    pub fn get_feat_info(&self, feat_id: FeatId, game_data: &GameData) -> Option<FeatInfo> {
        self.get_feat_info_display(feat_id, game_data, &FeatListContext::new(self, game_data))
    }

    pub fn get_feat_info_display(
        &self,
        feat_id: FeatId,
        game_data: &GameData,
        ctx: &FeatListContext,
    ) -> Option<FeatInfo> {
        let feats_table = game_data.get_table("feat")?;
        let feat_data = feats_table.get_by_id(feat_id.0)?;

        let label = feat_data
            .get("label")
            .and_then(|s| s.as_ref())
            .map_or_else(|| format!("feat_{}", feat_id.0), |s| s.clone());
        let name = Self::resolve_feat_name_from_data(&feat_data, game_data);
        let description = Self::resolve_feat_description(&feat_data, game_data);
        let icon = feat_data
            .get("icon")
            .and_then(|s| s.as_ref())
            .map_or("", std::string::String::as_str)
            .to_string();

        let mut feat_type = Self::parse_feat_type(&feat_data, &description);

        let from_domain_source = matches!(ctx.feat_sources.get(&feat_id), Some(FeatSource::Domain));
        let is_epithet = ctx.epithet_feats.contains(&feat_id.0);
        let is_domain = compute_is_domain(feat_type, &label, from_domain_source, is_epithet);
        if is_domain {
            feat_type = FeatType(feat_type.0 | FeatType::DOMAIN.0);
        }
        let category = FeatCategory::from_feat_type(feat_type, is_domain);

        let is_protected = ctx
            .feat_sources
            .get(&feat_id)
            .is_some_and(|s| matches!(s, FeatSource::Race | FeatSource::Background))
            || is_epithet;

        let is_custom = row_bool(&feat_data, "custom", false);
        let has_feat = ctx.feat_sources.contains_key(&feat_id);

        Some(FeatInfo {
            id: feat_id,
            label,
            name,
            description,
            icon,
            feat_type,
            category,
            is_protected,
            is_custom,
            has_feat,
            prerequisites: self.build_feat_prerequisites(&feat_data, game_data, &ctx.cache),
        })
    }

    pub(crate) fn parse_feat_type(
        feat_data: &ahash::AHashMap<String, Option<String>>,
        description: &str,
    ) -> FeatType {
        if let Some(type_str) = feat_data.get("featcategory").and_then(|s| s.as_ref()) {
            return FeatType::from_string(type_str);
        }

        if let Some(type_str) = feat_data.get("toolscategories").and_then(|s| s.as_ref()) {
            return FeatType::from_string(type_str);
        }

        static TYPE_OF_FEAT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"Type of Feat:\s*(\w+)").expect("Invalid regex"));

        if let Some(caps) = TYPE_OF_FEAT_RE.captures(description)
            && let Some(type_match) = caps.get(1)
        {
            return FeatType::from_string(type_match.as_str());
        }

        FeatType::GENERAL
    }

    /// Every requirement `feat.2da` declares for a feat, compared with the character.
    fn build_feat_prerequisites(
        &self,
        feat_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
        cache: &PrereqCache,
    ) -> FeatPrerequisites {
        let positive = |column: &str| Some(row_int(feat_data, column, 0)).filter(|&v| v > 0);
        let value = |required: i32, current: i32| PrereqValue {
            required,
            current,
            met: current >= required,
        };
        let prereq_feat = |column: &str| {
            let feat_id = FeatId(row_int(feat_data, column, -1));
            (feat_id.0 >= 0).then(|| PrereqFeat {
                id: feat_id,
                name: self.get_feat_name(feat_id, game_data),
                met: self.has_feat(feat_id),
            })
        };

        let mut prereqs = FeatPrerequisites {
            feats: ["prereqfeat1", "prereqfeat2"]
                .into_iter()
                .filter_map(prereq_feat)
                .collect(),
            any_feats: OR_REQ_FEAT_COLUMNS
                .into_iter()
                .filter_map(prereq_feat)
                .collect(),
            ..Default::default()
        };

        let abilities = cache
            .abilities
            .get_or_init(|| self.get_effective_abilities(game_data));
        for (ability, column) in AbilityIndex::all().into_iter().zip(ABILITY_MIN_COLUMNS) {
            if let Some(required) = positive(column) {
                let current = abilities.get(ability);
                prereqs.abilities.push(PrereqAbility {
                    ability: ability.name().to_string(),
                    required,
                    current,
                    met: current >= required,
                });
            }
        }

        if let Some(required) = positive("minattackbonus") {
            let current = *cache.bab.get_or_init(|| self.calculate_bab(game_data));
            prereqs.bab = Some(value(required, current));
        }

        let total_level = self.total_level();
        if let Some(required) = positive("minlevel") {
            let class_id = ClassId(row_int(feat_data, "minlevelclass", -1));
            if class_id.0 >= 0 {
                prereqs.class_level = Some(PrereqClassLevel {
                    class_name: self.get_class_name(class_id, game_data),
                    level: value(required, self.class_level(class_id)),
                });
            } else {
                prereqs.level = Some(value(required, total_level));
            }
        }

        if let Some(required) = positive("maxlevel") {
            prereqs.max_level = Some(PrereqValue {
                required,
                current: total_level,
                met: total_level <= required,
            });
        }

        if positive("prereqepic").is_some() {
            prereqs.epic = Some(value(EPIC_LEVEL_THRESHOLD + 1, total_level));
        }

        if let Some(required) = positive("minfortsave") {
            let current = *cache
                .base_fortitude
                .get_or_init(|| self.calculate_base_saves(game_data).fortitude);
            prereqs.base_fortitude = Some(value(required, current));
        }

        if let Some(required) = positive("mincasterlvl") {
            let current = *cache
                .caster_level
                .get_or_init(|| self.caster_level(None, game_data));
            prereqs.caster_level = Some(value(required, current));
        }

        if let Some(required) = positive("minspelllvl") {
            let current = *cache
                .max_spell_level
                .get_or_init(|| self.max_castable_spell_level(None, game_data));
            prereqs.spell_level = Some(value(required, current));
        }

        for (skill_column, ranks_column) in [
            ("reqskill", "reqskillminranks"),
            ("reqskill2", "reqskillminranks2"),
        ] {
            let skill_id = SkillId(row_int(feat_data, skill_column, -1));
            if skill_id.0 < 0 {
                continue;
            }
            let required = positive(ranks_column).unwrap_or(1);
            let current = self.skill_rank(skill_id);
            prereqs.skills.push(PrereqSkill {
                skill: self.get_skill_name(skill_id, game_data),
                required,
                current,
                met: current >= required,
            });
        }

        prereqs
    }

    pub fn get_feat_summary(&self, game_data: &GameData) -> FeatSummary {
        let feat_entries = self.feat_entries();
        let mut summary = FeatSummary {
            total: feat_entries.len() as i32,
            ..Default::default()
        };

        let ctx = FeatListContext::new(self, game_data);
        for entry in &feat_entries {
            let Some(feat_info) = self.get_feat_info_display(entry.feat_id, game_data, &ctx) else {
                continue;
            };

            if feat_info.is_custom {
                summary.custom_feats.push(feat_info);
            } else {
                match feat_info.category {
                    FeatCategory::Domain => summary.domain_feats.push(feat_info),
                    FeatCategory::Background | FeatCategory::History | FeatCategory::Heritage => {
                        summary.background_feats.push(feat_info);
                    }
                    FeatCategory::Class => summary.class_feats.push(feat_info),
                    _ => summary.general_feats.push(feat_info),
                }
            }
        }

        summary
    }

    pub fn get_feats_state(&self, game_data: &GameData) -> FeatsState {
        let domains = self
            .get_available_domains(game_data)
            .into_iter()
            .filter(|d| d.has_domain)
            .collect();

        FeatsState {
            summary: self.get_feat_summary(game_data),
            feat_slots: self.get_feat_slots(game_data),
            domains,
        }
    }

    pub fn get_available_domains(&self, game_data: &GameData) -> Vec<DomainInfo> {
        let Some(domains_table) = game_data.get_table("domains") else {
            trace!("get_available_domains: No domains table found");
            return Vec::new();
        };

        let character_domains = self.get_character_domains();
        trace!(
            "get_available_domains: character_domains = {:?}",
            character_domains
        );
        let mut available = Vec::new();
        let row_count = domains_table.row_count();
        trace!(
            "get_available_domains: domains_table row_count = {}",
            row_count
        );

        for domain_id in 0..row_count {
            let domain_id = DomainId(domain_id as i32);

            let Some(domain_data) = domains_table.get_by_id(domain_id.0) else {
                if domain_id.0 == 25 || domain_id.0 == 18 {
                    trace!(
                        "get_available_domains: domain_id {} - no domain_data found",
                        domain_id.0
                    );
                }
                continue;
            };

            let label = domain_data
                .get("label")
                .and_then(|s| s.as_ref())
                .map_or("", std::string::String::as_str);

            // Skip deleted/invalid domains but allow empty labels
            if label.starts_with("****") || label.starts_with("DEL_") {
                continue;
            }

            let name = Self::resolve_domain_name(&domain_data, game_data);
            let description = Self::resolve_domain_description(&domain_data, game_data);

            let (granted_feat, epithet_feat) = Self::domain_feats(&domain_data, game_data);

            let is_castable = domain_data
                .get("castablefeat")
                .and_then(|s| s.as_ref()?.parse::<i32>().ok())
                .is_some_and(|v| v != 0);

            let has_domain = character_domains.contains(&domain_id);

            if has_domain {
                trace!(
                    "get_available_domains: Domain {} ({}) has_domain=true",
                    domain_id.0, name
                );
            }

            available.push(DomainInfo {
                id: domain_id,
                name,
                description,
                granted_feat,
                is_castable,
                epithet_feat,
                has_domain,
            });
        }

        let with_domain: Vec<_> = available.iter().filter(|d| d.has_domain).collect();
        trace!(
            "get_available_domains: Total available={}, with has_domain=true: {}",
            available.len(),
            with_domain.len()
        );

        available
    }

    fn resolve_feat_name_from_data(
        feat_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        let name_strref = feat_data
            .get("feat")
            .and_then(|v| v.clone())
            .or_else(|| feat_data.get("name").and_then(|v| v.clone()))
            .and_then(|s| s.parse::<i32>().ok());

        if let Some(strref) = name_strref
            && let Some(name) = game_data.get_string(strref)
            && !name.is_empty()
        {
            return crate::utils::strip_markup(&name);
        }

        row_str(feat_data, "label").unwrap_or_else(|| "Unknown".to_string())
    }

    fn resolve_domain_name(
        domain_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        let name_strref = domain_data
            .get("name")
            .and_then(|s| s.as_ref()?.parse::<i32>().ok());

        if let Some(strref) = name_strref
            && let Some(name) = game_data.get_string(strref)
            && !name.is_empty()
        {
            return name;
        }

        domain_data
            .get("label")
            .and_then(|s| s.as_ref())
            .map_or("Unknown", std::string::String::as_str)
            .to_string()
    }

    fn resolve_domain_description(
        domain_data: &ahash::AHashMap<String, Option<String>>,
        game_data: &GameData,
    ) -> String {
        let desc_strref = domain_data
            .get("description")
            .and_then(|s| s.as_ref()?.parse::<i32>().ok());

        if let Some(strref) = desc_strref
            && let Some(desc) = game_data.get_string(strref)
            && !desc.is_empty()
        {
            return desc;
        }

        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    fn create_test_character_with_feats() -> Character {
        let mut fields = IndexMap::new();

        let mut feat1 = IndexMap::new();
        feat1.insert("Feat".to_string(), GffValue::Word(10));

        let mut feat2 = IndexMap::new();
        feat2.insert("Feat".to_string(), GffValue::Word(20));

        let mut feat3 = IndexMap::new();
        feat3.insert("Feat".to_string(), GffValue::Word(30));

        fields.insert(
            "FeatList".to_string(),
            GffValue::ListOwned(vec![feat1, feat2, feat3]),
        );

        Character::from_gff(fields).expect("character from gff")
    }

    fn create_empty_character() -> Character {
        Character::from_gff(IndexMap::new()).expect("character from gff")
    }

    #[test]
    fn test_feat_ids() {
        let character = create_test_character_with_feats();
        let feat_ids = character.feat_ids();

        assert_eq!(feat_ids.len(), 3);
        assert_eq!(feat_ids[0].0, 10);
        assert_eq!(feat_ids[1].0, 20);
        assert_eq!(feat_ids[2].0, 30);
    }

    #[test]
    fn test_feat_ids_empty() {
        let character = create_empty_character();
        let feat_ids = character.feat_ids();

        assert_eq!(feat_ids.len(), 0);
    }

    #[test]
    fn test_has_feat() {
        let character = create_test_character_with_feats();

        assert!(character.has_feat(FeatId(10)));
        assert!(character.has_feat(FeatId(20)));
        assert!(character.has_feat(FeatId(30)));
        assert!(!character.has_feat(FeatId(99)));
    }

    #[test]
    fn test_has_feat_empty() {
        let character = create_empty_character();

        assert!(!character.has_feat(FeatId(10)));
    }

    #[test]
    fn test_feat_count() {
        let character = create_test_character_with_feats();
        assert_eq!(character.feat_count(), 3);

        let empty = create_empty_character();
        assert_eq!(empty.feat_count(), 0);
    }

    #[test]
    fn test_add_feat() {
        let mut character = create_test_character_with_feats();

        let result = character.add_feat(FeatId(40));
        assert!(result.is_ok());

        assert_eq!(character.feat_count(), 4);
        assert!(character.has_feat(FeatId(40)));
        assert!(character.is_modified());
    }

    #[test]
    fn test_add_feat_to_empty() {
        let mut character = create_empty_character();

        let result = character.add_feat(FeatId(1));
        assert!(result.is_ok());

        assert_eq!(character.feat_count(), 1);
        assert!(character.has_feat(FeatId(1)));
    }

    #[test]
    fn test_add_feat_duplicate() {
        let mut character = create_test_character_with_feats();

        let result = character.add_feat(FeatId(10));
        assert!(result.is_err());

        match result.unwrap_err() {
            CharacterError::FeatAlreadyExists(id) => assert_eq!(id, 10),
            _ => panic!("Expected FeatAlreadyExists error"),
        }

        assert_eq!(character.feat_count(), 3);
    }

    #[test]
    fn test_remove_feat() {
        let mut character = create_test_character_with_feats();

        let result = character.remove_feat(FeatId(20));
        assert!(result.is_ok());

        assert_eq!(character.feat_count(), 2);
        assert!(!character.has_feat(FeatId(20)));
        assert!(character.has_feat(FeatId(10)));
        assert!(character.has_feat(FeatId(30)));
        assert!(character.is_modified());
    }

    #[test]
    fn test_remove_feat_not_found() {
        let mut character = create_test_character_with_feats();

        let result = character.remove_feat(FeatId(99));
        assert!(result.is_err());

        match result.unwrap_err() {
            CharacterError::FeatNotFound(id) => assert_eq!(id, 99),
            _ => panic!("Expected FeatNotFound error"),
        }

        assert_eq!(character.feat_count(), 3);
    }

    #[test]
    fn test_remove_feat_from_empty() {
        let mut character = create_empty_character();

        let result = character.remove_feat(FeatId(1));
        assert!(result.is_err());

        match result.unwrap_err() {
            CharacterError::FeatNotFound(id) => assert_eq!(id, 1),
            _ => panic!("Expected FeatNotFound error"),
        }
    }

    #[test]
    fn test_add_and_remove_cycle() {
        let mut character = create_test_character_with_feats();

        let result = character.add_feat(FeatId(50));
        assert!(result.is_ok());
        assert_eq!(character.feat_count(), 4);
        assert!(character.has_feat(FeatId(50)));

        let result = character.remove_feat(FeatId(50));
        assert!(result.is_ok());
        assert_eq!(character.feat_count(), 3);
        assert!(!character.has_feat(FeatId(50)));

        let result = character.add_feat(FeatId(50));
        assert!(result.is_ok());
        assert_eq!(character.feat_count(), 4);
        assert!(character.has_feat(FeatId(50)));
    }

    #[test]
    fn test_multiple_add_feats() {
        let mut character = create_empty_character();

        for id in 1..=10 {
            let result = character.add_feat(FeatId(id));
            assert!(result.is_ok());
        }

        assert_eq!(character.feat_count(), 10);

        for id in 1..=10 {
            assert!(character.has_feat(FeatId(id)));
        }
    }

    #[test]
    fn test_remove_all_feats() {
        let mut character = create_test_character_with_feats();

        let feat_ids: Vec<FeatId> = character.feat_ids();

        for feat_id in feat_ids {
            let result = character.remove_feat(feat_id);
            assert!(result.is_ok());
        }

        assert_eq!(character.feat_count(), 0);
    }
}
