use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

pub static LABEL_CLEANUPS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("TN", "True Neutral");
    m.insert("AC_Natural", "Natural Armor");
    m.insert("AC_Dodge", "Dodge");
    m.insert("AC_Deflection", "Deflection");
    m.insert("AC_Armor", "Armor");
    m.insert("AC_Shield", "Shield");
    m.insert("Bonus_1", "+1");
    m.insert("Bonus_2", "+2");
    m.insert("Bonus_3", "+3");
    m.insert("Bonus_4", "+4");
    m.insert("Bonus_5", "+5");
    m.insert("Bonus_6", "+6");
    m.insert("Bonus_7", "+7");
    m.insert("Bonus_8", "+8");
    m.insert("Bonus_9", "+9");
    m.insert("Bonus_10", "+10");
    m.insert("Bonus_11", "+11");
    m.insert("Bonus_12", "+12");
    m.insert("Bonus_14", "DC 14");
    m.insert("Bonus_15", "DC 15");
    m.insert("Bonus_16", "DC 16");
    m.insert("Bonus_17", "DC 17");
    m.insert("Bonus_18", "DC 18");
    m.insert("Bonus_19", "DC 19");
    m.insert("Bonus_20", "DC 20");
    m.insert("1d4", "1d4");
    m.insert("1d6", "1d6");
    m.insert("1d8", "1d8");
    m.insert("1d10", "1d10");
    m.insert("2d6", "2d6");
    m.insert("2d8", "2d8");
    m.insert("2d10", "2d10");
    m
});

pub static INVALID_LABEL_PATTERNS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "****",
        "**",
        "DEL_",
        "DELETED",
        "padding",
        "None",
        "REMOVED",
        "TEST",
        "INVALID",
        "RESERVED",
        "unused",
        "placeholder",
    ]
});

pub static HIDDEN_PROPERTY_IDS: LazyLock<HashSet<u32>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert(30); // DoubleStack - inventory system flag
    s.insert(31); // EnhancedContainer BonusSlot - flat, no options
    s.insert(33); // DamageMelee - has type but no damage amount
    s.insert(34); // DamageRanged - has type but no damage amount
    s.insert(47); // DamageNone - remove damage properties instead
    s.insert(54); // SpellSchool Immunity - flat, no school selection
    s.insert(72); // On Monster Hit
    s.insert(76); // Poison - misconfigured options
    s.insert(77); // Monster Damage
    s.insert(79); // Special Walk - only "Default"
    s.insert(82);
    s.insert(83); // Visual Effect - flat, no options
    s.insert(90); // Damage Reduction - ambiguous options
    s.insert(92); // Damage Vulnerability duplicate of 24
    s.insert(94);
    s.insert(96);
    s.insert(100);
    s
});

/// `itempropdef.2da` rows whose `Name` strref points at another property's text in
/// vanilla data (85 ArrowCatching -> "Turn Resistance", 86 Bashing -> "Trap",
/// 87 Animated -> "On Hit", 88 Wild and 89 Etherealness -> "Thieves' Tools").
/// These fall back to the cleaned label column instead.
pub static IGNORED_NAME_STRREF_IDS: LazyLock<HashSet<u32>> =
    LazyLock::new(|| HashSet::from([85, 86, 87, 88, 89]));

pub fn is_invalid_label(label: &str) -> bool {
    let label_upper = label.to_uppercase();
    INVALID_LABEL_PATTERNS
        .iter()
        .any(|pattern| label_upper.contains(&pattern.to_uppercase()))
}

pub fn clean_label(label: &str) -> String {
    if let Some(&cleaned) = LABEL_CLEANUPS.get(label) {
        return cleaned.to_string();
    }

    if !label.is_empty() && label.chars().all(|c| c.is_ascii_digit()) {
        return format!("+{label}");
    }

    let with_spaces = label.replace('_', " ");
    split_camel_case(&with_spaces)
}

fn split_camel_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 8);
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c.is_ascii_uppercase() && !result.is_empty() && !result.ends_with(' ') {
            let prev = result.chars().last().unwrap_or(' ');
            if prev.is_ascii_lowercase() {
                result.push(' ');
            } else if prev.is_ascii_uppercase()
                && let Some(&next) = chars.peek()
                && next.is_ascii_lowercase()
            {
                result.push(' ');
            }
        }
        result.push(c);
    }

    result
}

pub fn is_hidden_property(id: u32) -> bool {
    HIDDEN_PROPERTY_IDS.contains(&id)
}

pub fn ignores_2da_name(id: u32) -> bool {
    IGNORED_NAME_STRREF_IDS.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_cleanup() {
        assert_eq!(clean_label("AC_Natural"), "Natural Armor");
        assert_eq!(clean_label("Some_Thing"), "Some Thing");
    }

    #[test]
    fn test_camel_case_splitting() {
        assert_eq!(clean_label("OnHitCastSpell"), "On Hit Cast Spell");
        assert_eq!(clean_label("DamageRacialGroup"), "Damage Racial Group");
        assert_eq!(clean_label("BonusFeat"), "Bonus Feat");
        assert_eq!(clean_label("ACBonus"), "AC Bonus");
        assert_eq!(clean_label("simple"), "simple");
    }

    #[test]
    fn test_invalid_label_detection() {
        assert!(is_invalid_label("****"));
        assert!(is_invalid_label("DEL_Something"));
        assert!(is_invalid_label("DELETED"));
        assert!(!is_invalid_label("Valid Label"));
    }

    #[test]
    fn test_hidden_properties() {
        assert!(is_hidden_property(30));
        assert!(is_hidden_property(47));
        assert!(!is_hidden_property(0));
        assert!(!is_hidden_property(1));
    }
}
