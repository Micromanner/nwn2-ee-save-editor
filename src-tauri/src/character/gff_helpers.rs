use crate::parsers::gff::{GffValue, LocalizedString, LocalizedSubstring};
use indexmap::IndexMap;
use std::borrow::Cow;

impl super::Character {
    pub fn get_byte(&self, field: &str) -> Option<u8> {
        match self.gff.get(field)? {
            GffValue::Byte(v) => Some(*v),
            GffValue::Int(v) => Some(*v as u8),
            GffValue::Short(v) => Some(*v as u8),
            GffValue::Word(v) => Some(*v as u8),
            GffValue::Dword(v) => Some(*v as u8),
            _ => None,
        }
    }

    pub fn get_i32(&self, field: &str) -> Option<i32> {
        match self.gff.get(field)? {
            GffValue::Int(v) => Some(*v),
            GffValue::Short(v) => Some(i32::from(*v)),
            GffValue::Byte(v) => Some(i32::from(*v)),
            GffValue::Dword(v) => Some(*v as i32),
            GffValue::Word(v) => Some(i32::from(*v)),
            GffValue::Int64(v) => Some(*v as i32),
            GffValue::Dword64(v) => Some(*v as i32),
            _ => None,
        }
    }

    pub fn get_i64(&self, field: &str) -> Option<i64> {
        match self.gff.get(field)? {
            GffValue::Int64(v) => Some(*v),
            GffValue::Int(v) => Some(i64::from(*v)),
            GffValue::Short(v) => Some(i64::from(*v)),
            GffValue::Byte(v) => Some(i64::from(*v)),
            GffValue::Dword(v) => Some(i64::from(*v)),
            GffValue::Word(v) => Some(i64::from(*v)),
            GffValue::Dword64(v) => Some(*v as i64),
            _ => None,
        }
    }

    pub fn get_u8(&self, field: &str) -> Option<u8> {
        match self.gff.get(field)? {
            GffValue::Byte(v) => Some(*v),
            GffValue::Int(v) => Some(*v as u8),
            GffValue::Short(v) => Some(*v as u8),
            GffValue::Word(v) => Some(*v as u8),
            GffValue::Dword(v) => Some(*v as u8),
            _ => None,
        }
    }

    pub fn get_u16(&self, field: &str) -> Option<u16> {
        match self.gff.get(field)? {
            GffValue::Word(v) => Some(*v),
            GffValue::Byte(v) => Some(u16::from(*v)),
            GffValue::Short(v) => Some(*v as u16),
            GffValue::Int(v) => Some(*v as u16),
            GffValue::Dword(v) => Some(*v as u16),
            _ => None,
        }
    }

    pub fn get_u32(&self, field: &str) -> Option<u32> {
        match self.gff.get(field)? {
            GffValue::Dword(v) => Some(*v),
            GffValue::Byte(v) => Some(u32::from(*v)),
            GffValue::Word(v) => Some(u32::from(*v)),
            GffValue::Int(v) => Some(*v as u32),
            GffValue::Short(v) => Some(*v as u32),
            _ => None,
        }
    }

    pub fn get_f32(&self, field: &str) -> Option<f32> {
        match self.gff.get(field)? {
            GffValue::Float(v) => Some(*v),
            GffValue::Double(v) => Some(*v as f32),
            _ => None,
        }
    }

    pub fn get_f64(&self, field: &str) -> Option<f64> {
        match self.gff.get(field)? {
            GffValue::Double(v) => Some(*v),
            GffValue::Float(v) => Some(f64::from(*v)),
            _ => None,
        }
    }

    pub fn get_string(&self, field: &str) -> Option<&str> {
        match self.gff.get(field)? {
            GffValue::String(s) => Some(s),
            GffValue::ResRef(s) => Some(s),
            _ => None,
        }
    }

    pub fn get_string_owned(&self, field: &str) -> Option<String> {
        self.get_string(field).map(std::string::ToString::to_string)
    }

