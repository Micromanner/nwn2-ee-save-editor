//! Animal companion and familiar slots stored on the creature itself.
//!
//! NWN2 keeps the chosen animal companion and familiar on the creature GFF
//! rather than as separate objects: `CompanionName`/`CompanionType` index
//! `hen_companion.2da`, `FamiliarName`/`FamiliarType` index `hen_familiar.2da`.
//! The name is a CExoString and the type an Int - the engine is type-strict,
//! so both are written with the exact schema type even when the field is
//! absent from the save.
//!
//! A `*Type` of -1 (or a missing field) means no choice was recorded, which is
//! why `SummonFamiliar()` can silently do nothing. The blueprint the engine
//! summons comes from the row's `BASERESREF_PREFIX` plus a level-based suffix,
//! and `*Name` overrides the blueprint's own name.

use super::Character;
use super::types::FeatId;
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::borrow::Cow;
use tracing::info;

/// `feat.2da` row 199 (AnimalCompanion) - granted to druids at 1 and rangers at 4.
pub const FEAT_ANIMAL_COMPANION: FeatId = FeatId(199);

/// `feat.2da` row 303 (Familiar) - granted to wizards and sorcerers at 1.
pub const FEAT_FAMILIAR: FeatId = FeatId(303);

/// `*Type` value meaning "nothing chosen"; the game has nothing to summon.
pub const PET_TYPE_NONE: i32 = -1;

