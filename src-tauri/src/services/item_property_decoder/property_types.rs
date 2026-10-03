use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;

/// One `itempropdef.2da` row.
#[derive(Debug, Clone)]
pub struct PropertyDefinition {
    pub label: String,
    /// Lowercase `SubTypeResRef` table name.
    pub subtype_ref: Option<String>,
    /// `iprp_costtable.2da` row.
    pub cost_table: Option<u32>,
    /// `iprp_paramtable.2da` row.
    pub param1_table: Option<u32>,
    /// GameStrRef text, what the game prints before the subtype and value.
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct PropertyMetadata {
    pub id: u32,
    pub label: String,
    pub description: String,
    pub has_subtype: bool,
    pub subtype_options: HashMap<u32, String>,
    pub has_cost_table: bool,
    pub cost_table_options: HashMap<u32, String>,
    /// `iprp_costtable.2da` row this property's values come from. Written to the
    /// item's `CostTable` field; the engine and `calculate_item_cost` resolve the
    /// table from `itempropdef.2da` instead, but real game items store it.
    pub cost_table_idx: Option<u32>,
    pub has_param1: bool,
    pub param1_options: HashMap<u32, String>,
    /// `iprp_paramtable.2da` row, written to the item's `Param1` field.
    pub param1_idx: Option<u32>,
    /// `itemprops.2da` columns (= `baseitems.PropColumn` values) on which the
    /// engine honours this property. A property added to a base item outside
    /// this list is stored but neither shown nor applied in-game. `None` means
    /// `itemprops.2da` itself failed to load, so compatibility is unknown -
    /// distinct from `Some(vec![])`, which means the table loaded and this
    /// property is genuinely disallowed on every base item.
    pub allowed_columns: Option<Vec<u32>>,
}