    pub fn get_resref(&self, field: &str) -> Option<&str> {
        match self.gff.get(field)? {
            GffValue::ResRef(s) => Some(s),
            GffValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn get_localized_string(&self, field: &str) -> Option<&LocalizedString<'static>> {
        match self.gff.get(field)? {
            GffValue::LocString(ls) => Some(ls),
            _ => None,
        }
    }

    pub fn get_localized_string_value(&self, field: &str) -> Option<String> {
        let ls = self.get_localized_string(field)?;
        extract_localized_string(ls)
    }

    pub fn get_list(&self, field: &str) -> Option<&Vec<IndexMap<String, GffValue<'static>>>> {
        match self.gff.get(field)? {
            GffValue::ListOwned(maps) => Some(maps),
            GffValue::List(_lazy_structs) => None,
            _ => None,
        }
    }

    pub fn get_list_owned(&self, field: &str) -> Option<Vec<IndexMap<String, GffValue<'static>>>> {
        match self.gff.get(field)? {
            GffValue::ListOwned(maps) => Some(maps.clone()),
            GffValue::List(lazy_structs) => Some(
                lazy_structs
                    .iter()
                    .map(super::super::parsers::gff::types::LazyStruct::force_load)
                    .collect(),
            ),
            _ => None,
        }
    }

    pub fn get_list_mut(
        &mut self,
        field: &str,
    ) -> Option<&mut Vec<IndexMap<String, GffValue<'static>>>> {
        match self.gff.get_mut(field)? {
            GffValue::ListOwned(maps) => {
                self.modified = true;
                Some(maps)
            }
            _ => None,
        }
    }

    pub fn get_struct(&self, field: &str) -> Option<&IndexMap<String, GffValue<'static>>> {
        match self.gff.get(field)? {
            GffValue::StructOwned(map) => Some(map),
            GffValue::Struct(_lazy_struct) => None,
            _ => None,
        }
    }

    pub fn get_struct_owned(&self, field: &str) -> Option<IndexMap<String, GffValue<'static>>> {
        match self.gff.get(field)? {
            GffValue::StructOwned(map) => Some((**map).clone()),
            GffValue::Struct(lazy_struct) => Some(lazy_struct.force_load()),
            _ => None,
        }
    }

    pub fn set_byte(&mut self, field: &str, value: u8) {
        self.gff.insert(field.to_string(), GffValue::Byte(value));
        self.modified = true;
    }

    /// Set an integer field forcing the GFF Int type, converting a previously mistyped
    /// value. For the few creature fields the game schema stores as Int (e.g.
    /// NeverDrawHelmet) where the type-strict engine ignores other variants.
    pub fn set_int(&mut self, field: &str, value: i32) {
        self.gff.insert(field.to_string(), GffValue::Int(value));
        self.modified = true;
    }

    /// Set an integer field, preserving the existing GFF type (Byte/Short/Word/Dword/Int64/Dword64).
    /// The game engine can be strict about schema types (e.g. alignment resets to 0 if stored as
    /// Int instead of Byte). New fields default to Int.
    pub fn set_i32(&mut self, field: &str, value: i32) {
        let new_value = match self.gff.get(field) {
            Some(GffValue::Byte(_)) => GffValue::Byte(value as u8),
            Some(GffValue::Short(_)) => GffValue::Short(value as i16),
            Some(GffValue::Word(_)) => GffValue::Word(value as u16),
            Some(GffValue::Dword(_)) => GffValue::Dword(value as u32),
            Some(GffValue::Int64(_)) => GffValue::Int64(i64::from(value)),
            Some(GffValue::Dword64(_)) => GffValue::Dword64(value as u64),
            _ => GffValue::Int(value),
        };
        self.gff.insert(field.to_string(), new_value);
        self.modified = true;
    }

    pub fn set_i64(&mut self, field: &str, value: i64) {
        self.gff.insert(field.to_string(), GffValue::Int64(value));
        self.modified = true;
    }

    pub fn set_u8(&mut self, field: &str, value: u8) {
        self.gff.insert(field.to_string(), GffValue::Byte(value));
        self.modified = true;
    }

    pub fn set_u16(&mut self, field: &str, value: u16) {
        self.gff.insert(field.to_string(), GffValue::Word(value));
        self.modified = true;
    }

    pub fn set_u32(&mut self, field: &str, value: u32) {
        self.gff.insert(field.to_string(), GffValue::Dword(value));
        self.modified = true;
    }

    pub fn set_i16(&mut self, field: &str, value: i16) {
        self.gff.insert(field.to_string(), GffValue::Short(value));
        self.modified = true;
    }

    pub fn set_f32(&mut self, field: &str, value: f32) {
        self.gff.insert(field.to_string(), GffValue::Float(value));
        self.modified = true;
    }

    pub fn set_f64(&mut self, field: &str, value: f64) {
        self.gff.insert(field.to_string(), GffValue::Double(value));
        self.modified = true;
    }

    pub fn set_string(&mut self, field: &str, value: String) {
        self.gff
            .insert(field.to_string(), GffValue::String(Cow::Owned(value)));
        self.modified = true;
    }

    pub fn set_localized_string(&mut self, field: &str, value: String) {
        let (language, gender) = self
            .gff
            .get(field)
            .and_then(|v| match v {
                GffValue::LocString(ls) => ls.substrings.first().map(|s| (s.language, s.gender)),
                _ => None,
            })
            .unwrap_or((0, 0));

        let ls = LocalizedString {
            string_ref: -1,
            substrings: vec![LocalizedSubstring {
                string: Cow::Owned(value),
                language,
                gender,
            }],
        };
        self.gff.insert(field.to_string(), GffValue::LocString(ls));
        self.modified = true;
    }

    pub fn set_resref(&mut self, field: &str, value: String) {
        self.gff
            .insert(field.to_string(), GffValue::ResRef(Cow::Owned(value)));
        self.modified = true;
    }

    pub fn set_list(&mut self, field: &str, value: Vec<IndexMap<String, GffValue<'static>>>) {
        self.gff
            .insert(field.to_string(), GffValue::ListOwned(value));
        self.modified = true;
    }

    pub fn set_struct(&mut self, field: &str, value: IndexMap<String, GffValue<'static>>) {
        self.gff
            .insert(field.to_string(), GffValue::StructOwned(Box::new(value)));
        self.modified = true;
    }

    pub fn has_field(&self, field: &str) -> bool {
        self.gff.contains_key(field)
    }

    pub fn remove_field(&mut self, field: &str) -> Option<GffValue<'static>> {
        let removed = self.gff.shift_remove(field);
        if removed.is_some() {
            self.modified = true;
        }
        removed
    }

    pub fn field_names(&self) -> Vec<&str> {
        self.gff.keys().map(std::string::String::as_str).collect()
    }

    pub fn field_names_owned(&self) -> Vec<String> {
        self.gff.keys().cloned().collect()
    }
}

