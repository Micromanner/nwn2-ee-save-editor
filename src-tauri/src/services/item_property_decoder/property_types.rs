use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct PropertyDefinition {
    pub id: u32,
    pub label: String,
    pub subtype_ref: Option<String>,
    pub cost_table_ref: Option<String>,
    pub param1_ref: Option<String>,
    pub description: String,
    pub game_str_ref: Option<i32>,
    pub raw_label: String,
    pub raw_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DecodedProperty {
    pub property_id: u32,
    pub label: String,
    pub description: String,
    pub bonus_type: String,
    pub decoded: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ability: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bonus_value: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalty_value: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage_dice: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_id: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub spell_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub caster_level: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub charges: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses_per_day: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub immunity_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub resistance_value: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vulnerability_percent: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_element: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ac_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub feat_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_modifier: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub light_brightness: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub light_color: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dr_value: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dr_bypass: Option<String>,

    #[specta(skip)]
    pub raw_data: HashMap<String, serde_json::Value>,
}

impl Default for DecodedProperty {
    fn default() -> Self {
        Self {
            property_id: 0,
            label: String::new(),
            description: String::new(),
            bonus_type: "unknown".to_string(),
            decoded: false,
            ability: None,
            bonus_value: None,
            penalty_value: None,
            damage_type: None,
            damage_dice: None,
            spell_id: None,
            spell_name: None,
            caster_level: None,
            charges: None,
            uses_per_day: None,
            skill_name: None,
            immunity_type: None,
            resistance_value: None,
            vulnerability_percent: None,
            save_type: None,
            save_element: None,
            ac_type: None,
            feat_name: None,
            class_name: None,
            alignment: None,
            weight_modifier: None,
            light_brightness: None,
            light_color: None,
            dr_value: None,
            dr_bypass: None,
            raw_data: HashMap::new(),
        }
    }
}

impl DecodedProperty {
    pub fn new(property_id: u32, raw_data: HashMap<String, serde_json::Value>) -> Self {
        Self {
            property_id,
            raw_data,
            ..Default::default()
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_bonus_type(mut self, bonus_type: impl Into<String>) -> Self {
        self.bonus_type = bonus_type.into();
        self
    }

    pub fn mark_decoded(mut self) -> Self {
        self.decoded = true;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct PropertyMetadata {
    pub id: u32,
    pub label: String,
    pub original_label: String,
    pub description: String,
    pub has_subtype: bool,
    pub subtype_label: String,
    pub subtype_options: HashMap<u32, String>,
    pub has_cost_table: bool,
    pub cost_table_label: String,
    pub cost_table_options: HashMap<u32, String>,
    /// `iprp_costtable.2da` row this property's values come from. Written to the
    /// item's `CostTable` field; the engine and `ItemCostCalculator` resolve the
    /// table from `itempropdef.2da` instead, but real game items store it.
    pub cost_table_idx: Option<u32>,
    pub has_param1: bool,
    pub param1_label: String,
    pub param1_options: HashMap<u32, String>,
    /// `iprp_paramtable.2da` row, written to the item's `Param1` field.
    pub param1_idx: Option<u32>,
    pub is_flat: bool,
    /// `itemprops.2da` columns (= `baseitems.PropColumn` values) on which the
    /// engine honours this property. A property added to a base item outside
    /// this list is stored but neither shown nor applied in-game.
    pub allowed_columns: Vec<u32>,
}

impl Default for PropertyMetadata {
    fn default() -> Self {
        Self {
            id: 0,
            label: String::new(),
            original_label: String::new(),
            description: String::new(),
            has_subtype: false,
            subtype_label: "Subtype".to_string(),
            subtype_options: HashMap::new(),
            has_cost_table: false,
            cost_table_label: "Value".to_string(),
            cost_table_options: HashMap::new(),
            cost_table_idx: None,
            has_param1: false,
            param1_label: "Parameter".to_string(),
            param1_options: HashMap::new(),
            param1_idx: None,
            is_flat: true,
            allowed_columns: Vec::new(),
        }
    }
}

pub fn decode_ability_bonus(
    ability: &str,
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let bonus = cost_value as i32;

    DecodedProperty {
        property_id: 0,
        label: format!("{ability} +{bonus}"),
        description: format!("Ability Bonus: {ability} +{bonus}"),
        bonus_type: "ability".to_string(),
        decoded: true,
        ability: Some(ability.to_string()),
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_ability_penalty(
    ability: &str,
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let penalty = cost_value as i32;

    DecodedProperty {
        property_id: 27,
        label: format!("{ability} -{penalty}"),
        description: format!("Ability Penalty: {ability} -{penalty}"),
        bonus_type: "ability_penalty".to_string(),
        decoded: true,
        ability: Some(ability.to_string()),
        penalty_value: Some(penalty),
        raw_data,
        ..Default::default()
    }
}

/// AC Bonus (`itempropdef` row 1) has no subtype in NWN2 - the engine derives the
/// AC type from the item itself (armor enchantment on armor, shield enchantment on
/// shields, deflection on everything else), so no `iprp_acmodtype` row applies here.
pub fn decode_ac_bonus(
    bonus: i32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 1,
        label: format!("AC Bonus +{bonus}"),
        description: format!("Armor Class Bonus: +{bonus}"),
        bonus_type: "ac".to_string(),
        decoded: true,
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_enhancement_bonus(
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let bonus = cost_value as i32;

    DecodedProperty {
        property_id: 6,
        label: format!("Enhancement +{bonus}"),
        description: format!("Enhancement Bonus +{bonus}"),
        bonus_type: "enhancement".to_string(),
        decoded: true,
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_attack_bonus(
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let bonus = cost_value as i32;

    DecodedProperty {
        property_id: 56,
        label: format!("Attack Bonus +{bonus}"),
        description: format!("Attack Bonus +{bonus}"),
        bonus_type: "attack".to_string(),
        decoded: true,
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

/// `damage_dice` is the label of the property's `iprp_damagecost.2da` row
/// ("1d4", "2d6", "+3", ...), resolved by the caller from the cost table.
pub fn decode_damage_bonus(
    damage_type: &str,
    damage_dice: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 16,
        label: format!("{damage_type} Damage {damage_dice}"),
        description: format!("Damage Bonus: {damage_type} {damage_dice}"),
        bonus_type: "damage".to_string(),
        decoded: true,
        damage_type: Some(damage_type.to_string()),
        damage_dice: Some(damage_dice.to_string()),
        raw_data,
        ..Default::default()
    }
}

/// `resistance` is the `Amount` column of the property's `iprp_resistcost.2da` row.
pub fn decode_damage_resistance(
    damage_type: &str,
    resistance: i32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 23,
        label: format!("Resist {damage_type} {resistance}"),
        description: format!("Damage Resistance: {damage_type} {resistance}/- "),
        bonus_type: "damage_resistance".to_string(),
        decoded: true,
        damage_type: Some(damage_type.to_string()),
        resistance_value: Some(resistance),
        raw_data,
        ..Default::default()
    }
}

/// `vulnerability` is the `Value` column of the property's `iprp_damvulcost.2da`
/// row (5, 10, 25, 50, 75, 90 or 100 percent).
pub fn decode_damage_vulnerability(
    damage_type: &str,
    vulnerability: i32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 24,
        label: format!("Vulnerable {damage_type} {vulnerability}%"),
        description: format!("Damage Vulnerability: {damage_type} {vulnerability}% extra damage"),
        bonus_type: "damage_vulnerability".to_string(),
        decoded: true,
        damage_type: Some(damage_type.to_string()),
        vulnerability_percent: Some(vulnerability),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_saving_throw_bonus_named(
    save_type: &str,
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let bonus = cost_value as i32;

    DecodedProperty {
        property_id: 41,
        label: format!("{save_type} Save +{bonus}"),
        description: format!("Saving Throw Bonus: {save_type} +{bonus}"),
        bonus_type: "saving_throw".to_string(),
        decoded: true,
        save_type: Some(save_type.to_string()),
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_saving_throw_vs_element_named(
    element: &str,
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let bonus = cost_value as i32;

    DecodedProperty {
        property_id: 40,
        label: format!("Save vs {element} +{bonus}"),
        description: format!("Saving Throw Bonus vs {element}: +{bonus}"),
        bonus_type: "saving_throw_element".to_string(),
        decoded: true,
        save_element: Some(element.to_string()),
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_skill_bonus(
    skill_name: &str,
    cost_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let bonus = cost_value as i32;

    DecodedProperty {
        property_id: 52,
        label: format!("{skill_name} +{bonus}"),
        description: format!("Skill Bonus: {skill_name} +{bonus}"),
        bonus_type: "skill".to_string(),
        decoded: true,
        skill_name: Some(skill_name.to_string()),
        bonus_value: Some(bonus),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_immunity(
    immunity_type: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 37,
        label: format!("Immunity: {immunity_type}"),
        description: format!("Immunity to {immunity_type}"),
        bonus_type: "immunity".to_string(),
        decoded: true,
        immunity_type: Some(immunity_type.to_string()),
        raw_data,
        ..Default::default()
    }
}

/// Cast Spell (`itempropdef` row 15): the spell name comes from `iprp_spells`
/// (whose Name already carries the caster level, "Acid Fog (11)") and the cost
/// value is an `iprp_chargecost` row ("4 Uses/Day"), so the game prints
/// `Use: Acid Fog (11) 4 Uses/Day`.
pub fn decode_cast_spell(
    prefix: &str,
    spell_name: &str,
    uses_label: Option<&str>,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let description = match uses_label {
        Some(uses) => format!("{prefix} {spell_name} {uses}"),
        None => format!("{prefix} {spell_name}"),
    };

    DecodedProperty {
        property_id: 15,
        label: description.clone(),
        description,
        bonus_type: "cast_spell".to_string(),
        decoded: true,
        spell_name: Some(spell_name.to_string()),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_bonus_feat(
    feat_name: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 12,
        label: format!("Feat: {feat_name}"),
        description: format!("Bonus Feat: {feat_name}"),
        bonus_type: "feat".to_string(),
        decoded: true,
        feat_name: Some(feat_name.to_string()),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_damage_reduction(
    dr_value: u32,
    bypass_type: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 90,
        label: format!("DR {dr_value}/{bypass_type}"),
        description: format!("Damage Reduction: {dr_value}/- (bypassed by {bypass_type})"),
        bonus_type: "damage_reduction".to_string(),
        decoded: true,
        dr_value: Some(dr_value as i32),
        dr_bypass: Some(bypass_type.to_string()),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_spell_resistance(
    sr_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 39,
        label: format!("Spell Resistance {sr_value}"),
        description: format!("Spell Resistance: {sr_value}"),
        bonus_type: "spell_resistance".to_string(),
        decoded: true,
        bonus_value: Some(sr_value as i32),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_regeneration(
    regen_value: u32,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 51,
        label: format!("Regeneration +{regen_value}"),
        description: format!("Regeneration: {regen_value} HP per round"),
        bonus_type: "regeneration".to_string(),
        decoded: true,
        bonus_value: Some(regen_value as i32),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_freedom_of_movement(raw_data: HashMap<String, serde_json::Value>) -> DecodedProperty {
    DecodedProperty {
        property_id: 75,
        label: "Freedom of Movement".to_string(),
        description: "Grants Freedom of Movement".to_string(),
        bonus_type: "special".to_string(),
        decoded: true,
        raw_data,
        ..Default::default()
    }
}

pub fn decode_haste(raw_data: HashMap<String, serde_json::Value>) -> DecodedProperty {
    DecodedProperty {
        property_id: 35,
        label: "Haste".to_string(),
        description: "Grants Haste effect".to_string(),
        bonus_type: "special".to_string(),
        decoded: true,
        raw_data,
        ..Default::default()
    }
}

pub fn decode_true_seeing(raw_data: HashMap<String, serde_json::Value>) -> DecodedProperty {
    DecodedProperty {
        property_id: 71,
        label: "True Seeing".to_string(),
        description: "Grants True Seeing".to_string(),
        bonus_type: "special".to_string(),
        decoded: true,
        raw_data,
        ..Default::default()
    }
}

pub fn decode_use_limitation_class(
    class_name: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 63,
        label: format!("Use: {class_name} Only"),
        description: format!("Use Limitation: {class_name} class only"),
        bonus_type: "use_limitation".to_string(),
        decoded: true,
        class_name: Some(class_name.to_string()),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_use_limitation_alignment(
    alignment: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id: 62,
        label: format!("Use: {alignment} Only"),
        description: format!("Use Limitation: {alignment} alignment only"),
        bonus_type: "use_limitation".to_string(),
        decoded: true,
        alignment: Some(alignment.to_string()),
        raw_data,
        ..Default::default()
    }
}

pub fn decode_generic(
    property_id: u32,
    label: &str,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    DecodedProperty {
        property_id,
        label: label.to_string(),
        description: label.to_string(),
        bonus_type: "unknown".to_string(),
        decoded: true,
        raw_data,
        ..Default::default()
    }
}

/// `cost_label` is the property's cost-table row label, or None when the property
/// is flat or its stored value has no row (e.g. the strref-less "Random" row 0).
/// `prefix` is the property's GameStrRef text, the words the game itself puts
/// before the subtype and value on the item ("Armor Bonus vs.").
pub fn decode_generic_with_context(
    property_id: u32,
    label: &str,
    prefix: &str,
    subtype_name: Option<&str>,
    cost_label: Option<&str>,
    raw_data: HashMap<String, serde_json::Value>,
) -> DecodedProperty {
    let description = match (subtype_name, cost_label) {
        (Some(sub), Some(v)) => format!("{prefix} {sub} {v}"),
        (Some(sub), None) => format!("{prefix} {sub}"),
        (None, Some(v)) => format!("{prefix} {v}"),
        (None, None) => prefix.to_string(),
    };

    DecodedProperty {
        property_id,
        label: label.to_string(),
        description,
        bonus_type: "generic".to_string(),
        decoded: true,
        raw_data,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_ability_bonus() {
        let raw = HashMap::new();
        let decoded = decode_ability_bonus("Str", 4, raw);

        assert_eq!(decoded.label, "Str +4");
        assert_eq!(decoded.ability, Some("Str".to_string()));
        assert_eq!(decoded.bonus_value, Some(4));
        assert!(decoded.decoded);
    }

    #[test]
    fn test_decode_ac_bonus_has_no_subtype() {
        let raw = HashMap::new();
        let decoded = decode_ac_bonus(3, raw);

        assert_eq!(decoded.label, "AC Bonus +3");
        assert_eq!(decoded.ac_type, None);
        assert_eq!(decoded.bonus_value, Some(3));
    }

    #[test]
    fn test_decode_damage_resistance() {
        let raw = HashMap::new();
        let decoded = decode_damage_resistance("Acid", 10, raw);

        assert_eq!(decoded.label, "Resist Acid 10");
        assert_eq!(decoded.damage_type, Some("Acid".to_string()));
        assert_eq!(decoded.resistance_value, Some(10));
    }

    #[test]
    fn test_decode_damage_bonus_uses_cost_table_label() {
        let raw = HashMap::new();
        let decoded = decode_damage_bonus("Fire", "1d4", raw);

        assert_eq!(decoded.label, "Fire Damage 1d4");
        assert_eq!(decoded.damage_dice, Some("1d4".to_string()));
    }

    #[test]
    fn test_decode_cast_spell() {
        let raw = HashMap::new();
        let decoded = decode_cast_spell("Use:", "Fireball (10)", Some("5 Charges/Use"), raw);

        assert_eq!(decoded.description, "Use: Fireball (10) 5 Charges/Use");
        assert_eq!(decoded.spell_name.as_deref(), Some("Fireball (10)"));
    }
}
