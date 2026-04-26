use crate::character::CharacterError;
use crate::character::gff_helpers::gff_value_to_i32;
use crate::character::item_appearance::ItemAppearance;
use crate::character::types::BaseItemId;
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::services::ItemCostCalculator;
use crate::services::item_property_decoder::{DecodedProperty, ItemBonuses, ItemPropertyDecoder};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum EncumbranceStatus {
    Light,
    Medium,
    Heavy,
    Overloaded,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EncumbranceInfo {
    pub current_weight: f32,
    pub light_load: f32,
    pub medium_load: f32,
    pub heavy_load: f32,
    pub max_load: f32,
    pub status: EncumbranceStatus,
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
    pub success: bool,
    pub slot: EquipmentSlot,
    pub equipped_item: Option<InventoryItem>,
    pub swapped_item: Option<InventoryItem>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UnequipResult {
    pub success: bool,
    pub slot: EquipmentSlot,
    pub unequipped_item: Option<InventoryItem>,
    pub inventory_index: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AddItemResult {
    pub success: bool,
    pub inventory_index: Option<usize>,
    pub stacked: bool,
    pub message: String,
    pub item: Option<BasicItemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct RemoveItemResult {
    pub success: bool,
    pub removed_item: Option<InventoryItem>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BasicItemInfo {
    pub tag: String,
    pub base_item: i32,
    pub stack_size: i32,
    pub identified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Type)]
pub enum WeightStatus {
    #[default]
    Unencumbered,
    Light,
    Medium,
    Heavy,
    Overloaded,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BaseItemData {
    pub id: i32,
    pub name: String,
    pub weight: f32,
    pub base_ac: i32,
    pub max_stack: i32,
    pub num_dice: i32,
    pub die_to_roll: i32,
    pub crit_threat: i32,
    pub crit_multiplier: i32,
    pub weapon_size: i32,
    pub weapon_type: i32,
    pub armor_check_penalty: i32,
}

impl BaseItemData {
    pub fn damage_dice_string(&self) -> String {
        if self.num_dice > 0 && self.die_to_roll > 0 {
            format!("{}d{}", self.num_dice, self.die_to_roll)
        } else {
            "-".to_string()
        }
    }

    pub fn threat_range_string(&self) -> String {
        match self.crit_threat {
            1 => "20".to_string(),
            2 => "19-20".to_string(),
            3 => "18-20".to_string(),
            n if n > 3 => format!("{}-20", 21 - n),
            _ => "20".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EquipmentSummary {
    pub slots: Vec<EquipmentSlotInfo>,
    pub total_ac_bonus: i32,
    pub total_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EquipmentSlotInfo {
    pub slot: u32,
    pub slot_name: String,
    pub item: Option<BasicItemInfo>,
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
    pub decoded_properties: Vec<DecodedPropertyInfo>,
    pub appearance: ItemAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FullEncumbrance {
    pub total_weight: f32,
    pub light_load: f32,
    pub medium_load: f32,
    pub heavy_load: f32,
    pub encumbrance_level: String,
}

const PROPERTY_ID_AC_BONUS: u32 = 1;
const BASE_ITEM_ARMOR: i32 = 16;
const BASE_ITEM_SMALL_SHIELD: i32 = 14;
const BASE_ITEM_LARGE_SHIELD: i32 = 56;
const BASE_ITEM_TOWER_SHIELD: i32 = 57;

const FEAT_WEAPON_PROFICIENCY_SIMPLE: u16 = 44;
const FEAT_WEAPON_PROFICIENCY_MARTIAL: u16 = 45;
const FEAT_WEAPON_PROFICIENCY_EXOTIC: u16 = 46;
const FEAT_SHIELD_PROFICIENCY: u16 = 47;
const FEAT_TOWER_SHIELD_PROFICIENCY: u16 = 48;

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ProficiencyRequirement {
    pub feat_id: Option<u16>,
    pub feat_name: String,
    pub met: bool,
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ItemProficiencyInfo {
    pub is_proficient: bool,
    pub requirements: Vec<ProficiencyRequirement>,
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

    pub fn get_equipment_ac_bonus(&self, game_data: &GameData) -> i32 {
        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return 0;
        };

        let mut total_ac = 0;

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

            if let Some(base_ac) = self.get_base_item_ac(item_struct, base_item_id, game_data) {
                total_ac += base_ac;
            }

            if let Some(props_list) = item_struct.get("PropertiesList")
                && let GffValue::ListOwned(props) = props_list
            {
                for prop in props {
                    let property_id = prop
                        .get("PropertyName")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(0) as u32;

                    if property_id == PROPERTY_ID_AC_BONUS {
                        let cost_value = prop
                            .get("CostValue")
                            .and_then(gff_value_to_i32)
                            .unwrap_or(0);
                        total_ac += cost_value;
                    }
                }
            }
        }

        total_ac
    }

    pub fn get_armor_check_penalty(&self, game_data: &GameData) -> i32 {
        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return 0;
        };

        let mut total_penalty = 0;

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            // Only check armor (Chest=0x0002) and shields (LeftHand=0x0020)
            if struct_id != 0x0002 && struct_id != 0x0020 {
                continue;
            }

            let base_item_id = item_struct
                .get("BaseItem")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            if let Some(base_data) = self.get_base_item_data(base_item_id, game_data) {
                total_penalty += base_data.armor_check_penalty;
            }
        }

        total_penalty
    }

    pub fn calculate_total_weight(&self, game_data: &GameData) -> f32 {
        let mut total_weight = 0.0;

        if let Some(inv_list) = self.get_list_owned("ItemList") {
            for item_struct in &inv_list {
                let base_item_id = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                let stack_size = item_struct
                    .get("StackSize")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(1);

                if let Some(weight) = self.get_base_item_weight(base_item_id, game_data) {
                    total_weight += weight * stack_size as f32;
                }
            }
        }

        if let Some(equip_list) = self.get_list_owned("Equip_ItemList") {
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

                if let Some(weight) = self.get_base_item_weight(base_item_id, game_data) {
                    total_weight += weight;
                }
            }
        }

        total_weight
    }

    pub fn get_weight_status(&self, game_data: &GameData) -> WeightStatus {
        let total_weight = self.calculate_total_weight(game_data);
        let strength = self.base_ability(crate::character::types::AbilityIndex::STR);

        let (light, medium, heavy, max) = self.get_encumbrance_thresholds(strength, game_data);

        if total_weight > max {
            WeightStatus::Overloaded
        } else if total_weight > heavy {
            WeightStatus::Heavy
        } else if total_weight > medium {
            WeightStatus::Medium
        } else if total_weight > light {
            WeightStatus::Light
        } else {
            WeightStatus::Unencumbered
        }
    }

    pub fn get_base_item_data(
        &self,
        base_item_id: i32,
        game_data: &GameData,
    ) -> Option<BaseItemData> {
        let baseitems = game_data.get_table("baseitems")?;
        let row = baseitems.get_by_id(base_item_id)?;

        let name = game_data
            .get_string(row_int(&row, "name", -1))
            .unwrap_or_else(|| format!("Item {base_item_id}"));

        let weight = row
            .get("tenthlbs")
            .and_then(|s| s.as_ref())
            .and_then(|s| s.parse::<f32>().ok())
            .map_or(0.0, |w| w / 10.0);

        let base_ac = row_int(&row, "baseac", 0);

        let max_stack = row_int(&row, "stacking", 1);

        let num_dice = row_int(&row, "numdice", 0);

        let die_to_roll = row_int(&row, "dietoroll", 0);

        let crit_threat = row_int(&row, "critthreat", 1);

        let crit_multiplier = row_int(&row, "crithitmult", 2);

        let weapon_size = row_int(&row, "weaponsize", 0);

        let weapon_type = row_int(&row, "weapontype", 0);

        let armor_check_penalty = row
            .get("armourcheckpen")
            .or_else(|| row.get("armorcheckpen"))
            .and_then(|s| s.as_ref())
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        Some(BaseItemData {
            id: base_item_id,
            name,
            weight,
            base_ac,
            max_stack,
            num_dice,
            die_to_roll,
            crit_threat,
            crit_multiplier,
            weapon_size,
            weapon_type,
            armor_check_penalty,
        })
    }

    pub fn get_equipment_summary(&self, game_data: &GameData) -> EquipmentSummary {
        let mut slots = Vec::new();
        let mut equipped_items: HashMap<u32, BasicItemInfo> = HashMap::new();

        if let Some(equip_list) = self.get_list_owned("Equip_ItemList") {
            for item_struct in &equip_list {
                let struct_id = item_struct
                    .get("__struct_id__")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0) as u32;

                if struct_id == 0 {
                    continue;
                }

                if let Some(item_info) = self.parse_basic_item_info(item_struct) {
                    equipped_items.insert(struct_id, item_info);
                }
            }
        }

        for (slot_bitmask, slot_name) in EQUIPMENT_SLOTS {
            let item = equipped_items.get(&slot_bitmask).cloned();
            slots.push(EquipmentSlotInfo {
                slot: slot_bitmask,
                slot_name: slot_name.to_string(),
                item,
            });
        }

        let total_ac_bonus = self.get_equipment_ac_bonus(game_data);
        let total_weight = self.calculate_total_weight(game_data);

        EquipmentSummary {
            slots,
            total_ac_bonus,
            total_weight,
        }
    }

    pub fn get_full_inventory_summary(
        &self,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
        resource_manager: Option<&crate::services::resource_manager::ResourceManager>,
    ) -> FullInventorySummary {
        use tracing::{debug, warn};

        let mut inventory_items = Vec::new();
        let mut equipped_items = Vec::new();

        debug!(
            "Getting ItemList from character. Has field: {}",
            self.has_field("ItemList")
        );
        if let Some(inv_list) = self.get_list_owned("ItemList") {
            debug!("ItemList found with {} items", inv_list.len());
            for (index, item_struct) in inv_list.iter().enumerate() {
                let base_item_id = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                let base_item_data = self.get_base_item_data(base_item_id, game_data);
                let base_item_name = base_item_data
                    .as_ref()
                    .map_or_else(|| format!("Item {base_item_id}"), |d| d.name.clone());

                let is_custom = base_item_data.is_none();

                let name = self
                    .get_item_localized_name(item_struct, game_data)
                    .unwrap_or_else(|| base_item_name.clone());

                let description = self
                    .get_item_localized_description(item_struct, game_data)
                    .unwrap_or_default();

                let weight = self
                    .get_base_item_weight(base_item_id, game_data)
                    .unwrap_or(0.0);

                let stack_size = item_struct
                    .get("StackSize")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(1);

                let value = {
                    let calculator = ItemCostCalculator::new();
                    match calculator.calculate_item_cost(item_struct, game_data) {
                        Some(v) if v > 0 => v as i32,
                        _ => {
                            let cost = item_struct
                                .get("Cost")
                                .and_then(gff_value_to_i32)
                                .unwrap_or(0);
                            let modify_cost = item_struct
                                .get("ModifyCost")
                                .and_then(gff_value_to_i32)
                                .unwrap_or(0);
                            cost + modify_cost
                        }
                    }
                };

                let enhancement = item_struct
                    .get("Enhancement")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                let charges = item_struct.get("Charges").and_then(gff_value_to_i32);

                let identified = item_struct
                    .get("Identified")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(1)
                    != 0;

                let plot = item_struct
                    .get("Plot")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0)
                    == 1;

                let cursed = item_struct
                    .get("Cursed")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0)
                    == 1;

                let stolen = item_struct
                    .get("Stolen")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0)
                    == 1;

                let base_ac = self.get_base_item_ac(item_struct, base_item_id, game_data);

                let category = base_item_data
                    .as_ref()
                    .and_then(|_| game_data.get_table("baseitems"))
                    .and_then(|t| t.get_by_id(base_item_id))
                    .map(|row| get_item_category(&row))
                    .unwrap_or_else(|| "Miscellaneous".to_string());

                let (equippable_slots, default_slot) =
                    self.get_equippable_slots(base_item_id, game_data);

                let decoded_properties = self.decode_item_properties(item_struct, decoder);

                let item_data = gff_struct_to_json(item_struct);

                let icon = resolve_item_icon(item_struct, game_data);
                let appearance = {
                    let mut a = ItemAppearance::from_gff(item_struct);
                    a.normalize_for_ui(base_item_id, game_data);
                    // Fall back to the UTI template's Tintable when the save
                    // has no explicit tints, so the UI swatches show the
                    // colours actually rendered by the game.
                    if let Some(rm) = resource_manager
                        && let Some(effective) = Self::read_item_tint_with_fallback(item_struct, rm)
                    {
                        a.tints = effective;
                    }
                    a
                };

                inventory_items.push(FullInventoryItem {
                    index,
                    item: RawItemData(item_data),
                    base_item: base_item_id,
                    base_item_name,
                    name,
                    icon,
                    description,
                    weight,
                    value,
                    is_custom,
                    stack_size,
                    enhancement,
                    charges,
                    identified,
                    plot,
                    cursed,
                    stolen,
                    base_ac,
                    category,
                    equippable_slots,
                    default_slot,
                    decoded_properties,
                    appearance,
                });
            }
        } else {
            warn!("ItemList not found or empty");
        }

        debug!(
            "Getting Equip_ItemList. Has field: {}",
            self.has_field("Equip_ItemList")
        );
        if let Some(equip_list) = self.get_list_owned("Equip_ItemList") {
            debug!("Equip_ItemList found with {} slots", equip_list.len());
            for item_struct in &equip_list {
                let struct_id = item_struct
                    .get("__struct_id__")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0) as u32;

                if struct_id == 0 {
                    continue;
                }

                let slot_name = get_slot_name(struct_id).to_lowercase().replace(' ', "_");

                let base_item_id = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                let base_item_data = self.get_base_item_data(base_item_id, game_data);
                let base_item_name = base_item_data
                    .as_ref()
                    .map_or_else(|| format!("Item {base_item_id}"), |d| d.name.clone());

                let is_custom = base_item_data.is_none();

                let name = self
                    .get_item_localized_name(item_struct, game_data)
                    .unwrap_or_else(|| base_item_name.clone());

                let description = self
                    .get_item_localized_description(item_struct, game_data)
                    .unwrap_or_default();

                let weight = self
                    .get_base_item_weight(base_item_id, game_data)
                    .unwrap_or(0.0);

                let value = {
                    let calculator = ItemCostCalculator::new();
                    match calculator.calculate_item_cost(item_struct, game_data) {
                        Some(v) if v > 0 => v as i32,
                        _ => {
                            let cost = item_struct
                                .get("Cost")
                                .and_then(gff_value_to_i32)
                                .unwrap_or(0);
                            let modify_cost = item_struct
                                .get("ModifyCost")
                                .and_then(gff_value_to_i32)
                                .unwrap_or(0);
                            cost + modify_cost
                        }
                    }
                };

                let base_ac = self.get_base_item_ac(item_struct, base_item_id, game_data);

                let decoded_properties = self.decode_item_properties(item_struct, decoder);

                let item_data = gff_struct_to_json(item_struct);

                let icon = resolve_item_icon(item_struct, game_data);

                equipped_items.push(FullEquippedItem {
                    slot: slot_name,
                    base_item: base_item_id,
                    base_item_name,
                    custom: is_custom,
                    name,
                    icon,
                    description,
                    weight,
                    value,
                    item_data: RawItemData(item_data),
                    base_ac,
                    decoded_properties,
                    appearance: {
                        let mut a = ItemAppearance::from_gff(item_struct);
                        a.normalize_for_ui(base_item_id, game_data);
                        if let Some(rm) = resource_manager
                            && let Some(effective) =
                                Self::read_item_tint_with_fallback(item_struct, rm)
                        {
                            a.tints = effective;
                        }
                        a
                    },
                });
            }
        }

        let encumbrance_info = self.get_encumbrance_info(game_data);
        let encumbrance = FullEncumbrance {
            total_weight: encumbrance_info.current_weight,
            light_load: encumbrance_info.light_load,
            medium_load: encumbrance_info.medium_load,
            heavy_load: encumbrance_info.heavy_load,
            encumbrance_level: match encumbrance_info.status {
                EncumbranceStatus::Light => "light",
                EncumbranceStatus::Medium => "medium",
                EncumbranceStatus::Heavy => "heavy",
                EncumbranceStatus::Overloaded => "overloaded",
            }
            .to_string(),
        };

        FullInventorySummary {
            inventory: inventory_items,
            equipped: equipped_items,
            gold: self.gold(),
            encumbrance,
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
        let Some(baseitems) = game_data.get_table("baseitems") else {
            return (vec![], None);
        };

        let Some(row) = baseitems.get_by_id(base_item_id) else {
            return (vec![], None);
        };

        let equip_slots = row_str(&row, "equipableslots")
            .and_then(|s| {
                if let Some(stripped) = s.strip_prefix("0x") {
                    u32::from_str_radix(stripped, 16).ok()
                } else {
                    s.parse::<u32>().ok()
                }
            })
            .unwrap_or(0);

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

        let mut prop_maps = Vec::new();
        for prop in props {
            let mut map = HashMap::new();
            for (k, v) in prop {
                if let Some(json_val) = gff_to_json_primitive(v) {
                    map.insert(k.clone(), json_val);
                }
            }
            prop_maps.push(map);
        }

        decoder
            .decode_all_properties(&prop_maps)
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

            // 1. Base AC from Armor/Shields
            let base_item_id = item_struct
                .get("BaseItem")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            // Logic matching Python:
            // Armor (16)
            if base_item_id == BASE_ITEM_ARMOR {
                if let Some(armor_rules_type) =
                    item_struct.get("ArmorRulesType").and_then(gff_value_to_i32)
                    && let Some(stats) = game_data
                        .get_table("armorrulestats")
                        .and_then(|t| t.get_by_id(armor_rules_type))
                    && let Some(ac_bonus) = stats
                        .get("acbonus")
                        .and_then(|s| s.as_ref())
                        .and_then(|s| s.parse::<f32>().ok()) // Python uses float parse then int cast if likely
                        .map(|f| f as i32)
                    && ac_bonus > 0
                {
                    total_bonuses.ac_armor_bonus += ac_bonus;
                }
            }
            // Shields (14, 56, 57)
            else if [
                BASE_ITEM_SMALL_SHIELD,
                BASE_ITEM_LARGE_SHIELD,
                BASE_ITEM_TOWER_SHIELD,
            ]
            .contains(&base_item_id)
                && let Some(armor_rules_type) =
                    item_struct.get("ArmorRulesType").and_then(gff_value_to_i32)
                && let Some(stats) = game_data
                    .get_table("armorrulestats")
                    .and_then(|t| t.get_by_id(armor_rules_type))
                && let Some(ac_bonus) = stats
                    .get("acbonus")
                    .and_then(|s| s.as_ref())
                    .and_then(|s| s.parse::<f32>().ok())
                    .map(|f| f as i32)
                && ac_bonus > 0
            {
                total_bonuses.ac_shield_bonus += ac_bonus;
            }

            // 2. Item Properties
            if let Some(GffValue::ListOwned(props)) = item_struct.get("PropertiesList") {
                let mut prop_maps = Vec::new();
                for prop in props {
                    let mut map = HashMap::new();
                    // Convert GFF struct fields to simple JSON map for decoder
                    for (k, v) in prop {
                        if let Some(json_val) = gff_to_json_primitive(v) {
                            map.insert(k.clone(), json_val);
                        }
                    }
                    prop_maps.push(map);
                }

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

    fn get_base_item_weight(&self, base_item_id: i32, game_data: &GameData) -> Option<f32> {
        let baseitems = game_data.get_table("baseitems")?;
        let row = baseitems.get_by_id(base_item_id)?;

        row.get("tenthlbs")
            .and_then(|s| s.as_ref())
            .and_then(|s| s.parse::<f32>().ok())
            .map(|w| w / 10.0)
    }

    fn get_base_item_ac(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        base_item_id: i32,
        game_data: &GameData,
    ) -> Option<i32> {
        if base_item_id != BASE_ITEM_ARMOR
            && base_item_id != BASE_ITEM_SMALL_SHIELD
            && base_item_id != BASE_ITEM_LARGE_SHIELD
            && base_item_id != BASE_ITEM_TOWER_SHIELD
        {
            return None;
        }

        // Real base AC for armor/shields comes from armorrulestats.2da via ArmorRulesType;
        // baseitems.2da baseac is 0 for the generic "armor" base item.
        if let Some(armor_rules_type) = item_struct.get("ArmorRulesType").and_then(gff_value_to_i32)
            && let Some(stats) = game_data
                .get_table("armorrulestats")
                .and_then(|t| t.get_by_id(armor_rules_type))
            && let Some(ac_bonus) = stats
                .get("acbonus")
                .and_then(|s| s.as_ref())
                .and_then(|s| s.parse::<f32>().ok())
                .map(|f| f as i32)
        {
            return Some(ac_bonus);
        }

        let baseitems = game_data.get_table("baseitems")?;
        let row = baseitems.get_by_id(base_item_id)?;

        row_str(&row, "baseac").and_then(|s| s.parse::<i32>().ok())
    }

    fn get_encumbrance_thresholds(
        &self,
        strength: i32,
        game_data: &GameData,
    ) -> (f32, f32, f32, f32) {
        let clamped_str = strength.clamp(1, 100);

        if let Some(encumbrance) = game_data.get_table("encumbrance")
            && let Some(row) = encumbrance.get_by_id(clamped_str)
        {
            let light = row
                .get("light")
                .or_else(|| row.get("lightload"))
                .and_then(|s| s.as_ref())
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);

            let medium = row
                .get("medium")
                .or_else(|| row.get("mediumload"))
                .and_then(|s| s.as_ref())
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);

            let heavy = row
                .get("heavy")
                .or_else(|| row.get("heavyload"))
                .and_then(|s| s.as_ref())
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.0);

            let max = heavy * 2.0;

            return (light, medium, heavy, max);
        }

        let base = 10.0 * clamped_str as f32;
        (base * 0.33, base * 0.66, base, base * 2.0)
    }

    pub fn get_encumbrance_info(&self, game_data: &GameData) -> EncumbranceInfo {
        let current_weight = self.calculate_total_weight(game_data);
        let strength = self.base_ability(crate::character::types::AbilityIndex::STR);

        let (light, medium, heavy, max) = self.get_encumbrance_thresholds(strength, game_data);

        let status = if current_weight > max {
            EncumbranceStatus::Overloaded
        } else if current_weight > heavy {
            EncumbranceStatus::Heavy
        } else if current_weight > medium {
            EncumbranceStatus::Medium
        } else {
            EncumbranceStatus::Light
        };

        EncumbranceInfo {
            current_weight,
            light_load: light,
            medium_load: medium,
            heavy_load: heavy,
            max_load: max,
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
            let item_info = self.parse_basic_item_info(&item_struct);
            let item_name = item_info.as_ref().map_or("Item", |i| i.tag.as_str());
            return Ok(EquipResult {
                success: false,
                slot,
                equipped_item: None,
                swapped_item: None,
                message: format!("Cannot equip {} in {} slot", item_name, slot.display_name()),
            });
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
            success: true,
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
            message: format!("Equipped item in {}", slot.display_name()),
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
            return Ok(UnequipResult {
                success: false,
                slot,
                unequipped_item: None,
                inventory_index: None,
                message: format!("No item equipped in {} slot", slot.display_name()),
            });
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
            success: true,
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
            message: format!("Unequipped item from {}", slot.display_name()),
        })
    }

    pub fn add_item(
        &mut self,
        base_item_id: i32,
        stack_size: i32,
        icon_id: Option<u32>,
        game_data: &GameData,
    ) -> Result<AddItemResult, CharacterError> {
        let base_item_data = self.get_base_item_data(base_item_id, game_data);

        let max_stack = base_item_data.as_ref().map_or(1, |d| d.max_stack);

        if max_stack > 1 {
            let mut inv_list = self.get_list_owned("ItemList").unwrap_or_default();

            for (index, item_struct) in inv_list.iter_mut().enumerate() {
                let existing_base_item = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                if existing_base_item == base_item_id {
                    let current_stack = item_struct
                        .get("StackSize")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(1);

                    if current_stack < max_stack {
                        let new_stack = (current_stack + stack_size).min(max_stack);
                        item_struct
                            .insert("StackSize".to_string(), GffValue::Short(new_stack as i16));
                        self.set_list("ItemList", inv_list);

                        return Ok(AddItemResult {
                            success: true,
                            inventory_index: Some(index),
                            stacked: true,
                            message: format!("Stacked {stack_size} items"),
                            item: None,
                        });
                    }
                }
            }
        }

        let mut inv_list = self.get_list_owned("ItemList").unwrap_or_default();
        let mut new_item = IndexMap::new();
        new_item.insert("BaseItem".to_string(), GffValue::Int(base_item_id));
        new_item.insert(
            "StackSize".to_string(),
            GffValue::Short(stack_size.min(max_stack) as i16),
        );
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
                // BodyArmor: variant comes from Variation.
                new_item.insert("Variation".to_string(), GffValue::Int(1));
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

        let inventory_index = inv_list.len();
        inv_list.push(new_item);
        self.set_list("ItemList", inv_list);

        Ok(AddItemResult {
            success: true,
            inventory_index: Some(inventory_index),
            stacked: false,
            message: "Item added to inventory".to_string(),
            item: None,
        })
    }

    pub fn remove_item(&mut self, index: usize) -> Result<RemoveItemResult, CharacterError> {
        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        if index >= inv_list.len() {
            return Ok(RemoveItemResult {
                success: false,
                removed_item: None,
                message: format!("No item at index {index}"),
            });
        }

        let removed_struct = inv_list.remove(index);
        self.set_list("ItemList", inv_list);

        let removed_info = self.parse_basic_item_info(&removed_struct);

        Ok(RemoveItemResult {
            success: true,
            removed_item: removed_info.map(|info| InventoryItem {
                index,
                base_item_id: BaseItemId(info.base_item),
                name: info.tag.clone(),
                tag: info.tag,
                stack_size: info.stack_size,
                identified: info.identified,
            }),
            message: "Item removed from inventory".to_string(),
        })
    }

    fn can_item_equip_in_slot(
        &self,
        base_item_id: i32,
        slot: EquipmentSlot,
        game_data: &GameData,
    ) -> bool {
        let Some(baseitems) = game_data.get_table("baseitems") else {
            return false;
        };

        let Some(row) = baseitems.get_by_id(base_item_id) else {
            return false;
        };

        let equip_slots = row_str(&row, "equipableslots")
            .and_then(|s| {
                if let Some(stripped) = s.strip_prefix("0x") {
                    u32::from_str_radix(stripped, 16).ok()
                } else {
                    s.parse::<u32>().ok()
                }
            })
            .unwrap_or(0);

        equip_slots & slot.to_bitmask() != 0
    }

    pub fn get_item_proficiency_info(
        &self,
        base_item_id: i32,
        game_data: &GameData,
    ) -> ItemProficiencyInfo {
        let mut requirements = Vec::new();
        let mut is_proficient = true;

        let Some(baseitems) = game_data.get_table("baseitems") else {
            return ItemProficiencyInfo {
                is_proficient: true,
                requirements: vec![],
            };
        };

        let Some(row) = baseitems.get_by_id(base_item_id) else {
            return ItemProficiencyInfo {
                is_proficient: true,
                requirements: vec![],
            };
        };

        // Helper to check feat
        let has_feat = |feat_id: u16| -> bool {
            if let Some(feats) = self.get_list_owned("FeatList") {
                feats.iter().any(|f| {
                    f.get("Feat").and_then(gff_value_to_i32).unwrap_or(-1) as u16 == feat_id
                })
            } else {
                false
            }
        };

        // Helper to get feat name from feat.2da via TLK lookup
        let get_feat_name = |feat_id: u16| -> String {
            game_data
                .get_table("feat")
                .and_then(|t| t.get_by_id(i32::from(feat_id)))
                .and_then(|row| row_str(&row, "featname").and_then(|s| s.parse::<i32>().ok()))
                .and_then(|str_ref| game_data.get_string(str_ref))
                .unwrap_or_else(|| format!("Feat {feat_id}"))
        };

        // 1. Check ReqFeat0-4
        for i in 0..5 {
            let col = format!("reqfeat{i}");
            let feat_id_val = row_int(&row, &col, -1);
            // -1 means no requirement
            if feat_id_val >= 0 {
                let feat_id = feat_id_val as u16;
                let met = has_feat(feat_id);
                if !met {
                    is_proficient = false;
                }

                requirements.push(ProficiencyRequirement {
                    feat_id: Some(feat_id),
                    feat_name: get_feat_name(feat_id),
                    met,
                });
            }
        }

        // 2. Check WeaponType
        let weapon_type = row_int(&row, "weapontype", 0);
        if weapon_type > 0 {
            let mut required_feat = None;
            match weapon_type {
                1 => required_feat = Some(FEAT_WEAPON_PROFICIENCY_SIMPLE),
                2 => required_feat = Some(FEAT_WEAPON_PROFICIENCY_MARTIAL),
                3 => required_feat = Some(FEAT_WEAPON_PROFICIENCY_EXOTIC),
                // Note: Exotic usually has specific ReqFeat, but this is general category fallback
                _ => {}
            }

            // Special cases for Shields
            if [BASE_ITEM_SMALL_SHIELD, BASE_ITEM_LARGE_SHIELD].contains(&base_item_id) {
                required_feat = Some(FEAT_SHIELD_PROFICIENCY);
            } else if base_item_id == BASE_ITEM_TOWER_SHIELD {
                required_feat = Some(FEAT_TOWER_SHIELD_PROFICIENCY);
            }

            if let Some(feat_id) = required_feat {
                let met = has_feat(feat_id);
                if !met {
                    is_proficient = false;
                    requirements.push(ProficiencyRequirement {
                        feat_id: Some(feat_id),
                        feat_name: get_feat_name(feat_id),
                        met,
                    });
                }
            }
        }

        ItemProficiencyInfo {
            is_proficient,
            requirements,
        }
    }

    pub fn get_equipped_armor_max_dex(&self, game_data: &GameData) -> i32 {
        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return 999;
        };

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == EquipmentSlot::Chest.to_bitmask() {
                if let Some(armor_rules_type) =
                    item_struct.get("ArmorRulesType").and_then(gff_value_to_i32)
                    && let Some(stats) = game_data
                        .get_table("armorrulestats")
                        .and_then(|t| t.get_by_id(armor_rules_type))
                    && let Some(max_dex) =
                        row_str(&stats, "maxdexbonus").and_then(|s| s.parse::<i32>().ok())
                {
                    return max_dex;
                }
                break;
            }
        }

        999
    }

    pub fn get_equipped_armor_rank(&self, game_data: &GameData) -> String {
        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return "None".to_string();
        };

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == EquipmentSlot::Chest.to_bitmask() {
                if let Some(armor_rules_type) =
                    item_struct.get("ArmorRulesType").and_then(gff_value_to_i32)
                    && let Some(stats) = game_data
                        .get_table("armorrulestats")
                        .and_then(|t| t.get_by_id(armor_rules_type))
                    && let Some(rank) = row_str(&stats, "rank")
                {
                    return rank;
                }
                break;
            }
        }

        "None".to_string()
    }

    pub fn validate_inventory(&self) -> InventoryValidation {
        let mut errors = Vec::new();
        let warnings = Vec::new();

        if let Some(inv_list) = self.get_list_owned("ItemList") {
            for (idx, item_struct) in inv_list.iter().enumerate() {
                let stack_size = item_struct
                    .get("StackSize")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(1);

                if stack_size < 0 {
                    errors.push(format!(
                        "Inventory item {idx}: Invalid negative stack size {stack_size}"
                    ));
                } else if stack_size > 999 {
                    errors.push(format!(
                        "Inventory item {idx}: Stack size {stack_size} too large (max 999)"
                    ));
                }

                let base_item = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                if base_item < 0 {
                    errors.push(format!(
                        "Inventory item {idx}: Invalid negative base item ID {base_item}"
                    ));
                }
            }
        }

        if let Some(equip_list) = self.get_list_owned("Equip_ItemList") {
            for item_struct in &equip_list {
                let struct_id = item_struct
                    .get("__struct_id__")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0) as u32;

                if struct_id == 0 {
                    continue;
                }

                let base_item = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                if base_item < 0 {
                    let slot_name = get_slot_name(struct_id);
                    errors.push(format!(
                        "Equipped item in {slot_name}: Invalid negative base item ID {base_item}"
                    ));
                }
            }
        }

        InventoryValidation {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    pub fn get_custom_items(&self, game_data: &GameData) -> Vec<CustomItemInfo> {
        let mut custom_items = Vec::new();

        if let Some(equip_list) = self.get_list_owned("Equip_ItemList") {
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

                let baseitems = game_data.get_table("baseitems");
                let is_custom = baseitems.and_then(|t| t.get_by_id(base_item_id)).is_none();

                if is_custom {
                    let slot_name = get_slot_name(struct_id);
                    custom_items.push(CustomItemInfo {
                        location: format!("equipped_{slot_name}"),
                        base_item_id,
                        tag: item_struct
                            .get("Tag")
                            .and_then(|v| match v {
                                GffValue::String(s) => Some(s.to_string()),
                                _ => None,
                            })
                            .unwrap_or_default(),
                    });
                }
            }
        }

        if let Some(inv_list) = self.get_list_owned("ItemList") {
            for (idx, item_struct) in inv_list.iter().enumerate() {
                let base_item_id = item_struct
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);

                let baseitems = game_data.get_table("baseitems");
                let is_custom = baseitems.and_then(|t| t.get_by_id(base_item_id)).is_none();

                if is_custom {
                    custom_items.push(CustomItemInfo {
                        location: format!("inventory_{idx}"),
                        base_item_id,
                        tag: item_struct
                            .get("Tag")
                            .and_then(|v| match v {
                                GffValue::String(s) => Some(s.to_string()),
                                _ => None,
                            })
                            .unwrap_or_default(),
                    });
                }
            }
        }

        custom_items
    }

    pub fn has_custom_content(&self, game_data: &GameData) -> bool {
        !self.get_custom_items(game_data).is_empty()
    }

    pub fn get_all_weapons(&self, game_data: &GameData) -> Vec<BaseItemInfo> {
        let mut weapons = Vec::new();

        let Some(baseitems) = game_data.get_table("baseitems") else {
            return weapons;
        };

        for item_id in 0..baseitems.row_count() {
            let row_id = item_id as i32;
            let Some(row) = baseitems.get_by_id(row_id) else {
                continue;
            };

            let weapon_type = row_int(&row, "weapontype", 0);

            if weapon_type > 0 {
                let label = row_str(&row, "label").unwrap_or_else(|| format!("Weapon {row_id}"));

                if is_invalid_base_item_label(&label) {
                    continue;
                }

                let name = resolve_base_item_name(&row, row_id, game_data);

                weapons.push(BaseItemInfo {
                    id: row_id,
                    name,
                    label,
                    category: get_item_category(&row),
                    weapon_type: Some(weapon_type),
                    ac_type: None,
                });
            }
        }

        weapons.sort_by(|a, b| a.name.cmp(&b.name));
        weapons
    }

    pub fn get_all_armor(&self, game_data: &GameData) -> Vec<BaseItemInfo> {
        let mut armor = Vec::new();

        let Some(baseitems) = game_data.get_table("baseitems") else {
            return armor;
        };

        for item_id in 0..baseitems.row_count() {
            let row_id = item_id as i32;
            let Some(row) = baseitems.get_by_id(row_id) else {
                continue;
            };

            if row_id == BASE_ITEM_ARMOR
                || row_id == BASE_ITEM_SMALL_SHIELD
                || row_id == BASE_ITEM_LARGE_SHIELD
                || row_id == BASE_ITEM_TOWER_SHIELD
            {
                let label = row_str(&row, "label").unwrap_or_else(|| format!("Armor {row_id}"));

                if is_invalid_base_item_label(&label) {
                    continue;
                }

                let name = resolve_base_item_name(&row, row_id, game_data);

                let ac_type = row_str(&row, "actype").and_then(|s| s.parse::<i32>().ok());

                armor.push(BaseItemInfo {
                    id: row_id,
                    name,
                    label,
                    category: get_item_category(&row),
                    weapon_type: None,
                    ac_type,
                });
            }
        }

        armor.sort_by(|a, b| a.name.cmp(&b.name));
        armor
    }

    pub fn filter_items_by_category(
        &self,
        game_data: &GameData,
        category: &str,
    ) -> Vec<BaseItemInfo> {
        let mut items = Vec::new();

        let Some(baseitems) = game_data.get_table("baseitems") else {
            return items;
        };

        for item_id in 0..baseitems.row_count() {
            let row_id = item_id as i32;
            let Some(row) = baseitems.get_by_id(row_id) else {
                continue;
            };

            let item_category = get_item_category(&row);
            if item_category != category {
                continue;
            }

            let label = row_str(&row, "label").unwrap_or_else(|| format!("Item {row_id}"));

            if is_invalid_base_item_label(&label) {
                continue;
            }

            let name = resolve_base_item_name(&row, row_id, game_data);

            items.push(BaseItemInfo {
                id: row_id,
                name,
                label,
                category: item_category,
                weapon_type: None,
                ac_type: None,
            });
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));
        items
    }

    pub fn get_equipment_summary_by_slot(
        &self,
        game_data: &GameData,
    ) -> HashMap<String, Option<EquippedItemInfo>> {
        let mut summary: HashMap<String, Option<EquippedItemInfo>> = HashMap::new();

        for (_bitmask, name) in EQUIPMENT_SLOTS {
            summary.insert(name.to_string(), None);
        }

        let Some(equip_list) = self.get_list_owned("Equip_ItemList") else {
            return summary;
        };

        for item_struct in &equip_list {
            let struct_id = item_struct
                .get("__struct_id__")
                .and_then(gff_value_to_i32)
                .unwrap_or(0) as u32;

            if struct_id == 0 {
                continue;
            }

            let slot_name = get_slot_name(struct_id);

            let base_item_id = item_struct
                .get("BaseItem")
                .and_then(gff_value_to_i32)
                .unwrap_or(0);

            let tag = item_struct
                .get("Tag")
                .and_then(|v| match v {
                    GffValue::String(s) => Some(s.to_string()),
                    _ => None,
                })
                .unwrap_or_default();

            let baseitems = game_data.get_table("baseitems");
            let is_custom = baseitems.and_then(|t| t.get_by_id(base_item_id)).is_none();

            let name = if let Some(row) = baseitems.and_then(|t| t.get_by_id(base_item_id)) {
                resolve_base_item_name(&row, base_item_id, game_data)
            } else {
                format!("Custom Item {base_item_id}")
            };

            let weight = self
                .get_base_item_weight(base_item_id, game_data)
                .unwrap_or(0.0);

            summary.insert(
                slot_name.to_string(),
                Some(EquippedItemInfo {
                    base_item_id,
                    tag,
                    name,
                    weight,
                    is_custom,
                }),
            );
        }

        summary
    }

    pub fn get_item_property_descriptions(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        decoder: &ItemPropertyDecoder,
    ) -> Vec<DecodedProperty> {
        let Some(GffValue::ListOwned(props)) = item_struct.get("PropertiesList") else {
            return Vec::new();
        };

        let mut prop_maps = Vec::new();
        for prop in props {
            let mut map = HashMap::new();
            for (k, v) in prop {
                if let Some(json_val) = gff_to_json_primitive(v) {
                    map.insert(k.clone(), json_val);
                }
            }
            prop_maps.push(map);
        }

        decoder.decode_all_properties(&prop_maps)
    }

    pub fn get_enhanced_item_summary(
        &self,
        item_struct: &IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
        decoder: &ItemPropertyDecoder,
    ) -> EnhancedItemSummary {
        let base_item_id = item_struct
            .get("BaseItem")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);

        let tag = item_struct
            .get("Tag")
            .and_then(|v| match v {
                GffValue::String(s) => Some(s.to_string()),
                _ => None,
            })
            .unwrap_or_default();

        let baseitems = game_data.get_table("baseitems");
        let is_custom = baseitems.and_then(|t| t.get_by_id(base_item_id)).is_none();

        let name = if let Some(row) = baseitems.and_then(|t| t.get_by_id(base_item_id)) {
            resolve_base_item_name(&row, base_item_id, game_data)
        } else {
            format!("Custom Item {base_item_id}")
        };

        let enhancement = item_struct
            .get("Enhancement")
            .and_then(gff_value_to_i32)
            .unwrap_or(0);

        let charges = item_struct.get("Charges").and_then(gff_value_to_i32);

        let identified = item_struct
            .get("Identified")
            .and_then(gff_value_to_i32)
            .unwrap_or(1)
            != 0;

        let plot = item_struct
            .get("Plot")
            .and_then(gff_value_to_i32)
            .unwrap_or(0)
            == 1;

        let cursed = item_struct
            .get("Cursed")
            .and_then(gff_value_to_i32)
            .unwrap_or(0)
            == 1;

        let stolen = item_struct
            .get("Stolen")
            .and_then(gff_value_to_i32)
            .unwrap_or(0)
            == 1;

        let stack_size = item_struct
            .get("StackSize")
            .and_then(gff_value_to_i32)
            .unwrap_or(1);

        let properties = self.get_item_property_descriptions(item_struct, decoder);

        let weight = self
            .get_base_item_weight(base_item_id, game_data)
            .unwrap_or(0.0);

        let value = {
            let calculator = ItemCostCalculator::new();
            let calculated = calculator.calculate_item_cost(item_struct, game_data);

            match calculated {
                Some(v) if v > 0 => v as i32,
                _ => {
                    let cost = item_struct
                        .get("Cost")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(0);
                    let modify_cost = item_struct
                        .get("ModifyCost")
                        .and_then(gff_value_to_i32)
                        .unwrap_or(0);
                    cost + modify_cost
                }
            }
        };

        EnhancedItemSummary {
            base_item_id,
            tag,
            name,
            is_custom,
            enhancement,
            charges,
            identified,
            plot,
            cursed,
            stolen,
            stack_size,
            properties,
            weight,
            value,
        }
    }

    pub fn update_inventory_item(
        &mut self,
        index: usize,
        updated_data: &HashMap<String, JsonValue>,
    ) -> Result<(), CharacterError> {
        let mut inv_list = self
            .get_list_owned("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        if index >= inv_list.len() {
            return Err(CharacterError::InvalidOperation(format!(
                "No item at index {index}"
            )));
        }

        merge_json_into_gff_struct(&mut inv_list[index], updated_data);
        normalize_item_properties_list(&mut inv_list[index]);
        self.set_list("ItemList", inv_list);
        Ok(())
    }

    pub fn update_equipped_item(
        &mut self,
        slot: EquipmentSlot,
        updated_data: &HashMap<String, JsonValue>,
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

        merge_json_into_gff_struct(&mut equip_list[item_idx], updated_data);
        normalize_item_properties_list(&mut equip_list[item_idx]);
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
        routed.prepare_for_write(base_item_id, game_data);
        write_appearance_into_item(&mut inv_list[index], &routed);
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
        routed.prepare_for_write(base_item_id, game_data);
        write_appearance_into_item(&mut equip_list[item_idx], &routed);
        self.set_list("Equip_ItemList", equip_list);
        Ok(())
    }

    pub fn add_item_by_base_type(
        &mut self,
        base_item_id: i32,
        game_data: &GameData,
    ) -> Result<AddItemResult, CharacterError> {
        let base_item_data = self.get_base_item_data(base_item_id, game_data);

        if base_item_data.is_none() {
            return Err(CharacterError::ValidationFailed {
                field: "base_item_id",
                message: format!("Invalid base item ID: {base_item_id}"),
            });
        }

        self.add_item(base_item_id, 1, None, game_data)
    }

    pub fn get_all_base_items(&self, game_data: &GameData) -> Vec<BaseItemInfo> {
        let mut items = Vec::new();

        let Some(baseitems) = game_data.get_table("baseitems") else {
            return items;
        };

        for item_id in 0..baseitems.row_count() {
            let row_id = item_id as i32;
            let Some(row) = baseitems.get_by_id(row_id) else {
                continue;
            };

            let label = row_str(&row, "label").unwrap_or_else(|| format!("Item {row_id}"));

            if is_invalid_base_item_label(&label) {
                continue;
            }

            let name = resolve_base_item_name(&row, row_id, game_data);

            if is_invalid_base_item_label(&name) {
                continue;
            }

            let weapon_type = row_str(&row, "weapontype").and_then(|s| s.parse::<i32>().ok());

            let ac_type = row_str(&row, "actype").and_then(|s| s.parse::<i32>().ok());

            items.push(BaseItemInfo {
                id: row_id,
                name,
                label,
                category: get_item_category(&row),
                weapon_type,
                ac_type,
            });
        }

        items.sort_by(|a, b| a.name.cmp(&b.name));
        items
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
        item: IndexMap<String, GffValue<'static>>,
        game_data: &GameData,
    ) -> Result<AddItemResult, CharacterError> {
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
            .and_then(|v| match v {
                GffValue::Short(s) => Some(i32::from(*s)),
                GffValue::Int(i) => Some(*i),
                GffValue::Byte(b) => Some(i32::from(*b)),
                _ => None,
            })
            .unwrap_or(1);

        let max_stack = self
            .get_base_item_data(base_item_id, game_data)
            .map_or(1, |d| d.max_stack);

        if !self.has_field("ItemList") {
            self.set_list("ItemList", Vec::new());
        }

        if max_stack > 1 {
            let inv_list = self
                .get_list_mut("ItemList")
                .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

            for (index, existing) in inv_list.iter_mut().enumerate() {
                let existing_base = existing
                    .get("BaseItem")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(0);
                if existing_base != base_item_id {
                    continue;
                }

                let current_stack = existing
                    .get("StackSize")
                    .and_then(gff_value_to_i32)
                    .unwrap_or(1);

                if current_stack < max_stack {
                    let new_stack = (current_stack + stack_size).min(max_stack);
                    existing.insert("StackSize".to_string(), GffValue::Short(new_stack as i16));

                    return Ok(AddItemResult {
                        success: true,
                        inventory_index: Some(index),
                        stacked: true,
                        message: format!("Stacked {stack_size} onto existing {tag}"),
                        item: Some(BasicItemInfo {
                            base_item: base_item_id,
                            tag,
                            stack_size: new_stack,
                            identified: true,
                        }),
                    });
                }
            }
        }

        let inv_list = self
            .get_list_mut("ItemList")
            .ok_or(CharacterError::FieldMissing { field: "ItemList" })?;

        inv_list.push(item);
        let index = inv_list.len() - 1;

        Ok(AddItemResult {
            success: true,
            inventory_index: Some(index),
            stacked: false,
            message: format!("Added {tag}"),
            item: Some(BasicItemInfo {
                base_item: base_item_id,
                tag,
                stack_size,
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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct InventoryValidation {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CustomItemInfo {
    pub location: String,
    pub base_item_id: i32,
    pub tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct BaseItemInfo {
    pub id: i32,
    pub name: String,
    pub label: String,
    pub category: String,
    pub weapon_type: Option<i32>,
    pub ac_type: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EquippedItemInfo {
    pub base_item_id: i32,
    pub tag: String,
    pub name: String,
    pub weight: f32,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct EnhancedItemSummary {
    pub base_item_id: i32,
    pub tag: String,
    pub name: String,
    pub is_custom: bool,
    pub enhancement: i32,
    pub charges: Option<i32>,
    pub identified: bool,
    pub plot: bool,
    pub cursed: bool,
    pub stolen: bool,
    pub stack_size: i32,
    pub properties: Vec<DecodedProperty>,
    pub weight: f32,
    pub value: i32,
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

fn coerce_json_to_gff_type(
    json_val: &JsonValue,
    existing: GffValue<'static>,
) -> Option<GffValue<'static>> {
    match existing {
        GffValue::Byte(_) => json_val.as_u64().map(|n| GffValue::Byte(n as u8)),
        GffValue::Char(_) => json_val.as_u64().map(|n| GffValue::Char(n as u8 as char)),
        GffValue::Short(_) => json_val.as_i64().map(|n| GffValue::Short(n as i16)),
        GffValue::Word(_) => json_val.as_u64().map(|n| GffValue::Word(n as u16)),
        GffValue::Int(_) => json_val.as_i64().map(|n| GffValue::Int(n as i32)),
        GffValue::Dword(_) => json_val.as_u64().map(|n| GffValue::Dword(n as u32)),
        GffValue::Dword64(_) => json_val.as_u64().map(GffValue::Dword64),
        GffValue::Int64(_) => json_val.as_i64().map(GffValue::Int64),
        GffValue::Float(_) => json_val.as_f64().map(|n| GffValue::Float(n as f32)),
        GffValue::Double(_) => json_val.as_f64().map(GffValue::Double),
        GffValue::String(_) => json_val
            .as_str()
            .map(|s| GffValue::String(std::borrow::Cow::Owned(s.to_string()))),
        GffValue::ResRef(_) => json_val
            .as_str()
            .map(|s| GffValue::ResRef(std::borrow::Cow::Owned(s.to_string()))),
        GffValue::LocString(_) => json_to_locstring(json_val),
        GffValue::ListOwned(existing_list) => json_val
            .as_array()
            .map(|arr| GffValue::ListOwned(merge_json_list_into_gff_list(&existing_list, arr))),
        GffValue::StructOwned(existing_struct) => coerce_json_to_struct(json_val, *existing_struct),
        GffValue::Struct(lazy) => coerce_json_to_struct(json_val, lazy.force_load()),
        _ => json_to_gff_best_guess(json_val),
    }
}

fn coerce_json_to_struct(
    json_val: &JsonValue,
    mut existing: IndexMap<String, GffValue<'static>>,
) -> Option<GffValue<'static>> {
    let obj = json_val.as_object()?;
    let updates: HashMap<String, JsonValue> =
        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    merge_json_into_gff_struct(&mut existing, &updates);
    Some(GffValue::StructOwned(Box::new(existing)))
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
) {
    // Preserve existing numeric type where possible; fall back to the NWN2 schema defaults
    // (Variation: Int, ModelPartN: Byte).
    let typed_int = |existing: Option<&GffValue<'static>>, v: i32| -> GffValue<'static> {
        match existing {
            Some(GffValue::Byte(_)) => GffValue::Byte(v as u8),
            Some(GffValue::Short(_)) => GffValue::Short(v as i16),
            Some(GffValue::Word(_)) => GffValue::Word(v as u16),
            Some(GffValue::Int(_)) | None => GffValue::Int(v),
            _ => GffValue::Int(v),
        }
    };
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

    let new_variation = typed_int(item.get("Variation"), appearance.variation);
    let new_mp1 = typed_byte(item.get("ModelPart1"), appearance.model_parts[0]);
    let new_mp2 = typed_byte(item.get("ModelPart2"), appearance.model_parts[1]);
    let new_mp3 = typed_byte(item.get("ModelPart3"), appearance.model_parts[2]);

    write_num(item, "Variation", appearance.variation, new_variation);
    write_num(item, "ModelPart1", appearance.model_parts[0], new_mp1);
    write_num(item, "ModelPart2", appearance.model_parts[1], new_mp2);
    write_num(item, "ModelPart3", appearance.model_parts[2], new_mp3);

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

fn merge_json_into_gff_struct(
    existing: &mut IndexMap<String, GffValue<'static>>,
    updates: &HashMap<String, JsonValue>,
) {
    for (key, json_val) in updates {
        if key == "__struct_id__" {
            continue;
        }

        let new_val = if let Some(existing_val) = existing.shift_remove(key) {
            coerce_json_to_gff_type(json_val, existing_val)
        } else {
            json_to_gff_best_guess(json_val)
        };

        if let Some(val) = new_val {
            existing.insert(key.clone(), val);
        }
    }
}

fn merge_json_list_into_gff_list(
    existing_list: &[IndexMap<String, GffValue<'static>>],
    json_arr: &[JsonValue],
) -> Vec<IndexMap<String, GffValue<'static>>> {
    json_arr
        .iter()
        .enumerate()
        .map(|(i, json_item)| {
            let Some(obj) = json_item.as_object() else {
                return if i < existing_list.len() {
                    existing_list[i].clone()
                } else {
                    IndexMap::new()
                };
            };

            let updates: HashMap<String, JsonValue> =
                obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

            if i < existing_list.len() {
                let mut item = existing_list[i].clone();
                merge_json_into_gff_struct(&mut item, &updates);
                item
            } else {
                let mut item = IndexMap::new();
                for (k, v) in &updates {
                    if let Some(gff_val) = json_to_gff_best_guess(v) {
                        item.insert(k.clone(), gff_val);
                    }
                }
                item
            }
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::character::Character;
    use std::borrow::Cow;
    use std::fmt::Write as _;

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

        merge_json_into_gff_struct(&mut existing, &updates);

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

    #[test]
    fn test_calculate_total_weight_empty() {
        use crate::loaders::GameData;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        let game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));

        assert_eq!(character.calculate_total_weight(&game_data), 0.0);
    }

    #[test]
    fn test_get_equipment_ac_bonus_empty() {
        use crate::loaders::GameData;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        let game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));

        assert_eq!(character.get_equipment_ac_bonus(&game_data), 0);
    }

    #[test]
    fn test_weight_status_default() {
        use crate::loaders::GameData;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let mut fields = IndexMap::new();
        fields.insert("Str".to_string(), GffValue::Byte(10));
        let character = Character::from_gff(fields);
        let game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));

        let status = character.get_weight_status(&game_data);
        assert_eq!(status, WeightStatus::Unencumbered);
    }

    #[test]
    fn test_equipment_summary_empty() {
        use crate::loaders::GameData;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        let fields = IndexMap::new();
        let character = Character::from_gff(fields);
        let game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));

        let summary = character.get_equipment_summary(&game_data);
        assert_eq!(summary.slots.len(), 14);
        assert_eq!(summary.total_ac_bonus, 0);
        assert_eq!(summary.total_weight, 0.0);
        assert!(summary.slots.iter().all(|slot| slot.item.is_none()));
    }

    #[test]
    fn test_get_item_proficiency_info() {
        use crate::loaders::GameData;
        use crate::loaders::types::LoadedTable;
        use crate::parsers::tda::TDAParser;
        use crate::parsers::tlk::TLKParser;
        use std::sync::{Arc, RwLock};

        // Setup GameData
        let mut game_data = GameData::new(Arc::new(RwLock::new(TLKParser::default())));

        // Mock baseitems table via 2DA content
        // Item 0: Simple Weapon (Type 1), No specific feat
        // Item 1: Exotic Weapon (Type 3), Requires Feat 100
        let baseitems_content = "2DA V2.0
        
           Label       WeaponType ReqFeat0
0      Item0       1          -1
1      Item1       3          100
";
        let mut baseitems_parser = TDAParser::new();
        baseitems_parser
            .parse_from_bytes(baseitems_content.as_bytes())
            .unwrap();
        let baseitems_table = LoadedTable::new("baseitems".to_string(), Arc::new(baseitems_parser));
        game_data
            .tables
            .insert("baseitems".to_string(), baseitems_table);

        // Mock feat table
        // We need entry 100
        let mut feat_content = String::from("2DA V2.0\n\n       Label       FeatName\n");
        for i in 0..=100 {
            let _ = writeln!(feat_content, "{i}      Feat{i}      {i}");
        }

        let mut feat_parser = TDAParser::new();
        feat_parser
            .parse_from_bytes(feat_content.as_bytes())
            .unwrap();
        let feat_table = LoadedTable::new("feat".to_string(), Arc::new(feat_parser));
        game_data.tables.insert("feat".to_string(), feat_table);

        // Setup Character with Feat 44 (Simple) but NOT Feat 100
        let mut fields = IndexMap::new();
        let mut feat44 = IndexMap::new();
        feat44.insert("Feat".to_string(), GffValue::Word(44));
        fields.insert("FeatList".to_string(), GffValue::ListOwned(vec![feat44]));

        let character = Character::from_gff(fields);

        // Test Simple Weapon (Item 0)
        let info_simple = character.get_item_proficiency_info(0, &game_data);
        assert!(info_simple.is_proficient);

        // Test Exotic Weapon (Item 1)
        let info_exotic = character.get_item_proficiency_info(1, &game_data);
        assert!(!info_exotic.is_proficient);
        assert!(
            info_exotic
                .requirements
                .iter()
                .any(|r| r.feat_id == Some(100) && !r.met)
        );
    }
}
