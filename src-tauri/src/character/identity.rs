use super::types::{ALIGNMENT_EVIL_THRESHOLD, ALIGNMENT_GOOD_THRESHOLD};
use super::{Character, CharacterError};
use serde::{Deserialize, Serialize};
use specta::Type;

pub const ALIGNMENT_MIN: i32 = 0;
pub const ALIGNMENT_MAX: i32 = 100;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Type)]
pub struct Alignment {
    pub law_chaos: i32,
    pub good_evil: i32,
}

impl Alignment {
    pub fn new(law_chaos: i32, good_evil: i32) -> Self {
        Self {
            law_chaos,
            good_evil,
        }
    }

    pub fn alignment_string(&self) -> String {
        let law_axis = match self.law_chaos {
            0..=30 => "Chaotic",
            31..=69 => "Neutral",
            _ => "Lawful",
        };

        let good_axis = match self.good_evil {
            0..=30 => "Evil",
            31..=69 => "Neutral",
            _ => "Good",
        };

        if law_axis == "Neutral" && good_axis == "Neutral" {
            "True Neutral".to_string()
        } else {
            format!("{law_axis} {good_axis}")
        }
    }

    pub fn is_lawful(&self) -> bool {
        self.law_chaos >= ALIGNMENT_GOOD_THRESHOLD
    }

    pub fn is_chaotic(&self) -> bool {
        self.law_chaos <= ALIGNMENT_EVIL_THRESHOLD
    }

    pub fn is_good(&self) -> bool {
        self.good_evil >= ALIGNMENT_GOOD_THRESHOLD
    }

    pub fn is_evil(&self) -> bool {
        self.good_evil <= ALIGNMENT_EVIL_THRESHOLD
    }

    pub fn is_neutral_law_chaos(&self) -> bool {
        self.law_chaos > ALIGNMENT_EVIL_THRESHOLD && self.law_chaos < ALIGNMENT_GOOD_THRESHOLD
    }

    pub fn is_neutral_good_evil(&self) -> bool {
        self.good_evil > ALIGNMENT_EVIL_THRESHOLD && self.good_evil < ALIGNMENT_GOOD_THRESHOLD
    }
}

use crate::loaders::GameData;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct Biography {
    pub name: String,
    pub first_name: String,
    pub last_name: String,
    pub age: i32,
    pub gender: i32,
    pub deity: String,
    pub description: String,
    pub background: Option<String>,
    pub experience: i32,
    pub alignment: Alignment,
}

impl Character {
    pub fn biography(&self, game_data: &GameData) -> Biography {
        Biography {
            name: self.full_name(),
            first_name: self.first_name(),
            last_name: self.last_name(),
            age: self.age(),
            gender: self.gender(),
            deity: self.deity(),
            description: self.description(),
            background: self.background(game_data),
            experience: self.experience(),
            alignment: self.alignment(),
        }
    }

    pub fn first_name(&self) -> String {
        self.get_localized_string_value("FirstName")
            .unwrap_or_default()
    }

    pub fn last_name(&self) -> String {
        self.get_localized_string_value("LastName")
            .unwrap_or_default()
    }

    pub fn full_name(&self) -> String {
        let first = self.first_name();
        let last = self.last_name();
        format!("{first} {last}").trim().to_string()
    }

    /// StrRef backing `FirstName`, present when the field has no literal
    /// substring (e.g. companion `.ros` files, which only carry a TLK
    /// reference). `None` when the field has a literal substring already or
    /// no valid strref.
    pub fn first_name_strref(&self) -> Option<i32> {
        self.localized_string_ref("FirstName")
    }

    /// StrRef backing `LastName`. See [`Character::first_name_strref`].
    pub fn last_name_strref(&self) -> Option<i32> {
        self.localized_string_ref("LastName")
    }

    fn localized_string_ref(&self, field: &str) -> Option<i32> {
        let ls = self.get_localized_string(field)?;
        (ls.string_ref >= 0).then_some(ls.string_ref)
    }

    pub fn set_first_name(&mut self, name: String) {
        self.set_localized_string("FirstName", name);
    }

