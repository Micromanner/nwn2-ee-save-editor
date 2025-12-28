"""
Pydantic models for InventoryManager
Handles items, equipment, encumbrance, and item properties
"""

from typing import Dict, Any, Optional, List, Literal, Union
from pydantic import BaseModel, Field, ConfigDict


class ItemProperty(BaseModel):
    """Item property information matching GFF structure"""
    property_name: int = Field(..., alias="PropertyName", description="Property type ID")
    subtype: int = Field(0, alias="Subtype", description="Property subtype")
    cost_table: int = Field(0, alias="CostTable", description="Cost table ID")
    cost_value: int = Field(0, alias="CostValue", description="Cost value")
    param1: int = Field(0, alias="Param1", description="Parameter 1")
    param1_value: int = Field(0, alias="Param1Value", description="Parameter 1 value")
    chances_appear: int = Field(100, alias="ChancesAppear", description="Chance to appear (%)")
    useable: bool = Field(True, alias="Useable", description="Property is useable")
    spell_id: int = Field(-1, alias="SpellID", description="Associated spell ID")
    uses_per_day: int = Field(0, alias="UsesPerDay", description="Uses per day")
    
    # Decoded information
    property_name_str: Optional[str] = Field(None, description="Property name string")
    description: Optional[str] = Field(None, description="Property description")
    bonus_value: Optional[int] = Field(None, description="Calculated bonus value")
    
    model_config = ConfigDict(populate_by_name=True)


class ItemInfo(BaseModel):
    """Complete item information"""
    # Identification
    item_id: Optional[str] = Field(None, description="Unique item identifier")
    template_resref: Optional[str] = Field(None, description="Item template resref")
    tag: Optional[str] = Field(None, description="Item tag")
    
    # Basic properties
    name: str = Field("", description="Item name")
    description: Optional[str] = Field(None, description="Item description")
    base_item: int = Field(..., alias="BaseItem", description="Base item type ID")
    base_item_name: Optional[str] = Field(None, description="Base item type name")
    
    # Physical properties
    weight: float = Field(0.0, description="Item weight in pounds")
    value: int = Field(0, description="Item value in gold")
    stack_size: int = Field(1, alias="StackSize", description="Stack size")
    charges: Optional[int] = Field(None, description="Charges remaining")
    
    # Enhancement and properties
    enhancement_bonus: int = Field(0, description="Enhancement bonus")
    properties: List[ItemProperty] = Field(default_factory=list, alias="PropertiesList")
    
    # Visual
    icon: Optional[str] = Field(None, description="Item icon")
    model_variation: int = Field(0, description="Visual model variation")
    texture_variation: int = Field(0, description="Texture variation")
    
    # Item type classification
    item_type: Optional[str] = Field(None, description="Item category")
    equipment_slot: Optional[str] = Field(None, description="Equipment slot if equippable")
    
    # State flags
    is_equipped: bool = Field(False, description="Currently equipped")
    is_identified: bool = Field(True, description="Item is identified")
    is_custom: bool = Field(False, description="Custom/modded item")
    is_stolen: bool = Field(False, description="Item is stolen")
    is_plot: bool = Field(False, description="Plot item (cannot drop)")
    is_cursed: bool = Field(False, description="Cursed item")
    is_droppable: bool = Field(True, description="Can be dropped")
    
    # Calculated properties
    total_value: Optional[int] = Field(None, description="Total value including enhancements")
    total_weight: Optional[float] = Field(None, description="Total weight for stack")
    
    model_config = ConfigDict(populate_by_name=True)


class EquipmentSlot(BaseModel):
    """Equipment slot information"""
    slot_name: str = Field(..., description="Slot identifier")
    slot_display_name: str = Field(..., description="Human-readable slot name")
    slot_index: int = Field(..., description="Slot index in character data")
    
    # Slot properties
    required_base_items: List[int] = Field(default_factory=list, description="Valid base item types")
    size_restrictions: List[str] = Field(default_factory=list, description="Size restrictions")
    
    # Current state
    equipped_item: Optional[ItemInfo] = None
    is_occupied: bool = False
    
    # Bonuses from equipped item
    attribute_bonuses: Dict[str, int] = Field(default_factory=dict)
    skill_bonuses: Dict[str, int] = Field(default_factory=dict)
    combat_bonuses: Dict[str, int] = Field(default_factory=dict)
    save_bonuses: Dict[str, int] = Field(default_factory=dict)


