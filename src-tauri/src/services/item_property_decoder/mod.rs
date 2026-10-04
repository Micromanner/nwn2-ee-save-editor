pub mod context_maps;
pub mod property_types;

use std::collections::HashMap;

use tracing::{debug, warn};

use crate::services::resource_manager::ResourceManager;
use crate::utils::parsing::{row_int, row_str};

pub use context_maps::*;
pub use property_types::{PropertyDefinition, PropertyMetadata};

const PROPERTY_ID_ABILITY_BONUS: u32 = 0;
const PROPERTY_ID_AC_BONUS: u32 = 1;
const PROPERTY_ID_AC_BONUS_VS_ALIGNMENT: u32 = 5;
const PROPERTY_ID_ENHANCEMENT: u32 = 6;
const PROPERTY_ID_ENHANCEMENT_PENALTY: u32 = 10;
const PROPERTY_ID_DECREASED_ABILITY: u32 = 27;
const PROPERTY_ID_DECREASED_AC: u32 = 28;
const PROPERTY_ID_DECREASED_SKILL: u32 = 29;
const PROPERTY_ID_SPELL_RESISTANCE: u32 = 39;
const PROPERTY_ID_SAVE_VS_ELEMENT: u32 = 40;
const PROPERTY_ID_SAVE_SPECIFIC: u32 = 41;
const PROPERTY_ID_REDUCED_SAVE_VS_ELEMENT: u32 = 49;
const PROPERTY_ID_REDUCED_SAVE_SPECIFIC: u32 = 50;
const PROPERTY_ID_SKILL_BONUS: u32 = 52;
const PROPERTY_ID_ATTACK_BONUS: u32 = 56;
const PROPERTY_ID_ATTACK_PENALTY: u32 = 60;

/// `iprp_acmodtype.2da` rows, also the values of the `baseitems.2da`
/// `AC_Enchant` column.
const AC_MOD_DODGE: u32 = 0;
const AC_MOD_NATURAL: u32 = 1;
const AC_MOD_ARMOR: u32 = 2;
const AC_MOD_SHIELD: u32 = 3;
const AC_MOD_DEFLECTION: u32 = 4;

/// `iprp_savingthrow.2da` rows. Row 0 "All" has no Name strref, so no item carries it.
const SAVE_FORTITUDE: u32 = 1;
const SAVE_WILL: u32 = 2;
const SAVE_REFLEX: u32 = 3;

/// `iprp_saveelement.2da` row 0, the only element that applies to every save.
const SAVE_ELEMENT_UNIVERSAL: u32 = 0;

/// One `iprp_*cost` table: the display label per row plus the numeric column some
/// of those tables carry (`Amount` in `iprp_resistcost`, `Value` in
/// `iprp_srcost`/`iprp_bonuscost`/`iprp_neg5cost`/...).
#[derive(Debug, Clone, Default)]
struct CostTableData {
    labels: HashMap<u32, String>,
    amounts: HashMap<u32, i32>,
}

/// Item property names, values and bonuses, built once per game-data load.
///
/// Every table is read WITH overrides (override/, workshop, HAKs), the same view
/// `GameData` and the cost calculator use, so a modded `itempropdef` is named,
/// decoded and priced from one and the same file.
#[derive(Default)]
pub struct ItemPropertyDecoder {
    property_cache: HashMap<u32, PropertyDefinition>,
    /// Keyed by `iprp_costtable.2da` row.
    cost_tables: HashMap<u32, CostTableData>,
    /// Row names per lowercase `SubTypeResRef` (e.g. `iprp_combatdam`).
    subtype_tables: HashMap<String, HashMap<u32, String>>,
    /// `baseitems.2da` `AC_Enchant` column, an `iprp_acmodtype` row (amulet 1
    /// natural, boots 0 dodge, armor 2, shields 3, most others 4 deflection).
    ac_enchant: HashMap<i32, u32>,
    editor_metadata: Vec<PropertyMetadata>,
}

