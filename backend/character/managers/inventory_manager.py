"""
Data-Driven Inventory Manager - handles equipment effects, calculations, and item management
Uses DynamicGameDataLoader for mod-compatible item data
Provides informational data about proficiencies and requirements (no restrictions)
Focuses on save corruption prevention rather than game rule enforcement
"""

from typing import Dict, List, Set, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, FeatChangedEvent
from ..custom_content import CustomContentDetector
from ..services.item_property_decoder import ItemPropertyDecoder
from gamedata.dynamic_loader.field_mapping_utility import field_mapper


class InventoryManager(EventEmitter):
    """Data-driven manager for character inventory, equipment effects, and item information"""

    SLOT_INDEX_MAPPING = {
        0: 'head',
        1: 'chest',
        2: 'boots',
        3: 'gloves',
        4: 'right_hand',
        5: 'left_hand',
        6: 'cloak',
        7: 'left_ring',
        8: 'right_ring',
        9: 'neck',
        10: 'belt',
        11: 'arrows',
        12: 'bullets',
        13: 'bolts',
    }

    SLOT_TO_INDEX = {v: k for k, v in SLOT_INDEX_MAPPING.items()}

    def __init__(self, character_manager):
        """
        Initialize the data-driven InventoryManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_rules_service = character_manager.rules_service
        self.content_detector = CustomContentDetector(None)
        self.property_decoder = ItemPropertyDecoder(character_manager.rules_service)

        self._register_event_handlers()

        self._item_cache = {}
        self._proficiency_cache = set()
        self._feat_proficiency_map = {}
        self._proficiency_reverse_map = {}
        self._base_item_cache = {}

        self._build_proficiency_mappings()
        self._update_proficiency_cache()
    
    def _get_item_name(self, item: Dict[str, Any]) -> str:
        """Get the proper item name from LocalizedName or fallback to base item label"""
        localized_name = item.get('LocalizedName')
        if localized_name and isinstance(localized_name, dict):
            string_ref = localized_name.get('string_ref')
            if string_ref is not None and string_ref != 4294967295:
                try:
                    resolved_name = self.game_rules_service.rm.get_string(string_ref)
                    if resolved_name and not resolved_name.startswith('{StrRef:'):
                        return resolved_name
                except Exception:
                    pass

        base_item = item.get('BaseItem', 0)
        base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
        if base_item_data:
            return field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}')
        else:
            return f'Custom Item {base_item}'
    
    def _build_proficiency_mappings(self):
        """Build dynamic mapping of feat IDs to proficiency types"""
        self._feat_proficiency_map.clear()
        self._proficiency_reverse_map.clear()

        feats = self.game_rules_service.get_table('feat')

        proficiency_patterns = {
            'weapon_simple': ['weapon proficiency (simple)', 'simple weapon proficiency', 'wpnprofsimple'],
            'weapon_martial': ['weapon proficiency (martial)', 'martial weapon proficiency', 'wpnprofmartial'],
            'weapon_exotic': ['weapon proficiency (exotic)', 'exotic weapon proficiency', 'wpnprofexotic'],
            'weapon_rogue': ['weapon proficiency (rogue)', 'rogue weapon proficiency', 'wpnprofrogue'],
            'weapon_wizard': ['weapon proficiency (wizard)', 'wizard weapon proficiency', 'wpnprofwizard'],
            'weapon_elf': ['weapon proficiency (elf)', 'elf weapon proficiency', 'wpnprofelf'],
            'weapon_druid': ['weapon proficiency (druid)', 'druid weapon proficiency', 'wpnprofdruid'],
            'weapon_monk': ['weapon proficiency (monk)', 'monk weapon proficiency', 'wpnprofmonk'],
            'armor_light': ['armor proficiency (light)', 'light armor proficiency', 'armproflgt'],
            'armor_medium': ['armor proficiency (medium)', 'medium armor proficiency', 'armprofmed'],
            'armor_heavy': ['armor proficiency (heavy)', 'heavy armor proficiency', 'armprfhvy', 'armprohvy'],
            'shield': ['shield proficiency', 'shield'],
            'tower_shield': ['tower shield proficiency', 'towershield']
        }

        for feat_id, feat_data in enumerate(feats):
            if feat_data is None:
                continue

            feat_name = field_mapper.get_field_value(feat_data, 'label', '').lower()

            for prof_type, patterns in proficiency_patterns.items():
                if any(pattern in feat_name for pattern in patterns):
                    self._feat_proficiency_map[feat_id] = prof_type
                    self._proficiency_reverse_map[prof_type] = feat_id
                    break

        logger.info(f"Built proficiency mapping for {len(self._feat_proficiency_map)} feats")
    
    def _register_event_handlers(self):
        """Register handlers for relevant events"""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self.on_feat_added)
        self.character_manager.on(EventType.FEAT_REMOVED, self.on_feat_removed)
    
    def on_class_changed(self, event: ClassChangedEvent):
        """Handle class change event"""
        logger.info(f"InventoryManager handling class change: {event.old_class_id} -> {event.new_class_id}")

        self._update_proficiency_cache()

        equipment_info = self.get_equipment_info()
        logger.info(f"Class changed - equipment remains equipped: {len(equipment_info)} items")
    
    def on_feat_added(self, event: FeatChangedEvent):
        """Handle feat addition event"""
        if self._is_proficiency_feat(event.feat_id):
            logger.info(f"InventoryManager updating proficiencies for feat {event.feat_id}")
            self._update_proficiency_cache()

    def on_feat_removed(self, event: FeatChangedEvent):
        """Handle feat removal event"""
        if self._is_proficiency_feat(event.feat_id):
            logger.info(f"InventoryManager updating proficiencies after feat removal {event.feat_id}")
            self._update_proficiency_cache()
    
    def get_equipped_item(self, slot: str) -> Optional[Dict[str, Any]]:
        """
        Get item equipped in a specific slot from Equip_ItemList array

        Args:
            slot: Slot name (e.g., 'head', 'chest', 'right_hand')

        Returns:
            Item data dict or None
        """
        slot_index = self.SLOT_TO_INDEX.get(slot)
        if slot_index is None:
            return None

        equipped_items = self.gff.get('Equip_ItemList', [])
        if slot_index < len(equipped_items):
            item = equipped_items[slot_index]
            return item if item else None

        return None
    
    def equip_item(self, item_data: Dict[str, Any], slot: str, inventory_index: Optional[int] = None) -> Tuple[bool, List[str]]:
        """
        Equip an item in a specific slot using Equip_ItemList array

        Args:
            item_data: Item data to equip
            slot: Slot to equip in
            inventory_index: Optional index in ItemList to remove item from (prevents duplication)

        Returns:
            (success, list_of_warnings_or_errors)
        """
        warnings = []

        id_exists, id_messages = self.check_item_id_exists(item_data)
        warnings.extend(id_messages)

        slot_index = self.SLOT_TO_INDEX.get(slot)
        if slot_index is None:
            return False, ["Invalid equipment slot"]

        if inventory_index is not None:
            removed_item = self.remove_from_inventory(inventory_index)
            if removed_item is None:
                logger.warning(f"Could not remove item at inventory index {inventory_index}")

        equipped_items = self.gff.get('Equip_ItemList', [])

        while len(equipped_items) <= slot_index:
            equipped_items.append(None)

        current_item = equipped_items[slot_index]

        equipped_items[slot_index] = item_data
        self.gff.set('Equip_ItemList', equipped_items)

        if current_item:
            self.add_to_inventory(current_item)

        logger.info(f"Equipped item in {slot} at index {slot_index}")

        self.character_manager.emit(EventType.ITEM_EQUIPPED, {
            'slot': slot,
            'slot_index': slot_index,
            'item': item_data,
            'swapped_item': current_item
        })

        return True, warnings
    
    def unequip_item(self, slot: str) -> Optional[Dict[str, Any]]:
        """
        Unequip item from a slot using Equip_ItemList array

        Args:
            slot: Slot to unequip from

        Returns:
            The unequipped item data
        """
        slot_index = self.SLOT_TO_INDEX.get(slot)
        if slot_index is None:
            logger.warning(f"Invalid slot name: {slot}")
            return None

        equipped_items = self.gff.get('Equip_ItemList', [])

        if not isinstance(equipped_items, list):
            logger.error(f"Equip_ItemList is not a list: {type(equipped_items)}")
            return None

        if slot_index < len(equipped_items):
            item = equipped_items[slot_index]
            if item:
                equipped_items[slot_index] = None
                self.gff.set('Equip_ItemList', equipped_items)

                self.add_to_inventory(item)

                logger.info(f"Unequipped item from {slot} at index {slot_index}")

                self.character_manager.emit(EventType.ITEM_UNEQUIPPED, {
                    'slot': slot,
                    'slot_index': slot_index,
                    'item': item
                })

                return item

        return None
    
    def add_to_inventory(self, item_data: Dict[str, Any]) -> bool:
        """Add an item to inventory"""
        item_list = self.gff.get('ItemList', [])

        base_item = item_data.get('BaseItem', 0)
        base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

        if base_item_data:
            try:
                stacking = int(field_mapper.get_field_value(base_item_data, 'stacking', 0) or 0)
            except (ValueError, TypeError):
                stacking = 0
            if stacking > 1:
                for existing_item in item_list:
                    if existing_item.get('BaseItem') == base_item:
                        existing_stack = existing_item.get('StackSize', 1)
                        new_stack = item_data.get('StackSize', 1)
                        max_stack = stacking

        equipped_items = self.gff.get('Equip_ItemList', [])
        for slot_index, item in enumerate(equipped_items):
            if item and slot_index in self.SLOT_INDEX_MAPPING:
                slot_name = self.SLOT_INDEX_MAPPING[slot_index]
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

                results[slot_name] = {
                    'item': item,
                    'base_item': base_item,
                    'is_custom': base_item_data is None,
                    'name': field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}') if base_item_data else f'Custom Item {base_item}'
                }

        return results
    
    def _is_proficiency_feat(self, feat_id: int) -> bool:
        """Check if a feat grants proficiencies using dynamic mapping"""
        if feat_id in self._feat_proficiency_map:
            return True

        feat_data = self.game_rules_service.get_by_id('feat', feat_id)
        if feat_data:
            feat_label = field_mapper.get_field_value(feat_data, 'label', '').lower()
            return any(prof in feat_label for prof in [
                'weapon focus', 'weapon specialization',
                'improved critical', 'proficiency'
            ])

        return False
    
    def _update_proficiency_cache(self):
        """Update cached proficiency information"""
        self._proficiency_cache.clear()

        feat_list = self.gff.get('FeatList', [])
        feat_ids = {f.get('Feat') for f in feat_list}

        self._proficiency_cache.update(feat_ids)

        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            class_proficiencies = self._get_class_proficiencies(class_id)
            self._proficiency_cache.update(class_proficiencies)
    
    def _get_class_proficiencies(self, class_id: int) -> Set[int]:
        """Get proficiency feats granted by a class using dynamic data"""
        proficiencies = set()
        class_data = self.game_rules_service.get_by_id('classes', class_id)

        if not class_data:
            return proficiencies

        class_label = field_mapper.get_field_value(class_data, 'label', '').lower()

        def get_prof_feat_id(prof_type: str) -> Optional[int]:
            return self._proficiency_reverse_map.get(prof_type)

        simple_prof = get_prof_feat_id('weapon_simple')
        if simple_prof:
            proficiencies.add(simple_prof)

        if any(c in class_label for c in ['fighter', 'ranger', 'paladin', 'barbarian']):
            martial_prof = get_prof_feat_id('weapon_martial')
            light_armor = get_prof_feat_id('armor_light')
            medium_armor = get_prof_feat_id('armor_medium')
            heavy_armor = get_prof_feat_id('armor_heavy')
            shield_prof = get_prof_feat_id('shield')

            for prof in [martial_prof, light_armor, medium_armor, heavy_armor, shield_prof]:
                if prof:
                    proficiencies.add(prof)

        elif any(c in class_label for c in ['cleric', 'druid', 'bard']):
            light_armor = get_prof_feat_id('armor_light')
            medium_armor = get_prof_feat_id('armor_medium')

            for prof in [light_armor, medium_armor]:
                if prof:
                    proficiencies.add(prof)

            if 'cleric' in class_label:
                shield_prof = get_prof_feat_id('shield')
                if shield_prof:
                    proficiencies.add(shield_prof)

        elif any(c in class_label for c in ['rogue', 'warlock']):
            light_armor = get_prof_feat_id('armor_light')
            if light_armor:
                proficiencies.add(light_armor)

            if 'rogue' in class_label:
                rogue_prof = get_prof_feat_id('weapon_rogue')
                if rogue_prof:
                    proficiencies.add(rogue_prof)

        if 'wizard' in class_label or 'sorcerer' in class_label:
            wizard_prof = get_prof_feat_id('weapon_wizard')
            if wizard_prof:
                proficiencies.add(wizard_prof)
        elif 'druid' in class_label:
            druid_prof = get_prof_feat_id('weapon_druid')
            if druid_prof:
                proficiencies.add(druid_prof)
        elif 'monk' in class_label:
            monk_prof = get_prof_feat_id('weapon_monk')
            if monk_prof:
                proficiencies.add(monk_prof)
        
        return proficiencies
    
    def get_item_slot_info(self, base_item_data: Any, slot: str) -> Dict[str, Any]:
        """Get informational data about item-slot compatibility"""
        if not base_item_data:
            return {'item_type': 0, 'is_typical_for_slot': False, 'slot_name': slot}

        try:
            item_type = int(field_mapper.get_field_value(base_item_data, 'base_item', 0) or 0)
        except (ValueError, TypeError):
            item_type = 0

        typical_types = {
            'head': [85],
            'chest': [16],
            'boots': [26],
            'arms': [36],
            'cloak': [30],
            'belt': [21],
            'neck': [1],
            'left_ring': [52],
            'right_ring': [52],
            'right_hand': list(range(0, 60)),
            'left_hand': list(range(0, 60)) + [29],
        }

        typical_for_slot = item_type in typical_types.get(slot, [])

        return {
            'item_type': item_type,
            'is_typical_for_slot': typical_for_slot,
            'slot_name': slot,
            'note': 'This item type is not typical for this slot' if not typical_for_slot else None
        }
    
    def get_item_proficiency_info(self, base_item_data: Any) -> Dict[str, Any]:
        """Get informational data about item proficiency requirements"""
        if not base_item_data:
            return {'has_proficiency_requirements': False}

        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            return {'has_proficiency_requirements': False, 'note': 'FeatManager not available'}

        item_type = field_mapper.get_field_value(base_item_data, 'BaseItem', 0)
        weapon_type = field_mapper.get_field_value(base_item_data, 'WeaponType', 0)

        return feat_manager.get_item_proficiency_requirements(base_item_data, item_type, weapon_type)
    
    def get_item_ability_requirements(self, item_data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Get informational data about ability score requirements"""
        _ = item_data
        return []

    def get_item_class_requirements(self, item_data: Dict[str, Any]) -> List[str]:
        """Get informational data about class requirements"""
        _ = item_data
        return []

    def get_item_alignment_requirements(self, item_data: Dict[str, Any]) -> List[str]:
        """Get informational data about alignment requirements"""
        _ = item_data
        return []
    
    def calculate_encumbrance(self) -> Dict[str, Any]:
        """Calculate character's encumbrance level using dynamic data"""
        total_weight = 0.0

        equipped_items = self.gff.get('Equip_ItemList', [])
        for slot_index, item in enumerate(equipped_items):
            if item and slot_index in self.SLOT_INDEX_MAPPING:
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
                if base_item_data:
                    try:
                        tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                    except (ValueError, TypeError):
                        tenth_lbs = 0.0
                    if tenth_lbs > 0:
                        weight = tenth_lbs / 10.0
                        total_weight += weight

        item_list = self.gff.get('ItemList', [])
        for item in item_list:
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
            if base_item_data:
                try:
                    tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                except (ValueError, TypeError):
                    tenth_lbs = 0.0
                if tenth_lbs > 0:
                    weight = tenth_lbs / 10.0
                    stack_size = item.get('StackSize', 1)
                    total_weight += weight * stack_size

        strength = self.gff.get('Str', 10)

        try:
            encumbrance_data = self.game_rules_service.get_by_id('encumbrance', strength)
            if encumbrance_data:
                light_load = float(field_mapper._safe_int(field_mapper.get_field_value(encumbrance_data, 'light', strength * 3.3)))
                medium_load = float(field_mapper._safe_int(field_mapper.get_field_value(encumbrance_data, 'medium', strength * 6.6)))
                heavy_load = float(field_mapper._safe_int(field_mapper.get_field_value(encumbrance_data, 'heavy', strength * 10)))
            else:
                light_load = float(strength * 3.3)
                medium_load = float(strength * 6.6)
                heavy_load = float(strength * 10)
        except Exception:
            light_load = float(strength * 3.3)
            medium_load = float(strength * 6.6)
            heavy_load = float(strength * 10)

        if total_weight <= light_load:
            level = 'light'
        elif total_weight <= medium_load:
            level = 'medium'
        elif total_weight <= heavy_load:
            level = 'heavy'
        else:
            level = 'overloaded'

        return {
            'total_weight': float(total_weight),
            'light_load': float(light_load),
            'medium_load': float(medium_load),
            'heavy_load': float(heavy_load),
            'encumbrance_level': level
        }
    
    def get_inventory_summary(self) -> Dict[str, Any]:
        """Get summary of character's inventory using dynamic data"""
        item_list = self.gff.get('ItemList', [])

        inventory_items = []
        for idx, item in enumerate(item_list):
            if item:
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
                is_custom = base_item_data is None

                item_name = self._get_item_name(item)

                decoded_properties = self.get_item_property_descriptions(item)

                description = None
                localized_desc = item.get('DescIdentified')
                if localized_desc and isinstance(localized_desc, dict):
                    string_ref = localized_desc.get('string_ref')
                    if string_ref is not None and string_ref != 4294967295:
                        try:
                            resolved_desc = self.game_rules_service.rm.get_string(string_ref)
                            if resolved_desc and not resolved_desc.startswith('{StrRef:'):
                                description = resolved_desc
                        except Exception:
                            pass

                weight = 0.0
                if base_item_data:
                    try:
                        tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                        if tenth_lbs > 0:
                            weight = tenth_lbs / 10.0
                            stack_size = item.get('StackSize', 1)
                            if stack_size > 1:
                                weight *= stack_size
                    except (ValueError, TypeError):
                        weight = 0.0

                value = 0
                try:
                    item_cost = item.get('Cost', 0)
                    modify_cost = item.get('ModifyCost', 0)
                    value = int(item_cost) + int(modify_cost)
                except (ValueError, TypeError):
                    value = 0

                inventory_items.append({
                    'index': idx,
                    'item': item,
                    'base_item': base_item,
                    'name': item_name,
                    'description': description,
                    'weight': weight,
                    'value': value,
                    'is_custom': is_custom,
                    'stack_size': item.get('StackSize', 1),
                    'enhancement': item.get('Enhancement', 0),
                    'charges': item.get('Charges'),
                    'identified': item.get('Identified', 1) != 0,
                    'plot': item.get('Plot', 0) == 1,
                    'cursed': item.get('Cursed', 0) == 1,
                    'stolen': item.get('Stolen', 0) == 1,
                    'decoded_properties': decoded_properties,
                    'base_ac': self._get_item_base_ac(item)
                })
        
        summary = {
            'total_items': len(item_list),
            'inventory_items': inventory_items,
            'equipped_items': {},
            'custom_items': [],
            'encumbrance': self.calculate_encumbrance()
        }

        equipped_items_list = self.gff.get('Equip_ItemList', [])

        for slot_index, item in enumerate(equipped_items_list):
            if item and slot_index in self.SLOT_INDEX_MAPPING:
                slot_name = self.SLOT_INDEX_MAPPING[slot_index]
                base_item = item.get('BaseItem', 0)

                base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
                is_custom = base_item_data is None

                item_name = self._get_item_name(item)

                decoded_properties = self.get_item_property_descriptions(item)

                description = None
                localized_desc = item.get('DescIdentified')
                if localized_desc and isinstance(localized_desc, dict):
                    string_ref = localized_desc.get('string_ref')
                    if string_ref is not None and string_ref != 4294967295:
                        try:
                            resolved_desc = self.game_rules_service.rm.get_string(string_ref)
                            if resolved_desc and not resolved_desc.startswith('{StrRef:'):
                                description = resolved_desc
                        except Exception:
                            pass

                weight = 0.0
                if base_item_data:
                    try:
                        tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                        if tenth_lbs > 0:
                            weight = tenth_lbs / 10.0
                    except (ValueError, TypeError):
                        weight = 0.0

                value = 0
                try:
                    item_cost = item.get('Cost', 0)
                    modify_cost = item.get('ModifyCost', 0)
                    value = int(item_cost) + int(modify_cost)
                except (ValueError, TypeError):
                    value = 0

                summary['equipped_items'][slot_name] = {
                    'base_item': base_item,
                    'custom': is_custom,
                    'name': item_name,
                    'description': description,
                    'weight': weight,
                    'value': value,
                    'item_data': item,
                    'decoded_properties': decoded_properties,
                    'base_ac': self._get_item_base_ac(item)
                }
                
                if is_custom:
                    summary['custom_items'].append({
                        'slot': slot_name,
                        'base_item': base_item,
                        'name': item_name
                    })
        
        return summary
    
    def get_equipment_bonuses(self) -> Dict[str, Dict[str, int]]:
        """Calculate all equipment bonuses for other managers to use"""
        bonuses = {
            'ac': {'armor': 0, 'shield': 0, 'deflection': 0, 'natural': 0},
            'saves': {'fortitude': 0, 'reflex': 0, 'will': 0},
            'attributes': {'Str': 0, 'Dex': 0, 'Con': 0, 'Int': 0, 'Wis': 0, 'Cha': 0},
            'skills': {},
            'combat': {'attack': 0, 'damage': 0},
            'misc': {}
        }

        equipped_items_list = self.gff.get('Equip_ItemList', [])

        for item in equipped_items_list:
            if item:
                item_bonuses = self._calculate_item_bonuses(item)

                for category, category_bonuses in item_bonuses.items():
                    if category in bonuses:
                        for bonus_type, value in category_bonuses.items():
                            if bonus_type in bonuses[category]:
                                bonuses[category][bonus_type] += value
                            else:
                                bonuses[category][bonus_type] = value

        return bonuses
    
    def get_ac_bonus(self) -> int:
        """Get total AC bonus from equipment"""
        bonuses = self.get_equipment_bonuses()
        return sum(bonuses['ac'].values())
    
    def get_save_bonuses(self) -> Dict[str, int]:
        """Get saving throw bonuses from equipment"""
        bonuses = self.get_equipment_bonuses()
        return bonuses['saves']
    
    def get_attribute_bonuses(self) -> Dict[str, int]:
        """Get attribute bonuses from equipment"""
        bonuses = self.get_equipment_bonuses()
        return bonuses['attributes']
    
    def get_skill_bonuses(self) -> Dict[str, int]:
        """Get skill bonuses from equipment"""
        bonuses = self.get_equipment_bonuses()
        return bonuses['skills']
    
    def _get_item_base_ac(self, item_data: Dict[str, Any]) -> Optional[int]:
        """Get the base AC value for armor/shield items"""
        base_item = item_data.get('BaseItem', 0)
        base_item_int = int(base_item) if base_item else 0

        if base_item_int == 16 or base_item_int in [14, 56, 57]:
            armor_rules_type = item_data.get('ArmorRulesType', 0)
            if armor_rules_type is not None:
                armor_stats = self.game_rules_service.get_by_id('armorrulestats', armor_rules_type)
                if armor_stats:
                    ac_value = field_mapper.get_field_value(armor_stats, 'ACBONUS', 0)
                    try:
                        ac_int = int(ac_value) if ac_value else 0
                        if ac_int > 0:
                            return ac_int
                    except (ValueError, TypeError):
                        pass
        return None

    def _calculate_item_bonuses(self, item_data: Dict[str, Any]) -> Dict[str, Dict[str, int]]:
        """Calculate bonuses provided by a single item"""
        bonuses = {
            'ac': {},
            'saves': {},
            'attributes': {},
            'skills': {},
            'combat': {},
            'misc': {}
        }

        base_item = item_data.get('BaseItem', 0)
        base_item_int = int(base_item) if base_item else 0

        if base_item_int == 16:
            armor_rules_type = item_data.get('ArmorRulesType', 0)
            if armor_rules_type is not None:
                armor_stats = self.game_rules_service.get_by_id('armorrulestats', armor_rules_type)
                if armor_stats:
                    ac_value = field_mapper.get_field_value(armor_stats, 'ACBONUS', 0)
                    try:
                        ac_int = int(ac_value) if ac_value else 0
                        if ac_int > 0:
                            bonuses['ac']['armor'] = ac_int
                    except (ValueError, TypeError):
                        pass

        elif base_item_int in [14, 56, 57]:
            armor_rules_type = item_data.get('ArmorRulesType', 0)
            if armor_rules_type is not None:
                shield_stats = self.game_rules_service.get_by_id('armorrulestats', armor_rules_type)
                if shield_stats:
                    ac_value = field_mapper.get_field_value(shield_stats, 'ACBONUS', 0)
                    try:
                        ac_int = int(ac_value) if ac_value else 0
                        if ac_int > 0:
                            bonuses['ac']['shield'] = ac_int
                    except (ValueError, TypeError):
                        pass

        properties = item_data.get('PropertiesList', [])
        for prop in properties:
            prop_bonuses = self._parse_item_property(prop)

            for category, category_bonuses in prop_bonuses.items():
                if category not in bonuses:
                    bonuses[category] = {}
                for bonus_type, value in category_bonuses.items():
                    if bonus_type in bonuses[category]:
                        bonuses[category][bonus_type] += value
                    else:
                        bonuses[category][bonus_type] = value

        return bonuses
    
    def _parse_item_property(self, property_data: Dict[str, Any]) -> Dict[str, Dict[str, int]]:
        """Parse a single item property for bonuses using the property decoder"""
        bonuses = {
            'ac': {},
            'saves': {},
            'attributes': {},
            'skills': {},
            'combat': {},
            'misc': {}
        }

        decoded_bonuses = self.property_decoder.get_item_bonuses([property_data])

        bonuses['ac'] = decoded_bonuses.get('ac', {})
        bonuses['saves'] = decoded_bonuses.get('saves', {})
        bonuses['skills'] = decoded_bonuses.get('skills', {})
        bonuses['combat'] = decoded_bonuses.get('combat', {})
        bonuses['misc'] = decoded_bonuses.get('special', {})

        bonuses['attributes'].update(decoded_bonuses.get('abilities', {}))

        for immunity in decoded_bonuses.get('immunities', []):
            bonuses['misc'][f'immunity_{immunity}'] = 1

        return bonuses
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate inventory for corruption prevention only"""
        errors = []

        equipment_info = self.get_equipment_info()
        for slot, info in equipment_info.items():
            if info['is_custom']:
                logger.info(f"Custom item in {slot}: BaseItem {info['base_item']}")

        item_list = self.gff.get('ItemList', [])
        for idx, item in enumerate(item_list):
            stack_size = item.get('StackSize', 1)
            if stack_size < 0:
                errors.append(f"Inventory item {idx}: Invalid negative stack size {stack_size}")
            elif stack_size > 999:
                errors.append(f"Inventory item {idx}: Stack size {stack_size} too large (max 999)")

        return len(errors) == 0, errors

    def get_all_weapons(self) -> List[Dict[str, Any]]:
        """Get all available weapons"""
        all_weapons = []
        base_items = self.game_rules_service.get_table('baseitems')

        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue

            try:
                item_type = int(field_mapper.get_field_value(base_item_data, 'base_item', 0) or 0)
            except (ValueError, TypeError):
                item_type = 0

            if item_type < 60:
                weapon_name = field_mapper.get_field_value(base_item_data, 'label', f'Weapon {item_id}')
                proficiency_info = self.get_item_proficiency_info(base_item_data)
                
                all_weapons.append({
                    'id': item_id,
                    'name': weapon_name,
                    'type': item_type,
                    'weapon_type': field_mapper.get_field_value(base_item_data, 'WeaponType', 0),
                    'proficiency_info': proficiency_info
                })
        
        return all_weapons
    
    def get_all_armor(self) -> List[Dict[str, Any]]:
        """Get all available armor and shields"""
        all_armor = []
        base_items = self.game_rules_service.get_table('baseitems')

        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue

            try:
                item_type = int(field_mapper.get_field_value(base_item_data, 'base_item', 0) or 0)
            except (ValueError, TypeError):
                item_type = 0

            if item_type in [16, 29]:
                armor_name = field_mapper.get_field_value(base_item_data, 'label', f'Armor {item_id}')
                proficiency_info = self.get_item_proficiency_info(base_item_data)
                
                all_armor.append({
                    'id': item_id,
                    'name': armor_name,
                    'type': item_type,
                    'ac_type': field_mapper.get_field_value(base_item_data, 'ACType', 0),
                    'ac_value': field_mapper.get_field_value(base_item_data, 'BaseAC', 0),
                    'proficiency_info': proficiency_info
                })
        
        return all_armor
    
    def filter_items_by_type(self, item_type: int) -> List[Dict[str, Any]]:
        """Filter base items by type"""
        filtered_items = []
        base_items = self.game_rules_service.get_table('baseitems')
        
        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue
                
            if field_mapper.get_field_value(base_item_data, 'BaseItem', 0) == item_type:
                item_name = field_mapper.get_field_value(base_item_data, 'label', f'Item {item_id}')
                filtered_items.append({
                    'id': item_id,
                    'name': item_name,
                    'type': item_type
                })
        
        return filtered_items
    
    def get_custom_items(self) -> List[Dict[str, Any]]:
        """Get all custom/mod items in character's possession"""
        custom_items = []

        equipped_items = self.gff.get('Equip_ItemList', [])
        for slot_index, item in enumerate(equipped_items):
            if item and slot_index in self.SLOT_INDEX_MAPPING:
                slot_name = self.SLOT_INDEX_MAPPING[slot_index]
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

                if base_item_data is None:
                    custom_items.append({
                        'location': f'equipped_{slot_name}',
                        'base_item': base_item,
                        'item_data': item
                    })

        item_list = self.gff.get('ItemList', [])
        for idx, item in enumerate(item_list):
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

            if base_item_data is None:
                custom_items.append({
                    'location': f'inventory_{idx}',
                    'base_item': base_item,
                    'item_data': item
                })

        return custom_items
    
    def has_custom_content(self) -> bool:
        """Check if character has any custom/mod items"""
        return len(self.get_custom_items()) > 0
    
    def get_equipment_summary_by_slot(self) -> Dict[str, Optional[Dict[str, Any]]]:
        """Get detailed summary of equipped items by slot"""
        equipment_summary = {}

        equipped_items = self.gff.get('Equip_ItemList', [])
        for slot_index, item in enumerate(equipped_items):
            if slot_index in self.SLOT_INDEX_MAPPING:
                slot_name = self.SLOT_INDEX_MAPPING[slot_index]
                if item:
                    base_item = item.get('BaseItem', 0)
                    base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

                    equipment_summary[slot_name] = {
                        'base_item': base_item,
                        'item_data': item,
                        'is_custom': base_item_data is None,
                        'name': field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}') if base_item_data else f'Custom Item {base_item}',
                        'bonuses': self._calculate_item_bonuses(item)
                    }
                else:
                    equipment_summary[slot_name] = None

        return equipment_summary
    
    def get_item_property_descriptions(self, item_data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Get human-readable descriptions of all item properties"""
        properties = item_data.get('PropertiesList', [])
        if not properties:
            return []

        return self.property_decoder.decode_all_properties(properties)

    def get_enhanced_item_summary(self, item_data: Dict[str, Any]) -> Dict[str, Any]:
        """Get enhanced item summary with decoded properties"""
        base_item = item_data.get('BaseItem', 0)
        base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

        item_name = self._get_item_name(item_data)
        is_custom = base_item_data is None

        property_descriptions = self.get_item_property_descriptions(item_data)

        bonuses = self._calculate_item_bonuses(item_data)

        return {
            'name': item_name,
            'base_item': base_item,
            'is_custom': is_custom,
            'enhancement': item_data.get('Enhancement', 0),
            'charges': item_data.get('Charges'),
            'identified': item_data.get('Identified', 1) != 0,
            'plot': item_data.get('Plot', 0) == 1,
            'cursed': item_data.get('Cursed', 0) == 1,
            'stolen': item_data.get('Stolen', 0) == 1,
            'stack_size': item_data.get('StackSize', 1),
            'properties': property_descriptions,
            'bonuses': bonuses,
            'raw_data': item_data
        }

    def remove_from_inventory(self, item_index: int) -> Tuple[bool, Optional[Dict[str, Any]], str]:
        """
        Remove an item from inventory by index
        
        Args:
            item_index: Index in ItemList
            
        Returns:
            (success, item_data, message)
        """
        item_list = self.gff.get('ItemList', [])
        
        if not isinstance(item_list, list):
            return False, None, "Inventory is corrupted (ItemList not a list)"
            
        if item_index < 0 or item_index >= len(item_list):
            return False, None, f"Invalid item index {item_index}"
            
        try:
            item = item_list.pop(item_index)
            self.gff.set('ItemList', item_list)
            
            self.character_manager.emit(EventType.ITEM_REMOVED, {
                'index': item_index,
                'item': item
            })
            
            logger.info(f"Removed item at index {item_index} from inventory")
            return True, item, "Item removed"
        except Exception as e:
            logger.error(f"Error removing item: {e}")
            return False, None, str(e)

    def check_item_id_exists(self, item_data: Dict[str, Any]) -> Tuple[bool, List[str]]:
        """
        Check if item ID exists/is valid
        
        Args:
            item_data: Item GFF data
            
        Returns:
            (valid, messages)
        """
        # Stub implementation to prevent AttributeError
        # In the future, this could check for ObjectId uniqueness
        return True, []

    def get_equipment_info(self) -> Dict[str, Dict[str, Any]]:
        """
        Get information about all equipped items
        
        Returns:
            Dict mapping slot name to item info
        """
        info = {}
        equipped_items = self.gff.get('Equip_ItemList', [])
        
        for slot_index, item in enumerate(equipped_items):
            if item and slot_index in self.SLOT_INDEX_MAPPING:
                slot_name = self.SLOT_INDEX_MAPPING[slot_index]
                
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
                item_name = self._get_item_name(item)
                
                description = None
                localized_desc = item.get('DescIdentified')
                if localized_desc and isinstance(localized_desc, dict):
                    string_ref = localized_desc.get('string_ref')
                    if string_ref is not None and string_ref != 4294967295:
                        try:
                            resolved_desc = self.game_rules_service.rm.get_string(string_ref)
                            if resolved_desc and not resolved_desc.startswith('{StrRef:'):
                                description = resolved_desc
                        except Exception:
                            pass

                info[slot_name] = {
                    'base_item': base_item,
                    'name': item_name,
                    'description': description,
                    'item_data': item
                }
                
        return info