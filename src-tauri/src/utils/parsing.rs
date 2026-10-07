use crate::loaders::GameData;
use ahash::AHashMap;
use regex::Regex;
use std::sync::LazyLock;

pub type Row = AHashMap<String, Option<String>>;

pub fn row_int(row: &Row, key: &str, default: i32) -> i32 {
    safe_int(row.get(key).and_then(|v| v.as_deref()), default)
}

pub fn row_bool(row: &Row, key: &str, default: bool) -> bool {
    safe_bool(row.get(key).and_then(|v| v.as_deref()), default)
}

pub fn row_str(row: &Row, key: &str) -> Option<String> {
    row.get(key)?
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn row_string(game_data: &GameData, row: &Row, strref_column: &str) -> Option<String> {
    let strref = row_int(row, strref_column, -1);
    if strref >= 0 {
        game_data.get_string(strref)
    } else {
        None
    }
}

/// The localized name behind `strref_column`, falling back to the row's label.
pub fn row_name(game_data: &GameData, row: &Row, strref_column: &str) -> Option<String> {
    row_string(game_data, row, strref_column).or_else(|| row_str(row, "label"))
}

/// Parse an optional string to i32, returning `default` on None/empty/unparseable.
/// Handles hex values prefixed with `0x`.
pub fn safe_int(value: Option<&str>, default: i32) -> i32 {
    let Some(s) = value else { return default };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return default;
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        i32::from_str_radix(hex, 16).unwrap_or(default)
    } else {
        trimmed.parse().unwrap_or(default)
    }
}

/// Parse an optional string to bool.
pub fn safe_bool(value: Option<&str>, default: bool) -> bool {
    let Some(s) = value else { return default };
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return default;
    }
    match trimmed {
        "1" | "true" | "True" | "TRUE" | "yes" | "Yes" => true,
        "0" | "false" | "False" | "FALSE" | "no" | "No" => false,
        _ => trimmed.parse::<i32>().map(|v| v != 0).unwrap_or(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_helpers() {
        let mut row = Row::new();
        row.insert("hitdie".into(), Some("10".into()));
        row.insert("spellcaster".into(), Some("1".into()));
        row.insert("label".into(), Some("Fighter".into()));
        row.insert("empty".into(), Some(String::new()));
        row.insert("null".into(), None);

        assert_eq!(row_int(&row, "hitdie", 0), 10);
        assert_eq!(row_int(&row, "missing", 6), 6);
        assert_eq!(row_int(&row, "empty", 8), 8);
        assert_eq!(row_int(&row, "null", 4), 4);

        assert!(row_bool(&row, "spellcaster", false));
        assert!(!row_bool(&row, "missing", false));

        assert_eq!(row_str(&row, "label"), Some("Fighter".into()));
        assert_eq!(row_str(&row, "empty"), None);
        assert_eq!(row_str(&row, "null"), None);
        assert_eq!(row_str(&row, "missing"), None);
    }

    #[test]
    fn test_safe_int() {
        assert_eq!(safe_int(Some("42"), 0), 42);
        assert_eq!(safe_int(Some(""), 0), 0);
        assert_eq!(safe_int(None, 0), 0);
        assert_eq!(safe_int(Some("0x10"), 0), 16);
        assert_eq!(safe_int(Some("0X1A"), 0), 26);
        assert_eq!(safe_int(Some("invalid"), 5), 5);
    }

    #[test]
    fn test_safe_bool() {
        assert!(safe_bool(Some("1"), false));
        assert!(safe_bool(Some("true"), false));
        assert!(!safe_bool(Some("0"), true));
        assert!(!safe_bool(Some("false"), true));
        assert!(!safe_bool(None, false));
        assert!(safe_bool(None, true));
    }
}

/// Removes the markup tags (`<b>`, `<color=Gold>`, ...) the game text carries.
pub fn strip_markup(text: &str) -> String {
    static TAG: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid markup regex"));
    TAG.replace_all(text, "").into_owned()
}