impl ItemPropertyDecoder {
    pub fn load(rm: &ResourceManager) -> Self {
        let property_cache = load_property_definitions(rm);

        let cost_table_names = load_column(rm, "iprp_costtable", "name");
        let mut cost_tables = HashMap::new();
        for idx in property_cache.values().filter_map(|def| def.cost_table) {
            if let Some(name) = cost_table_names.get(&idx) {
                cost_tables
                    .entry(idx)
                    .or_insert_with(|| load_cost_table(rm, name));
            }
        }

        let mut subtype_tables = HashMap::new();
        for subtype_ref in property_cache
            .values()
            .filter_map(|def| def.subtype_ref.as_ref())
        {
            subtype_tables
                .entry(subtype_ref.clone())
                .or_insert_with(|| load_2da_options(rm, subtype_ref));
        }

        let mut decoder = Self {
            property_cache,
            cost_tables,
            subtype_tables,
            ac_enchant: load_baseitem_ac_enchant(rm),
            editor_metadata: Vec::new(),
        };
        decoder.editor_metadata = decoder.build_editor_metadata(rm);
        debug!(
            "ItemPropertyDecoder loaded {} property definitions",
            decoder.property_cache.len()
        );
        decoder
    }

    fn subtype_name(&self, def: &PropertyDefinition, subtype: u32) -> Option<&str> {
        self.subtype_tables
            .get(def.subtype_ref.as_ref()?)?
            .get(&subtype)
            .map(String::as_str)
    }

    fn cost_table_for(&self, property_id: u32) -> Option<&CostTableData> {
        self.cost_tables
            .get(&self.property_cache.get(&property_id)?.cost_table?)
    }

    /// Display text for a property's cost value ("1d4", "+ 3", "25%"), straight
    /// from the property's own cost table.
    fn cost_label(&self, property_id: u32, cost_value: u32) -> Option<&str> {
        self.cost_table_for(property_id)?
            .labels
            .get(&cost_value)
            .map(String::as_str)
    }

    /// Numeric column of a property's cost value, where its table has one.
    fn cost_amount(&self, property_id: u32, cost_value: u32) -> Option<i32> {
        self.cost_table_for(property_id)?
            .amounts
            .get(&cost_value)
            .copied()
    }

    /// `iprp_acmodtype` row the engine applies to an AC bonus property on this
    /// base item; base items without an `AC_Enchant` value give deflection.
    fn ac_bonus_type(&self, base_item_id: i32) -> u32 {
        self.ac_enchant
            .get(&base_item_id)
            .copied()
            .unwrap_or(AC_MOD_DEFLECTION)
    }