const COMPANION_NAME: &str = "CompanionName";
const COMPANION_TYPE: &str = "CompanionType";
const FAMILIAR_NAME: &str = "FamiliarName";
const FAMILIAR_TYPE: &str = "FamiliarType";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PetTypeOption {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PetSlot {
    pub name: String,
    pub type_id: i32,
    pub type_name: Option<String>,
    /// Whether the character has the feat the engine needs to summon this slot.
    pub has_feat: bool,
    pub options: Vec<PetTypeOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PetsState {
    pub animal_companion: PetSlot,
    pub familiar: PetSlot,
}

/// Selectable rows of a `hen_*.2da` table, in the game's own order. Padding
/// rows (no blueprint prefix) are skipped; the label prefers the TLK string
/// and falls back to the raw 2DA label.
fn pet_options(game_data: &GameData, table_name: &str) -> Vec<PetTypeOption> {
    let Some(table) = game_data.get_table(table_name) else {
        return Vec::new();
    };

    (0..table.row_count() as i32)
        .filter_map(|id| {
            let row = table.get_by_id(id)?;
            let label = row.get("name")?.clone()?;
            row.get("baseresref_prefix")?.as_ref()?;

            let name = row
                .get("strref")
                .and_then(|c| c.as_ref()?.parse::<i32>().ok())
                .and_then(|strref| game_data.get_string(strref))
                .filter(|s| !s.is_empty())
                .unwrap_or(label);

            Some(PetTypeOption { id, name })
        })
        .collect()
}

/// `baseresref_prefix` of a `hen_companion.2da` row; the summoned creature's
/// blueprint is this prefix plus a level tier suffix (`c_ancom_boar1`).
pub fn companion_blueprint_prefix(type_id: i32, game_data: &GameData) -> Option<String> {
    let row = game_data.get_table("hen_companion")?.get_by_id(type_id)?;
    row.get("baseresref_prefix")?
        .clone()
        .filter(|s| s != "****" && !s.is_empty())
}

/// Script local variable in which the Clangeddin Utilities workshop mod's
/// replacement `nw_s2_animalcom` caches the companion blueprint. Its summon
/// script reads this instead of `CompanionType` once it is set, so a type
/// change in the editor would otherwise keep summoning the old creature.
const MOD_COMPANION_VAR: &str = "cmi_animcomp";

/// Points the mod's cached companion blueprint at `prefix`, keeping its level
/// tier suffix. No-op when the variable is absent or already matches.
pub fn sync_mod_companion_variable(
    player_entry: &mut IndexMap<String, GffValue<'static>>,
    prefix: &str,
) {
    let Some(GffValue::ListOwned(vars)) = player_entry.get_mut("VarTable") else {
        return;
    };
    for var in vars.iter_mut() {
        let is_target =
            matches!(var.get("Name"), Some(GffValue::String(n)) if n == MOD_COMPANION_VAR);
        if !is_target {
            continue;
        }
        let Some(GffValue::String(value)) = var.get("Value") else {
            continue;
        };
        let suffix_start = value
            .rfind(|c: char| !c.is_ascii_digit())
            .map_or(value.len(), |i| i + 1);
        let (old_prefix, suffix) = value.split_at(suffix_start);
        if old_prefix.eq_ignore_ascii_case(prefix) || suffix.is_empty() {
            return;
        }
        let new_value = format!("{prefix}{suffix}");
        info!(
            "Retargeting {MOD_COMPANION_VAR} from '{value}' to '{new_value}' to follow CompanionType"
        );
        var.insert("Value".to_string(), GffValue::String(Cow::Owned(new_value)));
        return;
    }
}

impl Character {
    pub fn animal_companion_name(&self) -> String {
        self.get_string_owned(COMPANION_NAME).unwrap_or_default()
    }

    pub fn animal_companion_type(&self) -> i32 {
        self.get_i32(COMPANION_TYPE).unwrap_or(PET_TYPE_NONE)
    }

    pub fn set_animal_companion_name(&mut self, name: String) {
        self.set_string(COMPANION_NAME, name);
    }

    pub fn set_animal_companion_type(&mut self, type_id: i32) {
        self.set_int(COMPANION_TYPE, type_id);
    }

    pub fn familiar_name(&self) -> String {
        self.get_string_owned(FAMILIAR_NAME).unwrap_or_default()
    }

    pub fn familiar_type(&self) -> i32 {
        self.get_i32(FAMILIAR_TYPE).unwrap_or(PET_TYPE_NONE)
    }

    pub fn set_familiar_name(&mut self, name: String) {
        self.set_string(FAMILIAR_NAME, name);
    }

    pub fn set_familiar_type(&mut self, type_id: i32) {
        self.set_int(FAMILIAR_TYPE, type_id);
    }

    pub fn pets_state(&self, game_data: &GameData) -> PetsState {
        let companion_options = pet_options(game_data, "hen_companion");
        let familiar_options = pet_options(game_data, "hen_familiar");

        let companion_type = self.animal_companion_type();
        let familiar_type = self.familiar_type();

        PetsState {
            animal_companion: PetSlot {
                name: self.animal_companion_name(),
                type_id: companion_type,
                type_name: option_name(&companion_options, companion_type),
                has_feat: self.has_feat(FEAT_ANIMAL_COMPANION),
                options: companion_options,
            },
            familiar: PetSlot {
                name: self.familiar_name(),
                type_id: familiar_type,
                type_name: option_name(&familiar_options, familiar_type),
                has_feat: self.has_feat(FEAT_FAMILIAR),
                options: familiar_options,
            },
        }
    }
}

fn option_name(options: &[PetTypeOption], type_id: i32) -> Option<String> {
    options
        .iter()
        .find(|o| o.id == type_id)
        .map(|o| o.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(name: &str, value: &str) -> IndexMap<String, GffValue<'static>> {
        let mut v = IndexMap::new();
        v.insert(
            "Name".to_string(),
            GffValue::String(Cow::Owned(name.to_string())),
        );
        v.insert("Type".to_string(), GffValue::Dword(3));
        v.insert(
            "Value".to_string(),
            GffValue::String(Cow::Owned(value.to_string())),
        );
        v
    }

    fn var_value(entry: &IndexMap<String, GffValue<'static>>, name: &str) -> String {
        let GffValue::ListOwned(vars) = &entry["VarTable"] else {
            panic!("VarTable")
        };
        vars.iter()
            .find(|v| matches!(v.get("Name"), Some(GffValue::String(n)) if n == name))
            .and_then(|v| match v.get("Value") {
                Some(GffValue::String(s)) => Some(s.to_string()),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn mod_companion_variable_follows_the_new_prefix_and_keeps_the_tier() {
        let mut entry = IndexMap::new();
        entry.insert(
            "VarTable".to_string(),
            GffValue::ListOwned(vec![
                var("other", "x"),
                var("cmi_animcomp", "c_ancom_boar12"),
            ]),
        );
        sync_mod_companion_variable(&mut entry, "c_ancom_panther");
        assert_eq!(var_value(&entry, "cmi_animcomp"), "c_ancom_panther12");
        assert_eq!(var_value(&entry, "other"), "x");
    }

    #[test]
    fn mod_companion_variable_is_left_alone_when_matching_or_absent() {
        let mut entry = IndexMap::new();
        entry.insert(
            "VarTable".to_string(),
            GffValue::ListOwned(vec![var("cmi_animcomp", "c_ancom_boar1")]),
        );
        sync_mod_companion_variable(&mut entry, "c_ancom_boar");
        assert_eq!(var_value(&entry, "cmi_animcomp"), "c_ancom_boar1");

        let mut no_vars: IndexMap<String, GffValue<'static>> = IndexMap::new();
        sync_mod_companion_variable(&mut no_vars, "c_ancom_boar");
        assert!(no_vars.is_empty());
    }

    fn character_with_pets() -> Character {
        let mut fields = IndexMap::new();
        fields.insert(
            COMPANION_NAME.to_string(),
            GffValue::String(Cow::Owned("Karnwyr".to_string())),
        );
        fields.insert(COMPANION_TYPE.to_string(), GffValue::Int(1));
        Character::from_gff(fields)
    }

    #[test]
    fn reads_existing_animal_companion() {
        let character = character_with_pets();
        assert_eq!(character.animal_companion_name(), "Karnwyr");
        assert_eq!(character.animal_companion_type(), 1);
    }

    #[test]
    fn missing_fields_read_as_unset() {
        let character = Character::from_gff(IndexMap::new());
        assert_eq!(character.animal_companion_name(), "");
        assert_eq!(character.animal_companion_type(), PET_TYPE_NONE);
        assert_eq!(character.familiar_name(), "");
        assert_eq!(character.familiar_type(), PET_TYPE_NONE);
    }

    #[test]
    fn writes_schema_types_when_fields_are_absent() {
        let mut character = Character::from_gff(IndexMap::new());
        character.set_familiar_name("Whiskers".to_string());
        character.set_familiar_type(1);

        match character.gff().get(FAMILIAR_NAME) {
            Some(GffValue::String(s)) => assert_eq!(s.as_ref(), "Whiskers"),
            other => panic!("FamiliarName must be String, got {other:?}"),
        }
        match character.gff().get(FAMILIAR_TYPE) {
            Some(GffValue::Int(v)) => assert_eq!(*v, 1),
            other => panic!("FamiliarType must be Int, got {other:?}"),
        }
        assert!(character.is_modified());
    }

    #[test]
    fn retypes_mistyped_type_field_to_int() {
        let mut fields = IndexMap::new();
        fields.insert(COMPANION_TYPE.to_string(), GffValue::Byte(2));
        let mut character = Character::from_gff(fields);

        character.set_animal_companion_type(4);

        match character.gff().get(COMPANION_TYPE) {
            Some(GffValue::Int(v)) => assert_eq!(*v, 4),
            other => panic!("CompanionType must be Int, got {other:?}"),
        }
    }

    #[test]
    fn clearing_the_type_writes_none() {
        let mut character = character_with_pets();
        character.set_animal_companion_type(PET_TYPE_NONE);
        assert_eq!(character.animal_companion_type(), PET_TYPE_NONE);
    }

    #[test]
    fn option_name_resolves_only_known_rows() {
        let options = vec![
            PetTypeOption {
                id: 0,
                name: "Badger".to_string(),
            },
            PetTypeOption {
                id: 1,
                name: "Wolf".to_string(),
            },
        ];
        assert_eq!(option_name(&options, 1).as_deref(), Some("Wolf"));
        assert_eq!(option_name(&options, PET_TYPE_NONE), None);
        assert_eq!(option_name(&options, 7), None);
    }
}