pub fn extract_localized_string(ls: &LocalizedString<'_>) -> Option<String> {
    if let Some(substring) = ls.substrings.first()
        && !substring.string.is_empty()
    {
        return Some(substring.string.to_string());
    }
    None
}

pub fn extract_list_from_map(
    map: &IndexMap<String, GffValue<'static>>,
    field: &str,
) -> Option<Vec<IndexMap<String, GffValue<'static>>>> {
    match map.get(field)? {
        GffValue::ListOwned(maps) => Some(maps.clone()),
        GffValue::List(lazy_structs) => Some(
            lazy_structs
                .iter()
                .map(crate::parsers::gff::types::LazyStruct::force_load)
                .collect(),
        ),
        _ => None,
    }
}

pub fn gff_value_to_i32(value: &GffValue<'_>) -> Option<i32> {
    match value {
        GffValue::Int(v) => Some(*v),
        GffValue::Short(v) => Some(i32::from(*v)),
        GffValue::Byte(v) => Some(i32::from(*v)),
        GffValue::Dword(v) => Some(*v as i32),
        GffValue::Word(v) => Some(i32::from(*v)),
        GffValue::Int64(v) => Some(*v as i32),
        GffValue::Dword64(v) => Some(*v as i32),
        _ => None,
    }
}