class EncumbranceInfo(BaseModel):
    """Detailed encumbrance calculation"""
    # Character stats
    strength_score: int = Field(..., description="Character's Strength score")
    size_category: str = Field("Medium", description="Character size")
    
    # Weight limits (in pounds)
    light_load: float = Field(..., description="Light load maximum")
    medium_load: float = Field(..., description="Medium load maximum") 
    heavy_load: float = Field(..., description="Heavy load maximum")
    max_load: float = Field(..., description="Maximum carrying capacity")
    lift_overhead: float = Field(..., description="Can lift overhead")
    lift_off_ground: float = Field(..., description="Can lift off ground")
    push_drag: float = Field(..., description="Can push or drag")
    
    # Current state
    current_weight: float = Field(..., description="Total carried weight")
    encumbrance_level: Literal['light', 'medium', 'heavy', 'overloaded'] = Field(..., description="Current encumbrance")
    
    # Penalties
    movement_penalty: int = Field(0, description="Movement speed penalty")
    skill_check_penalty: int = Field(0, description="Skill check penalty")
    max_dex_bonus: Optional[int] = Field(None, description="Max Dex bonus to AC")
    run_multiplier: float = Field(4.0, description="Running speed multiplier")
    
    # Weight breakdown
    equipment_weight: float = Field(0.0, description="Weight of equipped items")
    inventory_weight: float = Field(0.0, description="Weight of inventory items")
    coin_weight: float = Field(0.0, description="Weight of coins")






# Item creation/modification not supported by manager - removed


class ItemEquipRequest(BaseModel):
    """Request to equip an item"""
    item_id: str = Field(..., description="Item to equip")
    slot: Optional[str] = Field(None, description="Specific slot (auto-detect if None)")
    force_unequip: bool = Field(False, description="Force unequip conflicting items")


class ItemEquipResponse(BaseModel):
    """Response after equipping an item"""
    success: bool
    message: str
    equipped_item: ItemInfo
    slot_used: str
    unequipped_items: List[ItemInfo] = Field(default_factory=list)
    bonuses_applied: Dict[str, Any] = Field(default_factory=dict)
    has_unsaved_changes: bool = True


class ItemUnequipRequest(BaseModel):
    """Request to unequip an item"""
    slot: str = Field(..., description="Equipment slot to clear")
    move_to_inventory: bool = Field(True, description="Move to inventory (vs destroy)")


class ItemUnequipResponse(BaseModel):
    """Response after unequipping an item"""
    success: bool
    message: str
    unequipped_item: Optional[ItemInfo] = None
    slot_cleared: str
    bonuses_removed: Dict[str, Any] = Field(default_factory=dict)
    has_unsaved_changes: bool = True


# Generic item move not supported by manager - removed


# Stacking, identification, and search not supported by manager - removed


class EquipmentBonuses(BaseModel):
    """Summary of all equipment bonuses"""
    # Attribute bonuses
    attribute_bonuses: Dict[str, int] = Field(default_factory=dict)
    
    # Combat bonuses
    armor_class: int = 0
    attack_bonus: int = 0
    damage_bonus: int = 0
    
    # Save bonuses
    save_bonuses: Dict[str, int] = Field(default_factory=dict)
    
    # Skill bonuses
    skill_bonuses: Dict[str, int] = Field(default_factory=dict)
    
    # Resistances
    damage_resistance: List[Dict[str, Any]] = Field(default_factory=list)
    energy_resistance: Dict[str, int] = Field(default_factory=dict)
    spell_resistance: int = 0
    
    # Special properties
    special_abilities: List[str] = Field(default_factory=list)
    active_properties: List[str] = Field(default_factory=list)


class InventoryValidation(BaseModel):
    """Inventory validation result"""
    valid: bool
    errors: List[str] = Field(default_factory=list)
    warnings: List[str] = Field(default_factory=list)
    
    # Specific validation issues
    overweight_items: List[str] = Field(default_factory=list)
    invalid_equipment: List[str] = Field(default_factory=list)
    corrupted_items: List[str] = Field(default_factory=list)
    
    # Suggestions
    suggestions: List[str] = Field(default_factory=list)


# Manager methods available for inventory operations:


class EquipItemRequest(BaseModel):
    """Request to equip an item"""
    item_data: Dict[str, Any] = Field(..., description="GFF item data to equip")
    slot: str = Field(..., description="Equipment slot name")
    inventory_index: Optional[int] = Field(None, description="Index in ItemList (prevents duplication)")


class EquipItemResponse(BaseModel):
    """Response after equipping an item"""
    success: bool
    warnings: List[str] = Field(default_factory=list)
    message: str
    has_unsaved_changes: bool = True


class UnequipItemRequest(BaseModel):
    """Request to unequip an item"""
    slot: str = Field(..., description="Equipment slot to unequip")


class UnequipItemResponse(BaseModel):
    """Response after unequipping an item"""
    success: bool
    item_data: Optional[Dict[str, Any]] = None
    message: str
    has_unsaved_changes: bool = True


class AddToInventoryRequest(BaseModel):
    """Request to add item to inventory"""
    item_data: Dict[str, Any] = Field(..., description="GFF item data to add")


class AddToInventoryResponse(BaseModel):
    """Response after adding to inventory"""
    success: bool
    message: str
    has_unsaved_changes: bool = True
    item_index: Optional[int] = None


class RemoveFromInventoryRequest(BaseModel):
    """Request to remove item from inventory by index"""
    item_index: int = Field(..., description="Index of item in inventory list")