    pub fn set_last_name(&mut self, name: String) {
        self.set_localized_string("LastName", name);
    }

    pub fn age(&self) -> i32 {
        self.get_i32("Age").unwrap_or(0)
    }

    pub fn set_age(&mut self, age: i32) -> Result<(), CharacterError> {
        if age < 0 {
            return Err(CharacterError::OutOfRange {
                field: "Age",
                value: age,
                min: 0,
                max: i32::MAX,
            });
        }
        self.set_i32("Age", age);
        Ok(())
    }

    pub fn experience(&self) -> i32 {
        self.get_i32("Experience").unwrap_or(0)
    }

    pub fn set_experience(&mut self, xp: i32) -> Result<(), CharacterError> {
        if xp < 0 {
            return Err(CharacterError::OutOfRange {
                field: "Experience",
                value: xp,
                min: 0,
                max: i32::MAX,
            });
        }
        self.set_i32("Experience", xp);
        Ok(())
    }

    pub fn alignment(&self) -> Alignment {
        let law_chaos = self.get_i32("LawfulChaotic").unwrap_or(50);
        let good_evil = self.get_i32("GoodEvil").unwrap_or(50);
        Alignment::new(law_chaos, good_evil)
    }

    pub fn set_alignment(
        &mut self,
        law_chaos: Option<i32>,
        good_evil: Option<i32>,
    ) -> Result<(), CharacterError> {
        let current = self.alignment();

        let new_law_chaos = law_chaos.unwrap_or(current.law_chaos);
        let new_good_evil = good_evil.unwrap_or(current.good_evil);

        let clamped_law_chaos = new_law_chaos.clamp(ALIGNMENT_MIN, ALIGNMENT_MAX);
        let clamped_good_evil = new_good_evil.clamp(ALIGNMENT_MIN, ALIGNMENT_MAX);

        // NWN2 stores LawfulChaotic/GoodEvil as Byte fields; writing as Int
        // causes the game to reset them to 0 on load (schema type mismatch).
        self.set_byte("LawfulChaotic", clamped_law_chaos as u8);
        self.set_byte("GoodEvil", clamped_good_evil as u8);

        Ok(())
    }

    /// Get the character's deity.
    pub fn deity(&self) -> String {
        // Deity can be stored as a simple String (CExoString) or LocString
        if let Some(s) = self.get_string("Deity") {
            return s.to_string();
        }
        self.get_localized_string_value("Deity").unwrap_or_default()
    }

    pub fn set_deity(&mut self, deity: String) {
        self.set_localized_string("Deity", deity);
    }

    pub fn description(&self) -> String {
        self.get_localized_string_value("Description")
            .unwrap_or_default()
    }

    pub fn set_description(&mut self, description: String) {
        self.set_localized_string("Description", description);
    }

    pub fn gender(&self) -> i32 {
        self.get_i32("Gender").unwrap_or(0)
    }

    pub fn set_gender(&mut self, gender: i32) -> Result<(), CharacterError> {
        if !(0..=1).contains(&gender) {
            return Err(CharacterError::OutOfRange {
                field: "Gender",
                value: gender,
                min: 0,
                max: 1,
            });
        }
        self.set_i32("Gender", gender);
        Ok(())
    }

    /// Get the character's selected background name.
    /// `CharBackground` (Dword) is authoritative — it indexes a row in `backgrounds.2da`.
    /// Returns the TLK-resolved `Name`, falling back to `Label`.
    pub fn background(&self, game_data: &GameData) -> Option<String> {
        let bg_id = self.get_character_background()?;
        let bg_table = game_data.get_table("backgrounds")?;
        let row = bg_table.get_by_id(bg_id.0)?;

        let name = row
            .get("name")
            .and_then(|s| s.as_ref()?.parse::<i32>().ok())
            .filter(|&s| s >= 0)
            .and_then(|strref| game_data.get_string(strref))
            .filter(|s| !s.is_empty());

        name.or_else(|| row.get("label").and_then(|s| s.clone()))
    }
}

