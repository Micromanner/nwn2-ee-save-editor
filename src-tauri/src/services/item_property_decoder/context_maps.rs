const INVALID_LABEL_PATTERNS: &[&str] = &[
    "****",
    "**",
    "DEL_",
    "DELETED",
    "PADDING",
    "NONE",
    "REMOVED",
    "TEST",
    "INVALID",
    "RESERVED",
    "UNUSED",
    "PLACEHOLDER",
];

const HIDDEN_PROPERTY_IDS: &[u32] = &[
    30,  // DoubleStack - inventory system flag
    31,  // EnhancedContainer BonusSlot - flat, no options
    33,  // DamageMelee - has type but no damage amount
    34,  // DamageRanged - has type but no damage amount
    47,  // DamageNone - remove damage properties instead
    54,  // SpellSchool Immunity - flat, no school selection
    72,  // On Monster Hit
    76,  // Poison - misconfigured options
    77,  // Monster Damage
    79,  // Special Walk - only "Default"
    82,  // OnHitCastSpell
    83,  // Visual Effect - flat, no options
    90,  // Damage Reduction - ambiguous options
    92,  // Damage Vulnerability duplicate of 24
    94,  // padding
    96,  // padding
    100, // padding
];

/// `itempropdef.2da` rows whose `Name` strref points at another property's text in
/// vanilla data (85 ArrowCatching -> "Turn Resistance", 86 Bashing -> "Trap",
/// 87 Animated -> "On Hit", 88 Wild and 89 Etherealness -> "Thieves' Tools").
/// These fall back to the cleaned label column instead.
const IGNORED_NAME_STRREF_IDS: &[u32] = &[85, 86, 87, 88, 89];

/// Engineer-facing 2DA label of a deleted or placeholder row.
pub fn is_invalid_label(label: &str) -> bool {
    let label_upper = label.to_uppercase();
    INVALID_LABEL_PATTERNS
        .iter()
        .any(|pattern| label_upper.contains(pattern))
}

pub fn clean_label(label: &str) -> String {
    let cleaned = match label {
        "TN" => Some("True Neutral"),
        "AC_Natural" => Some("Natural Armor"),
        "AC_Dodge" => Some("Dodge"),
        "AC_Deflection" => Some("Deflection"),
        "AC_Armor" => Some("Armor"),
        "AC_Shield" => Some("Shield"),
        _ => None,
    };
    if let Some(cleaned) = cleaned {
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
