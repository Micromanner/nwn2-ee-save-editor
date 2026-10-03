use crate::character::CharacterError;
use crate::character::gff_helpers::{extract_list_from_map, gff_value_to_i32, gff_value_to_string};
use crate::character::item_appearance::{ItemAppearance, parse_equip_slots};
use crate::character::types::BaseItemId;
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::services::ItemCostCalculator;
use crate::services::item_property_decoder::{ItemBonuses, ItemPropertyDecoder};
use crate::utils::parsing::{row_int, row_str};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use serde_json::Value as JsonValue;
use specta::Type;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[specta(transparent)]
pub struct RawItemData(pub HashMap<String, JsonValue>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSlot {
    Head,
    Chest,
    Boots,
    Gloves,
    RightHand,
    LeftHand,
    Cloak,
    LeftRing,
    RightRing,
    Neck,
    Belt,
    Arrows,
    Bullets,
    Bolts,
}

impl EquipmentSlot {
    pub fn to_bitmask(&self) -> u32 {
        match self {
            Self::Head => 0x0001,
            Self::Chest => 0x0002,
            Self::Boots => 0x0004,
            Self::Gloves => 0x0008,
            Self::RightHand => 0x0010,
            Self::LeftHand => 0x0020,
            Self::Cloak => 0x0040,
            Self::LeftRing => 0x0080,
            Self::RightRing => 0x0100,
            Self::Neck => 0x0200,
            Self::Belt => 0x0400,
            Self::Arrows => 0x0800,
            Self::Bullets => 0x1000,
            Self::Bolts => 0x2000,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Head => "Head",
            Self::Chest => "Chest",
            Self::Boots => "Boots",
            Self::Gloves => "Gloves",
            Self::RightHand => "Right Hand",
            Self::LeftHand => "Left Hand",
            Self::Cloak => "Cloak",
            Self::LeftRing => "Left Ring",
            Self::RightRing => "Right Ring",
            Self::Neck => "Neck",
            Self::Belt => "Belt",
            Self::Arrows => "Arrows",
            Self::Bullets => "Bullets",
            Self::Bolts => "Bolts",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct InventoryItem {
    pub index: usize,
    pub base_item_id: BaseItemId,
    pub name: String,
    pub tag: String,
    pub stack_size: i32,
    pub identified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EquipResult {
    pub slot: EquipmentSlot,
    pub equipped_item: Option<InventoryItem>,
    pub swapped_item: Option<InventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UnequipResult {
    pub slot: EquipmentSlot,
    pub unequipped_item: Option<InventoryItem>,
    pub inventory_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AddItemResult {
    pub inventory_index: Option<usize>,
    pub stacked: bool,
    pub item: Option<BasicItemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RemoveItemResult {
    pub removed_item: Option<InventoryItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TransformResult {}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BasicItemInfo {
    pub tag: String,
    pub base_item: i32,
    pub stack_size: i32,
    pub identified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BaseItemData {
    pub name: String,
    pub category: String,
    pub max_stack: i32,
    /// `baseitems.MaxProps` - how many item properties this base type is meant to
    /// carry. 0 when the row leaves it blank.
    pub max_props: i32,
    /// `baseitems.PropColumn` - the `itemprops.2da` column that says which
    /// properties the engine honours on this base type. -1 when blank.
    pub prop_column: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FullInventorySummary {
    pub inventory: Vec<FullInventoryItem>,
    pub equipped: Vec<FullEquippedItem>,
    pub gold: u32,
    pub encumbrance: FullEncumbrance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FullInventoryItem {
    pub index: usize,
    pub item: RawItemData,
    pub base_item: i32,
    pub base_item_name: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: String,
    pub weight: f32,
    pub value: i32,
    pub is_custom: bool,
    pub stack_size: i32,
    pub enhancement: i32,
    pub charges: Option<i32>,
    pub identified: bool,
    pub plot: bool,
    pub cursed: bool,
    pub stolen: bool,
    pub base_ac: Option<i32>,
    /// `baseitems.Stacking` - the largest stack the engine handles for this base
    /// type. Above it the game shows a single item with the whole stack's weight.
    pub max_stack: i32,
    /// `baseitems.MaxProps` - property count this base type is meant to carry.
    pub max_props: i32,
    /// See [`BaseItemData::prop_column`].
    pub prop_column: i32,
    pub category: String,
    pub equippable_slots: Vec<String>,
    pub default_slot: Option<String>,
    pub decoded_properties: Vec<DecodedPropertyInfo>,
    pub appearance: ItemAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DecodedPropertyInfo {
    pub property_name: String,
    pub subtype_name: Option<String>,
    pub cost_value: i32,
    pub param1_value: Option<i32>,
    pub display_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FullEquippedItem {
    pub slot: String,
    pub base_item: i32,
    pub base_item_name: String,
    pub custom: bool,
    pub name: String,
    pub icon: Option<String>,
    pub description: String,
    pub weight: f32,
    pub value: i32,
    pub item_data: RawItemData,
    pub base_ac: Option<i32>,
    /// See [`FullInventoryItem::max_stack`] / [`FullInventoryItem::max_props`].
    pub max_stack: i32,
    pub max_props: i32,
    pub prop_column: i32,
    pub decoded_properties: Vec<DecodedPropertyInfo>,
    pub appearance: ItemAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FullEncumbrance {
    pub total_weight: f32,
    /// `encumbrance.Heavy` in lbs: above it the character is overloaded.
    pub heavy_load: f32,
    pub status: EncumbranceStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum EncumbranceStatus {
    Normal,
    Encumbered,
    Overloaded,
}

const BASE_ITEM_ARMOR: i32 = 16;
const BASE_ITEM_SMALL_SHIELD: i32 = 14;
const BASE_ITEM_LARGE_SHIELD: i32 = 56;
const BASE_ITEM_TOWER_SHIELD: i32 = 57;

struct ItemDetails {
    base_item: i32,
    base_item_name: String,
    name: String,
    description: String,
    is_custom: bool,
    category: String,
    icon: Option<String>,
    weight: f32,
    stack_size: i32,
    value: i32,
    base_ac: Option<i32>,
    max_stack: i32,
    max_props: i32,
    prop_column: i32,
    decoded_properties: Vec<DecodedPropertyInfo>,
    item_data: RawItemData,
    appearance: ItemAppearance,
}

const EQUIPMENT_SLOTS: [(u32, &str); 14] = [
    (0x0001, "Head"),
    (0x0002, "Chest"),
    (0x0004, "Boots"),
    (0x0008, "Gloves"),
    (0x0010, "Right Hand"),
    (0x0020, "Left Hand"),
    (0x0040, "Cloak"),
    (0x0080, "Left Ring"),
    (0x0100, "Right Ring"),
    (0x0200, "Neck"),
    (0x0400, "Belt"),
    (0x0800, "Arrows"),
    (0x1000, "Bullets"),
    (0x2000, "Bolts"),
];

impl super::Character {
    pub fn gold(&self) -> u32 {
        self.get_u32("Gold").unwrap_or(0)
    }

    pub fn set_gold(&mut self, amount: u32) {
        self.set_u32("Gold", amount);
    }

    pub fn inventory_count(&self) -> usize {
        self.get_list_owned("ItemList").map_or(0, |v| v.len())
    }

    pub fn equipped_count(&self) -> usize {
        self.get_list_owned("Equip_ItemList").map_or(0, |list| {
            list.iter()
                .filter(|item| {
                    let struct_id = item
                        .get("__struct_id__")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(0) as u32;
                    struct_id > 0
                })
                .count()
        })
    }

    pub fn inventory_items(&self) -> Vec<BasicItemInfo> {
        self.get_list_owned("ItemList")
            .map(|list| {
                list.iter()
                    .filter_map(|item| self.parse_basic_item_info(item))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn equipped_items(&self) -> Vec<(usize, BasicItemInfo)> {
        self.get_list_owned("Equip_ItemList")
            .map(|list| {
                list.iter()
                    .enumerate()
                    .filter_map(|(index, item)| {
                        let struct_id = item
                            .get("__struct_id__")
                            .and_then(gff_value_to_i32)
                            .unwrap_or(0) as u32;

                        if struct_id == 0 {
                            return None;
                        }

                        self.parse_basic_item_info(item).map(|info| (index, info))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_basic_item_info(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
    ) -> Option<BasicItemInfo> {
        let tag = item_struct
            .get("Tag")
            .and_then(|v| match v {
                GffValue::String(s) => Some(s.to_string()),
                _ => None,
            })
            .unwrap_or_default();

        let base_item = item_struct
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);

        let stack_size = item_struct
            .get("StackSize")
            .and_then(gff_value_to_i32)
            .unwrap_or(1);

        let identified = item_struct
            .get("Identified")
            .and_then(gff_value_to_i32)
            .unwrap_or(1)
            != 0;

        Some(BasicItemInfo {
            tag,
            base_item,
            stack_size,
            identified,
        })
    }

    pub fn get_base_item_data(
        &self,
        base_item_id: i32,
        game_data: &GameData,
    ) -> Option<BaseItemData> {
        let baseitems = game_data.get_table("baseitems")?;
        let row = baseitems.get_by_id(base_item_id)?;

        Some(BaseItemData {
            name: game_data
                .get_string(row_int(&row, "name", -1))
                .unwrap_or_else(|| format!("Item {base_item_id}")),
            category: get_item_category(&row),
            max_stack: row_int(&row, "stacking", 1),
            max_props: row_int(&row, "maxprops", 0),
            prop_column: row_int(&row, "propcolumn", -1),
        })
    }

    /// `baseitems.Stacking`, 1 for a base item missing from `baseitems`.
    pub(crate) fn max_stack(&self, base_item_id: i32, game_data: &GameData) -> i32 {
        self.get_base_item_data(base_item_id, game_data)
            .map_or(1, |d| d.max_stack)
            .max(1)
    }

    pub fn get_full_inventory_summary(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
        resource_manager: Option<&crate::services::resource_manager::ResourceManager>,
    ) -> FullInventorySummary {
        let mut total_weight = 0.0;
        let inventory = self
            .get_list_owned("ItemList")
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(index, item_struct)| {
                let common = self.item_details(item_struct, game_data, decoder, resource_manager);
                total_weight += common.weight * common.stack_size as f32;
                let field = |name: &str| item_struct.get(name).and_then(gff_value_to_i32);
                let (equippable_slots, default_slot) =
                    self.get_equippable_slots(common.base_item, game_data);

                FullInventoryItem {
                    index,
                    item: common.item_data,
                    base_item: common.base_item,
                    base_item_name: common.base_item_name,
                    name: common.name,
                    icon: common.icon,
                    description: common.description,
                    weight: common.weight,
                    value: common.value,
                    is_custom: common.is_custom,
                    stack_size: common.stack_size,
                    enhancement: field("Enhancement").unwrap_or(0),
                    charges: field("Charges"),
                    identified: field("Identified").unwrap_or(1) != 0,
                    plot: field("Plot") == Some(1),
                    cursed: field("Cursed") == Some(1),
                    stolen: field("Stolen") == Some(1),
                    base_ac: common.base_ac,
                    max_stack: common.max_stack,
                    max_props: common.max_props,
                    prop_column: common.prop_column,
                    category: common.category,
                    equippable_slots,
                    default_slot,
                    decoded_properties: common.decoded_properties,
                    appearance: common.appearance,
                }
            })
            .collect();

        let equipped = self
            .get_list_owned("Equip_ItemList")
            .unwrap_or_default()
            .iter()
            .filter_map(|item_struct| {
                let struct_id = item_struct
                    .get("__struct_id__")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0) as u32;
                if struct_id == 0 {
                    return None;
                }
                let common = self.item_details(item_struct, game_data, decoder, resource_manager);
                total_weight += common.weight * common.stack_size as f32;

                Some(FullEquippedItem {
                    slot: get_slot_name(struct_id).to_lowercase().replace(' ', "_"),
                    base_item: common.base_item,
                    base_item_name: common.base_item_name,
                    custom: common.is_custom,
                    name: common.name,
                    icon: common.icon,
                    description: common.description,
                    weight: common.weight,
                    value: common.value,
                    item_data: common.item_data,
                    base_ac: common.base_ac,
                    max_stack: common.max_stack,
                    max_props: common.max_props,
                    prop_column: common.prop_column,
                    decoded_properties: common.decoded_properties,
                    appearance: common.appearance,
                })
            })
            .collect();

        FullInventorySummary {
            inventory,
            equipped,
            gold: self.gold(),
            encumbrance: self.get_encumbrance(total_weight, game_data, decoder),
        }
    }

    /// Display fields shared by inventory and equipped items.
    fn item_details(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
        resource_manager: Option<&crate::services::resource_manager::ResourceManager>,
    ) -> ItemDetails {
        let base_item = item_struct
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        let base_item_data = self.get_base_item_data(base_item, game_data);
        let base_item_name = base_item_data
            .as_ref()
            .map_or_else(|| format!("Item {base_item}"), |d| d.name.clone());

        let mut appearance = ItemAppearance::from_gff(item_struct);
        appearance.normalize_for_ui(base_item, game_data);
        // Fall back to the UTI template's Tintable when the save has no
        // explicit tints, so the UI swatches show the colours actually
        // rendered by the game.
        if let Some(rm) = resource_manager
            && let Some(effective) = Self::read_item_tint_with_fallback(item_struct, rm)
        {
            appearance.tints = effective;
        }

        ItemDetails {
            base_item,
            name: self
                .get_item_localized_name(item_struct, game_data)
                .unwrap_or_else(|| base_item_name.clone()),
            description: self
                .get_item_localized_description(item_struct, game_data)
                .unwrap_or_default(),
            base_item_name,
            is_custom: base_item_data.is_none(),
            category: base_item_data
                .as_ref()
                .map_or_else(|| "Miscellaneous".to_string(), |d| d.category.clone()),
            icon: resolve_item_icon(item_struct, game_data),
            weight: item_unit_weight(item_struct, game_data),
            stack_size: item_struct
                .get("StackSize")
                .and_then(gff_value_to_i32)
                .unwrap_or(1),
            value: item_value(item_struct, game_data),
            base_ac: self.get_base_item_ac(item_struct, base_item, game_data),
            max_stack: base_item_data.as_ref().map_or(1, |d| d.max_stack.max(1)),
            max_props: base_item_data.as_ref().map_or(0, |d| d.max_props),
            prop_column: base_item_data.as_ref().map_or(-1, |d| d.prop_column),
            decoded_properties: self.decode_item_properties(item_struct, decoder),
            item_data: RawItemData(gff_struct_to_json(item_struct)),
            appearance,
        }
    }

    fn get_item_localized_name(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Option<String> {
        if let Some(GffValue::LocString(ls)) = item_struct.get("LocalizedName") {
            if !ls.substrings.is_empty() {
                return ls.substrings.first().map(|sub| sub.string.to_string());
            }
            if ls.string_ref >= 0 {
                return game_data.get_string(ls.string_ref);
            }
        }
        None
    }

    fn get_item_localized_description(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Option<String> {
        if let Some(GffValue::LocString(ls)) = item_struct.get("DescIdentified") {
            if !ls.substrings.is_empty() {
                return ls.substrings.first().map(|sub| sub.string.to_string());
            }
            if ls.string_ref >= 0 {
                return game_data.get_string(ls.string_ref);
            }
        }
        if let Some(GffValue::LocString(ls)) = item_struct.get("Description") {
            if !ls.substrings.is_empty() {
                return ls.substrings.first().map(|sub| sub.string.to_string());
            }
            if ls.string_ref >= 0 {
                return game_data.get_string(ls.string_ref);
            }
        }
        None
    }

    fn get_equippable_slots(
        &self,
        base_item_id: i32,
        game_data: &GameData,
    ) -> (Vec<String>, Option<String>) {
        let equip_slots = equipable_slots_mask(base_item_id, game_data);

        let mut slots = Vec::new();
        let mut default_slot = None;

        for (bitmask, name) in EQUIPMENT_SLOTS {
            if equip_slots & bitmask != 0 {
                let slot_name = name.to_lowercase().replace(' ', "_");
                if default_slot.is_none() {
                    default_slot = Some(slot_name.clone());
                }
                slots.push(slot_name);
            }
        }

        (slots, default_slot)
    }

    fn decode_item_properties(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        decoder: &ItemPropertyDecoder,
    ) -> Vec<DecodedPropertyInfo> {
        let Some(GffValue::ListOwned(props)) = item_struct.get("PropertiesList") else {
            return Vec::new();
        };
        let base_item_id = item_struct
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);

        let prop_maps: Vec<_> = props.iter().map(gff_struct_to_json).collect();

        decoder
            .decode_item_properties(&prop_maps, base_item_id)
            .into_iter()
            .map(|dp| {
                let cost_value = dp
                    .raw_data
                    .get("CostValue")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32)
                    .unwrap_or(0);
                let param1_value = dp
                    .raw_data
                    .get("Param1Value")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);
                let subtype = dp
                    .raw_data
                    .get("Subtype")
                    .and_then(|v| v.as_i64())
                    .map(|v| v as i32);

                DecodedPropertyInfo {
                    property_name: dp.label.clone(),
                    subtype_name: subtype.map(|s| format!("Subtype {s}")),
                    cost_value,
                    param1_value,
                    display_string: dp.description,
                }
            })
            .collect()
    }

    pub fn get_equipment_bonuses(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> ItemBonuses {
        let mut total_bonuses = ItemBonuses::default();

        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return total_bonuses;
        };

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == 0 {
                continue;
            }

            let base_item_id = item_struct
                .get("BaseItem")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            if let Some(ac_bonus) = armor_rules_ac(item_struct, game_data) {
                if base_item_id == BASE_ITEM_ARMOR {
                    total_bonuses.ac_armor_bonus += ac_bonus.max(0);
                } else if is_armor_or_shield(base_item_id) {
                    total_bonuses.ac_shield_bonus += ac_bonus.max(0);
                }
            }

            if let Some(GffValue::ListOwned(props)) = item_struct.get("PropertiesList") {
                let prop_maps: Vec<_> = props.iter().map(gff_struct_to_json).collect();

                let item_bonuses = decoder.get_item_bonuses(&prop_maps, base_item_id);

                // Aggregate
                total_bonuses.str_bonus += item_bonuses.str_bonus;
                total_bonuses.dex_bonus += item_bonuses.dex_bonus;
                total_bonuses.con_bonus += item_bonuses.con_bonus;
                total_bonuses.int_bonus += item_bonuses.int_bonus;
                total_bonuses.wis_bonus += item_bonuses.wis_bonus;
                total_bonuses.cha_bonus += item_bonuses.cha_bonus;
                total_bonuses.ac_bonus += item_bonuses.ac_bonus;
                total_bonuses.ac_armor_bonus += item_bonuses.ac_armor_bonus;
                total_bonuses.ac_shield_bonus += item_bonuses.ac_shield_bonus;
                total_bonuses.ac_natural_bonus += item_bonuses.ac_natural_bonus;
                total_bonuses.ac_deflection_bonus += item_bonuses.ac_deflection_bonus;
                total_bonuses.ac_dodge_bonus += item_bonuses.ac_dodge_bonus;
                total_bonuses.attack_bonus += item_bonuses.attack_bonus;
                total_bonuses.damage_bonus += item_bonuses.damage_bonus;
                total_bonuses.fortitude_bonus += item_bonuses.fortitude_bonus;
                total_bonuses.reflex_bonus += item_bonuses.reflex_bonus;
                total_bonuses.will_bonus += item_bonuses.will_bonus;
                total_bonuses.spell_resistance = total_bonuses
                    .spell_resistance
                    .max(item_bonuses.spell_resistance);

                for (skill, val) in item_bonuses.skill_bonuses {
                    *total_bonuses.skill_bonuses.entry(skill).or_insert(0) += val;
                }
                for (dtype, val) in item_bonuses.damage_resistances {
                    *total_bonuses.damage_resistances.entry(dtype).or_insert(0) =
                        (*total_bonuses.damage_resistances.get(&dtype).unwrap_or(&0)).max(val);
                }
            }
        }

        total_bonuses
    }

    fn get_base_item_ac(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        base_item_id: i32,
        game_data: &GameData,
    ) -> Option<i32> {
        if !is_armor_or_shield(base_item_id) {
            return None;
        }

        // Real base AC for armor/shields comes from armorrulestats.2da via ArmorRulesType;
        // baseitems.2da baseac is 0 for the generic "armor" base item.
        if let Some(ac_bonus) = armor_rules_ac(item_struct, game_data) {
            return Some(ac_bonus);
        }

        let baseitems = game_data.get_table("baseitems")?;
        let row = baseitems.get_by_id(base_item_id)?;

        row_str(&row, "baseac").and_then(|s| s.parse::<i32>().ok())
    }

    /// Thresholds from `encumbrance.2da` (`Normal`/`Heavy`, tenths of a lb) at
    /// the total STR including racial and item bonuses.
    fn get_encumbrance(
        &self,
        total_weight: f32,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> FullEncumbrance {
        let strength = self.get_total_abilities(game_data, decoder).str_;

        let (normal_load, heavy_load) = game_data
            .get_table("encumbrance")
            .and_then(|table| {
                let last_row = i32::try_from(table.row_count()).ok()? - 1;
                table.get_by_id(strength.clamp(0, last_row))
            })
            .map_or((0.0, 0.0), |row| {
                let load = |column: &str| row_int(&row, column, 0) as f32 / 10.0;
                (load("normal"), load("heavy"))
            });

        let status = if total_weight > heavy_load {
            EncumbranceStatus::Overloaded
        } else if total_weight > normal_load {
            EncumbranceStatus::Encumbered
        } else {
            EncumbranceStatus::Normal
        };

        FullEncumbrance {
            total_weight,
            heavy_load,
            status,
        }
    }

    pub fn get_equipped_item_by_slot(&self, slot: EquipmentSlot) -> Option<BasicItemInfo> {
        let equip_list = self.get_list_owned("Equip_ItemList")?;
        let slot_bitmask = slot.to_bitmask();

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == slot_bitmask {
                return self.parse_basic_item_info(item_struct);
            }
        }

        None
    }

    pub fn equip_item(
        &mut self,
        inventory_index: usize,
        slot: EquipmentSlot,
        game_data: &GameData,
    ) -> Result<EquipResult, CharacterError> {
        let inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        if inventory_index >= inv_list.len() {
            return Err(CharacterError::ValidationFailed {
                field: "inventory_index",
                message: format!("Index {inventory_index} out of bounds"),
            });
        }

        let item_struct = inv_list[inventory_index].clone();
        drop(inv_list);

        let base_item_id = item_struct
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);

        let can_equip = self.can_item_equip_in_slot(base_item_id, slot, game_data);
        if !can_equip {
            return Err(CharacterError::InvalidOperation(format!(
                "Base item {base_item_id} cannot be equipped in {} slot",
                slot.display_name()
            )));
        }

        let mut swapped_item = None;
        if self.get_equipped_item_by_slot(slot).is_some() {
            let unequip_result = self.unequip_item(slot)?;
            swapped_item = unequip_result.unequipped_item;
        }

        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;
        inv_list.remove(inventory_index);
        self.set_list("ItemList", inv_list);

        let mut equip_list = self.get_list_owned("Equip_ItemList").unwrap_or_default();
        let mut equipped_struct = item_struct.clone();
        equipped_struct.insert(
            "__struct_id__".to_string(),
            GffValue::Dword(slot.to_bitmask()),
        );
        equip_list.push(equipped_struct);
        self.set_list("Equip_ItemList", equip_list);

        let equipped_item_info = self.parse_basic_item_info(&item_struct);

        Ok(EquipResult {
            slot,
            equipped_item: equipped_item_info.map(|info| InventoryItem {
                index: 0,
                base_item_id: BaseItemId(info.base_item),
                name: info.tag.clone(),
                tag: info.tag,
                stack_size: info.stack_size,
                identified: info.identified,
            }),
            swapped_item,
        })
    }

    pub fn unequip_item(&mut self, slot: EquipmentSlot) -> Result<UnequipResult, CharacterError> {
        let mut equip_list =
            self.get_list_owned("Equip_ItemList")
                .ok_or(CharacterError::FieldMissing {
                    field: "Equip_ItemList",
                })?;

        let slot_bitmask = slot.to_bitmask();
        let mut item_to_unequip = None;
        let mut found_index = None;

        for (index, item_struct) in equip_list.iter().enumerate() {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == slot_bitmask {
                item_to_unequip = Some(item_struct.clone());
                found_index = Some(index);
                break;
            }
        }

        let Some(item_struct) = item_to_unequip else {
            return Err(CharacterError::InvalidOperation(format!(
                "No item equipped in {} slot",
                slot.display_name()
            )));
        };

        if let Some(index) = found_index {
            equip_list.remove(index);
            self.set_list("Equip_ItemList", equip_list);
        }

        let mut inv_list = self.get_list_owned("ItemList").unwrap_or_default();
        let mut inv_item = item_struct.clone();
        inv_item.shift_remove("__struct_id__");
        let inventory_index = inv_list.len();
        inv_list.push(inv_item);
        self.set_list("ItemList", inv_list);

        let unequipped_info = self.parse_basic_item_info(&item_struct);

        Ok(UnequipResult {
            slot,
            unequipped_item: unequipped_info.map(|info| InventoryItem {
                index: inventory_index,
                base_item_id: BaseItemId(info.base_item),
                name: info.tag.clone(),
                tag: info.tag,
                stack_size: info.stack_size,
                identified: info.identified,
            }),
            inventory_index: Some(inventory_index),
        })
    }

    pub fn add_item(
        &mut self,
        base_item_id: i32,
        stack_size: i32,
        icon_id: Option<u32>,
        armor_visual_type: Option<u8>,
        game_data: &GameData,
    ) -> Result<AddItemResult, CharacterError> {
        let stack_size = stack_size.clamp(1, self.max_stack(base_item_id, game_data));

        let mut new_item = IndexMap::new();
        new_item.insert("BaseItem".to_string(), GffValue::Int(base_item_id));
        new_item.insert("StackSize".to_string(), GffValue::Word(stack_size as u16));
        new_item.insert("Identified".to_string(), GffValue::Byte(1));
        new_item.insert("Charges".to_string(), GffValue::Byte(0));
        new_item.insert("Cost".to_string(), GffValue::Int(0));
        if let Some(id) = icon_id {
            new_item.insert("Icon".to_string(), GffValue::Dword(id));
        }

        // Seed default model fields so the base item actually renders in-game.
        // Without these, the save has no ModelPart / Variation values and the
        // engine draws an empty model. Variant 1 is the universally-present
        // first variant for every in-world item family in vanilla NWN2.
        let modeltype = game_data
            .get_table("baseitems")
            .and_then(|t| t.get_by_id(base_item_id))
            .and_then(|row| row_str(&row, "modeltype"))
            .unwrap_or_default();
        match modeltype.trim() {
            "2" => {
                // ThreePartWeapon: ModelPart1/2/3 pick the three sub-meshes.
                new_item.insert("ModelPart1".to_string(), GffValue::Byte(1));
                new_item.insert("ModelPart2".to_string(), GffValue::Byte(1));
                new_item.insert("ModelPart3".to_string(), GffValue::Byte(1));
            }
            "0" => {
                // SinglePart (shields, misc): game reads ModelPart1 for
                // variant selection; Variation is vestigial for these.
                new_item.insert("ModelPart1".to_string(), GffValue::Byte(1));
            }
            "3" => {
                // BodyArmor: variant comes from Variation, material from
                // ArmorVisualType (armor.2da row). Chest, boots and gloves
                // render nothing without a material, so seed the template's
                // or fall back to row 0 (Cloth), which every slot ships.
                new_item.insert("Variation".to_string(), GffValue::Byte(0));
                new_item.insert(
                    "ArmorVisualType".to_string(),
                    GffValue::Byte(armor_visual_type.unwrap_or(0)),
                );
            }
            _ => {
                // Modeltype empty / unknown (rings, amulets, etc.) — no
                // in-world model, nothing to seed.
            }
        }

        // Seed an explicit all-zero Tintable for any item with an in-world
        // model so the in-game render matches the editor viewer (which also
        // multiplies by zero -> black). Without an explicit Tintable the
        // engine falls back to the base item's default coloring, which
        // diverges from what the user sees in the appearance preview.
        if matches!(modeltype.trim(), "0" | "2" | "3") {
            let tint = crate::character::appearance_helpers::build_tint_struct(
                &crate::character::appearance_helpers::TintChannels::default(),
            );
            let mut tintable_outer = IndexMap::new();
            tintable_outer.insert("Tint".to_string(), GffValue::StructOwned(Box::new(tint)));
            new_item.insert(
                "Tintable".to_string(),
                GffValue::StructOwned(Box::new(tintable_outer)),
            );
        }

        self.add_new_item(new_item, game_data)
    }

    /// Adds `item` as its own inventory entry, never merging it onto a stack:
    /// the user adds it to edit it next. Quest rewards go through
    /// [`Self::add_item_from_struct`], which stacks.
    pub fn add_new_item(
        &mut self,
        mut item: IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Result<AddItemResult, CharacterError> {
        mark_identified(&mut item);
        self.clamp_stack_size(&mut item, game_data);
        let info = self.parse_basic_item_info(&item);

        let mut inv_list = self.get_list_owned("ItemList").unwrap_or_default();
        let index = inv_list.len();
        inv_list.push(item);
        self.set_list("ItemList", inv_list);

        Ok(AddItemResult {
            inventory_index: Some(index),
            stacked: false,
            item: info,
        })
    }

    pub fn remove_item(&mut self, index: usize) -> Result<RemoveItemResult, CharacterError> {
        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        if index >= inv_list.len() {
            return Err(CharacterError::ValidationFailed {
                field: "index",
                message: format!("Index {index} out of bounds"),
            });
        }

        let removed_struct = inv_list.remove(index);
        self.set_list("ItemList", inv_list);

        let removed_info = self.parse_basic_item_info(&removed_struct);

        Ok(RemoveItemResult {
            removed_item: removed_info.map(|info| InventoryItem {
                index,
                base_item_id: BaseItemId(info.base_item),
                name: info.tag.clone(),
                tag: info.tag,
                stack_size: info.stack_size,
                identified: info.identified,
            }),
        })
    }

    /// Replaces the item at `index` with `new_item` (already-loaded template
    /// fields, the same shape `add_item_from_template` produces). A clean swap:
    /// nothing from the old item is preserved except its list position.
    pub fn transform_inventory_item(
        &mut self,
        index: usize,
        mut new_item: IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Result<TransformResult, CharacterError> {
        mark_identified(&mut new_item);
        self.clamp_stack_size(&mut new_item, game_data);
        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        if index >= inv_list.len() {
            return Err(CharacterError::ValidationFailed {
                field: "index",
                message: format!("Index {index} out of bounds"),
            });
        }

        inv_list[index] = new_item;
        self.set_list("ItemList", inv_list);

        Ok(TransformResult {})
    }

    /// Replaces the item equipped in `slot` with `new_item`, keeping only the
    /// slot itself. Rejects (like equip_item) when the new
    /// item's BaseItem can't legally occupy the slot, so a transform never
    /// silently relocates an item.
    pub fn transform_equipped_item(
        &mut self,
        slot: EquipmentSlot,
        mut new_item: IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Result<TransformResult, CharacterError> {
        mark_identified(&mut new_item);
        self.clamp_stack_size(&mut new_item, game_data);
        let base_item_id = new_item
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);
        if !self.can_item_equip_in_slot(base_item_id, slot, game_data) {
            return Err(CharacterError::InvalidOperation(format!(
                "New item cannot be equipped in {} slot",
                slot.display_name()
            )));
        }

        let mut equip_list =
            self.get_list_owned("Equip_ItemList")
                .ok_or(CharacterError::FieldMissing {
                    field: "Equip_ItemList",
                })?;

        let slot_bitmask = slot.to_bitmask();
        let Some(pos) = equip_list.iter().position(|item_struct| {
            item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32
                == slot_bitmask
        }) else {
            return Err(CharacterError::InvalidOperation(format!(
                "No item equipped in {} slot",
                slot.display_name()
            )));
        };

        let mut replacement = new_item;
        replacement.insert("__struct_id__".to_string(), GffValue::Dword(slot_bitmask));
        equip_list[pos] = replacement;
        self.set_list("Equip_ItemList", equip_list);

        Ok(TransformResult {})
    }

    fn can_item_equip_in_slot(
        &self,
        base_item_id: i32,
        slot: EquipmentSlot,
        game_data: &GameData,
    ) -> bool {
        equipable_slots_mask(base_item_id, game_data) & slot.to_bitmask() != 0
    }

    pub fn get_equipped_armor_max_dex(&self, game_data: &GameData) -> i32 {
        self.equipped_armor_rules_stat("maxdexbonus", game_data)
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(999)
    }

    pub fn get_equipped_armor_rank(&self, game_data: &GameData) -> String {
        self.equipped_armor_rules_stat("rank", game_data)
            .unwrap_or_else(|| "None".to_string())
    }

    fn equipped_armor_rules_stat(&self, column: &str, game_data: &GameData) -> Option<String> {
        let armor = self.get_equipped_item_raw(EquipmentSlot::Chest)?;
        row_str(&armor_rules_row(&armor, game_data)?, column)
    }

    pub fn update_inventory_item(
        &mut self,
        index: usize,
        updated_data: &HashMap<String, JsonValue>,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        if index >= inv_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "No item at index {index}"
            )));
        }

        let had_identified = inv_list[index].contains_key("Identified");
        merge_json_into_gff_struct(&mut inv_list[index], updated_data)
            .map_err(invalid_item_field)?;
        normalize_identified_field(&mut inv_list[index], had_identified);
        normalize_item_properties_list(&mut inv_list[index]);
        self.clamp_stack_size(&mut inv_list[index], game_data);
        self.set_list("ItemList", inv_list);
        Ok(())
    }

    /// Hold `StackSize` to `baseitems.Stacking`. The engine renders a single item
    /// for a non-stackable base type (a cloak is Stacking=1) but still multiplies
    /// encumbrance by the stack, so an over-sized stack reads as one cloak that
    /// weighs ten. A base item missing from `baseitems` keeps its stack.
    fn clamp_stack_size(
        &self,
        item: &mut IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) {
        let Some(stack_size) = item.get("StackSize").and_then(gff_value_to_i32) else {
            return;
        };

        let base_item_id = item.get("BaseItem").and_then(gff_value_to_i32).unwrap_or(0);
        let Some(base_item_data) = self.get_base_item_data(base_item_id, game_data) else {
            return;
        };
        let max_stack = base_item_data.max_stack.max(1);

        let clamped = stack_size.clamp(1, max_stack);
        if clamped == stack_size {
            return;
        }

        tracing::debug!(
            base_item_id,
            stack_size,
            max_stack,
            "Clamped StackSize to baseitems.Stacking"
        );

        let new_value = typed_stack_size(item.get("StackSize"), clamped);
        item.insert("StackSize".to_string(), new_value);
    }

    pub fn update_equipped_item(
        &mut self,
        slot: EquipmentSlot,
        updated_data: &HashMap<String, JsonValue>,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let mut equip_list =
            self.get_list_owned("Equip_ItemList")
                .ok_or(CharacterError::FieldMissing {
                    field: "Equip_ItemList",
                })?;

        let slot_bitmask = slot.to_bitmask();
        let item_idx = equip_list
            .iter()
            .position(|item| {
                item.get("__struct_id__")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0) as u32
                    == slot_bitmask
            })
            .ok_or(CharacterError::InvalidOperation(format!(
                "No equipped item in slot {}",
                slot.display_name()
            )))?;

        let had_identified = equip_list[item_idx].contains_key("Identified");
        merge_json_into_gff_struct(&mut equip_list[item_idx], updated_data)
            .map_err(invalid_item_field)?;
        normalize_identified_field(&mut equip_list[item_idx], had_identified);
        normalize_item_properties_list(&mut equip_list[item_idx]);
        self.clamp_stack_size(&mut equip_list[item_idx], game_data);
        self.set_list("Equip_ItemList", equip_list);
        Ok(())
    }

    pub fn apply_inventory_item_appearance(
        &mut self,
        index: usize,
        appearance: &crate::character::ItemAppearance,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;
        if index >= inv_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "No item at index {index}"
            )));
        }
        let base_item_id = inv_list[index]
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(-1);
        let mut routed = appearance.clone();
        let variation_is_ui_only = routed.prepare_for_write(base_item_id, game_data);
        write_appearance_into_item(&mut inv_list[index], &routed, !variation_is_ui_only);
        self.set_list("ItemList", inv_list);
        Ok(())
    }

    pub fn apply_equipped_item_appearance(
        &mut self,
        slot: EquipmentSlot,
        appearance: &crate::character::ItemAppearance,
        game_data: &GameData,
    ) -> Result<(), CharacterError> {
        let mut equip_list =
            self.get_list_owned("Equip_ItemList")
                .ok_or(CharacterError::FieldMissing {
                    field: "Equip_ItemList",
                })?;
        let slot_bitmask = slot.to_bitmask();
        let item_idx = equip_list
            .iter()
            .position(|item| {
                item.get("__struct_id__")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0) as u32
                    == slot_bitmask
            })
            .ok_or(CharacterError::InvalidOperation(format!(
                "No equipped item in slot {}",
                slot.display_name()
            )))?;
        let base_item_id = equip_list[item_idx]
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(-1);
        let mut routed = appearance.clone();
        let variation_is_ui_only = routed.prepare_for_write(base_item_id, game_data);
        write_appearance_into_item(&mut equip_list[item_idx], &routed, !variation_is_ui_only);
        self.set_list("Equip_ItemList", equip_list);
        Ok(())
    }

    pub fn get_inventory_item_at(
        &self,
        index: usize,
    ) -> Option<IndexMap<String, GffValue<'static>>> {
        let inv_list = self.get_list_owned("ItemList")?;
        inv_list.get(index).cloned()
    }

    pub fn add_item_from_struct(
        &mut self,
        mut item: IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Result<AddItemResult, CharacterError> {
        mark_identified(&mut item);
        self.clamp_stack_size(&mut item, game_data);
        let base_item_id = item.get("BaseItem").and_then(gff_value_to_i32).unwrap_or(0);

        let tag = item
            .get("Tag")
            .and_then(|v| match v {
                GffValue::String(s) => Some(s.to_string()),
                _ => None,
            })
            .unwrap_or_default();

        let stack_size = item
            .get("StackSize")
            .and_then(gff_value_to_i32)
            .unwrap_or(1);

        let max_stack = self.max_stack(base_item_id, game_data);

        if !self.has_field("ItemList") {
            self.set_list("ItemList", Vec::new());
        }
        let inv_list = self
            .get_list_mut("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        let (remaining, stacked_index) = if max_stack > 1 {
            let properties = extract_list_from_map(&item, "PropertiesList").unwrap_or_default();
            fill_matching_stacks(
                inv_list,
                base_item_id,
                &tag,
                &properties,
                max_stack,
                stack_size,
            )
        } else {
            (stack_size, None)
        };
        if remaining == 0
            && let Some(index) = stacked_index
        {
            return Ok(AddItemResult {
                inventory_index: Some(index),
                stacked: true,
                item: Some(BasicItemInfo {
                    base_item: base_item_id,
                    tag,
                    stack_size: inv_list[index]
                        .get("StackSize")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(stack_size),
                    identified: true,
                }),
            });
        }

        if remaining != stack_size {
            let value = typed_stack_size(item.get("StackSize"), remaining);
            item.insert("StackSize".to_string(), value);
        }
        inv_list.push(item);
        let index = inv_list.len() - 1;

        Ok(AddItemResult {
            inventory_index: Some(index),
            stacked: false,
            item: Some(BasicItemInfo {
                base_item: base_item_id,
                tag,
                stack_size: remaining,
                identified: true,
            }),
        })
    }

    pub fn get_equipped_item_raw(
        &self,
        slot: EquipmentSlot,
    ) -> Option<IndexMap<String, GffValue<'static>>> {
        let equip_list = self.get_list_owned("Equip_ItemList")?;
        let slot_bitmask = slot.to_bitmask();

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == slot_bitmask {
                return Some(item_struct.clone());
            }
        }

        None
    }

    pub fn get_base_item_name(&self, base_item_id: i32, game_data: &GameData) -> String {
        let Some(table) = game_data.get_table("baseitems") else {
            return format!("#{base_item_id}");
        };
        let Some(row) = table.get_by_id(base_item_id) else {
            return format!("#{base_item_id}");
        };
        resolve_base_item_name(&row, base_item_id, game_data)
    }
}

fn get_slot_name(bitmask: u32) -> &'static str {
    for (slot_bitmask, name) in EQUIPMENT_SLOTS {
        if slot_bitmask == bitmask {
            return name;
        }
    }
    "Unknown"
}

fn is_invalid_base_item_label(label: &str) -> bool {
    let lower = label.to_lowercase();
    lower.is_empty()
        || lower.starts_with("bad index")
        || lower == "****"
        || lower == "deleted"
        || lower.contains("padding")
        || lower.starts_with("del_")
        || lower == "none"
}

fn get_item_category(row: &ahash::AHashMap<String, Option<String>>) -> String {
    let store_panel = row_int(row, "storepanel", 4);

    match store_panel {
        0 => "Armor & Clothing",
        1 => "Weapons",
        2 => "Magic Items",
        3 => "Accessories",
        _ => "Miscellaneous",
    }
    .to_string()
}

fn resolve_base_item_name(
    row: &ahash::AHashMap<String, Option<String>>,
    row_id: i32,
    game_data: &GameData,
) -> String {
    let name_ref = row_str(row, "name").and_then(|s| s.parse::<i32>().ok());

    if let Some(str_ref) = name_ref
        && let Some(resolved) = game_data.get_string(str_ref)
        && !is_invalid_base_item_label(&resolved)
    {
        return resolved;
    }

    row_str(row, "label").unwrap_or_else(|| format!("Item {row_id}"))
}

fn is_armor_or_shield(base_item_id: i32) -> bool {
    matches!(
        base_item_id,
        BASE_ITEM_ARMOR | BASE_ITEM_SMALL_SHIELD | BASE_ITEM_LARGE_SHIELD | BASE_ITEM_TOWER_SHIELD
    )
}

pub(crate) fn armor_rules_row(
    item_struct: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> Option<ahash::AHashMap<String, Option<String>>> {
    let rules_type = item_struct
        .get("ArmorRulesType")
        .and_then(gff_value_to_i32)?;
    game_data.get_table("armorrulestats")?.get_by_id(rules_type)
}

fn armor_rules_ac(
    item_struct: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> Option<i32> {
    let row = armor_rules_row(item_struct, game_data)?;
    row_str(&row, "acbonus")
        .and_then(|s| s.parse::<f32>().ok())
        .map(|f| f as i32)
}

/// Weight of one unit in lbs. Armor and shields weigh `armorrulestats.WEIGHT`
/// by `ArmorRulesType` (full plate 50 lb); their `baseitems.TenthLBS` is generic
/// (every body armor 10 lb).
fn item_unit_weight(
    item_struct: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> f32 {
    let base_item_id = item_struct
        .get("BaseItem")
        .and_then(gff_value_to_i32)
        .unwrap_or(0);
    let armor_weight = is_armor_or_shield(base_item_id)
        .then(|| armor_rules_row(item_struct, game_data))
        .flatten()
        .and_then(|row| row_str(&row, "weight"));
    let tenth_lbs = armor_weight.or_else(|| {
        game_data
            .get_table("baseitems")?
            .get_by_id(base_item_id)
            .and_then(|row| row_str(&row, "tenthlbs"))
    });
    tenth_lbs
        .and_then(|s| s.parse::<f32>().ok())
        .map_or(0.0, |w| w / 10.0)
}

/// `baseitems.EquipableSlots` bitmask, 0 when the row is missing or blank.
fn equipable_slots_mask(base_item_id: i32, game_data: &GameData) -> u32 {
    game_data
        .get_table("baseitems")
        .and_then(|t| t.get_by_id(base_item_id))
        .and_then(|row| row_str(&row, "equipableslots"))
        .map_or(0, |s| parse_equip_slots(&s))
}

fn gff_to_json_primitive(val: &GffValue) -> Option<JsonValue> {
    match val {
        GffValue::Byte(v) => Some(JsonValue::Number(Number::from(*v))),
        GffValue::Char(v) => Some(JsonValue::Number(Number::from(*v as u8))),
        GffValue::Short(v) => Some(JsonValue::Number(Number::from(*v))),
        GffValue::Word(v) => Some(JsonValue::Number(Number::from(*v))),
        GffValue::Int(v) => Some(JsonValue::Number(Number::from(*v))),
        GffValue::Dword(v) => Some(JsonValue::Number(Number::from(*v))),
        GffValue::Float(v) => Number::from_f64(f64::from(*v)).map(JsonValue::Number),
        GffValue::Double(v) => Number::from_f64(*v).map(JsonValue::Number),
        GffValue::String(v) => Some(JsonValue::String(v.to_string())),
        GffValue::ResRef(v) => Some(JsonValue::String(v.to_string())),
        GffValue::LocString(ls) => {
            let substrings: Vec<JsonValue> = ls
                .substrings
                .iter()
                .map(|sub| {
                    serde_json::json!({
                        "language": sub.language,
                        "gender": sub.gender,
                        "string": sub.string.to_string()
                    })
                })
                .collect();
            Some(serde_json::json!({
                "string_ref": ls.string_ref,
                "substrings": substrings
            }))
        }
        GffValue::ListOwned(list) => {
            let items: Vec<JsonValue> = list
                .iter()
                .map(|item| {
                    let obj: serde_json::Map<String, JsonValue> = item
                        .iter()
                        .filter_map(|(k, v)| gff_to_json_primitive(v).map(|jv| (k.clone(), jv)))
                        .collect();
                    JsonValue::Object(obj)
                })
                .collect();
            Some(JsonValue::Array(items))
        }
        GffValue::StructOwned(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map.iter() {
                if let Some(json_val) = gff_to_json_primitive(v) {
                    obj.insert(k.clone(), json_val);
                }
            }
            Some(JsonValue::Object(obj))
        }
        GffValue::Void(_) => None,
        _ => None,
    }
}

fn resolve_item_icon(
    item_struct: &IndexMap<String, GffValue<'static>>,
    game_data: &GameData,
) -> Option<String> {
    let icon_id = item_struct.get("Icon").and_then(gff_value_to_i32)?;
    let icons_table = game_data.get_table("nwn2_icons")?;
    let row = icons_table.get_by_id(icon_id)?;
    let icon_resref = row.get("icon")?.as_ref()?;
    if icon_resref == "****" || icon_resref.is_empty() {
        return None;
    }
    Some(icon_resref.clone())
}

fn gff_struct_to_json(
    item_struct: &IndexMap<String, GffValue<'static>>,
) -> HashMap<String, JsonValue> {
    let mut map = HashMap::new();
    for (k, v) in item_struct {
        if let Some(json_val) = gff_to_json_primitive(v) {
            map.insert(k.clone(), json_val);
        }
    }
    map
}

/// Converts `json_val` to the GFF type of `existing`. `Err` carries the path
/// below `existing` that did not fit (empty when `existing` itself did not).
fn coerce_json_to_gff_type(
    json_val: &JsonValue,
    existing: GffValue<'static>,
) -> Result<GffValue<'static>, String> {
    let unsigned = || {
        json_val
            .as_u64()
            .or_else(|| json_val.as_bool().map(u64::from))
    };
    let signed = || {
        json_val
            .as_i64()
            .or_else(|| json_val.as_bool().map(i64::from))
    };
    let coerced = match existing {
        GffValue::Byte(_) => unsigned()
            .and_then(|n| u8::try_from(n).ok())
            .map(GffValue::Byte),
        GffValue::Char(_) => unsigned()
            .and_then(|n| u8::try_from(n).ok())
            .map(|n| GffValue::Char(n as char)),
        GffValue::Short(_) => signed()
            .and_then(|n| i16::try_from(n).ok())
            .map(GffValue::Short),
        GffValue::Word(_) => unsigned()
            .and_then(|n| u16::try_from(n).ok())
            .map(GffValue::Word),
        GffValue::Int(_) => signed()
            .and_then(|n| i32::try_from(n).ok())
            .map(GffValue::Int),
        GffValue::Dword(_) => unsigned()
            .and_then(|n| u32::try_from(n).ok())
            .map(GffValue::Dword),
        GffValue::Dword64(_) => unsigned().map(GffValue::Dword64),
        GffValue::Int64(_) => signed().map(GffValue::Int64),
        GffValue::Float(_) => json_val.as_f64().map(|n| GffValue::Float(n as f32)),
        GffValue::Double(_) => json_val.as_f64().map(GffValue::Double),
        GffValue::String(_) => json_val
            .as_str()
            .map(|s| GffValue::String(std::borrow::Cow::Owned(s.to_string()))),
        GffValue::ResRef(_) => json_val
            .as_str()
            .map(|s| GffValue::ResRef(std::borrow::Cow::Owned(s.to_string()))),
        GffValue::LocString(_) => json_to_locstring(json_val),
        GffValue::ListOwned(existing_list) => {
            let arr = json_val.as_array().ok_or_else(String::new)?;
            return merge_json_list_into_gff_list(&existing_list, arr).map(GffValue::ListOwned);
        }
        GffValue::StructOwned(existing_struct) => {
            return coerce_json_to_struct(json_val, *existing_struct);
        }
        GffValue::Struct(lazy) => return coerce_json_to_struct(json_val, lazy.force_load()),
        _ => json_to_gff_best_guess(json_val),
    };
    coerced.ok_or_else(String::new)
}

fn coerce_json_to_struct(
    json_val: &JsonValue,
    mut existing: IndexMap<String, GffValue<'static>>,
) -> Result<GffValue<'static>, String> {
    let obj = json_val.as_object().ok_or_else(String::new)?;
    let updates: HashMap<String, JsonValue> =
        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    merge_json_into_gff_struct(&mut existing, &updates)?;
    Ok(GffValue::StructOwned(Box::new(existing)))
}

fn json_to_locstring(json_val: &JsonValue) -> Option<GffValue<'static>> {
    let obj = json_val.as_object()?;
    let string_ref = obj.get("string_ref")?.as_i64()? as i32;
    let substrings = obj
        .get("substrings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|sub| {
                    let sub_obj = sub.as_object()?;
                    Some(crate::parsers::gff::types::LocalizedSubstring {
                        language: sub_obj.get("language")?.as_u64()? as u32,
                        gender: sub_obj.get("gender")?.as_u64()? as u32,
                        string: std::borrow::Cow::Owned(
                            sub_obj.get("string")?.as_str()?.to_string(),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Some(GffValue::LocString(
        crate::parsers::gff::types::LocalizedString {
            string_ref,
            substrings,
        },
    ))
}

fn json_to_list(json_val: &JsonValue) -> Option<GffValue<'static>> {
    let arr = json_val.as_array()?;
    let structs: Vec<IndexMap<String, GffValue<'static>>> = arr
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let mut map = IndexMap::new();
            for (k, v) in obj {
                if let Some(gff_val) = json_to_gff_best_guess(v) {
                    map.insert(k.clone(), gff_val);
                }
            }
            Some(map)
        })
        .collect();
    Some(GffValue::ListOwned(structs))
}

fn json_to_struct_owned(json_val: &JsonValue) -> Option<GffValue<'static>> {
    let obj = json_val.as_object()?;
    let mut map = IndexMap::new();
    for (k, v) in obj {
        if let Some(gff_val) = json_to_gff_best_guess(v) {
            map.insert(k.clone(), gff_val);
        }
    }
    Some(GffValue::StructOwned(Box::new(map)))
}

fn json_to_gff_best_guess(json_val: &JsonValue) -> Option<GffValue<'static>> {
    match json_val {
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(GffValue::Int(i as i32))
            } else {
                n.as_f64().map(|f| GffValue::Float(f as f32))
            }
        }
        JsonValue::String(s) => Some(GffValue::String(std::borrow::Cow::Owned(s.clone()))),
        JsonValue::Bool(b) => Some(GffValue::Byte(u8::from(*b))),
        JsonValue::Array(_) => json_to_list(json_val),
        JsonValue::Object(obj) => {
            if obj.contains_key("string_ref") {
                json_to_locstring(json_val)
            } else {
                json_to_struct_owned(json_val)
            }
        }
        JsonValue::Null => None,
    }
}

fn write_appearance_into_item(
    item: &mut IndexMap<String, GffValue<'static>>,
    appearance: &crate::character::ItemAppearance,
    create_variation: bool,
) {
    // Preserve the existing numeric type; new fields are Bytes like the game
    // writes them (Variation, ModelPartN, ArmorVisualType).
    let typed_byte = |existing: Option<&GffValue<'static>>, v: i32| -> GffValue<'static> {
        match existing {
            Some(GffValue::Int(_)) => GffValue::Int(v),
            Some(GffValue::Short(_)) => GffValue::Short(v as i16),
            Some(GffValue::Word(_)) => GffValue::Word(v as u16),
            _ => GffValue::Byte(v as u8),
        }
    };

    // Only write a numeric appearance field when it already existed in the
    // GFF or the new value is nonzero. Base-items-tab adds create an item
    // with no ModelPartN / Variation fields; unconditionally writing 0 into
    // them tells the engine "render model variant 0" (empty) instead of
    // falling back to the base item default.
    let write_num = |item: &mut IndexMap<String, GffValue<'static>>,
                     key: &str,
                     value: i32,
                     typed: GffValue<'static>| {
        if value != 0 || item.contains_key(key) {
            item.insert(key.to_string(), typed);
        }
    };

    let new_variation = typed_byte(item.get("Variation"), appearance.variation);
    let new_mp1 = typed_byte(item.get("ModelPart1"), appearance.model_parts[0]);
    let new_mp2 = typed_byte(item.get("ModelPart2"), appearance.model_parts[1]);
    let new_mp3 = typed_byte(item.get("ModelPart3"), appearance.model_parts[2]);

    // SinglePart items (shields etc.) store their variant in ModelPart1; the
    // UI-side `variation` is only a mirror of it, so never create a Variation
    // field on them — an existing one is still kept in sync.
    if create_variation || item.contains_key("Variation") {
        write_num(item, "Variation", appearance.variation, new_variation);
    }
    write_num(item, "ModelPart1", appearance.model_parts[0], new_mp1);
    write_num(item, "ModelPart2", appearance.model_parts[1], new_mp2);
    write_num(item, "ModelPart3", appearance.model_parts[2], new_mp3);

    // ArmorVisualType picks the armor material (armor.2da row); the game
    // writes it as a Byte. `from_gff` only reports it when the item carried
    // one, and the material select only sets it on armor-type items.
    if let Some(vt) = appearance.armor_visual_type {
        item.insert(
            "ArmorVisualType".to_string(),
            typed_byte(item.get("ArmorVisualType"), vt),
        );
    }

    // Tintable: same rule — skip if absent and all channels are (0,0,0).
    // Alpha isn't checked to match `read_item_tint_with_fallback`'s own
    // "has color" heuristic, so round-trip of a template-backed item with
    // non-default alpha but zero RGB stays stable either way.
    let tints_all_zero = [
        &appearance.tints.channel1,
        &appearance.tints.channel2,
        &appearance.tints.channel3,
    ]
    .iter()
    .all(|ch| ch.r == 0 && ch.g == 0 && ch.b == 0);
    if !tints_all_zero || item.contains_key("Tintable") {
        let tint = crate::character::appearance_helpers::build_tint_struct(&appearance.tints);
        let mut tintable_outer = IndexMap::new();
        tintable_outer.insert("Tint".to_string(), GffValue::StructOwned(Box::new(tint)));
        item.insert(
            "Tintable".to_string(),
            GffValue::StructOwned(Box::new(tintable_outer)),
        );
    }

    write_accessories_into_item(item, &appearance.accessories);
}

/// Write the 22 `AC*` accessory structs of a chest item. `None` slots are
/// left untouched (the field stays as it was); `Some` slots get their
/// `Accessory` id and `Tintable` written into the existing struct, so the
/// toolset's `UVScroll` and struct id survive. A slot the item never had is
/// cloned from a sibling `AC*` struct so the new one matches the item's own
/// layout, falling back to a bare `Accessory` + `Tintable` pair.
fn write_accessories_into_item(
    item: &mut IndexMap<String, GffValue<'static>>,
    accessories: &crate::character::ArmorAccessories,
) {
    use crate::character::AccessorySlot;
    use crate::character::appearance_helpers::build_tintable_struct;

    fn load_struct(value: &GffValue<'static>) -> Option<IndexMap<String, GffValue<'static>>> {
        match value {
            GffValue::StructOwned(s) => Some(s.as_ref().clone()),
            GffValue::Struct(lazy) => Some(lazy.force_load()),
            _ => None,
        }
    }
    fn sibling_template(
        item: &IndexMap<String, GffValue<'static>>,
        slot: AccessorySlot,
    ) -> IndexMap<String, GffValue<'static>> {
        let mut inner = AccessorySlot::all()
            .iter()
            .find_map(|s| item.get(s.uti_field()).and_then(load_struct))
            .unwrap_or_default();
        inner.insert(
            "__struct_id__".to_string(),
            GffValue::Dword(slot.struct_id()),
        );
        inner
    }

    for slot in AccessorySlot::all() {
        let Some(data) = accessories.get_data(*slot) else {
            continue;
        };
        let key = slot.uti_field();
        let mut inner = match item.get(key).and_then(load_struct) {
            Some(existing) => existing,
            None => sibling_template(item, *slot),
        };
        let accessory = match inner.get("Accessory") {
            Some(GffValue::Int(_)) => GffValue::Int(data.id),
            Some(GffValue::Short(_)) => GffValue::Short(data.id as i16),
            Some(GffValue::Word(_)) => GffValue::Word(data.id as u16),
            _ => GffValue::Byte(data.id as u8),
        };
        inner.insert("Accessory".to_string(), accessory);
        let t = &data.tints;
        let tints_all_zero = [&t.channel1, &t.channel2, &t.channel3]
            .iter()
            .all(|ch| ch.r == 0 && ch.g == 0 && ch.b == 0);
        if !tints_all_zero || inner.contains_key("Tintable") {
            inner.insert(
                "Tintable".to_string(),
                GffValue::StructOwned(Box::new(build_tintable_struct(t))),
            );
        }
        item.insert(key.to_string(), GffValue::StructOwned(Box::new(inner)));
    }
}

/// `merge_json_into_gff_struct` preserves `Identified`'s existing GFF type when
/// the field is already on the item, but creates a brand new field as
/// `GffValue::Int` (the JSON-number merge default) when the item never had
/// one. The engine only honours a Byte there, so rewrite just that
/// freshly-created case.
fn normalize_identified_field(
    item: &mut IndexMap<String, GffValue<'static>>,
    existed_before_merge: bool,
) {
    if existed_before_merge {
        return;
    }
    if let Some(value) = item.get("Identified").and_then(gff_value_to_i32) {
        item.insert(
            "Identified".to_string(),
            GffValue::Byte(value.clamp(0, 1) as u8),
        );
    }
}

/// Enforce NWN2 GFF schema types on every entry of an item's PropertiesList.
///
/// The frontend sends new property entries as generic JSON, which the merge path
/// coerces into GffValue::Int for numeric fields when no existing entry exists.
/// The engine reads the GFF with per-field type tags and silently drops properties
/// whose types don't match the expected schema on the next load, so every numeric
/// field must be re-typed to Byte/Word before writing.
fn normalize_item_properties_list(item: &mut IndexMap<String, GffValue<'static>>) {
    if let Some(GffValue::ListOwned(entries)) = item.get_mut("PropertiesList") {
        for entry in entries {
            normalize_item_property_entry(entry);
        }
    }
}

fn normalize_item_property_entry(prop: &mut IndexMap<String, GffValue<'static>>) {
    prop.shift_remove("ChancesAppear");
    prop.shift_remove("SpellID");

    const WORD_FIELDS: &[&str] = &["PropertyName", "Subtype", "CostValue"];
    const BYTE_FIELDS: &[&str] = &[
        "CostTable",
        "Param1",
        "Param1Value",
        "ChanceAppear",
        "Useable",
        "UsesPerDay",
    ];

    for key in WORD_FIELDS {
        if let Some(slot) = prop.get_mut(*key)
            && let Some(n) = gff_value_to_i32(slot)
        {
            *slot = GffValue::Word(n as u16);
        }
    }
    for key in BYTE_FIELDS {
        if let Some(slot) = prop.get_mut(*key)
            && let Some(n) = gff_value_to_i32(slot)
        {
            *slot = GffValue::Byte(n as u8);
        }
    }
}

/// Identity tuple for the GFF fields that define what an item property "is"
/// (excludes ChanceAppear/Useable/UsesPerDay, which are usage metadata, not
/// identity).
fn property_identity(prop: &IndexMap<String, GffValue<'static>>) -> (i32, i32, i32, i32, i32, i32) {
    let get = |key: &str| prop.get(key).and_then(gff_value_to_i32).unwrap_or(-1);
    (
        get("PropertyName"),
        get("Subtype"),
        get("CostTable"),
        get("CostValue"),
        get("Param1"),
        get("Param1Value"),
    )
}

fn property_lists_match(
    a: &[IndexMap<String, GffValue<'static>>],
    b: &[IndexMap<String, GffValue<'static>>],
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut a_ids: Vec<_> = a.iter().map(property_identity).collect();
    let mut b_ids: Vec<_> = b.iter().map(property_identity).collect();
    a_ids.sort_unstable();
    b_ids.sort_unstable();
    a_ids == b_ids
}

/// Item value as the game shows it: computed from the 2DAs, falling back to the
/// stored `Cost + ModifyCost` only when the base item is unknown.
fn item_value(item_struct: &IndexMap<String, GffValue<'static>>, game_data: &GameData) -> i32 {
    ItemCostCalculator::new()
        .calculate_item_cost(item_struct, game_data)
        .map_or_else(
            || {
                let cost = item_struct
                    .get("Cost")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                let modify_cost = item_struct
                    .get("ModifyCost")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                cost + modify_cost
            },
            |v| v as i32,
        )
}

/// Blueprints are loot templates: 65% of vanilla `.uti` files ship
/// `Identified = 0` so the game can hand them out unidentified. An item the
/// user adds or transforms in the editor is an explicit choice, so it is
/// stamped identified like the base-item add path always was. BYTE is the
/// type every real item uses for the field.
fn mark_identified(item: &mut IndexMap<String, GffValue<'static>>) {
    item.insert("Identified".to_string(), GffValue::Byte(1));
}

/// `StackSize` on a real .uti/.bic item is a Word. Preserve the field's existing
/// GFF type when replacing a value already on an item (the engine is
/// type-strict), and default to Word when there is nothing to preserve.
pub(crate) fn typed_stack_size(
    existing: Option<&GffValue<'static>>,
    value: i32,
) -> GffValue<'static> {
    match existing {
        Some(GffValue::Byte(_)) => GffValue::Byte(value as u8),
        Some(GffValue::Short(_)) => GffValue::Short(value as i16),
        Some(GffValue::Int(_)) => GffValue::Int(value),
        Some(GffValue::Dword(_)) => GffValue::Dword(value as u32),
        _ => GffValue::Word(value as u16),
    }
}

/// Strict stack-merge identity: same BaseItem, same Tag (empty matches empty),
/// and equivalent PropertiesList. Many unrelated consumables share a BaseItem
/// row (all potions/drinks are one row, all crafting essences another), so
/// matching on BaseItem alone would merge a healing potion onto an ale stack.
fn item_stack_identity_matches(
    existing: &IndexMap<String, GffValue<'static>>,
    base_item_id: i32,
    tag: &str,
    properties: &[IndexMap<String, GffValue<'static>>],
) -> bool {
    let existing_base = existing
        .get("BaseItem")
        .and_then(gff_value_to_i32)
        .unwrap_or(0);
    if existing_base != base_item_id {
        return false;
    }

    let existing_tag = existing
        .get("Tag")
        .and_then(gff_value_to_string)
        .unwrap_or_default();
    if existing_tag != tag {
        return false;
    }

    let existing_props = extract_list_from_map(existing, "PropertiesList").unwrap_or_default();
    property_lists_match(&existing_props, properties)
}

fn invalid_item_field(path: String) -> CharacterError {
    CharacterError::ValidationFailed {
        field: "item_data",
        message: format!("{path} has a value that does not fit its field type"),
    }
}

/// Tops up the inventory stacks matching `base_item_id`, `tag` and `properties`
/// to `max_stack`. Returns the units that did not fit and the last stack topped up.
fn fill_matching_stacks(
    inv_list: &mut [IndexMap<String, GffValue<'static>>],
    base_item_id: i32,
    tag: &str,
    properties: &[IndexMap<String, GffValue<'static>>],
    max_stack: i32,
    mut count: i32,
) -> (i32, Option<usize>) {
    let mut last_index = None;
    for (index, existing) in inv_list.iter_mut().enumerate() {
        if count == 0 {
            break;
        }
        if !item_stack_identity_matches(existing, base_item_id, tag, properties) {
            continue;
        }
        let current = existing
            .get("StackSize")
            .and_then(gff_value_to_i32)
            .unwrap_or(1);
        let added = (max_stack - current).clamp(0, count);
        if added == 0 {
            continue;
        }
        let value = typed_stack_size(existing.get("StackSize"), current + added);
        existing.insert("StackSize".to_string(), value);
        count -= added;
        last_index = Some(index);
    }
    (count, last_index)
}

/// Merges frontend JSON into an item struct, keeping each existing field's GFF
/// type. `Err` names the field path whose value does not fit its type; the
/// struct may then be partially merged and must be discarded.
fn merge_json_into_gff_struct(
    existing: &mut IndexMap<String, GffValue<'static>>,
    updates: &HashMap<String, JsonValue>,
) -> Result<(), String> {
    for (key, json_val) in updates {
        if key == "__struct_id__" {
            continue;
        }

        let new_val = match existing.get(key) {
            Some(existing_val) => {
                coerce_json_to_gff_type(json_val, existing_val.clone()).map_err(|path| {
                    if path.is_empty() {
                        key.clone()
                    } else if path.starts_with('[') {
                        format!("{key}{path}")
                    } else {
                        format!("{key}.{path}")
                    }
                })?
            }
            None => match json_to_gff_best_guess(json_val) {
                Some(val) => val,
                None => continue,
            },
        };
        existing.insert(key.clone(), new_val);
    }
    Ok(())
}

fn merge_json_list_into_gff_list(
    existing_list: &[IndexMap<String, GffValue<'static>>],
    json_arr: &[JsonValue],
) -> Result<Vec<IndexMap<String, GffValue<'static>>>, String> {
    json_arr
        .iter()
        .enumerate()
        .map(|(i, json_item)| {
            let obj = json_item.as_object().ok_or_else(|| format!("[{i}]"))?;
            let updates: HashMap<String, JsonValue> =
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

            let mut item = existing_list.get(i).cloned().unwrap_or_default();
            merge_json_into_gff_struct(&mut item, &updates)
                .map_err(|path| format!("[{i}].{path}"))?;
            Ok(item)
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::character::Character;
    use std::borrow::Cow;

    #[test]
    fn test_normalize_item_property_entry_types_and_aliases() {
        // Simulate what merge_json_into_gff_struct produces for a brand-new property
        // added from the frontend: every numeric field is coerced to GffValue::Int,
        // and the frontend typo ChancesAppear leaks through.
        let mut prop = IndexMap::new();
        prop.insert("PropertyName".to_string(), GffValue::Int(52));
        prop.insert("Subtype".to_string(), GffValue::Int(21));
        prop.insert("CostTable".to_string(), GffValue::Int(25));
        prop.insert("CostValue".to_string(), GffValue::Int(8));
        prop.insert("Param1".to_string(), GffValue::Int(255));
        prop.insert("Param1Value".to_string(), GffValue::Int(0));
        prop.insert("ChancesAppear".to_string(), GffValue::Int(100));
        prop.insert("Useable".to_string(), GffValue::Byte(1));
        prop.insert("UsesPerDay".to_string(), GffValue::Int(255));
        prop.insert("SpellID".to_string(), GffValue::Int(65535));

        normalize_item_property_entry(&mut prop);

        assert!(matches!(prop.get("PropertyName"), Some(GffValue::Word(52))));
        assert!(matches!(prop.get("Subtype"), Some(GffValue::Word(21))));
        assert!(matches!(prop.get("CostValue"), Some(GffValue::Word(8))));
        assert!(matches!(prop.get("CostTable"), Some(GffValue::Byte(25))));
        assert!(matches!(prop.get("Param1"), Some(GffValue::Byte(255))));
        assert!(matches!(prop.get("Param1Value"), Some(GffValue::Byte(0))));
        assert!(prop.get("ChanceAppear").is_none());
        assert!(prop.get("ChancesAppear").is_none());
        assert!(matches!(prop.get("Useable"), Some(GffValue::Byte(1))));
        assert!(matches!(prop.get("UsesPerDay"), Some(GffValue::Byte(255))));
        assert!(prop.get("SpellID").is_none());
    }

    #[test]
    fn test_normalize_item_property_entry_fixes_chance_appear_typo() {
        let mut prop = IndexMap::new();
        prop.insert("ChanceAppear".to_string(), GffValue::Int(100));
        normalize_item_property_entry(&mut prop);
        assert!(matches!(
            prop.get("ChanceAppear"),
            Some(GffValue::Byte(100))
        ));
    }

    #[test]
    fn test_normalize_item_properties_list_walks_all_entries() {
        let mut item = IndexMap::new();
        let mut entry1 = IndexMap::new();
        entry1.insert("PropertyName".to_string(), GffValue::Int(1));
        entry1.insert("ChanceAppear".to_string(), GffValue::Int(100));
        let mut entry2 = IndexMap::new();
        entry2.insert("PropertyName".to_string(), GffValue::Int(2));
        entry2.insert("ChanceAppear".to_string(), GffValue::Int(50));
        item.insert(
            "PropertiesList".to_string(),
            GffValue::ListOwned(vec![entry1, entry2]),
        );

        normalize_item_properties_list(&mut item);

        let GffValue::ListOwned(entries) = item.get("PropertiesList").expect("list missing") else {
            panic!("PropertiesList not ListOwned");
        };
        assert_eq!(entries.len(), 2);
        assert!(matches!(
            entries[0].get("PropertyName"),
            Some(GffValue::Word(1))
        ));
        assert!(matches!(
            entries[1].get("PropertyName"),
            Some(GffValue::Word(2))
        ));
        assert!(matches!(
            entries[0].get("ChanceAppear"),
            Some(GffValue::Byte(100))
        ));
        assert!(matches!(
            entries[1].get("ChanceAppear"),
            Some(GffValue::Byte(50))
        ));
    }

    #[test]
    fn test_coerce_nested_struct_preserves_byte_type() {
        // Build an existing Tintable like the one loaded from GFF:
        //   Tintable.Tint."1".{r,g,b,a}: Byte
        let mut channel = IndexMap::new();
        channel.insert("r".to_string(), GffValue::Byte(0));
        channel.insert("g".to_string(), GffValue::Byte(0));
        channel.insert("b".to_string(), GffValue::Byte(0));
        channel.insert("a".to_string(), GffValue::Byte(0));

        let mut tint = IndexMap::new();
        tint.insert(
            "1".to_string(),
            GffValue::StructOwned(Box::new(channel.clone())),
        );

        let mut tintable = IndexMap::new();
        tintable.insert("Tint".to_string(), GffValue::StructOwned(Box::new(tint)));

        let mut existing = IndexMap::new();
        existing.insert(
            "Tintable".to_string(),
            GffValue::StructOwned(Box::new(tintable)),
        );

        // Frontend-shaped update
        let updates: HashMap<String, JsonValue> = [(
            "Tintable".to_string(),
            serde_json::json!({ "Tint": { "1": { "r": 200, "g": 100, "b": 50, "a": 0 } } }),
        )]
        .into_iter()
        .collect();

        merge_json_into_gff_struct(&mut existing, &updates).unwrap();

        // Walk back down and assert the numbers stayed Byte-typed
        let tintable_v = existing.get("Tintable").expect("Tintable missing");
        let tintable_map = match tintable_v {
            GffValue::StructOwned(m) => m,
            _ => panic!("Tintable not StructOwned"),
        };
        let tint_v = tintable_map.get("Tint").expect("Tint missing");
        let tint_map = match tint_v {
            GffValue::StructOwned(m) => m,
            _ => panic!("Tint not StructOwned"),
        };
        let ch1_v = tint_map.get("1").expect("Channel 1 missing");
        let ch1 = match ch1_v {
            GffValue::StructOwned(m) => m,
            _ => panic!("Channel 1 not StructOwned"),
        };
        assert!(
            matches!(ch1.get("r"), Some(GffValue::Byte(200))),
            "r not Byte(200): {:?}",
            ch1.get("r")
        );
        assert!(matches!(ch1.get("g"), Some(GffValue::Byte(100))));
        assert!(matches!(ch1.get("b"), Some(GffValue::Byte(50))));
        assert!(matches!(ch1.get("a"), Some(GffValue::Byte(0))));
    }

    fn create_test_fields() -> IndexMap<String, GffValue<'static>> {
        let mut fields = IndexMap::new();
        fields.insert("Gold".to_string(), GffValue::Dword(1000));

        let item1 = {
            let mut item = IndexMap::new();
            item.insert(
                "Tag".to_string(),
                GffValue::String(Cow::Owned("test_sword".to_string())),
            );
            item.insert("BaseItem".to_string(), GffValue::Int(42));
            item.insert("StackSize".to_string(), GffValue::Short(1));
            item.insert("Identified".to_string(), GffValue::Byte(1));
            item
        };

        let item2 = {
            let mut item = IndexMap::new();
            item.insert(
                "Tag".to_string(),
                GffValue::String(Cow::Owned("potion".to_string())),
            );
            item.insert("BaseItem".to_string(), GffValue::Int(100));
            item.insert("StackSize".to_string(), GffValue::Short(5));
            item.insert("Identified".to_string(), GffValue::Byte(0));
            item
        };

        fields.insert(
            "ItemList".to_string(),
            GffValue::ListOwned(vec![item1, item2]),
        );

        let equipped_item = {
            let mut item = IndexMap::new();
            item.insert("__struct_id__".to_string(), GffValue::Word(0x0010));
            item.insert(
                "Tag".to_string(),
                GffValue::String(Cow::Owned("equipped_sword".to_string())),
            );
            item.insert("BaseItem".to_string(), GffValue::Int(50));
            item.insert("StackSize".to_string(), GffValue::Short(1));
            item.insert("Identified".to_string(), GffValue::Byte(1));
            item
        };

        let empty_slot = {
            let mut item = IndexMap::new();
            item.insert("__struct_id__".to_string(), GffValue::Word(0x0000));
            item
        };

        fields.insert(
            "Equip_ItemList".to_string(),
            GffValue::ListOwned(vec![equipped_item, empty_slot]),
        );

        fields
    }

    #[test]
    fn test_gold() {
        let character = Character::from_gff(create_test_fields());
        assert_eq!(character.gold(), 1000);
    }

    #[test]
    fn test_set_gold() {
        let mut character = Character::from_gff(create_test_fields());
        character.set_gold(500);
        assert_eq!(character.gold(), 500);
        assert!(character.is_modified());
    }

    #[test]
    fn test_inventory_count() {
        let character = Character::from_gff(create_test_fields());
        assert_eq!(character.inventory_count(), 2);
    }

    #[test]
    fn test_equipped_count() {
        let character = Character::from_gff(create_test_fields());
        assert_eq!(character.equipped_count(), 1);
    }

    #[test]
    fn test_inventory_items() {
        let character = Character::from_gff(create_test_fields());
        let items = character.inventory_items();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].tag, "test_sword");
        assert_eq!(items[0].base_item, 42);
        assert_eq!(items[0].stack_size, 1);
        assert!(items[0].identified);

        assert_eq!(items[1].tag, "potion");
        assert_eq!(items[1].base_item, 100);
        assert_eq!(items[1].stack_size, 5);
        assert!(!items[1].identified);
    }

    #[test]
    fn test_equipped_items() {
        let character = Character::from_gff(create_test_fields());
        let items = character.equipped_items();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, 0);
        assert_eq!(items[0].1.tag, "equipped_sword");
        assert_eq!(items[0].1.base_item, 50);
        assert_eq!(items[0].1.stack_size, 1);
        assert!(items[0].1.identified);
    }

    #[test]
    fn test_empty_inventory() {
        let fields = IndexMap::new();
        let character = Character::from_gff(fields);

        assert_eq!(character.gold(), 0);
        assert_eq!(character.inventory_count(), 0);
        assert_eq!(character.equipped_count(), 0);
        assert!(character.inventory_items().is_empty());
        assert!(character.equipped_items().is_empty());
    }

    /// GameData whose baseitems row 0 allows stacks of 10, so the stack-merge
    /// branch in add_item/add_item_from_struct is reachable.
    fn game_data_with_stackable_baseitem() -> GameData {
        use crate::loaders::types::LoadedTable;
        use crate::parsers::tda::TDAParser;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let mut game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));
        let content = "2DA V2.0

   Label    Stacking  EquipableSlots
0  Potion   10        0x0010
";
        let mut parser = TDAParser::new();
        parser.parse_from_bytes(content.as_bytes()).unwrap();
        game_data.tables.insert(
            "baseitems".to_string(),
            LoadedTable::new("baseitems".to_string(), Arc::new(parser)),
        );
        game_data
    }

    fn stackable_item(
        tag: &str,
        stack: i16,
        props: &[(u16, u16)],
    ) -> IndexMap<String, GffValue<'static>> {
        let mut item = IndexMap::new();
        item.insert("BaseItem".to_string(), GffValue::Int(0));
        item.insert(
            "Tag".to_string(),
            GffValue::String(Cow::Owned(tag.to_string())),
        );
        item.insert("StackSize".to_string(), GffValue::Short(stack));
        if !props.is_empty() {
            let list: Vec<_> = props
                .iter()
                .map(|(name, subtype)| {
                    let mut p = IndexMap::new();
                    p.insert("PropertyName".to_string(), GffValue::Word(*name));
                    p.insert("Subtype".to_string(), GffValue::Word(*subtype));
                    p
                })
                .collect();
            item.insert("PropertiesList".to_string(), GffValue::ListOwned(list));
        }
        item
    }

    fn character_with_items(items: Vec<IndexMap<String, GffValue<'static>>>) -> Character {
        let mut fields = IndexMap::new();
        fields.insert("ItemList".to_string(), GffValue::ListOwned(items));
        Character::from_gff(fields)
    }

    fn stack_size_at(character: &Character, index: usize) -> i32 {
        let item = character.get_inventory_item_at(index).unwrap();
        item.get("StackSize").and_then(gff_value_to_i32).unwrap()
    }

    #[test]
    fn test_add_item_does_not_stack_onto_tagged_item() {
        // Regression for user issue 2: adding a fresh base-type item must not
        // merge onto an unrelated tagged item sharing the same BaseItem row.
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![stackable_item("ale", 3, &[])]);

        let result = character.add_item(0, 1, None, None, &game_data).unwrap();

        assert!(!result.stacked);
        assert_eq!(character.inventory_count(), 2);
        assert_eq!(stack_size_at(&character, 0), 3);
    }

    #[test]
    fn test_add_item_never_merges_onto_matching_stack() {
        let game_data = game_data_with_stackable_baseitem();
        let mut untagged = stackable_item("", 3, &[]);
        untagged.shift_remove("Tag");
        let mut character = character_with_items(vec![untagged]);

        let result = character.add_item(0, 2, None, None, &game_data).unwrap();

        assert!(!result.stacked);
        assert_eq!(result.inventory_index, Some(1));
        assert_eq!(stack_size_at(&character, 0), 3);
        assert_eq!(stack_size_at(&character, 1), 2);
    }

    #[test]
    fn test_add_item_from_struct_does_not_stack_across_tags() {
        // The ale/healing-potion scenario: same BaseItem row, different blueprints.
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![stackable_item("ale", 3, &[])]);

        let result = character
            .add_item_from_struct(stackable_item("healing_potion", 1, &[]), &game_data)
            .unwrap();

        assert!(!result.stacked);
        assert_eq!(character.inventory_count(), 2);
        assert_eq!(stack_size_at(&character, 0), 3);
    }

    #[test]
    fn test_add_item_from_struct_stacks_on_identical_tag_and_properties() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character =
            character_with_items(vec![stackable_item("cft_ess_fire4", 2, &[(52, 21)])]);

        let result = character
            .add_item_from_struct(stackable_item("cft_ess_fire4", 1, &[(52, 21)]), &game_data)
            .unwrap();

        assert!(result.stacked);
        assert_eq!(character.inventory_count(), 1);
        assert_eq!(stack_size_at(&character, 0), 3);
    }

    #[test]
    fn test_add_item_from_struct_does_not_stack_on_differing_properties() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![stackable_item("ring", 1, &[(52, 21)])]);

        let result = character
            .add_item_from_struct(stackable_item("ring", 1, &[(52, 22)]), &game_data)
            .unwrap();

        assert!(!result.stacked);
        assert_eq!(character.inventory_count(), 2);
    }

    #[test]
    fn test_property_lists_match_is_order_independent() {
        let a = stackable_item("x", 1, &[(52, 21), (53, 1)]);
        let b = stackable_item("x", 1, &[(53, 1), (52, 21)]);
        let props_a = extract_list_from_map(&a, "PropertiesList").unwrap();
        let props_b = extract_list_from_map(&b, "PropertiesList").unwrap();
        assert!(property_lists_match(&props_a, &props_b));
    }

    #[test]
    fn test_transform_inventory_item_replaces_struct_cleanly() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());

        character
            .transform_inventory_item(0, stackable_item("new_item", 1, &[(52, 21)]), &game_data)
            .unwrap();

        assert_eq!(character.inventory_count(), 2);
        let item = character.get_inventory_item_at(0).unwrap();
        assert_eq!(
            item.get("Tag").and_then(gff_value_to_string).unwrap(),
            "new_item"
        );
        assert_eq!(item.get("Identified").and_then(gff_value_to_i32), Some(1));
    }

    #[test]
    fn test_transform_inventory_item_out_of_bounds_errors() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());
        let result =
            character.transform_inventory_item(99, stackable_item("x", 1, &[]), &game_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_transform_equipped_item_preserves_slot_bitmask() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());

        character
            .transform_equipped_item(
                EquipmentSlot::RightHand,
                stackable_item("new_blade", 1, &[]),
                &game_data,
            )
            .unwrap();

        let item = character
            .get_equipped_item_raw(EquipmentSlot::RightHand)
            .unwrap();
        assert_eq!(
            item.get("Tag").and_then(gff_value_to_string).unwrap(),
            "new_blade"
        );
        assert_eq!(
            item.get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap() as u32,
            EquipmentSlot::RightHand.to_bitmask()
        );
    }

    #[test]
    fn test_transform_equipped_item_rejects_illegal_slot() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());

        let result = character.transform_equipped_item(
            EquipmentSlot::Boots,
            stackable_item("new_boots", 1, &[]),
            &game_data,
        );

        assert!(result.is_err());
        let unchanged = character
            .get_equipped_item_raw(EquipmentSlot::RightHand)
            .unwrap();
        assert_eq!(
            unchanged.get("Tag").and_then(gff_value_to_string).unwrap(),
            "equipped_sword"
        );
    }

    #[test]
    fn test_add_item_from_struct_reads_word_stack_size() {
        // .uti templates store StackSize as a GFF Word; it must not fall back to 1.
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![]);

        let mut item = stackable_item("bundle", 1, &[]);
        item.insert("StackSize".to_string(), GffValue::Word(5));
        let result = character.add_item_from_struct(item, &game_data).unwrap();

        assert!(!result.stacked);
        assert_eq!(result.item.unwrap().stack_size, 5);
        assert_eq!(stack_size_at(&character, 0), 5);
    }

    #[test]
    fn test_add_item_from_struct_stack_merge_preserves_word_stack_size_type() {
        let game_data = game_data_with_stackable_baseitem();
        let mut existing = stackable_item("cft_ess_fire4", 2, &[(52, 21)]);
        existing.insert("StackSize".to_string(), GffValue::Word(2));
        let mut character = character_with_items(vec![existing]);

        character
            .add_item_from_struct(stackable_item("cft_ess_fire4", 1, &[(52, 21)]), &game_data)
            .unwrap();

        let stored = character.get_inventory_item_at(0).unwrap();
        assert!(
            matches!(stored.get("StackSize"), Some(GffValue::Word(3))),
            "StackSize should stay Word(3): {:?}",
            stored.get("StackSize")
        );
    }

    #[test]
    fn test_add_item_new_entry_uses_word_stack_size() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![]);

        character.add_item(0, 1, None, None, &game_data).unwrap();

        let stored = character.get_inventory_item_at(0).unwrap();
        assert!(
            matches!(stored.get("StackSize"), Some(GffValue::Word(1))),
            "new StackSize should be Word(1): {:?}",
            stored.get("StackSize")
        );
    }

    #[test]
    fn test_transform_inventory_item_clamps_stack_to_new_base_item() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());

        character
            .transform_inventory_item(0, stackable_item("bundle", 50, &[]), &game_data)
            .unwrap();

        // game_data_with_stackable_baseitem's only row (BaseItem 0) caps at 10.
        assert_eq!(stack_size_at(&character, 0), 10);
    }

    #[test]
    fn test_add_item_from_struct_overflow_fills_stack_then_adds_remainder() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![stackable_item("arrow", 8, &[])]);

        let result = character
            .add_item_from_struct(stackable_item("arrow", 5, &[]), &game_data)
            .unwrap();

        assert!(!result.stacked);
        assert_eq!(result.item.unwrap().stack_size, 3);
        assert_eq!(stack_size_at(&character, 0), 10);
        assert_eq!(stack_size_at(&character, 1), 3);
    }

    #[test]
    fn test_add_item_bounds_stack_size() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![]);

        character.add_item(0, 0, None, None, &game_data).unwrap();
        character.add_item(0, -5, None, None, &game_data).unwrap();
        character.add_item(0, 50, None, None, &game_data).unwrap();

        assert_eq!(stack_size_at(&character, 0), 1);
        assert_eq!(stack_size_at(&character, 1), 1);
        assert_eq!(stack_size_at(&character, 2), 10);
    }

    #[test]
    fn test_clamp_keeps_stack_of_unknown_base_item() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());
        let mut item = stackable_item("custom_ammo", 50, &[]);
        item.insert("BaseItem".to_string(), GffValue::Int(9999));

        character
            .transform_inventory_item(0, item, &game_data)
            .unwrap();

        assert_eq!(stack_size_at(&character, 0), 50);
    }

    #[test]
    fn test_clamp_preserves_dword_stack_size() {
        let game_data = game_data_with_stackable_baseitem();
        let mut character = Character::from_gff(create_test_fields());
        let mut item = stackable_item("bundle", 1, &[]);
        item.insert("StackSize".to_string(), GffValue::Dword(50));

        character
            .transform_inventory_item(0, item, &game_data)
            .unwrap();

        let stored = character.get_inventory_item_at(0).unwrap();
        assert!(matches!(stored.get("StackSize"), Some(GffValue::Dword(10))));
    }

    #[test]
    fn test_update_inventory_item_rejects_value_that_does_not_fit() {
        let game_data = game_data_with_stackable_baseitem();
        let mut item = stackable_item("arrow", 5, &[(1, 0)]);
        item.insert("StackSize".to_string(), GffValue::Word(5));
        let mut character = character_with_items(vec![item]);

        for bad in [
            serde_json::json!({ "StackSize": -1 }),
            serde_json::json!({ "StackSize": 70000 }),
            serde_json::json!({ "StackSize": null }),
            serde_json::json!({ "PropertiesList": [{ "Subtype": "x" }] }),
        ] {
            let updates: HashMap<String, JsonValue> = serde_json::from_value(bad).unwrap();
            let err = character
                .update_inventory_item(0, &updates, &game_data)
                .unwrap_err();
            assert!(matches!(err, CharacterError::ValidationFailed { .. }));
        }

        let stored = character.get_inventory_item_at(0).unwrap();
        assert!(matches!(stored.get("StackSize"), Some(GffValue::Word(5))));
    }

    #[test]
    fn test_merge_accepts_bool_for_byte_flag() {
        let mut item = IndexMap::new();
        item.insert("Plot".to_string(), GffValue::Byte(0));
        let updates: HashMap<String, JsonValue> =
            serde_json::from_value(serde_json::json!({ "Plot": true })).unwrap();

        merge_json_into_gff_struct(&mut item, &updates).unwrap();

        assert!(matches!(item.get("Plot"), Some(GffValue::Byte(1))));
    }

    #[test]
    fn test_armor_weight_comes_from_armorrulestats() {
        use crate::loaders::types::LoadedTable;
        use crate::parsers::tda::TDAParser;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let mut game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));
        for (name, content) in [
            (
                "baseitems",
                "2DA V2.0

   Label  TenthLBS
0  Ring  1
16  Armor  100
",
            ),
            (
                "armorrulestats",
                "2DA V2.0

   Label  ACBONUS  WEIGHT
0  Cloth  0  100
1  Plate  8  500
",
            ),
        ] {
            let mut parser = TDAParser::new();
            parser.parse_from_bytes(content.as_bytes()).unwrap();
            game_data.tables.insert(
                name.to_string(),
                LoadedTable::new(name.to_string(), Arc::new(parser)),
            );
        }

        let mut armor = IndexMap::new();
        armor.insert("BaseItem".to_string(), GffValue::Int(16));
        armor.insert("ArmorRulesType".to_string(), GffValue::Byte(1));
        let mut ring = IndexMap::new();
        ring.insert("BaseItem".to_string(), GffValue::Int(0));
        ring.insert("ArmorRulesType".to_string(), GffValue::Byte(1));

        assert_eq!(item_unit_weight(&armor, &game_data), 50.0);
        assert_eq!(item_unit_weight(&ring, &game_data), 0.1);
    }

    #[test]
    fn test_update_inventory_item_identified_absent_field_normalizes_to_byte() {
        // Item never had an Identified field in the GFF; the merge creates one
        // as an Int by default. The engine only honours a Byte there.
        let mut item = stackable_item("unidentified_ring", 1, &[]);
        item.shift_remove("Identified");
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![item]);

        let updates: HashMap<String, JsonValue> =
            [("Identified".to_string(), serde_json::json!(0))]
                .into_iter()
                .collect();
        character
            .update_inventory_item(0, &updates, &game_data)
            .unwrap();

        let stored = character.get_inventory_item_at(0).unwrap();
        assert!(
            matches!(stored.get("Identified"), Some(GffValue::Byte(0))),
            "Identified should be Byte(0): {:?}",
            stored.get("Identified")
        );
    }

    #[test]
    fn test_update_inventory_item_identified_existing_field_preserves_variant() {
        // If the item's GFF already had an Identified field, the update must
        // keep its existing variant instead of retyping it.
        let mut item = stackable_item("weird_ring", 1, &[]);
        item.insert("Identified".to_string(), GffValue::Int(1));
        let game_data = game_data_with_stackable_baseitem();
        let mut character = character_with_items(vec![item]);

        let updates: HashMap<String, JsonValue> =
            [("Identified".to_string(), serde_json::json!(0))]
                .into_iter()
                .collect();
        character
            .update_inventory_item(0, &updates, &game_data)
            .unwrap();

        let stored = character.get_inventory_item_at(0).unwrap();
        assert!(
            matches!(stored.get("Identified"), Some(GffValue::Int(0))),
            "Identified should stay Int(0): {:?}",
            stored.get("Identified")
        );
    }

    #[test]
    fn test_update_equipped_item_identified_absent_field_normalizes_to_byte() {
        let game_data = game_data_with_stackable_baseitem();

        let mut equipped_item = IndexMap::new();
        equipped_item.insert(
            "__struct_id__".to_string(),
            GffValue::Word(EquipmentSlot::RightHand.to_bitmask() as u16),
        );
        equipped_item.insert("BaseItem".to_string(), GffValue::Int(0));
        equipped_item.insert(
            "Tag".to_string(),
            GffValue::String(Cow::Owned("unidentified_blade".to_string())),
        );
        equipped_item.insert("StackSize".to_string(), GffValue::Short(1));

        let mut fields = IndexMap::new();
        fields.insert(
            "Equip_ItemList".to_string(),
            GffValue::ListOwned(vec![equipped_item]),
        );
        let mut character = Character::from_gff(fields);

        assert!(
            character
                .get_equipped_item_raw(EquipmentSlot::RightHand)
                .unwrap()
                .get("Identified")
                .is_none(),
            "test fixture must start without an Identified field"
        );

        let updates: HashMap<String, JsonValue> =
            [("Identified".to_string(), serde_json::json!(0))]
                .into_iter()
                .collect();
        character
            .update_equipped_item(EquipmentSlot::RightHand, &updates, &game_data)
            .unwrap();

        let stored = character
            .get_equipped_item_raw(EquipmentSlot::RightHand)
            .unwrap();
        assert!(
            matches!(stored.get("Identified"), Some(GffValue::Byte(0))),
            "Identified should be Byte(0): {:?}",
            stored.get("Identified")
        );
    }

    fn appearance_with_variation(variation: i32) -> crate::character::ItemAppearance {
        crate::character::ItemAppearance {
            variation,
            model_parts: [variation, 0, 0],
            tints: crate::character::TintChannels::default(),
            armor_visual_type: None,
            boots: None,
            gloves: None,
            accessories: crate::character::ArmorAccessories::default(),
        }
    }

    #[test]
    fn write_appearance_does_not_inject_variation_on_single_part_items() {
        let mut item = IndexMap::new();
        item.insert("ModelPart1".to_string(), GffValue::Byte(2));

        write_appearance_into_item(&mut item, &appearance_with_variation(3), false);

        assert!(
            item.get("Variation").is_none(),
            "round-trip must not create a Variation field the item never had"
        );
        assert!(matches!(item.get("ModelPart1"), Some(GffValue::Byte(3))));
        assert!(
            item.get("Tintable").is_none(),
            "all-zero tints must not materialise a Tintable struct"
        );
    }

    #[test]
    fn write_appearance_keeps_existing_variation_in_sync() {
        let mut item = IndexMap::new();
        item.insert("Variation".to_string(), GffValue::Int(2));
        item.insert("ModelPart1".to_string(), GffValue::Byte(2));

        write_appearance_into_item(&mut item, &appearance_with_variation(3), false);

        assert!(matches!(item.get("Variation"), Some(GffValue::Int(3))));
        assert!(matches!(item.get("ModelPart1"), Some(GffValue::Byte(3))));
    }

    #[test]
    fn write_appearance_writes_armor_visual_type_preserving_type() {
        let mut appearance = appearance_with_variation(2);
        appearance.armor_visual_type = Some(4);

        let mut fresh = IndexMap::new();
        write_appearance_into_item(&mut fresh, &appearance, true);
        assert!(
            matches!(fresh.get("ArmorVisualType"), Some(GffValue::Byte(4))),
            "new field must use the game's Byte type: {:?}",
            fresh.get("ArmorVisualType")
        );

        let mut legacy = IndexMap::new();
        legacy.insert("ArmorVisualType".to_string(), GffValue::Int(1));
        write_appearance_into_item(&mut legacy, &appearance, true);
        assert!(matches!(
            legacy.get("ArmorVisualType"),
            Some(GffValue::Int(4))
        ));

        appearance.armor_visual_type = None;
        let mut untouched = IndexMap::new();
        write_appearance_into_item(&mut untouched, &appearance, true);
        assert!(untouched.get("ArmorVisualType").is_none());
    }

    #[test]
    fn write_appearance_creates_variation_for_variation_items() {
        let mut item = IndexMap::new();
        let appearance = crate::character::ItemAppearance {
            variation: 4,
            model_parts: [0, 0, 0],
            tints: crate::character::TintChannels::default(),
            armor_visual_type: None,
            boots: None,
            gloves: None,
            accessories: crate::character::ArmorAccessories::default(),
        };

        write_appearance_into_item(&mut item, &appearance, true);

        assert!(matches!(item.get("Variation"), Some(GffValue::Byte(4))));
        assert!(item.get("ModelPart1").is_none());
    }

    fn ac_struct(id: u8, tintable: bool) -> GffValue<'static> {
        let mut inner = IndexMap::new();
        inner.insert("__struct_id__".to_string(), GffValue::Dword(0));
        inner.insert("Accessory".to_string(), GffValue::Byte(id));
        if tintable {
            let tints = crate::character::TintChannels::default();
            inner.insert(
                "Tintable".to_string(),
                GffValue::StructOwned(Box::new(
                    crate::character::appearance_helpers::build_tintable_struct(&tints),
                )),
            );
        }
        let mut uv = IndexMap::new();
        uv.insert("Scroll".to_string(), GffValue::Byte(0));
        inner.insert("UVScroll".to_string(), GffValue::StructOwned(Box::new(uv)));
        GffValue::StructOwned(Box::new(inner))
    }

    fn struct_fields<'a>(value: &'a GffValue<'static>) -> &'a IndexMap<String, GffValue<'static>> {
        match value {
            GffValue::StructOwned(s) => s,
            other => panic!("expected struct, got {other:?}"),
        }
    }

    #[test]
    fn write_appearance_writes_accessories_into_existing_and_new_ac_structs() {
        use crate::character::{AccessorySlot, AccessorySlotData, TintChannel, TintChannels};

        let mut item = IndexMap::new();
        item.insert("ACLtShoulder".to_string(), ac_struct(3, true));
        item.insert("ACRtShoulder".to_string(), ac_struct(0, true));

        let mut appearance = appearance_with_variation(0);
        let red = TintChannels {
            channel1: TintChannel {
                r: 200,
                g: 10,
                b: 10,
                a: 255,
            },
            ..TintChannels::default()
        };
        appearance.accessories.set_data(
            AccessorySlot::LtShoulder,
            Some(AccessorySlotData {
                id: 7,
                tints: red.clone(),
            }),
        );
        appearance.accessories.set_data(
            AccessorySlot::LtKnee,
            Some(AccessorySlotData { id: 2, tints: red }),
        );

        write_appearance_into_item(&mut item, &appearance, true);

        let shoulder = struct_fields(item.get("ACLtShoulder").unwrap());
        assert!(matches!(shoulder.get("Accessory"), Some(GffValue::Byte(7))));
        assert!(shoulder.contains_key("UVScroll"), "existing layout kept");
        let tint = struct_fields(shoulder.get("Tintable").unwrap());
        let ch1 = struct_fields(struct_fields(tint.get("Tint").unwrap()).get("1").unwrap());
        assert!(matches!(ch1.get("r"), Some(GffValue::Byte(200))));

        // Untouched `None` slot keeps its struct verbatim.
        let rt = struct_fields(item.get("ACRtShoulder").unwrap());
        assert!(matches!(rt.get("Accessory"), Some(GffValue::Byte(0))));

        // A slot the item never had is cloned from a sibling so it carries
        // the same UVScroll layout, with the game's struct id for its slot.
        let knee = struct_fields(item.get("ACLtKnee").unwrap());
        assert!(matches!(knee.get("Accessory"), Some(GffValue::Byte(2))));
        assert!(knee.contains_key("UVScroll"));
        assert!(matches!(
            knee.get("__struct_id__"),
            Some(GffValue::Dword(16))
        ));
        assert!(knee.contains_key("Tintable"));
        assert!(item.get("ACRtKnee").is_none());
    }

    #[test]
    fn write_appearance_accessory_keeps_legacy_int_type_and_skips_zero_tint_without_tintable() {
        use crate::character::{AccessorySlot, AccessorySlotData, TintChannels};

        let mut legacy_inner = IndexMap::new();
        legacy_inner.insert("Accessory".to_string(), GffValue::Int(1));
        let mut item = IndexMap::new();
        item.insert(
            "ACLtBracer".to_string(),
            GffValue::StructOwned(Box::new(legacy_inner)),
        );

        let mut appearance = appearance_with_variation(0);
        appearance.accessories.set_data(
            AccessorySlot::LtBracer,
            Some(AccessorySlotData {
                id: 5,
                tints: TintChannels::default(),
            }),
        );
        write_appearance_into_item(&mut item, &appearance, true);

        let bracer = struct_fields(item.get("ACLtBracer").unwrap());
        assert!(matches!(bracer.get("Accessory"), Some(GffValue::Int(5))));
        assert!(bracer.get("Tintable").is_none());
    }
}