pub fn gff_value_to_u32(value: &GffValue<'_>) -> Option<u32> {
    gff_value_to_i32(value).and_then(|n| u32::try_from(n).ok())
}

pub fn gff_value_to_string(value: &GffValue<'_>) -> Option<String> {
    match value {
        GffValue::String(s) => Some(s.to_string()),
        GffValue::ResRef(s) => Some(s.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::Character;

    fn create_test_fields() -> IndexMap<String, GffValue<'static>> {
        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(16));
        fields.insert("Dex".to_string(), GffValue::Byte(14));
        fields.insert("Con".to_string(), GffValue::Byte(12));
        fields.insert("Int".to_string(), GffValue::Int(10));
        fields.insert(
            "Name".to_string(),
            GffValue::String(Cow::Owned("Test".to_string())),
        );
        fields
    }

    #[test]
    fn test_get_i32_from_byte() {
        let character = Character::from_gff(create_test_fields());
        let value = character.get_i32("Str");
        assert_eq!(value, Some(16));
    }

    #[test]
    fn test_get_i32_from_int() {
        let character = Character::from_gff(create_test_fields());
        let value = character.get_i32("Int");
        assert_eq!(value, Some(10));
    }

    #[test]
    fn test_get_string() {
        let character = Character::from_gff(create_test_fields());
        let value = character.get_string("Name");
        assert_eq!(value, Some("Test"));
    }

    #[test]
    fn test_set_i32() {
        let mut character = Character::from_gff(create_test_fields());
        assert!(!character.is_modified());

        character.set_i32("Str", 20);
        assert!(character.is_modified());

        let value = character.get_i32("Str");
        assert_eq!(value, Some(20));
    }

    #[test]
    fn test_set_i32_preserves_existing_type() {
        let mut fields = IndexMap::new();
        fields.insert("ByteField".to_string(), GffValue::Byte(5));
        fields.insert("ShortField".to_string(), GffValue::Short(5));
        fields.insert("WordField".to_string(), GffValue::Word(5));
        fields.insert("DwordField".to_string(), GffValue::Dword(5));
        fields.insert("IntField".to_string(), GffValue::Int(5));
        let mut character = Character::from_gff(fields);

        character.set_i32("ByteField", 50);
        character.set_i32("ShortField", 1000);
        character.set_i32("WordField", 1000);
        character.set_i32("DwordField", 100_000);
        character.set_i32("IntField", -42);
        character.set_i32("NewField", 7);

        assert!(matches!(
            character.gff.get("ByteField"),
            Some(GffValue::Byte(50))
        ));
        assert!(matches!(
            character.gff.get("ShortField"),
            Some(GffValue::Short(1000))
        ));
        assert!(matches!(
            character.gff.get("WordField"),
            Some(GffValue::Word(1000))
        ));
        assert!(matches!(
            character.gff.get("DwordField"),
            Some(GffValue::Dword(100_000))
        ));
        assert!(matches!(
            character.gff.get("IntField"),
            Some(GffValue::Int(-42))
        ));
        assert!(matches!(
            character.gff.get("NewField"),
            Some(GffValue::Int(7))
        ));
    }

    #[test]
    fn test_has_field() {
        let character = Character::from_gff(create_test_fields());
        assert!(character.has_field("Str"));
        assert!(!character.has_field("NonExistent"));
    }

    #[test]
    fn test_remove_field() {
        let mut character = Character::from_gff(create_test_fields());
        assert!(character.has_field("Str"));
        assert!(!character.is_modified());

        let removed = character.remove_field("Str");
        assert!(removed.is_some());
        assert!(!character.has_field("Str"));
        assert!(character.is_modified());
    }

    #[test]
    fn test_field_names() {
        let character = Character::from_gff(create_test_fields());
        let names = character.field_names();
        assert!(names.contains(&"Str"));
        assert!(names.contains(&"Dex"));
        assert!(names.contains(&"Name"));
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn test_gff_value_to_i32() {
        assert_eq!(gff_value_to_i32(&GffValue::Int(42)), Some(42));
        assert_eq!(gff_value_to_i32(&GffValue::Byte(10)), Some(10));
        assert_eq!(gff_value_to_i32(&GffValue::Short(-5)), Some(-5));
        assert_eq!(
            gff_value_to_i32(&GffValue::String(Cow::Borrowed("test"))),
            None
        );
    }

    #[test]
    fn test_set_string() {
        let mut character = Character::from_gff(create_test_fields());
        character.set_string("NewField", "NewValue".to_string());
        assert_eq!(character.get_string("NewField"), Some("NewValue"));
        assert!(character.is_modified());
    }

    #[test]
    #[ignore = "diagnostic only; run with --ignored --nocapture"]
    fn diagnostic_dump_list_field_types() {
        use crate::parsers::gff::{GffParser, GffValue};
        use std::collections::BTreeMap;
        use std::path::PathBuf;

        fn variant_name(v: &GffValue<'_>) -> &'static str {
            match v {
                GffValue::Byte(_) => "Byte",
                GffValue::Char(_) => "Char",
                GffValue::Word(_) => "Word",
                GffValue::Short(_) => "Short",
                GffValue::Dword(_) => "Dword",
                GffValue::Int(_) => "Int",
                GffValue::Dword64(_) => "Dword64",
                GffValue::Int64(_) => "Int64",
                GffValue::Float(_) => "Float",
                GffValue::Double(_) => "Double",
                GffValue::String(_) => "String",
                GffValue::ResRef(_) => "ResRef",
                GffValue::LocString(_) => "LocString",
                GffValue::Void(_) => "Void",
                GffValue::Struct(_) | GffValue::StructOwned(_) | GffValue::StructRef(_) => "Struct",
                GffValue::List(_) | GffValue::ListOwned(_) | GffValue::ListRef(_) => "List",
            }
        }

        let fixture_base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let fixtures = [
            "tests/fixtures/gff/ryathstrongarm/ryathstrongarm1.bic",
            "tests/fixtures/gff/sagemelchior/sagemelchior1.bic",
            "tests/fixtures/gff/qaraofblacklake/qaraofblacklake1.bic",
            "tests/fixtures/gff/okkugodofbears/okkugodofbears1.bic",
            "tests/fixtures/gff/oneofmany/oneofmany1.bic",
            "tests/fixtures/gff/theconstruct/theconstruct1.bic",
            "tests/fixtures/gff/occidiooctavon/occidiooctavon1.bic",
            "tests/fixtures/gff/occidiooctavon/occidiooctavon4.bic",
            "tests/fixtures/gff/sagemelchior/sagemelchior4.bic",
            "tests/fixtures/gff/player.bic",
        ];

        let lists_to_inspect = [
            "ClassList",
            "FeatList",
            "SkillList",
            "LvlStatList",
            "SpellKnownList0",
            "SpellKnownList1",
            "SpellKnownList2",
            "SpellKnownList3",
            "SpellKnownList4",
            "SpellKnownList5",
            "SpellKnownList6",
            "SpellKnownList7",
            "SpellKnownList8",
            "SpellMemorizedList0",
            "SpellMemorizedList1",
            "SpellMemorizedList2",
            "SpellMemorizedList3",
            "SpellMemorizedList4",
            "SpellMemorizedList5",
            "SpellMemorizedList6",
            "SpellMemorizedList7",
            "SpellMemorizedList8",
            "Equip_ItemList",
            "ItemList",
            "HotbarList",
            "DmgReduction",
        ];

        // list_name -> field_name -> type -> count
        let mut agg: BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>> = BTreeMap::new();

        fn walk_lists(
            struct_fields: &indexmap::IndexMap<String, GffValue<'_>>,
            agg: &mut BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>>,
        ) {
            for (field_name, value) in struct_fields {
                let entries: Vec<_> = match value {
                    GffValue::List(lazy) => lazy
                        .iter()
                        .map(crate::parsers::gff::types::LazyStruct::force_load)
                        .collect(),
                    GffValue::ListOwned(v) => v.clone(),
                    _ => continue,
                };
                for entry in &entries {
                    for (name, val) in entry {
                        *agg.entry(field_name.clone())
                            .or_default()
                            .entry(name.clone())
                            .or_default()
                            .entry(
                                match val {
                                    GffValue::Byte(_) => "Byte",
                                    GffValue::Char(_) => "Char",
                                    GffValue::Word(_) => "Word",
                                    GffValue::Short(_) => "Short",
                                    GffValue::Dword(_) => "Dword",
                                    GffValue::Int(_) => "Int",
                                    GffValue::Dword64(_) => "Dword64",
                                    GffValue::Int64(_) => "Int64",
                                    GffValue::Float(_) => "Float",
                                    GffValue::Double(_) => "Double",
                                    GffValue::String(_) => "String",
                                    GffValue::ResRef(_) => "ResRef",
                                    GffValue::LocString(_) => "LocString",
                                    GffValue::Void(_) => "Void",
                                    GffValue::Struct(_)
                                    | GffValue::StructOwned(_)
                                    | GffValue::StructRef(_) => "Struct",
                                    GffValue::List(_)
                                    | GffValue::ListOwned(_)
                                    | GffValue::ListRef(_) => "List",
                                }
                                .to_string(),
                            )
                            .or_insert(0) += 1;
                    }
                    walk_lists(entry, agg);
                }
            }
        }

        let _ = lists_to_inspect; // unused now — we walk recursively
        let _ = variant_name;

        for fixture_rel in &fixtures {
            let fixture = fixture_base.join(fixture_rel);
            if !fixture.exists() {
                continue;
            }
            let data = std::fs::read(&fixture).expect("read bic");
            let parser = GffParser::from_bytes(data).expect("parse bic");
            let root = parser.read_struct_fields(0).expect("root");
            walk_lists(&root, &mut agg);
        }

        for (list_name, fields) in &agg {
            println!("\n=== {list_name} ===");
            for (field, types) in fields {
                let mut tv: Vec<_> = types.iter().collect();
                tv.sort_by(|a, b| b.1.cmp(a.1));
                let types_str = tv
                    .iter()
                    .map(|(t, n)| format!("{t}:{n}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("  {field}: {types_str}");
            }
        }
    }

    #[test]
    fn test_set_localized_string_preserves_language() {
        let mut fields = IndexMap::new();
        let ls = LocalizedString {
            string_ref: -1,
            substrings: vec![LocalizedSubstring {
                string: Cow::Owned("Original".to_string()),
                language: 5,
                gender: 2,
            }],
        };
        fields.insert("Description".to_string(), GffValue::LocString(ls));

        let mut character = Character::from_gff(fields);
        character.set_localized_string("Description", "Updated".to_string());

        let updated = character.get_localized_string("Description").unwrap();
        assert_eq!(updated.substrings[0].language, 5);
        assert_eq!(updated.substrings[0].gender, 2);
        assert_eq!(updated.substrings[0].string.as_ref(), "Updated");
    }
}
