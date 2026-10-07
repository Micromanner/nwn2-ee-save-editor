//! Helpers for editing GFF field maps without mutating the schema's field type.
//!
//! NWN2's game engine treats GFF schema types as load-bearing: e.g. `LvlCap`
//! stored as `Byte` will be silently reset if rewritten as `Dword`. These
//! helpers read the existing type for `key` and emit the same variant when
//! writing, falling back to a sensible default for new fields.

use super::GffValue;
use indexmap::IndexMap;

/// Insert a signed-integer value, preserving the existing GFF type variant.
/// Falls back to `Int` for new fields.
pub fn insert_i32_preserving_type(
    fields: &mut IndexMap<String, GffValue<'static>>,
    key: &str,
    value: i32,
) {
    let new_value = match fields.get(key) {
        Some(GffValue::Byte(_)) => GffValue::Byte(value as u8),
        Some(GffValue::Short(_)) => GffValue::Short(value as i16),
        Some(GffValue::Word(_)) => GffValue::Word(value as u16),
        Some(GffValue::Dword(_)) => GffValue::Dword(value as u32),
        Some(GffValue::Int64(_)) => GffValue::Int64(i64::from(value)),
        Some(GffValue::Dword64(_)) => GffValue::Dword64(value as u64),
        _ => GffValue::Int(value),
    };
    fields.insert(key.to_string(), new_value);
}

/// Insert an unsigned-integer value, preserving the existing GFF type variant.
/// Falls back to `Dword` for new fields.
pub fn insert_u32_preserving_type(
    fields: &mut IndexMap<String, GffValue<'static>>,
    key: &str,
    value: u32,
) {
    let new_value = match fields.get(key) {
        Some(GffValue::Byte(_)) => GffValue::Byte(value as u8),
        Some(GffValue::Short(_)) => GffValue::Short(value as i16),
        Some(GffValue::Word(_)) => GffValue::Word(value as u16),
        Some(GffValue::Int(_)) => GffValue::Int(value as i32),
        Some(GffValue::Int64(_)) => GffValue::Int64(i64::from(value)),
        Some(GffValue::Dword64(_)) => GffValue::Dword64(u64::from(value)),
        _ => GffValue::Dword(value),
    };
    fields.insert(key.to_string(), new_value);
}

/// Insert a boolean value, preserving the existing GFF type variant.
/// Falls back to `Byte` for new fields.
pub fn insert_bool_preserving_type(
    fields: &mut IndexMap<String, GffValue<'static>>,
    key: &str,
    value: bool,
) {
    let v = u8::from(value);
    let new_value = match fields.get(key) {
        Some(GffValue::Int(_)) => GffValue::Int(i32::from(v)),
        Some(GffValue::Short(_)) => GffValue::Short(i16::from(v)),
        Some(GffValue::Word(_)) => GffValue::Word(u16::from(v)),
        Some(GffValue::Dword(_)) => GffValue::Dword(u32::from(v)),
        _ => GffValue::Byte(v),
    };
    fields.insert(key.to_string(), new_value);
}

/// Any integer variant as `i64`; `None` for other types and `Dword64` values above `i64::MAX`.
pub fn gff_integer(value: &GffValue<'_>) -> Option<i64> {
    Some(match value {
        GffValue::Byte(v) => i64::from(*v),
        GffValue::Word(v) => i64::from(*v),
        GffValue::Short(v) => i64::from(*v),
        GffValue::Dword(v) => i64::from(*v),
        GffValue::Int(v) => i64::from(*v),
        GffValue::Dword64(v) => i64::try_from(*v).ok()?,
        GffValue::Int64(v) => *v,
        _ => return None,
    })
}

/// A String or ResRef field, trimmed.
pub fn gff_text(fields: &IndexMap<String, GffValue<'_>>, name: &str) -> Option<String> {
    match fields.get(name)? {
        GffValue::String(s) => Some(s.trim().to_string()),
        GffValue::ResRef(s) => Some(s.trim().to_string()),
        _ => None,
    }
}

/// Name of the `GffValue` variant for diagnostics and assertions.
pub fn variant_name(v: &GffValue<'_>) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fields() -> IndexMap<String, GffValue<'static>> {
        let mut f = IndexMap::new();
        f.insert("AsByte".to_string(), GffValue::Byte(1));
        f.insert("AsInt".to_string(), GffValue::Int(2));
        f.insert("AsDword".to_string(), GffValue::Dword(3));
        f.insert("AsShort".to_string(), GffValue::Short(4));
        f.insert("AsWord".to_string(), GffValue::Word(5));
        f.insert("AsString".to_string(), GffValue::String("hi".into()));
        f
    }

    #[test]
    fn insert_i32_preserves_each_variant() {
        let mut f = make_fields();
        insert_i32_preserving_type(&mut f, "AsByte", 50);
        insert_i32_preserving_type(&mut f, "AsInt", -42);
        insert_i32_preserving_type(&mut f, "AsDword", 100_000);
        insert_i32_preserving_type(&mut f, "AsShort", 1000);
        insert_i32_preserving_type(&mut f, "AsWord", 1000);
        insert_i32_preserving_type(&mut f, "NewField", 7);

        assert!(matches!(f.get("AsByte"), Some(GffValue::Byte(50))));
        assert!(matches!(f.get("AsInt"), Some(GffValue::Int(-42))));
        assert!(matches!(f.get("AsDword"), Some(GffValue::Dword(100_000))));
        assert!(matches!(f.get("AsShort"), Some(GffValue::Short(1000))));
        assert!(matches!(f.get("AsWord"), Some(GffValue::Word(1000))));
        assert!(matches!(f.get("NewField"), Some(GffValue::Int(7))));
    }

    #[test]
    fn insert_u32_preserves_each_variant() {
        let mut f = make_fields();
        insert_u32_preserving_type(&mut f, "AsByte", 50);
        insert_u32_preserving_type(&mut f, "AsInt", 99);
        insert_u32_preserving_type(&mut f, "AsDword", 100_000);
        insert_u32_preserving_type(&mut f, "NewField", 7);

        assert!(matches!(f.get("AsByte"), Some(GffValue::Byte(50))));
        assert!(matches!(f.get("AsInt"), Some(GffValue::Int(99))));
        assert!(matches!(f.get("AsDword"), Some(GffValue::Dword(100_000))));
        assert!(matches!(f.get("NewField"), Some(GffValue::Dword(7))));
    }

    #[test]
    fn insert_bool_preserves_each_variant() {
        let mut f = make_fields();
        insert_bool_preserving_type(&mut f, "AsByte", true);
        insert_bool_preserving_type(&mut f, "AsInt", true);
        insert_bool_preserving_type(&mut f, "AsDword", false);
        insert_bool_preserving_type(&mut f, "NewBool", true);

        assert!(matches!(f.get("AsByte"), Some(GffValue::Byte(1))));
        assert!(matches!(f.get("AsInt"), Some(GffValue::Int(1))));
        assert!(matches!(f.get("AsDword"), Some(GffValue::Dword(0))));
        assert!(matches!(f.get("NewBool"), Some(GffValue::Byte(1))));
    }

    #[test]
    fn insert_overwrites_unrelated_type_with_default() {
        let mut f = make_fields();
        insert_i32_preserving_type(&mut f, "AsString", 9);
        assert!(matches!(f.get("AsString"), Some(GffValue::Int(9))));
    }
}
