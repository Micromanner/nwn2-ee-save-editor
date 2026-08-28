use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashSet;
use tracing::{debug, trace};

use super::{Character, CharacterError};
use crate::character::feats::{FeatSource, FeatType};
use crate::character::types::{AbilityIndex, AbilityModifiers, ClassId, FeatId, RaceId};
use crate::loaders::{GameData, LoadedTable};
use crate::utils::parsing::{row_int, row_str};

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
pub struct AbilityChange {
    pub attribute: String,
    pub old_value: i32,
    pub new_value: i32,
    pub modifier: i32,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeatChange {
    pub feat_id: i32,
    pub feat_name: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SizeChange {
    pub old_size: i32,
    pub new_size: i32,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeedChange {
    pub old_speed: i32,
    pub new_speed: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RaceChangeResult {
    pub old_race_id: i32,
    pub old_race_name: String,
    pub old_subrace: Option<String>,
    pub new_race_id: i32,
    pub new_race_name: String,
    pub new_subrace: Option<String>,
    pub ability_changes: Vec<AbilityChange>,
    pub feats_removed: Vec<FeatChange>,
    pub feats_added: Vec<FeatChange>,
    pub size_change: Option<SizeChange>,
    pub speed_change: Option<SpeedChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacialProperties {
    pub race_id: i32,
    pub race_name: String,
    pub subrace: Option<String>,
    pub size: i32,
    pub size_name: String,
    pub base_speed: i32,
    pub ability_modifiers: AbilityModifiers,
    pub racial_feats: Vec<i32>,
    pub favored_class: Option<i32>,
    pub available_subraces: Vec<SubraceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn failure(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

impl Character {
    pub fn race_id(&self) -> RaceId {
        RaceId(self.get_i32("Race").unwrap_or(0))
    }

    pub fn set_race(&mut self, race_id: RaceId) {
        self.set_byte("Race", race_id.0 as u8);
    }

    pub fn subrace_index(&self) -> Option<i32> {
        let val = self.get_i32("Subrace")?;
        if val > 0 { Some(val) } else { None }
    }

    pub fn subrace_string(&self) -> Option<String> {
        match self.get_string("Subrace") {
            Some(s) if !s.is_empty() && s != "0" => Some(s.to_string()),
            _ => None,
        }
    }

    pub fn subrace(&self) -> Option<String> {
        self.subrace_string()
    }

    pub fn subrace_name(&self, game_data: &GameData) -> Option<String> {
        if let Some(name) = self.subrace_string() {
            return Some(name);
        }
        if let Some(idx) = self.subrace_index() {
            return self.get_subrace_name_by_index(idx, game_data);
        }
        None
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

    /// Resolves a subrace string (2DA label or TLK name) to its TLK display name.
    pub fn get_subrace_display_name_by_string(
        &self,
        raw: &str,
        game_data: &GameData,
    ) -> Option<String> {
        let subrace_table = game_data.get_table("racialsubtypes")?;

        for row_idx in 0..subrace_table.row_count() {
            let Ok(row_data) = subrace_table.parser.get_row_dict(row_idx) else {
                continue;
            };

            let row_label = row_str(&row_data, "label").unwrap_or_default();
            let tlk_name: Option<String> = row_data
                .get("name")
                .and_then(|v| v.as_ref())
                .and_then(|s| s.parse::<i32>().ok())
                .and_then(|strref| game_data.get_string(strref));

            if row_label.eq_ignore_ascii_case(raw)
                || tlk_name
                    .as_ref()
                    .is_some_and(|n| n.eq_ignore_ascii_case(raw))
            {
                return tlk_name.or(Some(row_label));
            }
        }
        None
    }

    /// TLK-resolved subrace name if present, else race name. Matches the
    /// format NWN2 writes to playerinfo.bin for the load-menu race display.
    pub fn race_display_name(&self, game_data: &GameData) -> String {
        if let Some(idx) = self.subrace_index()
            && let Some(name) = self.get_subrace_name_by_index(idx, game_data)
        {
            return name;
        }
        if let Some(raw) = self.subrace_string() {
            if let Some(name) = self.get_subrace_display_name_by_string(&raw, game_data) {
                return name;
            }
            return raw;
        }
        self.get_race_name_by_id(self.race_id().0, game_data)
    }

    /// `Subrace` is a Byte row index into `racialsubtypes.2da` in every save the
    /// game writes (.bic and .ros); 0 means none. Writing a string here made the
    /// type-strict engine ignore the field.
    pub fn set_subrace_index(&mut self, index: Option<i32>) {
        let value = index.filter(|&i| i > 0).unwrap_or(0);
        self.set_byte("Subrace", value.clamp(0, i32::from(u8::MAX)) as u8);
    }

    pub fn creature_size(&self) -> i32 {
        self.get_i32("CreatureSize").unwrap_or(4)
    }

    pub fn set_creature_size(&mut self, size: i32) {
        self.set_i32("CreatureSize", size);
    }

    pub fn size_category(&self) -> SizeCategory {
        SizeCategory::from_id(self.creature_size())
    }

    pub fn race_name(&self, game_data: &GameData) -> String {
        if let Some(subrace_name) = self.subrace_name(game_data) {
            return subrace_name;
        }
        self.get_race_name_by_id(self.race_id().0, game_data)
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

    pub fn get_subrace_data(
        &self,
        subrace_name: &str,
        game_data: &GameData,
    ) -> Option<SubraceData> {
        if subrace_name.is_empty() {
            return None;
        }

        let subrace_table = game_data.get_table("racialsubtypes")?;
        let subrace_lower = subrace_name.to_lowercase();

        for row_idx in 0..subrace_table.row_count() {
            let Ok(row_data) = subrace_table.parser.get_row_dict(row_idx) else {
                continue;
            };

            let name = row_str(&row_data, "name").unwrap_or_default();
            let label = row_str(&row_data, "label").unwrap_or_else(|| name.clone());

            if name.to_lowercase() != subrace_lower && label.to_lowercase() != subrace_lower {
                continue;
            }

            let base_race = row_int(&row_data, "baserace", -1);
            let player_race = row_int(&row_data, "playerrace", 1) == 1;

            let favored_class = row_data
                .get("favoredclass")
                .and_then(|v| v.as_ref())
                .and_then(|s| s.parse::<i32>().ok())
                .filter(|&v| v >= 0);

            let feats_table = row_str(&row_data, "featstable").filter(|s| s != "****");

            let ability_modifiers = AbilityModifiers::new(
                row_int(&row_data, "stradjust", 0),
                row_int(&row_data, "dexadjust", 0),
                row_int(&row_data, "conadjust", 0),
                row_int(&row_data, "intadjust", 0),
                row_int(&row_data, "wisadjust", 0),
                row_int(&row_data, "chaadjust", 0),
            );

            return Some(SubraceData {
                id: row_idx as i32,
                name,
                label,
                base_race,
                ability_modifiers,
                player_race,
                favored_class,
                feats_table,
            });
        }

        None
    }

    pub fn get_racial_ability_modifiers_for_race(
        &self,
        race_id: i32,
        game_data: &GameData,
    ) -> AbilityModifiers {
        let Some(racialtypes_table) = game_data.get_table("racialtypes") else {
            return AbilityModifiers::default();
        };

        let row_index = race_id as usize;
        if row_index >= racialtypes_table.row_count() {
            return AbilityModifiers::default();
        }

        let Ok(row_data) = racialtypes_table.get_row(row_index) else {
            return AbilityModifiers::default();
        };

        AbilityModifiers::new(
            row_int(&row_data, "stradjust", 0),
            row_int(&row_data, "dexadjust", 0),
            row_int(&row_data, "conadjust", 0),
            row_int(&row_data, "intadjust", 0),
            row_int(&row_data, "wisadjust", 0),
            row_int(&row_data, "chaadjust", 0),
        )
    }

    pub fn get_race_size(&self, race_id: i32, game_data: &GameData) -> Option<i32> {
        let races = game_data.get_table("racialtypes")?;
        let row = races.get_by_id(race_id)?;

        row.get("size")
            .and_then(std::clone::Clone::clone)
            .and_then(|s| s.parse::<i32>().ok())
    }

    pub fn get_base_speed(&self, game_data: &GameData) -> i32 {
        self.get_base_speed_for_race(self.race_id().0, game_data)
    }

    pub fn get_base_speed_for_race(&self, race_id: i32, game_data: &GameData) -> i32 {
        let Some(races) = game_data.get_table("racialtypes") else {
            return 30;
        };

        let Some(row) = races.get_by_id(race_id) else {
            return 30;
        };

        row_int(&row, "movementrate", 30)
    }

    pub fn get_favored_class(&self, game_data: &GameData) -> Option<ClassId> {
        self.get_favored_class_for_race(self.race_id().0, game_data)
    }

    pub fn get_favored_class_for_race(
        &self,
        race_id: i32,
        game_data: &GameData,
    ) -> Option<ClassId> {
        let races = game_data.get_table("racialtypes")?;
        let row = races.get_by_id(race_id)?;

        row.get("favoredclass")
            .and_then(std::clone::Clone::clone)
            .and_then(|s| s.parse::<i32>().ok())
            .filter(|&v| v >= 0)
            .map(ClassId)
    }

    pub fn get_racial_feats_for_race(&self, race_id: i32, game_data: &GameData) -> Vec<FeatId> {
        let mut feats = HashSet::new();

        let Some(races) = game_data.get_table("racialtypes") else {
            return Vec::new();
        };

        let Some(row) = races.get_by_id(race_id) else {
            return Vec::new();
        };

        if let Some(feats_table_name) = row
            .get("featstable")
            .and_then(std::clone::Clone::clone)
            .filter(|s| !s.is_empty() && s != "****")
            && let Some(feats_table) = game_data.get_table(&feats_table_name.to_lowercase())
        {
            for row_idx in 0..feats_table.row_count() {
                if let Ok(feat_row) = feats_table.parser.get_row_dict(row_idx)
                    && let Some(feat_id) = feat_row
                        .get("featindex")
                        .and_then(|v| v.as_ref())
                        .and_then(|s| s.parse::<i32>().ok())
                        .filter(|&v| v >= 0)
                {
                    feats.insert(FeatId(feat_id));
                }
            }
        }

        for i in 1..=5 {
            let field_name = format!("Feat{i}");
            if let Some(feat_id) = row
                .get(&field_name)
                .and_then(std::clone::Clone::clone)
                .and_then(|s| s.parse::<i32>().ok())
                .filter(|&v| v >= 0)
            {
                feats.insert(FeatId(feat_id));
            }
        }

        feats.into_iter().collect()
    }

    pub fn get_all_racial_feats(&self, game_data: &GameData) -> Vec<FeatId> {
        let mut feats = HashSet::new();

        for feat in self.get_racial_feats_for_race(self.race_id().0, game_data) {
            feats.insert(feat);
        }

        if let Some(subrace_name) = self.subrace()
            && let Some(subrace_data) = self.get_subrace_data(&subrace_name, game_data)
            && let Some(feats_table_name) = subrace_data.feats_table
            && let Some(feats_table) = game_data.get_table(&feats_table_name.to_lowercase())
        {
            for row_idx in 0..feats_table.row_count() {
                if let Ok(feat_row) = feats_table.parser.get_row_dict(row_idx)
                    && let Some(feat_id) = feat_row
                        .get("featindex")
                        .and_then(|v| v.as_ref())
                        .and_then(|s| s.parse::<i32>().ok())
                        .filter(|&v| v >= 0)
                {
                    feats.insert(FeatId(feat_id));
                }
            }
        }

        feats.into_iter().collect()
    }

    pub fn validate_race_change(
        &self,
        new_race_id: i32,
        new_subrace: Option<&str>,
        game_data: &GameData,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        let Some(races) = game_data.get_table("racialtypes") else {
            errors.push("Could not load racialtypes table".to_string());
            return ValidationResult::failure(errors);
        };

        if races.get_by_id(new_race_id).is_none() {
            errors.push(format!("Unknown race ID: {new_race_id}"));
            return ValidationResult::failure(errors);
        }

        if let Some(subrace_name) = new_subrace {
            let result = self.validate_subrace(new_race_id, subrace_name, game_data);
            if !result.valid {
                errors.extend(result.errors);
            }
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }

    pub fn validate_subrace(
        &self,
        race_id: i32,
        subrace_name: &str,
        game_data: &GameData,
    ) -> ValidationResult {
        if subrace_name.is_empty() {
            return ValidationResult::success();
        }

        let Some(subrace_data) = self.get_subrace_data(subrace_name, game_data) else {
            return ValidationResult::failure(vec![format!("Unknown subrace: {}", subrace_name)]);
        };

        if subrace_data.base_race != race_id {
            return ValidationResult::failure(vec![format!(
                "Subrace '{}' does not belong to race ID {}",
                subrace_name, race_id
            )]);
        }

        if !subrace_data.player_race {
            return ValidationResult::failure(vec![format!(
                "Subrace '{}' is not available to players",
                subrace_name
            )]);
        }

        ValidationResult::success()
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
        let old_subrace = self.subrace_name(game_data);
        let old_race_name = self.race_name(game_data);

        let mut result = RaceChangeResult {
            old_race_id,
            old_race_name,
            old_subrace: old_subrace.clone(),
            new_race_id,
            new_race_name: self.get_race_name_by_id(new_race_id, game_data),
            new_subrace: new_subrace.clone(),
            ..Default::default()
        };

        let old_racial_mods = self.get_racial_ability_modifiers_for_race(old_race_id, game_data);
        self.apply_ability_modifier_changes(&old_racial_mods, false, "race_removed", &mut result);

        if let Some(ref old_sub_name) = old_subrace
            && let Some(old_sub_data) = self.get_subrace_data(old_sub_name, game_data)
        {
            self.apply_ability_modifier_changes(
                &old_sub_data.ability_modifiers,
                false,
                "subrace_removed",
                &mut result,
            );
        }

        self.set_race(RaceId(new_race_id));
        let new_subrace_index = new_subrace
            .as_deref()
            .and_then(|name| self.get_subrace_data(name, game_data))
            .map(|data| data.id);
        self.set_subrace_index(new_subrace_index);

        let new_racial_mods = self.get_racial_ability_modifiers_for_race(new_race_id, game_data);
        self.apply_ability_modifier_changes(&new_racial_mods, true, "race", &mut result);

        if let Some(ref new_sub_name) = new_subrace
            && let Some(new_sub_data) = self.get_subrace_data(new_sub_name, game_data)
        {
            self.apply_ability_modifier_changes(
                &new_sub_data.ability_modifiers,
                true,
                "subrace",
                &mut result,
            );
        }

        let old_size = self.creature_size();
        if let Some(new_size) = self.get_race_size(new_race_id, game_data)
            && old_size != new_size
        {
            self.set_creature_size(new_size);
            result.size_change = Some(SizeChange {
                old_size,
                new_size,
                old_name: self.get_size_name_from_2da(old_size, game_data),
                new_name: self.get_size_name_from_2da(new_size, game_data),
            });
        }

        let old_speed = self.get_base_speed_for_race(old_race_id, game_data);
        let new_speed = self.get_base_speed_for_race(new_race_id, game_data);
        if old_speed != new_speed {
            result.speed_change = Some(SpeedChange {
                old_speed,
                new_speed,
            });
        }

        if !preserve_feats {
            let old_racial_feats = self.get_racial_feats_for_race(old_race_id, game_data);
            for feat_id in old_racial_feats {
                if self.has_feat(feat_id) && self.remove_feat(feat_id).is_ok() {
                    result.feats_removed.push(FeatChange {
                        feat_id: feat_id.0,
                        feat_name: self.get_feat_name(feat_id, game_data),
                        source: "race".to_string(),
                    });
                }
            }

            if let Some(ref old_sub_name) = old_subrace
                && let Some(old_sub_data) = self.get_subrace_data(old_sub_name, game_data)
                && let Some(feats_table_name) = old_sub_data.feats_table
                && let Some(feats_table) = game_data.get_table(&feats_table_name.to_lowercase())
            {
                for feat_id in featstable_feats(feats_table) {
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

        let new_racial_feats = self.get_racial_feats_for_race(new_race_id, game_data);
        for feat_id in new_racial_feats {
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

        if let Some(ref new_sub_name) = new_subrace
            && let Some(new_sub_data) = self.get_subrace_data(new_sub_name, game_data)
            && let Some(feats_table_name) = new_sub_data.feats_table
            && let Some(feats_table) = game_data.get_table(&feats_table_name.to_lowercase())
        {
            for feat_id in featstable_feats(feats_table) {
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
                new_subrace.as_deref(),
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
        new_subrace: Option<&str>,
        game_data: &GameData,
        result: &mut RaceChangeResult,
    ) {
        let mut new_race_feats: HashSet<FeatId> = self
            .get_racial_feats_for_race(new_race_id, game_data)
            .into_iter()
            .collect();

        if let Some(sub_name) = new_subrace
            && let Some(sub_data) = self.get_subrace_data(sub_name, game_data)
            && let Some(feats_table_name) = sub_data.feats_table
            && let Some(feats_table) = game_data.get_table(&feats_table_name.to_lowercase())
        {
            new_race_feats.extend(featstable_feats(feats_table));
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

    fn apply_ability_modifier_changes(
        &mut self,
        mods: &AbilityModifiers,
        adding: bool,
        source: &str,
        result: &mut RaceChangeResult,
    ) {
        let mod_values = [
            (AbilityIndex::STR, mods.str_mod),
            (AbilityIndex::DEX, mods.dex_mod),
            (AbilityIndex::CON, mods.con_mod),
            (AbilityIndex::INT, mods.int_mod),
            (AbilityIndex::WIS, mods.wis_mod),
            (AbilityIndex::CHA, mods.cha_mod),
        ];

        for (ability, modifier) in mod_values {
            if modifier == 0 {
                continue;
            }

            let current = self.base_ability(ability);
            let new_value = if adding {
                current + modifier
            } else {
                current - modifier
            };

            if self.set_ability(ability, new_value).is_ok() {
                result.ability_changes.push(AbilityChange {
                    attribute: ability.gff_field().to_string(),
                    old_value: current,
                    new_value,
                    modifier: if adding { modifier } else { -modifier },
                    source: source.to_string(),
                });
            }
        }
    }

    pub fn get_racial_properties(&self, game_data: &GameData) -> RacialProperties {
        let race_id = self.race_id().0;
        let subrace = self.subrace();

        let mut ability_modifiers = self.get_racial_ability_modifiers_for_race(race_id, game_data);

        if let Some(ref subrace_name) = subrace
            && let Some(subrace_data) = self.get_subrace_data(subrace_name, game_data)
        {
            ability_modifiers = AbilityModifiers::new(
                ability_modifiers.str_mod + subrace_data.ability_modifiers.str_mod,
                ability_modifiers.dex_mod + subrace_data.ability_modifiers.dex_mod,
                ability_modifiers.con_mod + subrace_data.ability_modifiers.con_mod,
                ability_modifiers.int_mod + subrace_data.ability_modifiers.int_mod,
                ability_modifiers.wis_mod + subrace_data.ability_modifiers.wis_mod,
                ability_modifiers.cha_mod + subrace_data.ability_modifiers.cha_mod,
            );
        }

        let size = self.creature_size();

        RacialProperties {
            race_id,
            race_name: self.race_name(game_data),
            subrace,
            size,
            size_name: SizeCategory::from_id(size).name().to_string(),
            base_speed: self.get_base_speed(game_data),
            ability_modifiers,
            racial_feats: self
                .get_all_racial_feats(game_data)
                .into_iter()
                .map(|f| f.0)
                .collect(),
            favored_class: self.get_favored_class(game_data).map(|c| c.0),
            available_subraces: self.available_subraces(game_data),
        }
    }

    pub fn get_size_modifier(&self, size_id: i32, game_data: &GameData) -> i32 {
        let Some(size_table) = game_data.get_table("creaturesize") else {
            return SizeCategory::from_id(size_id).ac_modifier_default();
        };

        let Some(row) = size_table.get_by_id(size_id) else {
            return SizeCategory::from_id(size_id).ac_modifier_default();
        };

        row_int(
            &row,
            "acattackmod",
            SizeCategory::from_id(size_id).ac_modifier_default(),
        )
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

    pub fn validate_race(&self, game_data: &GameData) -> ValidationResult {
        let mut errors = Vec::new();

        let race_id = self.race_id().0;

        let Some(races) = game_data.get_table("racialtypes") else {
            errors.push("Could not load racialtypes table".to_string());
            return ValidationResult::failure(errors);
        };

        if races.get_by_id(race_id).is_none() {
            errors.push(format!("Invalid race ID: {race_id}"));
        }

        if errors.is_empty() {
            ValidationResult::success()
        } else {
            ValidationResult::failure(errors)
        }
    }

    pub fn get_subrace_data_by_index(
        &self,
        index: i32,
        game_data: &GameData,
    ) -> Option<SubraceData> {
        let subrace_table = game_data.get_table("racialsubtypes")?;
        let row_data = subrace_table.parser.get_row_dict(index as usize).ok()?;

        let name = row_str(&row_data, "name").unwrap_or_default();
        let label = row_str(&row_data, "label").unwrap_or_else(|| name.clone());

        let base_race = row_int(&row_data, "baserace", -1);
        let player_race = row_int(&row_data, "playerrace", 1) == 1;

        let favored_class = row_data
            .get("favoredclass")
            .and_then(|v| v.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .filter(|&v| v >= 0);

        let feats_table = row_str(&row_data, "featstable").filter(|s| s != "****");

        let ability_modifiers = AbilityModifiers::new(
            row_int(&row_data, "stradjust", 0),
            row_int(&row_data, "dexadjust", 0),
            row_int(&row_data, "conadjust", 0),
            row_int(&row_data, "intadjust", 0),
            row_int(&row_data, "wisadjust", 0),
            row_int(&row_data, "chaadjust", 0),
        );

        Some(SubraceData {
            id: index,
            name,
            label,
            base_race,
            ability_modifiers,
            player_race,
            favored_class,
            feats_table,
        })
    }

    pub fn get_racial_modifier_deltas(&self, game_data: &GameData) -> AbilityModifiers {
        let race_id = self.race_id().0;

        if let Some(ref subrace_name) = self.subrace_string()
            && let Some(subrace_data) = self.get_subrace_data(subrace_name, game_data)
        {
            trace!(
                "get_racial_modifier_deltas: found subrace by name '{}': {:?}",
                subrace_name, subrace_data.ability_modifiers
            );
            return subrace_data.ability_modifiers;
        }

        if let Some(idx) = self.subrace_index()
            && let Some(subrace_data) = self.get_subrace_data_by_index(idx, game_data)
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

fn featstable_feats(table: &LoadedTable) -> Vec<FeatId> {
    let mut feats = Vec::new();
    for row_idx in 0..table.row_count() {
        if let Ok(row) = table.parser.get_row_dict(row_idx)
            && let Some(id) = row
                .get("featindex")
                .and_then(|v| v.as_ref())
                .and_then(|s| s.parse::<i32>().ok())
                .filter(|&v| v >= 0)
        {
            feats.push(FeatId(id));
        }
    }
    feats
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;
    use indexmap::IndexMap;
    use std::borrow::Cow;

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert("Race".to_string(), GffValue::Byte(1));
        fields.insert(
            "Subrace".to_string(),
            GffValue::String(Cow::Owned("Moon Elf".to_string())),
        );
        fields.insert("CreatureSize".to_string(), GffValue::Int(3)); // 3 = Medium in NWN2
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
        character.set_race(RaceId(5));
        assert_eq!(character.race_id(), RaceId(5));
        assert!(character.is_modified());
    }

    #[test]
    fn test_subrace() {
        let character = create_test_character();
        assert_eq!(character.subrace(), Some("Moon Elf".to_string()));
    }

    #[test]
    fn test_subrace_empty() {
        let mut fields = IndexMap::new();
        fields.insert("Race".to_string(), GffValue::Byte(1));
        fields.insert(
            "Subrace".to_string(),
            GffValue::String(Cow::Owned(String::new())),
        );
        let character = Character::from_gff(fields);
        assert_eq!(character.subrace(), None);
    }

    #[test]
    fn test_set_subrace_index_writes_byte() {
        let mut character = create_test_character();
        character.set_subrace_index(Some(14));
        assert_eq!(character.subrace_index(), Some(14));
        assert!(matches!(
            character.gff().get("Subrace"),
            Some(GffValue::Byte(14))
        ));
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_subrace_index_none_writes_zero_byte() {
        let mut character = create_test_character();
        character.set_subrace_index(None);
        assert_eq!(character.subrace_index(), None);
        assert!(matches!(
            character.gff().get("Subrace"),
            Some(GffValue::Byte(0))
        ));
        assert!(character.is_modified());
    }

    #[test]
    fn test_creature_size() {
        let character = create_test_character();
        assert_eq!(character.creature_size(), 3);
    }

    #[test]
    fn test_set_creature_size() {
        let mut character = create_test_character();
        character.set_creature_size(2);
        assert_eq!(character.creature_size(), 2);
        assert!(character.is_modified());
    }

    #[test]
    fn test_size_category() {
        let character = create_test_character();
        assert_eq!(character.size_category(), SizeCategory::Medium);
    }

    #[test]
    fn test_size_category_from_id() {
        // NWN2 creaturesize.2da: 0=Invalid, 1=Tiny, 2=Small, 3=Medium, 4=Large, 5=Huge
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
