use indexmap::IndexMap;
use tracing::{trace, warn};

use crate::character::armor_rules_row;
use crate::character::gff_helpers::gff_value_to_i32;
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::row_str;

/// NWN2 cost tables use multipliers that represent thousands of gold.
const GOLD_MULTIPLIER: f64 = 1000.0;
/// `itempropdef.2da` row whose cost is already in gold (see
/// `calculate_all_property_costs`).
const PROPERTY_ID_CAST_SPELL: i32 = 15;

/// Calculates item costs from 2DA tables rather than relying on stored GFF values.
///
/// Formula, verified against the game's own `.uti` templates (2026-08-28, exact
/// on every template whose stored Cost was toolset-computed, e.g.
/// `cmi_bghound_bite` = 38720 = 2.0 x 1000 x 4.4^2):
///
/// `Cost = BaseCost + ItemMultiplier * 1000 * (sum of property costs)^2 + cast-spell gold + ModifyCost`
///
/// where one property costs `itempropdef.Cost x <cost table>.Cost x <param
/// table>.Cost`; a blank `itempropdef.Cost` (`****`, e.g. Immunity, Damage
/// Resistance, Cast Spell, Bonus Feat) means the subtype table's Cost column
/// takes its place, and a table without a Cost column contributes 1.
/// The sum is squared, so stacking ten properties on one item is far dearer
/// than ten items with one property each. The value is per unit, as the game
/// shows it on the item; it does not scale with StackSize.
///
/// Returns None when the base item is unknown.
pub fn calculate_item_cost(
    item_struct: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> Option<u32> {
    let base_item_id = item_struct
        .get("BaseItem")
        .and_then(gff_value_to_i32)
        .unwrap_or(0);

    // Plot items are worthless in-game whatever they carry (user-verified).
    let is_plot = item_struct.get("Plot").and_then(gff_value_to_i32) == Some(1);
    if is_plot {
        return Some(0);
    }

    let Some((base_cost, item_multiplier)) =
        get_base_item_costs(item_struct, base_item_id, game_data)
    else {
        trace!(base_item_id, "Failed to get base item costs");
        return None;
    };

    let (property_costs, spell_gold) = calculate_all_property_costs(item_struct, game_data);

    // The engine adds the item's ModifyCost on top and ignores the stored
    // Cost field entirely (an essence with Cost 200000 / ModifyCost 100000
    // and no priced properties reads 100000 in-game).
    let modify_cost = f64::from(
        item_struct
            .get("ModifyCost")
            .and_then(gff_value_to_i32)
            .unwrap_or(0),
    );

    let total = base_cost
        + item_multiplier * GOLD_MULTIPLIER * property_costs * property_costs
        + spell_gold
        + modify_cost;

    Some(total.max(0.0).round() as u32)
}

/// `baseitems.BaseCost` and `ItemMultiplier`. Armor leaves BaseCost blank
/// and prices the body by `armorrulestats.Cost` (leather 10, full plate 1500).
fn get_base_item_costs(
    item_struct: &IndexMap<String, GffValue<'static>>,
    base_item_id: i32,
    game_data: &GameData,
) -> Option<(f64, f64)> {
    let baseitems = game_data.get_table("baseitems")?;
    let row = baseitems.get_by_id(base_item_id)?;

    let base_cost = row_str(&row, "basecost")
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            armor_rules_row(item_struct, game_data)
                .and_then(|r| row_str(&r, "cost"))
                .and_then(|s| s.parse::<f64>().ok())
        })
        .unwrap_or(0.0);

    let item_multiplier = row_str(&row, "itemmultiplier")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0);

    Some((base_cost, item_multiplier))
}