/// Resolve a localized name field for display, falling back to the TLK
/// string when the GFF value has no literal substring (companion `.ros`
/// files store `FirstName`/`LastName` as a bare strref). Does not touch the
/// GFF data — the resolved value is never written back, so saving a
/// companion never bakes a TLK-derived name over the strref.
pub fn resolve_localized_name(value: String, strref: Option<i32>, game_data: &GameData) -> String {
    if !value.is_empty() {
        return value;
    }
    strref
        .and_then(|r| game_data.get_string(r))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::{GffValue, LocalizedString, LocalizedSubstring};
    use crate::parsers::tlk::TLKParser;
    use indexmap::IndexMap;
    use std::borrow::Cow;
    use std::sync::{Arc, RwLock};

    fn create_test_game_data() -> GameData {
        GameData::new(Arc::new(RwLock::new(TLKParser::default())))
    }

    fn create_test_character() -> Character {
        let mut fields = IndexMap::new();

        fields.insert(
            "FirstName".to_string(),
            GffValue::LocString(LocalizedString {
                string_ref: -1,
                substrings: vec![LocalizedSubstring {
                    string: Cow::Owned("TestFirst".to_string()),
                    language: 0,
                    gender: 0,
                }],
            }),
        );

        fields.insert(
            "LastName".to_string(),
            GffValue::LocString(LocalizedString {
                string_ref: -1,
                substrings: vec![LocalizedSubstring {
                    string: Cow::Owned("TestLast".to_string()),
                    language: 0,
                    gender: 0,
                }],
            }),
        );

        fields.insert("Age".to_string(), GffValue::Int(30));
        fields.insert("Experience".to_string(), GffValue::Int(5000));
        fields.insert("LawfulChaotic".to_string(), GffValue::Byte(75));
        fields.insert("GoodEvil".to_string(), GffValue::Byte(80));
        fields.insert("Gender".to_string(), GffValue::Byte(0));

        fields.insert(
            "Description".to_string(),
            GffValue::LocString(LocalizedString {
                string_ref: -1,
                substrings: vec![LocalizedSubstring {
                    string: Cow::Owned("A brave hero".to_string()),
                    language: 0,
                    gender: 0,
                }],
            }),
        );

        fields.insert(
            "Deity".to_string(),
            GffValue::LocString(LocalizedString {
                string_ref: -1,
                substrings: vec![LocalizedSubstring {
                    string: Cow::Owned(String::new()),
                    language: 0,
                    gender: 0,
                }],
            }),
        );

        Character::from_gff(fields)
    }

    #[test]
    fn test_first_name() {
        let character = create_test_character();
        assert_eq!(character.first_name(), "TestFirst");
    }

    #[test]
    fn test_last_name() {
        let character = create_test_character();
        assert_eq!(character.last_name(), "TestLast");
    }

    #[test]
    fn test_full_name() {
        let character = create_test_character();
        assert_eq!(character.full_name(), "TestFirst TestLast");
    }

    #[test]
    fn test_set_first_name() {
        let mut character = create_test_character();
        character.set_first_name("NewFirst".to_string());
        assert_eq!(character.first_name(), "NewFirst");
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_last_name() {
        let mut character = create_test_character();
        character.set_last_name("NewLast".to_string());
        assert_eq!(character.last_name(), "NewLast");
        assert!(character.is_modified());
    }

    #[test]
    fn test_age() {
        let character = create_test_character();
        assert_eq!(character.age(), 30);
    }

    #[test]
    fn test_set_age() {
        let mut character = create_test_character();
        let result = character.set_age(50);
        assert!(result.is_ok());
        assert_eq!(character.age(), 50);
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_age_negative_error() {
        let mut character = create_test_character();
        let result = character.set_age(-5);
        assert!(result.is_err());
    }

    #[test]
    fn test_experience() {
        let character = create_test_character();
        assert_eq!(character.experience(), 5000);
    }

    #[test]
    fn test_set_experience() {
        let mut character = create_test_character();
        let result = character.set_experience(10000);
        assert!(result.is_ok());
        assert_eq!(character.experience(), 10000);
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_experience_negative_error() {
        let mut character = create_test_character();
        let result = character.set_experience(-100);
        assert!(result.is_err());
    }

    #[test]
    fn test_alignment() {
        let character = create_test_character();
        let alignment = character.alignment();
        assert_eq!(alignment.law_chaos, 75);
        assert_eq!(alignment.good_evil, 80);
        assert_eq!(alignment.alignment_string(), "Lawful Good");
    }

    #[test]
    fn test_set_alignment() {
        let mut character = create_test_character();
        let result = character.set_alignment(Some(0), Some(0));
        assert!(result.is_ok());

        let alignment = character.alignment();
        assert_eq!(alignment.law_chaos, 0);
        assert_eq!(alignment.good_evil, 0);
        assert_eq!(alignment.alignment_string(), "Chaotic Evil");
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_alignment_clamping() {
        let mut character = create_test_character();
        let result = character.set_alignment(Some(150), Some(-50));
        assert!(result.is_ok());

        let alignment = character.alignment();
        assert_eq!(alignment.law_chaos, 100);
        assert_eq!(alignment.good_evil, 0);
    }

    #[test]
    fn test_set_alignment_preserves_byte_type() {
        let mut character = create_test_character();
        character.set_alignment(Some(42), Some(73)).unwrap();

        match character.gff.get("LawfulChaotic") {
            Some(GffValue::Byte(v)) => assert_eq!(*v, 42),
            other => panic!("LawfulChaotic must be Byte, got {other:?}"),
        }
        match character.gff.get("GoodEvil") {
            Some(GffValue::Byte(v)) => assert_eq!(*v, 73),
            other => panic!("GoodEvil must be Byte, got {other:?}"),
        }
    }

    #[test]
    fn test_set_alignment_partial() {
        let mut character = create_test_character();
        let result = character.set_alignment(Some(25), None);
        assert!(result.is_ok());

        let alignment = character.alignment();
        assert_eq!(alignment.law_chaos, 25);
        assert_eq!(alignment.good_evil, 80);
    }

    #[test]
    fn test_deity() {
        let character = create_test_character();
        assert_eq!(character.deity(), "");
    }

    #[test]
    fn test_set_deity() {
        let mut character = create_test_character();
        character.set_deity("Tyr".to_string());
        assert_eq!(character.deity(), "Tyr");
        assert!(character.is_modified());
    }

    #[test]
    fn test_description() {
        let character = create_test_character();
        assert_eq!(character.description(), "A brave hero");
    }

    #[test]
    fn test_set_description() {
        let mut character = create_test_character();
        let new_desc = "A legendary warrior from the North".to_string();
        character.set_description(new_desc.clone());
        assert_eq!(character.description(), new_desc);
        assert!(character.is_modified());
    }

    #[test]
    fn test_gender() {
        let character = create_test_character();
        assert_eq!(character.gender(), 0);
    }

    #[test]
    fn test_set_gender() {
        let mut character = create_test_character();
        let result = character.set_gender(1);
        assert!(result.is_ok());
        assert_eq!(character.gender(), 1);
        assert!(character.is_modified());
    }

    #[test]
    fn test_set_gender_invalid() {
        let mut character = create_test_character();
        let result = character.set_gender(2);
        assert!(result.is_err());
    }

    #[test]
    fn test_alignment_is_lawful() {
        let alignment = Alignment::new(80, 50);
        assert!(alignment.is_lawful());
        assert!(!alignment.is_chaotic());
    }

    #[test]
    fn test_alignment_is_chaotic() {
        let alignment = Alignment::new(20, 50);
        assert!(alignment.is_chaotic());
        assert!(!alignment.is_lawful());
    }

    #[test]
    fn test_alignment_is_good() {
        let alignment = Alignment::new(50, 80);
        assert!(alignment.is_good());
        assert!(!alignment.is_evil());
    }

    #[test]
    fn test_alignment_is_evil() {
        let alignment = Alignment::new(50, 20);
        assert!(alignment.is_evil());
        assert!(!alignment.is_good());
    }

    #[test]
    fn test_alignment_true_neutral() {
        let alignment = Alignment::new(50, 50);
        assert_eq!(alignment.alignment_string(), "True Neutral");
        assert!(alignment.is_neutral_law_chaos());
        assert!(alignment.is_neutral_good_evil());
    }

    #[test]
    fn test_alignment_boundaries() {
        assert_eq!(Alignment::new(30, 50).alignment_string(), "Chaotic Neutral");
        assert_eq!(Alignment::new(31, 50).alignment_string(), "True Neutral");
        assert_eq!(Alignment::new(69, 50).alignment_string(), "True Neutral");
        assert_eq!(Alignment::new(70, 50).alignment_string(), "Lawful Neutral");

        assert_eq!(Alignment::new(50, 30).alignment_string(), "Neutral Evil");
        assert_eq!(Alignment::new(50, 31).alignment_string(), "True Neutral");
        assert_eq!(Alignment::new(50, 69).alignment_string(), "True Neutral");
        assert_eq!(Alignment::new(50, 70).alignment_string(), "Neutral Good");
    }

    fn create_strref_only_character() -> Character {
        let mut fields = IndexMap::new();
        fields.insert(
            "FirstName".to_string(),
            GffValue::LocString(LocalizedString {
                string_ref: 126460,
                substrings: vec![],
            }),
        );
        fields.insert(
            "LastName".to_string(),
            GffValue::LocString(LocalizedString {
                string_ref: -1,
                substrings: vec![],
            }),
        );
        fields.insert("Age".to_string(), GffValue::Byte(0));
        Character::from_gff(fields)
    }

    #[test]
    fn first_name_strref_exposes_ref_when_no_substring() {
        let character = create_strref_only_character();
        assert_eq!(character.first_name(), "");
        assert_eq!(character.first_name_strref(), Some(126460));
    }

    #[test]
    fn last_name_strref_none_when_ref_is_negative() {
        let character = create_strref_only_character();
        assert_eq!(character.last_name_strref(), None);
    }

    #[test]
    fn strref_absent_when_literal_substring_present() {
        let character = create_test_character();
        assert_eq!(character.first_name_strref(), None);
    }

    #[test]
    fn resolve_localized_name_prefers_literal_value() {
        let game_data = create_test_game_data();
        assert_eq!(
            resolve_localized_name("Khelgar".to_string(), Some(999), &game_data),
            "Khelgar"
        );
    }

    #[test]
    fn resolve_localized_name_falls_back_to_tlk_strref() {
        use crate::parsers::tlk::TLKStringEntry;
        use crate::parsers::tlk::types::CachedString;

        let mut tlk = TLKParser::default();
        tlk.entries.push(TLKStringEntry {
            flags: 1,
            sound_resref: None,
            volume_variance: 0,
            pitch_variance: 0,
            data_offset: 0,
            string_size: 0,
        });
        let symbol = tlk.interner.get_or_intern("Khelgar Ironfist");
        tlk.string_cache.insert(
            0,
            CachedString {
                symbol,
                byte_length: 0,
            },
        );
        let game_data = GameData::new(Arc::new(RwLock::new(tlk)));

        assert_eq!(
            resolve_localized_name(String::new(), Some(0), &game_data),
            "Khelgar Ironfist"
        );
    }

    #[test]
    fn resolve_localized_name_empty_when_strref_unresolvable() {
        let game_data = create_test_game_data();
        assert_eq!(resolve_localized_name(String::new(), None, &game_data), "");
        assert_eq!(
            resolve_localized_name(String::new(), Some(9999), &game_data),
            ""
        );
    }

    #[test]
    fn test_biography() {
        let character = create_test_character();
        let game_data = create_test_game_data();
        let bio = character.biography(&game_data);

        assert_eq!(bio.name, "TestFirst TestLast");
        assert_eq!(bio.first_name, "TestFirst");
        assert_eq!(bio.last_name, "TestLast");
        assert_eq!(bio.age, 30);
        assert_eq!(bio.experience, 5000);
        assert_eq!(bio.gender, 0);
        assert_eq!(bio.description, "A brave hero");
        assert_eq!(bio.alignment.law_chaos, 75);
        assert_eq!(bio.alignment.good_evil, 80);
        assert_eq!(bio.alignment.alignment_string(), "Lawful Good");
    }
}
