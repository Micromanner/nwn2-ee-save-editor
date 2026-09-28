use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use tracing::{debug, trace, warn};

use super::{Character, CharacterError};
use crate::character::feats::{FeatSource, FeatType};
use crate::character::types::{AbilityIndex, AbilityModifiers, FeatId, RaceId, ValidationResult};
use crate::loaders::GameData;
use crate::utils::parsing::{Row, row_int, row_str};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[repr(i32)]
/// NWN2 creature size IDs matching creaturesize.2da rows
pub enum SizeCategory {
    Invalid = 0,
    Tiny = 1,
    Small = 2,
    Medium = 3,
    Large = 4,
    Huge = 5,
}

impl SizeCategory {
    pub fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Invalid,
            1 => Self::Tiny,
            2 => Self::Small,
            3 => Self::Medium,
            4 => Self::Large,
            5 => Self::Huge,
            _ => Self::Medium,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Invalid => "Invalid",
            Self::Tiny => "Tiny",
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
            Self::Huge => "Huge",
        }
    }

    pub fn ac_modifier_default(&self) -> i32 {
        match self {
            Self::Invalid => 0,
            Self::Tiny => 2,
            Self::Small => 1,
            Self::Medium => 0,
            Self::Large => -1,
            Self::Huge => -2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SubraceInfo {
    pub id: i32,
    pub name: String,
    pub label: String,
    pub parent_race: RaceId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SubraceData {
    pub id: i32,
    pub name: String,
    pub label: String,
    pub base_race: i32,
    pub ability_modifiers: AbilityModifiers,
    pub player_race: bool,
    pub favored_class: Option<i32>,
    pub feats_table: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatChange {
    pub feat_id: i32,
    pub feat_name: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RaceChangeResult {
    pub old_race_id: i32,
    pub old_race_name: String,
    pub old_subrace: Option<String>,
    pub new_race_id: i32,
    pub new_race_name: String,
    pub new_subrace: Option<String>,
    pub feats_removed: Vec<FeatChange>,
    pub feats_added: Vec<FeatChange>,
}

impl Character {
    pub fn race_id(&self) -> RaceId {
        RaceId(self.get_i32("Race").unwrap_or(0))
    }

    pub fn set_race(&mut self, race_id: RaceId) -> Result<(), CharacterError> {
        let value = u8::try_from(race_id.0).map_err(|_| CharacterError::ValidationFailed {
            field: "race",
            message: format!("Race ID {} does not fit the Race byte", race_id.0),
        })?;
        self.set_byte("Race", value);
        Ok(())
    }

    pub fn subrace_index(&self) -> Option<i32> {
        self.get_i32("Subrace")
    }

    /// TLK display name of the `Subrace` byte's `racialsubtypes` row, else its label.
    pub fn subrace_name(&self, game_data: &GameData) -> Option<String> {
        let idx = self.subrace_index()?;
        self.get_subrace_name_by_index(idx, game_data)
    }

    pub fn get_subrace_name_by_index(&self, index: i32, game_data: &GameData) -> Option<String> {
        let subrace_table = game_data.get_table("racialsubtypes")?;
        let row = subrace_table.get_by_id(index)?;

        if let Some(Some(strref_str)) = row.get("name")
            && let Ok(strref) = strref_str.parse::<i32>()
            && let Some(name) = game_data.get_string(strref)
        {
            return Some(name);
        }

        row.get("label").and_then(|v| v.clone())
    }

    /// TLK-resolved subrace name if present, else race name. Matches the
    /// format NWN2 writes to playerinfo.bin for the load-menu race display.
    pub fn race_display_name(&self, game_data: &GameData) -> String {
        if let Some(name) = self.subrace_name(game_data) {
            return name;
        }
        self.get_race_name_by_id(self.race_id().0, game_data)
    }

    /// `Subrace` is a Byte row index into `racialsubtypes.2da` in every save the
    /// game writes (.bic and .ros); row 0 is Gold Dwarf, not "none". Writing a
    /// string here made the type-strict engine ignore the field.
    pub fn set_subrace_index(&mut self, index: i32) -> Result<(), CharacterError> {
        let value = u8::try_from(index).map_err(|_| CharacterError::ValidationFailed {
            field: "subrace",
            message: format!("Subrace ID {index} does not fit the Subrace byte"),
        })?;
        self.set_byte("Subrace", value);
        Ok(())
    }

    /// `racialtypes.DefaultSubRace`, else the first `racialsubtypes` row of the race.
    pub fn default_subrace_for_race(&self, race_id: i32, game_data: &GameData) -> Option<i32> {
        let from_race = game_data
            .get_table("racialtypes")
            .and_then(|races| races.get_by_id(race_id))
            .map(|row| row_int(&row, "defaultsubrace", -1))
            .filter(|&id| id >= 0);
        from_race.or_else(|| {
            let table = game_data.get_table("racialsubtypes")?;
            (0..table.row_count())
                .find(|&row_idx| {
                    table
                        .parser
                        .get_row_dict(row_idx)
                        .is_ok_and(|row| row_int(&row, "baserace", -1) == race_id)
                })
                .and_then(|row_idx| i32::try_from(row_idx).ok())
        })
    }

    pub fn creature_size(&self) -> i32 {
        self.get_i32("CreatureSize").unwrap_or(3)
    }

    pub fn size_category(&self) -> SizeCategory {
        SizeCategory::from_id(self.creature_size())
    }

    pub fn race_name(&self, game_data: &GameData) -> String {
        self.race_display_name(game_data)
    }

    pub fn get_race_name_by_id(&self, race_id: i32, game_data: &GameData) -> String {
        let Some(races) = game_data.get_table("racialtypes") else {
            return format!("Race {race_id}");
        };

        let Some(row) = races.get_by_id(race_id) else {
            return format!("Race {race_id}");
        };

        if let Some(Some(strref_str)) = row.get("name")
            && let Ok(strref) = strref_str.parse::<i32>()
            && let Some(name) = game_data.get_string(strref)
        {
            return name;
        }

        if let Some(label) = row.get("label").and_then(std::clone::Clone::clone) {
            return label;
        }

        format!("Race {race_id}")
    }

    pub fn available_subraces(&self, game_data: &GameData) -> Vec<SubraceInfo> {
        self.get_available_subraces_for_race(self.race_id().0, game_data)
    }

    pub fn get_available_subraces_for_race(
        &self,
        race_id: i32,
        game_data: &GameData,
    ) -> Vec<SubraceInfo> {
        let Some(subrace_table) = game_data.get_table("racialsubtypes") else {
            return Vec::new();
        };

        let mut subraces = Vec::new();

        for row_idx in 0..subrace_table.row_count() {
            let Ok(row_data) = subrace_table.parser.get_row_dict(row_idx) else {
                continue;
            };

            let base_race_id = row_int(&row_data, "baserace", -1);

            if base_race_id != race_id {
                continue;
            }

            let player_race = row_int(&row_data, "playerrace", 1);

            if player_race == 0 {
                continue;
            }

            let name = row_str(&row_data, "name").unwrap_or_default();
            let label = row_str(&row_data, "label").unwrap_or_else(|| name.clone());

            subraces.push(SubraceInfo {
                id: row_idx as i32,
                name,
                label,
                parent_race: RaceId(race_id),
            });
        }

        subraces
    }

    /// Finds a subrace by `racialsubtypes` label or TLK display name (case-insensitive).
    pub fn get_subrace_data(
        &self,
        subrace_name: &str,
        game_data: &GameData,
    ) -> Option<SubraceData> {
        if subrace_name.is_empty() {
            return None;
        }

        let subrace_table = game_data.get_table("racialsubtypes")?;
        let wanted = subrace_name.to_lowercase();

        (0..subrace_table.row_count())
            .find(|&row_idx| {
                let Ok(row_data) = subrace_table.parser.get_row_dict(row_idx) else {
                    return false;
                };
                let label = row_str(&row_data, "label").unwrap_or_default();
                let strref = row_str(&row_data, "name").unwrap_or_default();
                let tlk_name = strref
                    .parse::<i32>()
                    .ok()
                    .and_then(|strref| game_data.get_string(strref));

                label.to_lowercase() == wanted
                    || strref.to_lowercase() == wanted
                    || tlk_name.is_some_and(|n| n.to_lowercase() == wanted)
            })
            .and_then(|row_idx| self.get_subrace_data_by_index(row_idx as i32, game_data))
    }

    pub fn get_subrace_data_by_index(
        &self,
        index: i32,
        game_data: &GameData,
    ) -> Option<SubraceData> {
        let subrace_table = game_data.get_table("racialsubtypes")?;
        let row_data = subrace_table
            .parser
            .get_row_dict(usize::try_from(index).ok()?)
            .ok()?;

        let name = row_str(&row_data, "name").unwrap_or_default();
        let label = row_str(&row_data, "label").unwrap_or_else(|| name.clone());

        Some(SubraceData {
            id: index,
            name,
            label,
            base_race: row_int(&row_data, "baserace", -1),
            ability_modifiers: ability_modifiers_from_row(&row_data),
            player_race: row_int(&row_data, "playerrace", 1) == 1,
            favored_class: Some(row_int(&row_data, "favored", -1)).filter(|&v| v >= 0),
            feats_table: row_str(&row_data, "featstable").filter(|s| s != "****"),
        })
    }

    pub fn get_racial_ability_modifiers_for_race(
        &self,
        race_id: i32,
        game_data: &GameData,
    ) -> AbilityModifiers {
        let Some(racialtypes_table) = game_data.get_table("racialtypes") else {
            return AbilityModifiers::default();
        };

        let Ok(row_index) = usize::try_from(race_id) else {
            return AbilityModifiers::default();
        };
        if row_index >= racialtypes_table.row_count() {
            return AbilityModifiers::default();
        }

        let Ok(row_data) = racialtypes_table.get_row(row_index) else {
            return AbilityModifiers::default();
        };

        ability_modifiers_from_row(&row_data)
    }

    pub fn get_racial_feats_for_race(&self, race_id: i32, game_data: &GameData) -> Vec<FeatId> {
        let Some(races) = game_data.get_table("racialtypes") else {
            return Vec::new();
        };

        let Some(row) = races.get_by_id(race_id) else {
            return Vec::new();
        };

        featstable_feats(game_data, row_str(&row, "featstable").as_deref())
    }

    pub fn validate_race_change(
        &self,
        new_race_id: i32,
        new_subrace: Option<&str>,
        game_data: &GameData,
    ) -> ValidationResult {
        let Some(races) = game_data.get_table("racialtypes") else {
            return ValidationResult::error("Could not load racialtypes table");
        };

        if u8::try_from(new_race_id).is_err() || races.get_by_id(new_race_id).is_none() {
            return ValidationResult::error(format!("Unknown race ID: {new_race_id}"));
        }

        match new_subrace {
            Some(subrace_name) => self.validate_subrace(new_race_id, subrace_name, game_data),
            None => ValidationResult::ok(),
        }
    }

    pub fn validate_subrace(
        &self,
        race_id: i32,
        subrace_name: &str,
        game_data: &GameData,
    ) -> ValidationResult {
        if subrace_name.is_empty() {
            return ValidationResult::ok();
        }

        let Some(subrace_data) = self.get_subrace_data(subrace_name, game_data) else {
            return ValidationResult::error(format!("Unknown subrace: {subrace_name}"));
        };

        if subrace_data.base_race != race_id {
            return ValidationResult::error(format!(
                "Subrace '{subrace_name}' does not belong to race ID {race_id}"
            ));
        }

        ValidationResult::ok()
    }

    pub fn change_race(
        &mut self,
        new_race_id: i32,
        new_subrace: Option<String>,
        preserve_feats: bool,
        game_data: &GameData,
    ) -> Result<RaceChangeResult, CharacterError> {
        debug!(
            "Changing race to {} (subrace: {:?})",
            new_race_id, new_subrace
        );

        let validation = self.validate_race_change(new_race_id, new_subrace.as_deref(), game_data);
        if !validation.valid {
            return Err(CharacterError::ValidationFailed {
                field: "race",
                message: validation.errors.join("; "),
            });
        }

        let old_race_id = self.race_id().0;
        let old_subrace_data = self
            .subrace_index()
            .and_then(|idx| self.get_subrace_data_by_index(idx, game_data));
        let new_subrace_data = match new_subrace.as_deref().filter(|s| !s.is_empty()) {
            Some(name) => self.get_subrace_data(name, game_data),
            None => self
                .default_subrace_for_race(new_race_id, game_data)
                .and_then(|id| self.get_subrace_data_by_index(id, game_data)),
        };
        let new_subrace_id = new_subrace_data.as_ref().map_or_else(
            || {
                warn!("No default subrace for race {new_race_id}; writing Subrace 0");
                0
            },
            |data| data.id,
        );
        u8::try_from(new_subrace_id).map_err(|_| CharacterError::ValidationFailed {
            field: "subrace",
            message: format!("Subrace ID {new_subrace_id} does not fit the Subrace byte"),
        })?;

        let mut result = RaceChangeResult {
            old_race_id,
            old_race_name: self.race_name(game_data),
            old_subrace: self.subrace_name(game_data),
            new_race_id,
            new_race_name: self.get_race_name_by_id(new_race_id, game_data),
            new_subrace: new_subrace_data.as_ref().map(|data| data.label.clone()),
            ..Default::default()
        };

        let base_con = self.base_ability(AbilityIndex::CON);
        let old_con = base_con + self.get_racial_modifier_deltas(game_data).con_mod;

        self.set_race(RaceId(new_race_id))?;
        self.set_subrace_index(new_subrace_id)?;

        let new_con = base_con + self.get_racial_modifier_deltas(game_data).con_mod;
        self.recalculate_hit_points(old_con, new_con);

        if !preserve_feats {
            for feat_id in self.get_racial_feats_for_race(old_race_id, game_data) {
                if self.has_feat(feat_id) && self.remove_feat(feat_id).is_ok() {
                    result.feats_removed.push(FeatChange {
                        feat_id: feat_id.0,
                        feat_name: self.get_feat_name(feat_id, game_data),
                        source: "race".to_string(),
                    });
                }
            }

            if let Some(old_sub_data) = &old_subrace_data {
                for feat_id in featstable_feats(game_data, old_sub_data.feats_table.as_deref()) {
                    if self.has_feat(feat_id) && self.remove_feat(feat_id).is_ok() {
                        result.feats_removed.push(FeatChange {
                            feat_id: feat_id.0,
                            feat_name: self.get_feat_name(feat_id, game_data),
                            source: "subrace".to_string(),
                        });
                    }
                }
            }
        }

        for feat_id in self.get_racial_feats_for_race(new_race_id, game_data) {
            if !self.has_feat(feat_id)
                && self.add_feat_with_source(feat_id, FeatSource::Race).is_ok()
            {
                result.feats_added.push(FeatChange {
                    feat_id: feat_id.0,
                    feat_name: self.get_feat_name(feat_id, game_data),
                    source: "race".to_string(),
                });
            }
        }

        if let Some(new_sub_data) = &new_subrace_data {
            for feat_id in featstable_feats(game_data, new_sub_data.feats_table.as_deref()) {
                if !self.has_feat(feat_id)
                    && self.add_feat_with_source(feat_id, FeatSource::Race).is_ok()
                {
                    result.feats_added.push(FeatChange {
                        feat_id: feat_id.0,
                        feat_name: self.get_feat_name(feat_id, game_data),
                        source: "subrace".to_string(),
                    });
                }
            }
        }

        if !preserve_feats {
            self.remove_stale_racial_feats(
                new_race_id,
                new_subrace_data.as_ref(),
                game_data,
                &mut result,
            );
        }

        debug!("Race change complete: {:?}", result);
        Ok(result)
    }

    fn remove_stale_racial_feats(
        &mut self,
        new_race_id: i32,
        new_subrace: Option<&SubraceData>,
        game_data: &GameData,
        result: &mut RaceChangeResult,
    ) {
        let mut new_race_feats: HashSet<FeatId> = self
            .get_racial_feats_for_race(new_race_id, game_data)
            .into_iter()
            .collect();

        if let Some(sub_data) = new_subrace {
            new_race_feats.extend(featstable_feats(game_data, sub_data.feats_table.as_deref()));
        }

        let Some(feats_table) = game_data.get_table("feat") else {
            return;
        };

        let stale: Vec<FeatId> = self
            .feat_entries()
            .iter()
            .filter(|e| !new_race_feats.contains(&e.feat_id))
            .filter(|e| matches!(e.source, FeatSource::Race | FeatSource::Unknown))
            .filter_map(|e| {
                let row = feats_table.get_by_id(e.feat_id.0)?;
                let cat = row
                    .get("featcategory")
                    .and_then(|s: &Option<String>| s.as_ref())?;
                (FeatType::from_string(cat) == FeatType::RACIAL).then_some(e.feat_id)
            })
            .collect();

        for feat_id in stale {
            if self.remove_feat(feat_id).is_ok() {
                result.feats_removed.push(FeatChange {
                    feat_id: feat_id.0,
                    feat_name: self.get_feat_name(feat_id, game_data),
                    source: "stale_racial".to_string(),
                });
            }
        }
    }

    pub fn get_size_name_from_2da(&self, size_id: i32, game_data: &GameData) -> String {
        let Some(size_table) = game_data.get_table("creaturesize") else {
            return SizeCategory::from_id(size_id).name().to_string();
        };

        let Some(row) = size_table.get_by_id(size_id) else {
            return SizeCategory::from_id(size_id).name().to_string();
        };

        if let Some(label) = row
            .get("label")
            .and_then(std::clone::Clone::clone)
            .filter(|s| !s.is_empty() && s != "INVALID")
        {
            let mut chars = label.chars();
            match chars.next() {
                None => label,
                Some(first) => {
                    first.to_uppercase().collect::<String>()
                        + chars.as_str().to_lowercase().as_str()
                }
            }
        } else {
            SizeCategory::from_id(size_id).name().to_string()
        }
    }

    pub fn get_racial_modifier_deltas(&self, game_data: &GameData) -> AbilityModifiers {
        let race_id = self.race_id().0;
        if let Some(idx) = self.subrace_index()
            && let Some(subrace_data) = self.get_subrace_data_by_index(idx, game_data)
            && subrace_data.base_race == race_id
        {
            trace!(
                "get_racial_modifier_deltas: found subrace by index {}: {:?}",
                idx, subrace_data.ability_modifiers
            );
            return subrace_data.ability_modifiers;
        }

        let mods = self.get_racial_ability_modifiers_for_race(race_id, game_data);
        trace!(
            "get_racial_modifier_deltas: using base race modifiers: {:?}",
            mods
        );
        mods
    }
}

fn ability_modifiers_from_row(row: &Row) -> AbilityModifiers {
    AbilityModifiers::new(
        row_int(row, "stradjust", 0),
        row_int(row, "dexadjust", 0),
        row_int(row, "conadjust", 0),
        row_int(row, "intadjust", 0),
        row_int(row, "wisadjust", 0),
        row_int(row, "chaadjust", 0),
    )
}

fn featstable_feats(game_data: &GameData, table_name: Option<&str>) -> Vec<FeatId> {
    let Some(name) = table_name.filter(|s| !s.is_empty() && *s != "****") else {
        return Vec::new();
    };
    let Some(table) = game_data.get_table(&name.to_lowercase()) else {
        return Vec::new();
    };

    (0..table.row_count())
        .filter_map(|row_idx| table.parser.get_row_dict(row_idx).ok())
        .map(|row| row_int(&row, "featindex", -1))
        .filter(|&id| id >= 0)
        .map(FeatId)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;
    use indexmap::IndexMap;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("Race".to_string(), GffValue::Byte(1));
        fields.insert("Subrace".to_string(), GffValue::Byte(0));
        fields.insert("CreatureSize".to_string(), GffValue::Int(3));
        Character::from_gff(fields)
    }

    #[test]
    fn test_race_id() {
        let character = create_test_character();
        assert_eq!(character.race_id(), RaceId(1));
    }

    #[test]
    fn test_set_race() {
        let mut character = create_test_character();
        character.set_race(RaceId(5)).unwrap();
        assert_eq!(character.race_id(), RaceId(5));
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_race_rejects_out_of_byte_range() {
        let mut character = create_test_character();
        assert!(character.set_race(RaceId(256)).is_err());
        assert!(character.set_race(RaceId(-1)).is_err());
        assert_eq!(character.race_id(), RaceId(1));
        assert!(!character.is_modified());
    }

    #[test]
    fn test_subrace_index_reads_byte() {
        let mut fields = IndexMap::new();
        fields.insert("Race".to_string(), GffValue::Byte(21));
        fields.insert("Subrace".to_string(), GffValue::Byte(13));
        let character = Character::from_gff(fields);
        assert_eq!(character.subrace_index(), Some(13));
    }

    #[test]
    fn test_subrace_index_zero_is_a_real_row() {
        let character = create_test_character();
        assert_eq!(character.subrace_index(), Some(0));
    }

    #[test]
    fn test_subrace_index_missing_field_is_none() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.subrace_index(), None);
    }

    #[test]
    fn test_set_subrace_index_writes_byte() {
        let mut character = create_test_character();
        character.set_subrace_index(14).unwrap();
        assert_eq!(character.subrace_index(), Some(14));
        assert!(matches!(
            character.gff().get("Subrace"),
            Some(GffValue::Byte(14))
        ));
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_subrace_index_rejects_out_of_byte_range() {
        let mut character = create_test_character();
        assert!(character.set_subrace_index(256).is_err());
        assert!(character.set_subrace_index(-1).is_err());
        assert!(!character.is_modified());
    }

    #[test]
    fn test_creature_size() {
        let character = create_test_character();
        assert_eq!(character.creature_size(), 3);
    }

    #[test]
    fn test_creature_size_defaults_to_medium() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.creature_size(), 3);
    }

    #[test]
    fn test_size_category() {
        let character = create_test_character();
        assert_eq!(character.size_category(), SizeCategory::Medium);
    }

    #[test]
    fn test_size_category_from_id() {
        assert_eq!(SizeCategory::from_id(0), SizeCategory::Invalid);
        assert_eq!(SizeCategory::from_id(1), SizeCategory::Tiny);
        assert_eq!(SizeCategory::from_id(2), SizeCategory::Small);
        assert_eq!(SizeCategory::from_id(3), SizeCategory::Medium);
        assert_eq!(SizeCategory::from_id(4), SizeCategory::Large);
        assert_eq!(SizeCategory::from_id(5), SizeCategory::Huge);
        assert_eq!(SizeCategory::from_id(100), SizeCategory::Medium);
    }

    #[test]
    fn test_size_category_modifiers() {
        assert_eq!(SizeCategory::Tiny.ac_modifier_default(), 2);
        assert_eq!(SizeCategory::Small.ac_modifier_default(), 1);
        assert_eq!(SizeCategory::Medium.ac_modifier_default(), 0);
        assert_eq!(SizeCategory::Large.ac_modifier_default(), -1);
        assert_eq!(SizeCategory::Huge.ac_modifier_default(), -2);
    }

    #[test]
    fn test_size_category_names() {
        assert_eq!(SizeCategory::Tiny.name(), "Tiny");
        assert_eq!(SizeCategory::Medium.name(), "Medium");
        assert_eq!(SizeCategory::Huge.name(), "Huge");
    }
}
