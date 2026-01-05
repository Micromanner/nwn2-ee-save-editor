"""Manages character inventory, equipment effects, and item data."""

from typing import Dict, List, Set, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, FeatChangedEvent
from ..custom_content import CustomContentDetector
from services.gamedata.item_property_decoder import ItemPropertyDecoder
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

    SLOT_BITMASK_MAPPING = {
        0x0001: 'head',
        0x0002: 'chest',
        0x0004: 'boots',
        0x0008: 'gloves',
        0x0010: 'right_hand',
        0x0020: 'left_hand',
        0x0040: 'cloak',
        0x0080: 'left_ring',
        0x0100: 'right_ring',
        0x0200: 'neck',
        0x0400: 'belt',
        0x0800: 'arrows',
        0x1000: 'bullets',
        0x2000: 'bolts',
    }

    SLOT_TO_BITMASK = {v: k for k, v in SLOT_BITMASK_MAPPING.items()}

    HAND_SLOTS_BITMASK = 0x0030
    AMMO_SLOTS_BITMASK = 0x3800
    ACCESSORY_SLOTS_BITMASK = 0x07CD

    def __init__(self, character_manager):
        """Initialize the InventoryManager."""
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

    def _get_raw_equip_item_list(self) -> List[Tuple[int, Dict[str, Any]]]:
        """Get equipped item list with bitmask-based struct IDs."""
        equipped_items = self.gff.get('Equip_ItemList', [])
        if not equipped_items:
            return []

        result = []
        for item in equipped_items:
            if not item:
                continue
            bitmask = item.get('__struct_id__', 0)
            if bitmask == 0:
                logger.warning("Item in Equip_ItemList missing __struct_id__, skipping")
                continue
            result.append((bitmask, item))

        return result

    def _get_equipped_item_by_bitmask(self, bitmask: int) -> Optional[Dict[str, Any]]:
        """Get equipped item by its slot bitmask."""
        for item_bitmask, item_dict in self._get_raw_equip_item_list():
            if item_bitmask == bitmask:
                return item_dict
        return None

    def _set_equipped_item_by_bitmask(self, bitmask: int, item_data: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        """Set equipped item at slot bitmask, returning previous item."""
        equipped_items = self.gff.get('Equip_ItemList', [])
        if equipped_items is None:
            equipped_items = []

        previous_item = None
        found_index = None

        for i, item in enumerate(equipped_items):
            if item and item.get('__struct_id__') == bitmask:
                previous_item = dict(item)
                found_index = i
                break

        new_item = dict(item_data)
        new_item['__struct_id__'] = bitmask

        if found_index is not None:
            equipped_items[found_index] = new_item
        else:
            equipped_items.append(new_item)

        self.gff.set('Equip_ItemList', equipped_items)
        return previous_item

    def _remove_equipped_item_by_bitmask(self, bitmask: int) -> Optional[Dict[str, Any]]:
        """Remove equipped item at slot bitmask, returning removed item."""
        equipped_items = self.gff.get('Equip_ItemList', [])
        if not equipped_items:
            return None

        for i, item in enumerate(equipped_items):
            if item and item.get('__struct_id__') == bitmask:
                removed_item = dict(item)
                equipped_items.pop(i)
                self.gff.set('Equip_ItemList', equipped_items)
                return removed_item

        return None

    def _resolve_locstring(self, locstring_struct: Dict[str, Any]) -> Optional[str]:
        """Resolve a localized string structure to text"""
        if not locstring_struct or not isinstance(locstring_struct, dict):
            return None
        
        string_ref = locstring_struct.get('string_ref')
        if string_ref is not None and string_ref != 4294967295:
            return self.game_rules_service.rm.get_string(string_ref)

        substrings = locstring_struct.get('substrings', [])
        if substrings:
            return substrings[0].get('value')

        return None

    def _get_item_name(self, item: Dict[str, Any]) -> str:
        """Get item name from LocalizedName or base item label"""
        localized_name = item.get('LocalizedName')
        name = self._resolve_locstring(localized_name)
        if name:
            return name

        base_item = item.get('BaseItem', 0)
        base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
        if base_item_data:
            return field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}')
        
        return f'Custom Item {base_item}'

    def _get_equippable_slots(self, base_item_data: Any) -> List[str]:
        """Get list of slot names where item can be equipped."""
        if not base_item_data:
            return []

        equippable_slots_raw = field_mapper.get_field_value(base_item_data, 'EquipableSlots', '0x00000')
        try:
            if isinstance(equippable_slots_raw, str) and equippable_slots_raw.startswith('0x'):
                equippable_bitmask = int(equippable_slots_raw, 16)
            else:
                equippable_bitmask = int(equippable_slots_raw or 0)
        except (ValueError, TypeError):
            return []

        if equippable_bitmask == 0:
            return []

        slots = []
        for bitmask, slot_name in self.SLOT_BITMASK_MAPPING.items():
            if equippable_bitmask & bitmask:
                slots.append(slot_name)

        return slots

    def _get_default_equip_slot(self, base_item_data: Any) -> Optional[str]:
        """Get default slot for equipping this item."""
        slots = self._get_equippable_slots(base_item_data)
        if not slots:
            return None

        priority_order = ['chest', 'right_hand', 'head', 'neck', 'cloak', 'gloves',
                         'belt', 'boots', 'left_hand', 'left_ring', 'arrows', 'bullets', 'bolts']
        for slot in priority_order:
            if slot in slots:
                return slot

        return slots[0] if slots else None

    STORE_PANEL_CATEGORIES = {
        0: 'Armor & Clothing',
        1: 'Weapons',
        2: 'Magic Items',
        3: 'Accessories',
        4: 'Miscellaneous'
    }

    def _get_item_category(self, base_item_id: int, base_item_data: Any) -> str:
        """
        Determine item category based on baseitems.2da StorePanel column.
        Returns category name matching ITEM_CATEGORIES (Armor & Clothing, Weapons, etc.)
        """
        if not base_item_data:
            return 'Miscellaneous'

        store_panel = field_mapper.get_field_value(base_item_data, 'StorePanel', None)
        if store_panel is not None and store_panel != '****':
            try:
                panel_id = int(store_panel)
                return self.STORE_PANEL_CATEGORIES.get(panel_id, 'Miscellaneous')
            except (ValueError, TypeError):
                pass

        return 'Miscellaneous'

    def _build_proficiency_mappings(self):
        """Build dynamic mapping of feat IDs to proficiency types."""
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
        """Register handlers for relevant events."""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self.on_feat_added)
        self.character_manager.on(EventType.FEAT_REMOVED, self.on_feat_removed)
    
    def on_class_changed(self, event: ClassChangedEvent):
        """Handle class change event."""
        logger.info(f"InventoryManager handling class change: {event.old_class_id} -> {event.new_class_id}")

        self._update_proficiency_cache()

        equipment_info = self.get_equipment_info()
        logger.info(f"Class changed - equipment remains equipped: {len(equipment_info)} items")
    
    def on_feat_added(self, event: FeatChangedEvent):
        """Handle feat addition event."""
        if self._is_proficiency_feat(event.feat_id):
            logger.info(f"InventoryManager updating proficiencies for feat {event.feat_id}")
            self._update_proficiency_cache()

    def on_feat_removed(self, event: FeatChangedEvent):
        """Handle feat removal event."""
        if self._is_proficiency_feat(event.feat_id):
            logger.info(f"InventoryManager updating proficiencies after feat removal {event.feat_id}")
            self._update_proficiency_cache()
    
    def get_equipped_item(self, slot: str) -> Optional[Dict[str, Any]]:
        """Get item equipped in specific slot by name."""
        bitmask = self.SLOT_TO_BITMASK.get(slot)
        if bitmask is None:
            return None

        return self._get_equipped_item_by_bitmask(bitmask)
    
    def equip_item(self, item_data: Dict[str, Any], slot: str, inventory_index: Optional[int] = None) -> Tuple[bool, List[str]]:
        """Equip item in specific slot, optionally removing from inventory."""
        warnings = []

        if slot not in self.SLOT_TO_BITMASK:
             return False, ["Invalid equipment slot"]

        bitmask = self.SLOT_TO_BITMASK[slot]

        if inventory_index is not None:
            removed_item = self.remove_from_inventory(inventory_index)
            if removed_item is None:
                logger.warning(f"Could not remove item at inventory index {inventory_index}")

        current_item = self._set_equipped_item_by_bitmask(bitmask, item_data)

        if current_item:
            self.add_to_inventory(current_item)

        slot_index = self.SLOT_TO_INDEX.get(slot, 0)
        logger.info(f"Equipped item in {slot} (bitmask 0x{bitmask:04X})")

        self.character_manager.emit(EventType.ITEM_EQUIPPED, {
            'slot': slot,
            'slot_index': slot_index,
            'item': item_data,
            'swapped_item': current_item
        })

        return True, warnings
    
    def unequip_item(self, slot: str) -> Optional[Dict[str, Any]]:
        """Unequip item from slot by name."""
        bitmask = self.SLOT_TO_BITMASK.get(slot)
        if bitmask is None:
            logger.warning(f"Invalid slot name: {slot}")
            return None

        item = self._remove_equipped_item_by_bitmask(bitmask)
        if item:
            self.add_to_inventory(item)

            slot_index = self.SLOT_TO_INDEX.get(slot, 0)
            logger.info(f"Unequipped item from {slot} (bitmask 0x{bitmask:04X})")

            self.character_manager.emit(EventType.ITEM_UNEQUIPPED, {
                'slot': slot,
                'slot_index': slot_index,
                'item': item
            })

            return item

        return None
    
    def add_to_inventory(self, item_data: Dict[str, Any]) -> Tuple[bool, Optional[int]]:
        """Add item to inventory, returning success and item index."""
        item_list = self.gff.get('ItemList', [])

        base_item = item_data.get('BaseItem', 0)
        base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

        if base_item_data:
            try:
                stacking = int(field_mapper.get_field_value(base_item_data, 'stacking', 0) or 0)
            except (ValueError, TypeError):
                stacking = 0
            if stacking > 1:
                for idx, existing_item in enumerate(item_list):
                    if existing_item.get('BaseItem') == base_item:
                        existing_stack = existing_item.get('StackSize', 1)
                        new_stack = item_data.get('StackSize', 1)
                        total = existing_stack + new_stack
                        if total <= stacking:
                            existing_item['StackSize'] = total
                            self.gff.set('ItemList', item_list)
                            return True, idx

        new_index = len(item_list)
        item_list.append(item_data)
        self.gff.set('ItemList', item_list)
        return True, new_index
    
    def _is_proficiency_feat(self, feat_id: int) -> bool:
        """Check if feat grants proficiencies using dynamic mapping."""
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
        """Update cached proficiency information."""
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
        """Get proficiency feats granted by class using dynamic data."""
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
        """Get informational data about item-slot compatibility."""
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
        """Get informational data about item proficiency requirements."""
        if not base_item_data:
            return {'has_proficiency_requirements': False}

        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            return {'has_proficiency_requirements': False, 'note': 'FeatManager not available'}

        item_type = field_mapper.get_field_value(base_item_data, 'BaseItem', 0)
        weapon_type = field_mapper.get_field_value(base_item_data, 'WeaponType', 0)

        return feat_manager.get_item_proficiency_requirements(base_item_data, item_type, weapon_type)
    

    
    def calculate_encumbrance(self) -> Dict[str, Any]:
        """Calculate character's encumbrance level."""
        total_weight = 0.0

        for _, item in self._get_raw_equip_item_list():
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
            if base_item_data:
                tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                if tenth_lbs > 0:
                    weight = tenth_lbs / 10.0
                    total_weight += weight

        item_list = self.gff.get('ItemList', [])
        for item in item_list:
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
            if base_item_data:
                tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                if tenth_lbs > 0:
                    weight = tenth_lbs / 10.0
                    stack_size = item.get('StackSize', 1)
                    total_weight += weight * stack_size

        strength = self.gff.get('Str', 10)

        # Strict lookup - if encumbrance table is missing/corrupt, we want to know
        encumbrance_data = self.game_rules_service.get_by_id('encumbrance', strength)
        if encumbrance_data:
            light_load = float(field_mapper._safe_int(field_mapper.get_field_value(encumbrance_data, 'light', strength * 3.3)))
            medium_load = float(field_mapper._safe_int(field_mapper.get_field_value(encumbrance_data, 'medium', strength * 6.6)))
            heavy_load = float(field_mapper._safe_int(field_mapper.get_field_value(encumbrance_data, 'heavy', strength * 10.0)))
        else:
            # Fallback only if table entry missing for specific strength (logic hole), typically shouldn't happen
            # converting to float to match types
            light_load = float(strength * 3.3)
            medium_load = float(strength * 6.6)
            heavy_load = float(strength * 10.0)

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
        """Get summary of character's inventory."""
        if self.gff is None:
            raise ValueError("InventoryManager error: self.gff is None")

        item_list = self.gff.get('ItemList', [])

        inventory_items = []
        for idx, item in enumerate(item_list):
            if not item:
                continue

            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
            is_custom = base_item_data is None

            base_item_name = None
            if base_item_data:
                base_item_name = field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}')

            item_name = self._get_item_name(item)
            decoded_properties = self.get_item_property_descriptions(item)
            description = self._resolve_locstring(item.get('DescIdentified'))

            weight = 0.0
            if base_item_data:
                tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                if tenth_lbs > 0:
                    weight = tenth_lbs / 10.0
                    stack_size = item.get('StackSize', 1)
                    if stack_size > 1:
                        weight *= stack_size

            value = 0
            item_cost = item.get('Cost', 0)
            modify_cost = item.get('ModifyCost', 0)
            value = int(item_cost) + int(modify_cost)

            inventory_items.append({
                'index': idx,
                'item': item,
                'base_item': base_item,
                'base_item_name': base_item_name,
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
                'base_ac': self._get_item_base_ac(item),
                'category': self._get_item_category(base_item, base_item_data),
                'equippable_slots': self._get_equippable_slots(base_item_data),
                'default_slot': self._get_default_equip_slot(base_item_data)
            })
        
        encumbrance = self.calculate_encumbrance()

        summary = {
            'total_items': len(item_list),
            'inventory_items': inventory_items,
            'equipped_items': {},
            'custom_items': [],
            'encumbrance': encumbrance
        }

        for bitmask, item in self._get_raw_equip_item_list():
            if item is None:
                continue
            
            slot_name = self.SLOT_BITMASK_MAPPING.get(bitmask)
            if not slot_name:
                continue
            
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
            is_custom = base_item_data is None

            base_item_name = None
            if base_item_data:
                base_item_name = field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}')

            item_name = self._get_item_name(item)
            decoded_properties = self.get_item_property_descriptions(item)
            description = self._resolve_locstring(item.get('DescIdentified'))

            weight = 0.0
            if base_item_data:
                tenth_lbs = float(field_mapper.get_field_value(base_item_data, 'TenthLBS', 0.0) or 0.0)
                if tenth_lbs > 0:
                    weight = tenth_lbs / 10.0

            value = 0
            item_cost = item.get('Cost', 0)
            modify_cost = item.get('ModifyCost', 0)
            value = int(item_cost) + int(modify_cost)

            summary['equipped_items'][slot_name] = {
                'base_item': base_item,
                'base_item_name': base_item_name,
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
        """Calculate all equipment bonuses."""
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
                                target_val = bonuses[category][bonus_type]
                                
                                # Robust handling for mixed types (e.g. integer sums vs detailed bonus lists)
                                if isinstance(value, list):
                                    if isinstance(target_val, list):
                                        target_val.extend(value)
                                    elif target_val == 0:
                                        bonuses[category][bonus_type] = value
                                    else:
                                        bonuses[category][bonus_type] = [target_val] + value
                                elif isinstance(target_val, list):
                                    target_val.append(value)
                                else:
                                    # Standard integer sum - strict, no fallback
                                    bonuses[category][bonus_type] += int(value)
                            else:
                                bonuses[category][bonus_type] = value

        return bonuses
    
    def get_ac_bonus(self) -> int:
        """Get total AC bonus from equipment."""
        bonuses = self.get_equipment_bonuses()
        return sum(bonuses['ac'].values())
    
    def get_save_bonuses(self) -> Dict[str, int]:
        """Get saving throw bonuses from equipment."""
        bonuses = self.get_equipment_bonuses()
        return bonuses['saves']
    
    def get_attribute_bonuses(self) -> Dict[str, int]:
        """Get attribute bonuses from equipment."""
        bonuses = self.get_equipment_bonuses()
        return bonuses['attributes']
    
    def get_skill_bonuses(self) -> Dict[str, int]:
        """Get skill bonuses from equipment."""
        bonuses = self.get_equipment_bonuses()
        return bonuses['skills']
    
    def _get_item_base_ac(self, item_data: Dict[str, Any]) -> Optional[int]:
        """Get base AC value for armor/shield items."""
        base_item = item_data.get('BaseItem', 0)
        base_item_int = int(base_item) if base_item else 0

        if base_item_int == 16 or base_item_int in [14, 56, 57]:
            armor_rules_type = item_data.get('ArmorRulesType', 0)
            if armor_rules_type is not None:
                armor_stats = self.game_rules_service.get_by_id('armorrulestats', armor_rules_type)
                if armor_stats:
                    ac_value = field_mapper.get_field_value(armor_stats, 'ACBONUS', 0)
                    ac_int = int(ac_value) if ac_value else 0
                    if ac_int > 0:
                        return ac_int
        return None

    def _calculate_item_bonuses(self, item_data: Dict[str, Any]) -> Dict[str, Dict[str, int]]:
        """Calculate bonuses provided by single item."""
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
                    ac_int = int(ac_value) if ac_value else 0
                    if ac_int > 0:
                        bonuses['ac']['armor'] = ac_int

        elif base_item_int in [14, 56, 57]:
            armor_rules_type = item_data.get('ArmorRulesType', 0)
            if armor_rules_type is not None:
                shield_stats = self.game_rules_service.get_by_id('armorrulestats', armor_rules_type)
                if shield_stats:
                    ac_value = field_mapper.get_field_value(shield_stats, 'ACBONUS', 0)
                    ac_int = int(ac_value) if ac_value else 0
                    if ac_int > 0:
                        bonuses['ac']['shield'] = ac_int

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
        """Parse single item property for bonuses using property decoder."""
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
        """Validate inventory for corruption prevention."""
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
        """Get all available weapons."""
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
        """Get all available armor and shields."""
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
        """Filter base items by type."""
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
        """Get all custom/mod items in character's possession."""
        custom_items = []

        for bitmask, item in self._get_raw_equip_item_list():
            slot_name = self.SLOT_BITMASK_MAPPING.get(bitmask)
            if not slot_name:
                continue
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
        """Check if character has custom/mod items."""
        return len(self.get_custom_items()) > 0
    
    def get_equipment_summary_by_slot(self) -> Dict[str, Optional[Dict[str, Any]]]:
        """Get detailed summary of equipped items by slot."""
        equipment_summary = {slot_name: None for slot_name in self.SLOT_TO_BITMASK.keys()}

        for bitmask, item in self._get_raw_equip_item_list():
            slot_name = self.SLOT_BITMASK_MAPPING.get(bitmask)
            if not slot_name:
                continue

            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)

            equipment_summary[slot_name] = {
                'base_item': base_item,
                'item_data': item,
                'is_custom': base_item_data is None,
                'name': field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}') if base_item_data else f'Custom Item {base_item}',
                'bonuses': self._calculate_item_bonuses(item)
            }

        return equipment_summary
    
    def get_item_property_descriptions(self, item_data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Get human-readable descriptions of all item properties."""
        properties = item_data.get('PropertiesList', [])
        if not properties:
            return []

        return self.property_decoder.decode_all_properties(properties)

    def get_enhanced_item_summary(self, item_data: Dict[str, Any]) -> Dict[str, Any]:
        """Get enhanced item summary with decoded properties."""
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
        """Remove item from inventory by index."""
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

    def update_item(self, item_index: Optional[int], slot: Optional[str], item_data: Dict[str, Any]) -> Tuple[bool, str]:
        """Update item in inventory or equipment."""
        if slot:
            # Update equipped item
            bitmask = self.SLOT_TO_BITMASK.get(slot)
            if bitmask is None:
                return False, f"Invalid slot: {slot}"
            
            # Ensure __struct_id__ is set correctly for the slot
            item_data['__struct_id__'] = bitmask
            self._set_equipped_item_by_bitmask(bitmask, item_data)
            logger.info(f"Updated equipped item in slot {slot}")
            return True, "Item updated in slot"
            
        elif item_index is not None:
            # Update inventory item
            item_list = self.gff.get('ItemList', [])
            if not isinstance(item_list, list) or item_index < 0 or item_index >= len(item_list):
                return False, f"Invalid inventory index: {item_index}"
            
            item_list[item_index] = item_data
            self.gff.set('ItemList', item_list)
            logger.info(f"Updated inventory item at index {item_index}")
            return True, "Item updated in inventory"
            
        return False, "Neither slot nor index provided"

    def add_item_by_base_type(self, base_item_id: int) -> Tuple[bool, Optional[Dict[str, Any]], str, Optional[int]]:
        """Create new item based on base item type."""
        base_item_data = self.game_rules_service.get_by_id('baseitems', base_item_id)
        if not base_item_data:
            return False, None, f"Invalid base item ID: {base_item_id}", None

        new_item = {
            'BaseItem': base_item_id,
            'Tag': f"new_item_{base_item_id}",
            'LocalizedName': {
                'string_ref': 4294967295,
                'substrings': [{'lang_id': 0, 'value': self._get_item_name({'BaseItem': base_item_id})}]
            },
            'Description': {
                'string_ref': 4294967295,
                'substrings': []
            },
            'Identified': 1,
            'PropertiesList': [],
            'StackSize': 1,
            'Charges': 0,
            'Cost': 0,
            'Stolen': 0,
            'Plot': 0,
            'Cursed': 0
        }

        success, item_index = self.add_to_inventory(new_item)
        if success:
            return True, new_item, "Item added to inventory", item_index
        return False, None, "Failed to add item to inventory", None

    def get_all_base_items(self) -> List[Dict[str, Any]]:
        """Get all available base items for creation, filtering internal items."""
        all_base_items = []
        base_items = self.game_rules_service.get_table('baseitems')

        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue

            # Resolve name: Name (StrRef/String) > label
            name = field_mapper.get_field_value(base_item_data, 'Name')
            if isinstance(name, int):
                name = self.game_rules_service.rm.get_string(name)
            
            label = field_mapper.get_field_value(base_item_data, 'label', f'Item {item_id}')
            if not name or name == '****':
                name = label

            # Filter out internal/empty labels and explicit "padding" items
            if not name or name.lower().startswith('bad index') or name.lower() == '****' or name.upper() == 'DELETED':
                continue
            
            if 'padding' in name.lower() or 'padding' in label.lower():
                continue

            try:
                item_type = int(field_mapper.get_field_value(base_item_data, 'base_item', 0) or 0)
            except (ValueError, TypeError):
                item_type = 0

            all_base_items.append({
                'id': item_id,
                'name': name,
                'type': item_type,
                'category': self._get_item_category(item_id, base_item_data)
            })
        
        return sorted(all_base_items, key=lambda x: x['name'])

    def get_item_editor_metadata(self) -> Dict[str, Any]:
        """Get all metadata needed for item editor UI."""
        # Gather all metadata context for the decoder
        abilities = self.property_decoder._ability_map
        skills = self._get_all_skills()
        damage_types = self.property_decoder._get_all_damage_types()
        alignment_groups = {0: 'Good', 1: 'Evil', 2: 'Lawful', 3: 'Chaotic'}
        alignments = {
            0: 'Lawful Good', 1: 'Neutral Good', 2: 'Chaotic Good',
            3: 'Lawful Neutral', 4: 'True Neutral', 5: 'Chaotic Neutral',
            6: 'Lawful Evil', 7: 'Neutral Evil', 8: 'Chaotic Evil'
        }
        # Use racial types from rules
        racial_groups = self._get_all_races() 
        saving_throws = self.property_decoder._save_map
        immunity_types = self.property_decoder._get_all_immunity_types()
        save_elements = self.property_decoder._get_all_save_elements()
        classes = self._get_all_classes()
        spells = self._get_all_iprp_spells()
        feats = self._get_all_iprp_feats()
        
        context = {
            'abilities': abilities,
            'skills': skills,
            'damage_types': damage_types,
            'alignment_groups': alignment_groups,
            'alignments': alignments,
            'racial_groups': racial_groups,
            'saving_throws': saving_throws,
            'immunity_types': immunity_types,
            'save_elements': save_elements,
            'classes': classes,
            'spells': spells,
            'iprp_feats': feats
        }
        
        property_metadata = self.property_decoder.get_editor_property_metadata(context)

        return {
            'property_types': property_metadata,
            'abilities': abilities,
            'skills': skills,
            'damage_types': damage_types,
            'alignment_groups': alignment_groups,
            'alignments': alignments,
            'racial_groups': racial_groups,
            'saving_throws': saving_throws,
            'immunity_types': immunity_types,
            'classes': classes,
            'spells': spells
        }

    def _get_all_skills(self) -> Dict[int, str]:
        """Get all skills from rules service with StrRef resolution."""
        skills = {}
        skill_table = self.game_rules_service.get_table('skills')
        if skill_table:
            for skill_id, skill_data in enumerate(skill_table):
                if not skill_data: continue
                name = field_mapper.get_field_value(skill_data, 'Name')
                if isinstance(name, int):
                    name = self.game_rules_service.rm.get_string(name)
                if not name or name == '****':
                    name = field_mapper.get_field_value(skill_data, 'Label', f'Skill {skill_id}')
                if name and not name.startswith('DEL_'):
                    skills[skill_id] = name
        return skills

    def _get_all_races(self) -> Dict[int, str]:
        """Get all races from rules service with StrRef resolution."""
        races = {}
        race_table = self.game_rules_service.get_table('racialtypes')
        if race_table:
            for race_id, race_data in enumerate(race_table):
                if not race_data: continue
                name = field_mapper.get_field_value(race_data, 'Name')
                if isinstance(name, int):
                    name = self.game_rules_service.rm.get_string(name)
                if not name or name == '****':
                    name = field_mapper.get_field_value(race_data, 'Label', f'Race {race_id}')
                races[race_id] = name
        return races

    def _get_all_classes(self) -> Dict[int, str]:
        """Get all classes from rules service with StrRef resolution."""
        classes = {}
        class_table = self.game_rules_service.get_table('classes')
        if class_table:
            for class_id, class_data in enumerate(class_table):
                if not class_data: continue
                name = field_mapper.get_field_value(class_data, 'Name')
                if isinstance(name, int):
                    name = self.game_rules_service.rm.get_string(name)
                if not name or name == '****':
                    name = field_mapper.get_field_value(class_data, 'Label', f'Class {class_id}')
                classes[class_id] = name
        return classes

    def get_available_templates(self) -> List[Dict[str, Any]]:
        return self.game_rules_service.rm.build_item_template_index()

    def add_item_from_template(self, template_resref: str) -> Tuple[bool, Optional[Dict[str, Any]], str]:
        resref = template_resref.lower()
        if not resref.endswith('.uti'):
            resref += '.uti'
            
        all_templates = self.game_rules_service.rm.get_all_item_templates()
        template_info = all_templates.get(resref)
        
        if not template_info:
            return False, None, f"Template {template_resref} not found"
            
        item_data = self.game_rules_service.rm.get_item_template_data(template_info)

        if not item_data:
            return False, None, f"Failed to load data for {template_resref}"

        success = self.add_to_inventory(item_data)

        if success:
            name = self._get_item_name(item_data)
            return True, item_data, f"Added {name}"
            
        return False, None, "Failed to add item to inventory"

    def _get_all_iprp_spells(self) -> Dict[int, str]:
        """Get item property spells with StrRef resolution."""
        spells = {}
        spell_table = self.game_rules_service.get_table('iprp_spells')
        if spell_table:
            for spell_row_id, spell_data in enumerate(spell_table):
                if not spell_data: continue
                name = field_mapper.get_field_value(spell_data, 'Name')
                if isinstance(name, int):
                    name = self.game_rules_service.rm.get_string(name)
                if not name or name == '****':
                    name = field_mapper.get_field_value(spell_data, 'Label', f'Spell {spell_row_id}')
                spells[spell_row_id] = name
        return spells

    def _get_all_iprp_feats(self) -> Dict[int, str]:
        """Get item property feats with StrRef resolution."""
        feats = {}
        feat_table = self.game_rules_service.get_table('iprp_feats')
        if feat_table:
            for i, row in enumerate(feat_table):
                if not row: continue
                name = field_mapper.get_field_value(row, 'Name')
                if isinstance(name, int):
                    name = self.game_rules_service.rm.get_string(name)
                if not name or name == '****':
                    name = field_mapper.get_field_value(row, 'Label', f'Feat {i}')
                feats[i] = name
        return feats



    def update_gold(self, amount: int) -> int:
        """Update character's gold amount, returning the new amount."""
        self.gff.set('Gold', amount)
        # Emit generic character changed event to ensure UI updates
        from ..events import EventType
        self.character_manager.emit(EventType.CHARACTER_CHANGED, {'field': 'Gold', 'value': amount})
        logger.info(f"Updated character gold to {amount}")
        return amount

    def get_equipment_info(self) -> Dict[str, Dict[str, Any]]:
        """Get information about all equipped items."""
        info = {}

        for bitmask, item in self._get_raw_equip_item_list():
            slot_name = self.SLOT_BITMASK_MAPPING.get(bitmask)
            if not slot_name:
                continue

            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_rules_service.get_by_id('baseitems', base_item)
            item_name = self._get_item_name(item)

            description = self._resolve_locstring(item.get('DescIdentified'))

            info[slot_name] = {
                'base_item': base_item,
                'name': item_name,
                'description': description,
                'item_data': item,
                'is_custom': base_item_data is None
            }

        return info