    /// The property as the game prints it on the item: its GameStrRef text
    /// ("Enhancement Bonus:", "Armor Bonus vs.") followed by the subtype and the
    /// cost-table value, all from the localized TLK.
    pub fn decode_property(
        &self,
        property_data: &HashMap<String, serde_json::Value>,
    ) -> Option<String> {
        let property_id = get_u32(property_data, "PropertyName")?;
        let subtype = get_u32(property_data, "Subtype").unwrap_or(0);
        let cost_value = get_u32(property_data, "CostValue").unwrap_or(0);

        let Some(def) = self.property_cache.get(&property_id) else {
            return Some(format!("Property {property_id}"));
        };
        let prefix = if def.description.is_empty() {
            &def.label
        } else {
            &def.description
        };

        let description = [
            Some(prefix.as_str()),
            self.subtype_name(def, subtype),
            self.cost_label(property_id, cost_value),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
        Some(description)
    }

    /// Decodes an item's properties and names the AC type of the AC family
    /// (rows 1-5), which the engine derives from the base item since the
    /// property itself does not store one.
    pub fn decode_item_properties(
        &self,
        properties: &[HashMap<String, serde_json::Value>],
        base_item_id: i32,
    ) -> Vec<String> {
        let ac_type_name = self
            .subtype_tables
            .get("iprp_acmodtype")
            .and_then(|names| names.get(&self.ac_bonus_type(base_item_id)));

        properties
            .iter()
            .filter_map(|prop| {
                let description = self.decode_property(prop)?;
                let is_ac_family = get_u32(prop, "PropertyName").is_some_and(|id| {
                    (PROPERTY_ID_AC_BONUS..=PROPERTY_ID_AC_BONUS_VS_ALIGNMENT).contains(&id)
                });
                Some(match ac_type_name {
                    Some(name) if is_ac_family => format!("{description} ({name})"),
                    _ => description,
                })
            })
            .collect()
    }

    /// Adds one item's stat bonuses to `bonuses`, matched on property and subtype
    /// ids so they count in every UI language. Values come from the property's
    /// cost table (`Value` column, negative in the `iprp_neg*cost` penalty tables).
    pub fn add_item_bonuses(
        &self,
        bonuses: &mut ItemBonuses,
        properties: &[HashMap<String, serde_json::Value>],
        base_item_id: i32,
    ) {
        let ac_bonus_type = self.ac_bonus_type(base_item_id);
        let enhances_ac = matches!(ac_bonus_type, AC_MOD_ARMOR | AC_MOD_SHIELD);
        // Attack Bonus and Enhancement on one item do not stack: the higher
        // one counts toward attack (NWN2 wiki "Attack bonus").
        let mut item_attack = 0;

        for prop in properties {
            let Some(property_id) = get_u32(prop, "PropertyName") else {
                continue;
            };
            let subtype = get_u32(prop, "Subtype").unwrap_or(0);
            let cost_value = get_u32(prop, "CostValue").unwrap_or(0);
            let value = self
                .cost_amount(property_id, cost_value)
                .unwrap_or(cost_value as i32);
            let penalty = -value.abs();

            match property_id {
                PROPERTY_ID_ABILITY_BONUS => bonuses.add_ability(subtype, value),
                PROPERTY_ID_DECREASED_ABILITY => bonuses.add_ability(subtype, penalty),
                PROPERTY_ID_AC_BONUS => bonuses.add_ac(ac_bonus_type, value),
                PROPERTY_ID_DECREASED_AC => bonuses.add_ac(subtype, penalty),
                PROPERTY_ID_ENHANCEMENT if enhances_ac => bonuses.add_ac(ac_bonus_type, value),
                PROPERTY_ID_ENHANCEMENT_PENALTY if enhances_ac => {
                    bonuses.add_ac(ac_bonus_type, penalty);
                }
                PROPERTY_ID_ENHANCEMENT | PROPERTY_ID_ATTACK_BONUS => {
                    item_attack = item_attack.max(value);
                }
                PROPERTY_ID_ENHANCEMENT_PENALTY | PROPERTY_ID_ATTACK_PENALTY => {
                    bonuses.attack_bonus += penalty;
                }
                PROPERTY_ID_SAVE_SPECIFIC => bonuses.add_save(subtype, value),
                PROPERTY_ID_REDUCED_SAVE_SPECIFIC => bonuses.add_save(subtype, penalty),
                PROPERTY_ID_SAVE_VS_ELEMENT if subtype == SAVE_ELEMENT_UNIVERSAL => {
                    bonuses.add_all_saves(value);
                }
                PROPERTY_ID_REDUCED_SAVE_VS_ELEMENT if subtype == SAVE_ELEMENT_UNIVERSAL => {
                    bonuses.add_all_saves(penalty);
                }
                PROPERTY_ID_SKILL_BONUS => bonuses.add_skill(subtype, value),
                PROPERTY_ID_DECREASED_SKILL => bonuses.add_skill(subtype, penalty),
                PROPERTY_ID_SPELL_RESISTANCE => {
                    bonuses.spell_resistance = bonuses.spell_resistance.max(value);
                }
                _ => {}
            }
        }
        bonuses.attack_bonus += item_attack;
    }

    /// Item property editor metadata (subtypes, cost tables, param tables),
    /// sorted by label.
    pub fn editor_metadata(&self) -> &[PropertyMetadata] {
        &self.editor_metadata
    }

    fn build_editor_metadata(&self, rm: &ResourceManager) -> Vec<PropertyMetadata> {
        let allowed_columns = load_itemprops_columns(rm);
        let param_table_names = load_column(rm, "iprp_paramtable", "tableresref");
        let mut param_tables: HashMap<u32, HashMap<u32, String>> = HashMap::new();

        let mut metadata: Vec<PropertyMetadata> = self
            .property_cache
            .iter()
            .filter(|(id, _)| !is_hidden_property(**id))
            .map(|(id, def)| {
                let subtype_options = def
                    .subtype_ref
                    .as_ref()
                    .and_then(|r| self.subtype_tables.get(r))
                    .cloned()
                    .unwrap_or_default();
                let cost_table_options = def
                    .cost_table
                    .and_then(|idx| self.cost_tables.get(&idx))
                    .map(|table| table.labels.clone())
                    .unwrap_or_default();
                let param1_options = def
                    .param1_table
                    .and_then(|idx| {
                        let name = param_table_names.get(&idx)?;
                        Some(
                            param_tables
                                .entry(idx)
                                .or_insert_with(|| load_2da_options(rm, name))
                                .clone(),
                        )
                    })
                    .unwrap_or_default();

                PropertyMetadata {
                    id: *id,
                    label: def.label.clone(),
                    description: def.description.clone(),
                    has_subtype: !subtype_options.is_empty(),
                    subtype_options,
                    has_cost_table: !cost_table_options.is_empty(),
                    cost_table_options,
                    cost_table_idx: def.cost_table,
                    has_param1: !param1_options.is_empty(),
                    param1_options,
                    param1_idx: def.param1_table,
                    allowed_columns: allowed_columns
                        .as_ref()
                        .map(|cols| cols.get(id).cloned().unwrap_or_default()),
                }
            })
            .collect();

        metadata.sort_by(|a, b| a.label.cmp(&b.label));
        metadata
    }
}

#[derive(Debug, Clone, Default)]
pub struct ItemBonuses {
    pub str_bonus: i32,
    pub dex_bonus: i32,
    pub con_bonus: i32,
    pub int_bonus: i32,
    pub wis_bonus: i32,
    pub cha_bonus: i32,
    pub ac_armor_bonus: i32,
    pub ac_shield_bonus: i32,
    pub ac_deflection_bonus: i32,
    pub ac_natural_bonus: i32,
    pub ac_dodge_bonus: i32,
    pub attack_bonus: i32,
    pub fortitude_bonus: i32,
    pub reflex_bonus: i32,
    pub will_bonus: i32,
    pub spell_resistance: i32,
    /// Keyed by `skills.2da` row.
    pub skill_bonuses: HashMap<u32, i32>,
    highest_abilities: [i32; 6],
    highest_deflection: i32,
    highest_natural: i32,
}

/// Adds a bonus that does not stack across items: only the highest counts,
/// penalties still add up.
fn add_highest(total: &mut i32, highest: &mut i32, value: i32) {
    if value <= 0 {
        *total += value;
    } else if value > *highest {
        *total += value - *highest;
        *highest = value;
    }
}

impl ItemBonuses {
    /// `ability` is an `iprp_abilities` row (0 Str ... 5 Cha). Bonuses to the
    /// same ability from several items do not stack (user-verified in-game).
    fn add_ability(&mut self, ability: u32, value: i32) {
        let total = match ability {
            0 => &mut self.str_bonus,
            1 => &mut self.dex_bonus,
            2 => &mut self.con_bonus,
            3 => &mut self.int_bonus,
            4 => &mut self.wis_bonus,
            5 => &mut self.cha_bonus,
            _ => return,
        };
        add_highest(total, &mut self.highest_abilities[ability as usize], value);
    }