class RemoveFromInventoryResponse(BaseModel):
    """Response after removing from inventory"""
    success: bool
    item_data: Optional[Dict[str, Any]] = None
    message: str
    has_unsaved_changes: bool = True


class InventorySummary(BaseModel):
    """Summary from get_inventory_summary()"""
    total_items: int
    inventory_items: List[Dict[str, Any]]
    equipped_items: Dict[str, Dict[str, Any]]
    custom_items: List[Dict[str, Any]]
    encumbrance: Dict[str, Any]








class CarryCapacity(BaseModel):
    """Character carrying capacity information"""
    strength_score: int = Field(..., description="Character's Strength score")
    size_modifier: float = Field(1.0, description="Size modifier to carrying capacity")
    
    # Capacity limits
    light_load: float = Field(..., description="Light load limit")
    medium_load: float = Field(..., description="Medium load limit")
    heavy_load: float = Field(..., description="Heavy load limit")
    max_capacity: float = Field(..., description="Maximum carrying capacity")
    
    # Current status
    current_load: float = Field(..., description="Current carried weight")
    encumbrance_level: Literal['light', 'medium', 'heavy', 'overloaded'] = Field(..., description="Current encumbrance level")
    
    # Penalties
    movement_penalty: int = Field(0, description="Movement speed penalty percentage")
    skill_penalty: int = Field(0, description="Armor check penalty equivalent")
    max_dex_bonus: Optional[int] = Field(None, description="Max Dex bonus to AC")


# Models matching manager method return values:

class EquipmentInfoResponse(BaseModel):
    """Response from get_equipment_info()"""
    equipment: Dict[str, Dict[str, Any]]


class EquipmentBonusesResponse(BaseModel):
    """Response from get_equipment_bonuses()"""
    bonuses: Dict[str, Dict[str, int]]


class AllWeaponsResponse(BaseModel):
    """Response from get_all_weapons()"""
    weapons: List[Dict[str, Any]]


class AllArmorResponse(BaseModel):
    """Response from get_all_armor()"""
    armor: List[Dict[str, Any]]


class CustomItemsResponse(BaseModel):
    """Response from get_custom_items()"""
    custom_items: List[Dict[str, Any]]


class FilterItemsResponse(BaseModel):
    """Response from filter_items_by_type()"""
    items: List[Dict[str, Any]]


class EquipmentSummaryResponse(BaseModel):
    """Response from get_equipment_summary_by_slot()"""
    equipment_summary: Dict[str, Optional[Dict[str, Any]]]


class EncumbranceResponse(BaseModel):
    """Response from calculate_encumbrance()"""
    encumbrance: Dict[str, Any]


class ACBonusResponse(BaseModel):
    """Response from get_ac_bonus()"""
    ac_bonus: int


class SaveBonusesResponse(BaseModel):
    """Response from get_save_bonuses()"""
    save_bonuses: Dict[str, int]


class AttributeBonusesResponse(BaseModel):
    """Response from get_attribute_bonuses()"""
    attribute_bonuses: Dict[str, int]


class SkillBonusesResponse(BaseModel):
    """Response from get_skill_bonuses()"""
    skill_bonuses: Dict[str, int]


class InventorySummaryResponse(BaseModel):
    """Response from get_inventory_summary()"""
    summary: InventorySummary


class UpdateGoldRequest(BaseModel):
    """Request to update character's gold"""
    gold: int = Field(..., ge=0, le=2147483647, description="Gold amount (0 to 2,147,483,647)")


class UpdateGoldResponse(BaseModel):
    """Response after updating gold"""
    success: bool
    gold: int
    message: str
    has_unsaved_changes: bool = True


class UpdateItemRequest(BaseModel):
    """Request to update an item in inventory or equipment"""
    item_index: Optional[int] = Field(None, description="Index in ItemList if in inventory")
    slot: Optional[str] = Field(None, description="Slot name if equipped")
    item_data: Dict[str, Any] = Field(..., description="Full GFF item data")


class UpdateItemResponse(BaseModel):
    """Response after updating an item"""
    success: bool
    message: str
    has_unsaved_changes: bool = True


class AddItemByBaseTypeRequest(BaseModel):
    """Request to add a new item by base type ID"""
    base_item_id: int = Field(..., description="Base item ID from baseitems.2da")


class PropertyMetadata(BaseModel):
    """Metadata for a single property type"""
    id: int
    label: str
    description: str
    has_subtype: bool
    has_cost_table: bool
    has_param1: bool
    subtype_options: Optional[Dict[int, str]] = None
    cost_table_options: Optional[Dict[int, str]] = None
    param1_options: Optional[Dict[int, str]] = None


class ItemEditorMetadataResponse(BaseModel):
    """Metadata for the item editor UI"""
    property_types: List[PropertyMetadata]
    abilities: Dict[int, str]
    skills: Dict[int, str]
    damage_types: Dict[int, str]
    alignment_groups: Dict[int, str]
    racial_groups: Dict[int, str]
    saving_throws: Dict[int, str]
    immunity_types: Dict[int, str]
    classes: Dict[int, str]
    spells: Dict[int, str]