/// Returns (sum of ordinary property costs, cast-spell gold). Cast Spell
/// is priced in gold by `iprp_spells.Cost` x `iprp_chargecost.Cost` and
/// added on its own, outside the squared sum (`n2_itset_rog3`: 1 + 22058 x 3
/// = 66175; squaring it would run into the billions). With several Cast
/// Spell properties the dearest counts in full and the rest at half
/// (`n2_pca_hscout`: 10 + 1000 x 1.9^2 + 19508 + 4903 x 1.75 / 2 = 27418).
fn calculate_all_property_costs(
    item_struct: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> (f64, f64) {
    let Some(GffValue::ListOwned(props)) = item_struct.get("PropertiesList") else {
        return (0.0, 0.0);
    };

    let mut total = 0.0;
    let mut spell_costs = Vec::new();
    for prop in props {
        let Some(cost) = calculate_property_cost(prop, game_data) else {
            continue;
        };
        let is_cast_spell =
            prop.get("PropertyName").and_then(gff_value_to_i32) == Some(PROPERTY_ID_CAST_SPELL);
        if is_cast_spell {
            spell_costs.push(cost);
        } else {
            total += cost;
        }
    }
    spell_costs.sort_by(|a, b| b.total_cmp(a));
    let spell_gold = spell_costs
        .iter()
        .enumerate()
        .map(|(i, c)| if i == 0 { *c } else { c / 2.0 })
        .sum();
    (total, spell_gold)
}

fn calculate_property_cost(
    prop: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> Option<f64> {
    let property_type = prop.get("PropertyName").and_then(gff_value_to_i32)? as u32;

    let cost_value = prop
        .get("CostValue")
        .and_then(gff_value_to_i32)
        .unwrap_or(0) as u32;

    let itempropdef = game_data.get_table("itempropdef")?;
    let prop_def_row = itempropdef.get_by_id(property_type as i32)?;

    // A blank itempropdef.Cost means the subtype table carries the cost
    // (Immunity, Damage Resistance, Cast Spell, Bonus Feat, On Hit...);
    // when itempropdef has its own cost the subtype's Cost column is not
    // applied on top, as verified against the toolset-computed templates.
    let prop_def_cost = row_str(&prop_def_row, "cost")
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or_else(|| {
            row_str(&prop_def_row, "subtyperesref")
                .filter(|s| s != "****")
                .and_then(|table_name| {
                    let subtype = prop.get("Subtype").and_then(gff_value_to_i32).unwrap_or(0);
                    game_data
                        .get_table(&table_name.to_lowercase())
                        .and_then(|t| t.get_by_id(subtype))
                        .and_then(|r| row_str(&r, "cost"))
                        .and_then(|s| s.parse::<f64>().ok())
                })
                .unwrap_or(1.0)
        });

    let cost_table_ref =
        row_str(&prop_def_row, "costtableresref").and_then(|s| s.parse::<u32>().ok());

    // A property without a cost table (Haste, True Seeing, Darkvision...)
    // is flat: itempropdef.Cost counts at face value.
    let cost_multiplier = cost_table_ref
        .map(|table_id| lookup_cost_table_value(table_id, cost_value, game_data))
        .unwrap_or(1.0);

    let param_multiplier = row_str(&prop_def_row, "param1resref")
        .and_then(|s| s.parse::<u32>().ok())
        .map(|table_id| {
            let param_value = prop
                .get("Param1Value")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;
            lookup_param_table_value(table_id, param_value, game_data)
        })
        .unwrap_or(1.0);

    Some(prop_def_cost * cost_multiplier * param_multiplier)
}

/// `iprp_paramtable` row -> its table -> that row's `Cost`; 1.0 when the
/// table carries no cost column, which is most of them.
fn lookup_param_table_value(table_id: u32, param_value: u32, game_data: &GameData) -> f64 {
    let Some(paramtable) = game_data.get_table("iprp_paramtable") else {
        return 1.0;
    };
    let Some(table_name) = paramtable
        .get_by_id(table_id as i32)
        .and_then(|r| row_str(&r, "tableresref"))
        .filter(|n| n != "****")
        .map(|n| n.to_lowercase())
    else {
        return 1.0;
    };
    game_data
        .get_table(&table_name)
        .and_then(|t| t.get_by_id(param_value as i32))
        .and_then(|r| row_str(&r, "cost"))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0)
}

/// `Cost` of the property's cost-table row. A property whose table is
/// empty (`IPRP_BASE1`, table 0), missing, or has no row for the value is
/// flat and multiplies by 1; only a real row can scale it (including to 0).
fn lookup_cost_table_value(table_id: u32, cost_value: u32, game_data: &GameData) -> f64 {
    let Some(iprp_costtable) = game_data.get_table("iprp_costtable") else {
        warn!("iprp_costtable.2da not loaded");
        return 1.0;
    };

    let Some(table_name) = iprp_costtable
        .get_by_id(table_id as i32)
        .and_then(|row| row_str(&row, "name"))
        .filter(|name| name != "****")
        .map(|name| name.to_lowercase())
    else {
        return 1.0;
    };

    let Some(cost_table) = game_data.get_table(&table_name) else {
        trace!(table_name, "Cost table not loaded");
        return 1.0;
    };

    cost_table
        .get_by_id(cost_value as i32)
        .and_then(|row| row_str(&row, "cost"))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0)
}