    /// Deflection and natural armor bonuses from several items do not stack
    /// (in-game: "the effects will not stack"); dodge does.
    fn add_ac(&mut self, ac_mod_type: u32, value: i32) {
        match ac_mod_type {
            AC_MOD_DODGE => self.ac_dodge_bonus += value,
            AC_MOD_NATURAL => {
                add_highest(&mut self.ac_natural_bonus, &mut self.highest_natural, value);
            }
            AC_MOD_ARMOR => self.ac_armor_bonus += value,
            AC_MOD_SHIELD => self.ac_shield_bonus += value,
            AC_MOD_DEFLECTION => add_highest(
                &mut self.ac_deflection_bonus,
                &mut self.highest_deflection,
                value,
            ),
            _ => {}
        }
    }

    fn add_save(&mut self, save: u32, value: i32) {
        let field = match save {
            SAVE_FORTITUDE => &mut self.fortitude_bonus,
            SAVE_WILL => &mut self.will_bonus,
            SAVE_REFLEX => &mut self.reflex_bonus,
            _ => return,
        };
        *field += value;
    }

    fn add_all_saves(&mut self, value: i32) {
        self.fortitude_bonus += value;
        self.reflex_bonus += value;
        self.will_bonus += value;
    }

    fn add_skill(&mut self, skill: u32, value: i32) {
        *self.skill_bonuses.entry(skill).or_insert(0) += value;
    }
}

fn load_property_definitions(rm: &ResourceManager) -> HashMap<u32, PropertyDefinition> {
    let mut definitions = HashMap::new();
    let itempropdef = match rm.get_2da_with_overrides("itempropdef") {
        Ok(parser) => parser,
        Err(e) => {
            warn!("Failed to load itempropdef.2da: {}", e);
            return definitions;
        }
    };

    for row_idx in 0..itempropdef.row_count() {
        let Ok(row) = itempropdef.get_row_dict(row_idx) else {
            continue;
        };
        let id = row_idx as u32;

        let label = row_str(&row, "label").unwrap_or_default();
        if label.is_empty() || is_invalid_label(&label) {
            continue;
        }

        let table_index = |column| {
            row_str(&row, column)
                .filter(|s| s != "****")
                .and_then(|s| s.parse::<u32>().ok())
        };

        // The Name strref carries the property name the game itself shows
        // ("AC Bonus vs. Damage Type"), which beats deriving one from the
        // engineer-facing label column ("ArmorDamageType").
        let name_str_ref = row_int(&row, "name", -1);
        let name = if name_str_ref > 0 && !ignores_2da_name(id) {
            Some(rm.get_string(name_str_ref)).filter(|s| !s.is_empty())
        } else {
            None
        };

        definitions.insert(
            id,
            PropertyDefinition {
                label: name.unwrap_or_else(|| clean_label(&label)),
                subtype_ref: row_str(&row, "subtyperesref")
                    .filter(|s| s != "****")
                    .map(|s| s.to_lowercase()),
                cost_table: table_index("costtableresref"),
                param1_table: table_index("param1resref"),
                description: rm.get_string(row_int(&row, "gamestrref", -1)),
            },
        );
    }
    definitions
}

/// `itemprops.2da`: one row per `itempropdef` row, one column per base-item
/// class (`0_Melee` ... `21_Glove`, indexed by `baseitems.PropColumn`), `1`
/// where the engine honours the property on that class.
///
/// Returns `None` if the table itself failed to load, distinct from a loaded
/// table where a property's row is empty (genuinely disallowed everywhere).
fn load_itemprops_columns(rm: &ResourceManager) -> Option<HashMap<u32, Vec<u32>>> {
    let mut allowed = HashMap::new();
    let Ok(table) = rm.get_2da_with_overrides("itemprops") else {
        warn!("itemprops.2da not available; property/base-item compatibility unknown");
        return None;
    };

    let columns: Vec<(u32, String)> = table
        .column_names()
        .into_iter()
        .filter_map(|name| {
            let digits: String = name.chars().take_while(char::is_ascii_digit).collect();
            digits
                .parse::<u32>()
                .ok()
                .map(|idx| (idx, name.to_string()))
        })
        .collect();

    for row_idx in 0..table.row_count() {
        let cols: Vec<u32> = columns
            .iter()
            .filter(|(_, name)| {
                table
                    .get_cell_by_name(row_idx, name)
                    .ok()
                    .flatten()
                    .is_some_and(|v| v == "1")
            })
            .map(|(idx, _)| *idx)
            .collect();
        allowed.insert(row_idx as u32, cols);
    }
    Some(allowed)
}

/// `baseitems.2da` `AC_Enchant` column, keyed by base item row: the engine's
/// own record of which AC family (armor/shield/deflection) an item's AC
/// enchantment belongs to.
fn load_baseitem_ac_enchant(rm: &ResourceManager) -> HashMap<i32, u32> {
    let mut ac_enchant = HashMap::new();
    let Ok(table) = rm.get_2da_with_overrides("baseitems") else {
        warn!("baseitems.2da not available; AC bonuses will default to deflection");
        return ac_enchant;
    };

    for row_idx in 0..table.row_count() {
        let Ok(row) = table.get_row_dict(row_idx) else {
            continue;
        };
        if let Ok(value) = u32::try_from(row_int(&row, "ac_enchant", -1)) {
            ac_enchant.insert(row_idx as i32, value);
        }
    }
    ac_enchant
}

/// One string column of a 2DA, keyed by row (`iprp_costtable` / `iprp_paramtable`
/// map a row index to the table holding the values).
fn load_column(rm: &ResourceManager, table_name: &str, column: &str) -> HashMap<u32, String> {
    let Ok(table) = rm.get_2da_with_overrides(table_name) else {
        return HashMap::new();
    };
    (0..table.row_count())
        .filter_map(|row_idx| {
            let row = table.get_row_dict(row_idx).ok()?;
            Some((row_idx as u32, row_str(&row, column)?))
        })
        .collect()
}

/// Row display names of a subtype or param table: the `Name` strref, else the
/// `GameString` strref, else the cleaned label column.
fn load_2da_options(rm: &ResourceManager, table_name: &str) -> HashMap<u32, String> {
    let mut options = HashMap::new();
    let Ok(table) = rm.get_2da_with_overrides(table_name) else {
        return options;
    };

    for row_idx in 0..table.row_count() {
        let Ok(row) = table.get_row_dict(row_idx) else {
            continue;
        };

        let resolve = |column, min_str_ref| {
            let str_ref = row_int(&row, column, -1);
            if str_ref < min_str_ref {
                return None;
            }
            Some(rm.get_string(str_ref))
                .filter(|s| !s.is_empty() && !s.chars().all(|c| c.is_ascii_digit()))
        };

        let display_name = resolve("name", 101)
            .or_else(|| resolve("gamestring", 0))
            .or_else(|| {
                row_str(&row, "label")
                    .filter(|s| !is_invalid_label(s))
                    .map(|s| clean_label(&s))
            });

        if let Some(display_name) = display_name {
            options.insert(row_idx as u32, display_name);
        }
    }
    options
}

/// Load one `iprp_*cost` table.
///
/// `amounts` is recorded for every row that carries a parsable numeric column
/// (`Amount` or `Value`), independent of whether the row has a usable display
/// name: item bonuses and penalties read it through `cost_amount`.
///
/// `labels` drops rows without a resolvable `Name` strref. The engine builds an
/// item property's description from that strref, so a row it cannot resolve
/// renders as "Bad StrRef" on the item in game - which is exactly what row 0
/// ("Random", Name `****`) of every cost table does. Random is an
/// authoring-time instruction for the treasure generator, not a value a
/// finished item can carry.
fn load_cost_table(rm: &ResourceManager, table_name: &str) -> CostTableData {
    let mut data = CostTableData::default();
    let Ok(table) = rm.get_2da_with_overrides(table_name) else {
        return data;
    };

    for row_idx in 0..table.row_count() {
        let Ok(row) = table.get_row_dict(row_idx) else {
            continue;
        };

        for column in ["amount", "value"] {
            if let Some(amount) = row_str(&row, column).and_then(|s| s.parse::<i32>().ok()) {
                data.amounts.insert(row_idx as u32, amount);
                break;
            }
        }

        let name_str_ref = row_int(&row, "name", -1);
        if name_str_ref <= 100 {
            continue;
        }

        // The game prints this text as is ("+ 3", "16", "-2"); the label column
        // is engineer-facing ("Bonus_16" would read as a DC).
        let resolved = rm.get_string(name_str_ref);
        let resolved = resolved.trim();
        if !resolved.is_empty() {
            data.labels.insert(row_idx as u32, resolved.to_string());
        }
    }

    data
}

fn get_u32(data: &HashMap<String, serde_json::Value>, key: &str) -> Option<u32> {
    data.get(key).and_then(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_i64().map(|n| n as u32))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_u32() {
        let mut data = HashMap::new();
        data.insert(
            "PropertyName".to_string(),
            serde_json::Value::Number(serde_json::Number::from(5u64)),
        );

        assert_eq!(get_u32(&data, "PropertyName"), Some(5));
        assert_eq!(get_u32(&data, "Missing"), None);
    }

    fn property(
        property_id: u32,
        subtype: u32,
        cost_value: u32,
    ) -> HashMap<String, serde_json::Value> {
        HashMap::from([
            ("PropertyName".to_string(), property_id.into()),
            ("Subtype".to_string(), subtype.into()),
            ("CostValue".to_string(), cost_value.into()),
        ])
    }

    fn bonuses_of(properties: &[HashMap<String, serde_json::Value>]) -> ItemBonuses {
        let mut bonuses = ItemBonuses::default();
        ItemPropertyDecoder::default().add_item_bonuses(&mut bonuses, properties, 0);
        bonuses
    }

    #[test]
    fn penalties_lower_the_matching_stat() {
        let bonuses = bonuses_of(&[
            property(PROPERTY_ID_SAVE_SPECIFIC, SAVE_WILL, 4),
            property(PROPERTY_ID_REDUCED_SAVE_SPECIFIC, SAVE_WILL, 3),
            property(
                PROPERTY_ID_REDUCED_SAVE_VS_ELEMENT,
                SAVE_ELEMENT_UNIVERSAL,
                1,
            ),
            property(PROPERTY_ID_ATTACK_PENALTY, 0, 2),
            property(PROPERTY_ID_DECREASED_AC, AC_MOD_DODGE, 2),
            property(PROPERTY_ID_DECREASED_SKILL, 7, 5),
            property(PROPERTY_ID_DECREASED_ABILITY, 0, 2),
        ]);

        assert_eq!(bonuses.will_bonus, 0);
        assert_eq!(bonuses.fortitude_bonus, -1);
        assert_eq!(bonuses.reflex_bonus, -1);
        assert_eq!(bonuses.attack_bonus, -2);
        assert_eq!(bonuses.ac_dodge_bonus, -2);
        assert_eq!(bonuses.skill_bonuses.get(&7), Some(&-5));
        assert_eq!(bonuses.str_bonus, -2);
    }

    #[test]
    fn deflection_from_several_items_takes_the_highest() {
        let decoder = ItemPropertyDecoder::default();
        let mut bonuses = ItemBonuses::default();
        for item in [
            [property(PROPERTY_ID_AC_BONUS, 0, 2)],
            [property(PROPERTY_ID_AC_BONUS, 0, 3)],
            [property(PROPERTY_ID_AC_BONUS, 0, 1)],
            [property(PROPERTY_ID_DECREASED_AC, AC_MOD_DEFLECTION, 1)],
        ] {
            decoder.add_item_bonuses(&mut bonuses, &item, 0);
        }

        assert_eq!(bonuses.ac_deflection_bonus, 2);
    }

    #[test]
    fn ability_bonus_from_several_items_takes_the_highest() {
        let decoder = ItemPropertyDecoder::default();
        let mut bonuses = ItemBonuses::default();
        for item in [
            [property(PROPERTY_ID_ABILITY_BONUS, 0, 3)],
            [property(PROPERTY_ID_ABILITY_BONUS, 0, 2)],
            [property(PROPERTY_ID_ABILITY_BONUS, 1, 1)],
            [property(PROPERTY_ID_DECREASED_ABILITY, 0, 1)],
        ] {
            decoder.add_item_bonuses(&mut bonuses, &item, 0);
        }

        assert_eq!(bonuses.str_bonus, 2);
        assert_eq!(bonuses.dex_bonus, 1);
    }

    #[test]
    fn weapon_attack_takes_the_higher_of_attack_and_enhancement() {
        let bonuses = bonuses_of(&[
            property(PROPERTY_ID_ATTACK_BONUS, 0, 3),
            property(PROPERTY_ID_ENHANCEMENT, 0, 5),
            property(PROPERTY_ID_ATTACK_PENALTY, 0, 1),
        ]);

        assert_eq!(bonuses.attack_bonus, 4);
    }

    #[test]
    fn saves_match_on_the_iprp_savingthrow_row() {
        let bonuses = bonuses_of(&[
            property(PROPERTY_ID_SAVE_SPECIFIC, SAVE_FORTITUDE, 1),
            property(PROPERTY_ID_SAVE_SPECIFIC, SAVE_WILL, 2),
            property(PROPERTY_ID_SAVE_SPECIFIC, SAVE_REFLEX, 3),
        ]);

        assert_eq!(bonuses.fortitude_bonus, 1);
        assert_eq!(bonuses.will_bonus, 2);
        assert_eq!(bonuses.reflex_bonus, 3);
    }
}
