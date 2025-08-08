"""
Data-Driven Inventory Manager - handles equipment effects, calculations, and item management
Uses DynamicGameDataLoader for mod-compatible item data
Provides informational data about proficiencies and requirements (no restrictions)
Focuses on save corruption prevention rather than game rule enforcement
"""

from typing import Dict, List, Set, Tuple, Optional, Any
import logging
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, FeatChangedEvent
from ..custom_content import CustomContentDetector
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)


class InventoryManager(EventEmitter):
    """Data-driven manager for character inventory, equipment effects, and item information"""
    
    # Equipment slot names in GFF
    EQUIPMENT_SLOTS = {
        'head': 'Head',
        'chest': 'Chest',
        'boots': 'Boots',
        'arms': 'Arms',
        'right_hand': 'RightHand',
        'left_hand': 'LeftHand',
        'cloak': 'Cloak',
        'left_ring': 'LeftRing',
        'right_ring': 'RightRing',
        'neck': 'Neck',
        'belt': 'Belt',
        'arrows': 'Arrows',
        'bullets': 'Bullets',
        'bolts': 'Bolts',
        'cweapon_l': 'CreatureWeaponLeft',
        'cweapon_r': 'CreatureWeaponRight',
        'cweapon_b': 'CreatureWeaponBite',
        'carmour': 'CreatureArmour'
    }
    
    def __init__(self, character_manager):
        """
        Initialize the data-driven InventoryManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_data_loader = character_manager.game_data_loader
        self.content_detector = CustomContentDetector(None)  # Will use dynamic detection
        
        # Register for events
        self._register_event_handlers()
        
        # Dynamic caches for performance
        self._item_cache = {}
        self._proficiency_cache = set()
        self._feat_proficiency_map = {}  # Maps feat IDs to proficiency types
        self._base_item_cache = {}
        
        # Initialize caches
        self._build_proficiency_mappings()
        self._update_proficiency_cache()
    
    def _build_proficiency_mappings(self):
        """Build dynamic mapping of feat IDs to proficiency types"""
        self._feat_proficiency_map.clear()
        
        # Get all feats and identify proficiency feats by name patterns
        feats = self.game_data_loader.get_table('feat')
        
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
        
        # Update proficiency cache (for informational purposes)
        self._update_proficiency_cache()
        
        # Just log the change - no validation blocking
        equipment_info = self.get_equipment_info()
        logger.info(f"Class changed - equipment remains equipped: {len(equipment_info)} items")
    
    def on_feat_added(self, event: FeatChangedEvent):
        """Handle feat addition event"""
        # Check if it's a proficiency feat
        if self._is_proficiency_feat(event.feat_id):
            logger.info(f"InventoryManager updating proficiencies for feat {event.feat_id}")
            self._update_proficiency_cache()
    
    def on_feat_removed(self, event: FeatChangedEvent):
        """Handle feat removal event"""
        # Check if it's a proficiency feat
        if self._is_proficiency_feat(event.feat_id):
            logger.info(f"InventoryManager updating proficiencies after feat removal {event.feat_id}")
            self._update_proficiency_cache()
            # No validation - just update cache for informational purposes
    
    def get_equipped_item(self, slot: str) -> Optional[Dict[str, Any]]:
        """
        Get item equipped in a specific slot
        
        Args:
            slot: Slot name (e.g., 'head', 'chest', 'right_hand')
            
        Returns:
            Item data or None
        """
        gff_slot = self.EQUIPMENT_SLOTS.get(slot)
        if not gff_slot:
            return None
        
        return self.gff.get(gff_slot)
    
    def equip_item(self, item_data: Dict[str, Any], slot: str) -> Tuple[bool, List[str]]:
        """
        Equip an item in a specific slot (no restrictions)
        
        Args:
            item_data: Item data to equip
            slot: Slot to equip in
            
        Returns:
            (success, list_of_warnings_or_errors)
        """
        warnings = []
        
        # Only check for corruption prevention - item ID existence
        id_exists, id_messages = self.check_item_id_exists(item_data)
        warnings.extend(id_messages)
        
        # Get GFF slot name
        gff_slot = self.EQUIPMENT_SLOTS.get(slot)
        if not gff_slot:
            return False, ["Invalid equipment slot"]
        
        # Store current item if any
        current_item = self.gff.get(gff_slot)
        
        # Equip new item (no restrictions - user freedom!)
        self.gff.set(gff_slot, item_data)
        
        # If there was an item, add it to inventory
        if current_item:
            self.add_to_inventory(current_item)
        
        logger.info(f"Equipped item in {slot} (no restrictions)")
        return True, warnings
    
    def unequip_item(self, slot: str) -> Optional[Dict[str, Any]]:
        """
        Unequip item from a slot
        
        Args:
            slot: Slot to unequip from
            
        Returns:
            The unequipped item data
        """
        gff_slot = self.EQUIPMENT_SLOTS.get(slot)
        if not gff_slot:
            return None
        
        item = self.gff.get(gff_slot)
        if item:
            # Clear slot
            self.gff.set(gff_slot, None)
            
            # Add to inventory
            self.add_to_inventory(item)
            
            logger.info(f"Unequipped item from {slot}")
        
        return item
    
    def add_to_inventory(self, item_data: Dict[str, Any]) -> bool:
        """
        Add an item to inventory
        
        Args:
            item_data: Item to add
            
        Returns:
            True if successful
        """
        item_list = self.gff.get('ItemList', [])
        
        # Check for stackable items using dynamic data
        base_item = item_data.get('BaseItem', 0)
        base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
        
        if base_item_data:
            stacking = field_mapper.get_field_value(base_item_data, 'stacking', 0)
            if stacking > 1:
                # Try to stack with existing item
                for existing_item in item_list:
                    if existing_item.get('BaseItem') == base_item:
                        # Stack items
                        existing_stack = existing_item.get('StackSize', 1)
                        new_stack = item_data.get('StackSize', 1)
                        max_stack = stacking
                        
                        total_stack = existing_stack + new_stack
                        if total_stack <= max_stack:
                            existing_item['StackSize'] = total_stack
                            return True
        
        # Add as new item
        item_list.append(item_data)
        self.gff.set('ItemList', item_list)
        
        return True
    
    def remove_from_inventory(self, item_index: int) -> Optional[Dict[str, Any]]:
        """Remove item from inventory by index"""
        item_list = self.gff.get('ItemList', [])
        
        if 0 <= item_index < len(item_list):
            item = item_list.pop(item_index)
            self.gff.set('ItemList', item_list)
            return item
        
        return None
    
    def check_item_id_exists(self, item_data: Dict[str, Any]) -> Tuple[bool, List[str]]:
        """
        Check if the item's BaseItem ID exists to prevent crashes
        This is the only validation we keep - prevents corruption/crashes
        
        Returns:
            (id_exists, list_of_errors)
        """
        errors = []
        base_item = item_data.get('BaseItem', 0)
        base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
        
        if not base_item_data:
            # Log warning but allow custom content to be equipped
            logger.warning(f"Unknown base item type: {base_item} - may be custom content")
            # Return True to allow custom items but note it in error for UI info
            return True, [f"Custom/unknown item type: {base_item}"]
        
        return True, []
    
    def get_equipment_info(self) -> Dict[str, Dict[str, Any]]:
        """
        Get information about all currently equipped items (no validation)
        
        Returns:
            Dict mapping slot to item info
        """
        results = {}
        
        for slot, gff_slot in self.EQUIPMENT_SLOTS.items():
            item = self.gff.get(gff_slot)
            if item:
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
                
                results[slot] = {
                    'item': item,
                    'base_item': base_item,
                    'is_custom': base_item_data is None,
                    'name': field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}') if base_item_data else f'Custom Item {base_item}'
                }
        
        return results
    
    def _is_proficiency_feat(self, feat_id: int) -> bool:
        """Check if a feat grants proficiencies using dynamic mapping"""
        # Check if feat is in our proficiency mapping
        if feat_id in self._feat_proficiency_map:
            return True
        
        # Also check for weapon focus/specialization feats that affect equipment usage
        feat_data = self.game_data_loader.get_by_id('feat', feat_id)
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
        
        # Get all character feats
        feat_list = self.gff.get('FeatList', [])
        feat_ids = {f.get('Feat') for f in feat_list}
        
        # Add all proficiency feats
        self._proficiency_cache.update(feat_ids)
        
        # Add class-granted proficiencies
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            class_proficiencies = self._get_class_proficiencies(class_id)
            self._proficiency_cache.update(class_proficiencies)
    
    def _get_class_proficiencies(self, class_id: int) -> Set[int]:
        """Get proficiency feats granted by a class using dynamic data"""
        proficiencies = set()
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        
        if not class_data:
            return proficiencies
        
        # Get class name using field mapper
        class_label = field_mapper.get_field_value(class_data, 'label', '').lower()
        
        # Helper function to get proficiency feat ID by type
        def get_prof_feat_id(prof_type: str) -> Optional[int]:
            for feat_id, mapped_type in self._feat_proficiency_map.items():
                if mapped_type == prof_type:
                    return feat_id
            return None
        
        # All classes get simple weapons
        simple_prof = get_prof_feat_id('weapon_simple')
        if simple_prof:
            proficiencies.add(simple_prof)
        
        # Martial classes
        if any(c in class_label for c in ['fighter', 'ranger', 'paladin', 'barbarian']):
            martial_prof = get_prof_feat_id('weapon_martial')
            light_armor = get_prof_feat_id('armor_light')
            medium_armor = get_prof_feat_id('armor_medium')
            heavy_armor = get_prof_feat_id('armor_heavy')
            shield_prof = get_prof_feat_id('shield')
            
            for prof in [martial_prof, light_armor, medium_armor, heavy_armor, shield_prof]:
                if prof:
                    proficiencies.add(prof)
        
        # Medium armor classes
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
        
        # Light armor classes
        elif any(c in class_label for c in ['rogue', 'warlock']):
            light_armor = get_prof_feat_id('armor_light')
            if light_armor:
                proficiencies.add(light_armor)
            
            if 'rogue' in class_label:
                rogue_prof = get_prof_feat_id('weapon_rogue')
                if rogue_prof:
                    proficiencies.add(rogue_prof)
        
        # Special weapon proficiencies
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
        """Get informational data about item-slot compatibility (no restrictions)"""
        if not base_item_data:
            return {'item_type': 0, 'is_typical_for_slot': False, 'slot_name': slot}
        
        item_type = field_mapper.get_field_value(base_item_data, 'base_item', 0)
        
        # Typical item types for slots (informational only - no restrictions)
        typical_types = {
            'head': [85],  # Helmet
            'chest': [16], # Armor
            'boots': [26], # Boots
            'arms': [36],  # Gauntlets
            'cloak': [30], # Cloak
            'belt': [21],  # Belt
            'neck': [1],   # Amulet
            'left_ring': [52],   # Ring
            'right_ring': [52],  # Ring
            'right_hand': list(range(0, 60)),  # Weapons
            'left_hand': list(range(0, 60)) + [29],  # Weapons + Shield
        }
        
        typical_for_slot = item_type in typical_types.get(slot, [])
        
        return {
            'item_type': item_type,
            'is_typical_for_slot': typical_for_slot,
            'slot_name': slot,
            'note': 'This item type is not typical for this slot' if not typical_for_slot else None
        }
    
    def get_item_proficiency_info(self, base_item_data: Any) -> Dict[str, Any]:
        """Get informational data about item proficiency requirements (no restrictions)"""
        if not base_item_data:
            return {'has_proficiency_requirements': False}
        
        item_type = field_mapper.get_field_value(base_item_data, 'base_item', 0)
        weapon_type = field_mapper.get_field_value(base_item_data, 'weapon_type', 0)
        
        info = {
            'has_proficiency_requirements': False,
            'required_proficiencies': [],
            'character_has_proficiencies': [],
            'missing_proficiencies': []
        }
        
        # Helper function to find proficiency feat ID by type
        def get_prof_feat_id(prof_type: str) -> Optional[int]:
            for feat_id, mapped_type in self._feat_proficiency_map.items():
                if mapped_type == prof_type:
                    return feat_id
            return None
        
        # Check weapon proficiencies
        if item_type < 60:  # Weapons
            if weapon_type in [1, 2, 3]:  # Simple, Martial, Exotic
                info['has_proficiency_requirements'] = True
                weapon_names = {1: 'Simple Weapon', 2: 'Martial Weapon', 3: 'Exotic Weapon'}
                prof_name = weapon_names.get(weapon_type, 'Unknown Weapon')
                info['required_proficiencies'].append(prof_name)
                
                # Check if character has it (informational only)
                prof_types = {1: 'weapon_simple', 2: 'weapon_martial', 3: 'weapon_exotic'}
                required_prof = get_prof_feat_id(prof_types[weapon_type])
                if required_prof and required_prof in self._proficiency_cache:
                    info['character_has_proficiencies'].append(prof_name)
                else:
                    info['missing_proficiencies'].append(prof_name)
        
        # Check armor proficiencies
        elif item_type == 16:  # Armor
            ac_type = field_mapper.get_field_value(base_item_data, 'ac_type', 0)
            info['has_proficiency_requirements'] = True
            
            if ac_type <= 3:
                prof_name = 'Light Armor'
                prof_type = 'armor_light'
            elif ac_type <= 6:
                prof_name = 'Medium Armor'
                prof_type = 'armor_medium'
            else:
                prof_name = 'Heavy Armor'
                prof_type = 'armor_heavy'
            
            info['required_proficiencies'].append(prof_name)
            required_prof = get_prof_feat_id(prof_type)
            if required_prof and required_prof in self._proficiency_cache:
                info['character_has_proficiencies'].append(prof_name)
            else:
                info['missing_proficiencies'].append(prof_name)
        
        # Check shield proficiencies
        elif item_type == 29:  # Shield
            info['has_proficiency_requirements'] = True
            info['required_proficiencies'].append('Shield')
            shield_prof = get_prof_feat_id('shield')
            if shield_prof and shield_prof in self._proficiency_cache:
                info['character_has_proficiencies'].append('Shield')
            else:
                info['missing_proficiencies'].append('Shield')
        
        return info
    
    def get_item_ability_requirements(self, item_data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Get informational data about ability score requirements (no restrictions)"""
        requirements = []
        
        # Parse item properties for ability requirements (informational only)
        properties = item_data.get('PropertiesList', [])
        
        for prop in properties:
            # Parse property for ability requirements - this would need full implementation
            # For now, return empty list (no restrictions applied)
            pass
        
        return requirements
    
    def get_item_class_requirements(self, item_data: Dict[str, Any]) -> List[str]:
        """Get informational data about class requirements (no restrictions)"""
        requirements = []
        
        # Parse item properties for class requirements (informational only)
        properties = item_data.get('PropertiesList', [])
        
        # Would need to parse USE_LIMITATION properties
        # For now, return empty list (no restrictions applied)
        
        return requirements
    
    def get_item_alignment_requirements(self, item_data: Dict[str, Any]) -> List[str]:
        """Get informational data about alignment requirements (no restrictions)"""
        requirements = []
        
        # Parse item properties for alignment requirements (informational only)
        # For now, return empty list (no restrictions applied)
        
        return requirements
    
    def calculate_encumbrance(self) -> Dict[str, Any]:
        """Calculate character's encumbrance level using dynamic data"""
        total_weight = 0.0
        
        # Calculate weight of equipped items
        for slot, gff_slot in self.EQUIPMENT_SLOTS.items():
            item = self.gff.get(gff_slot)
            if item:
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
                if base_item_data:
                    weight = field_mapper.get_field_value(base_item_data, 'weight', 0.0)
                    if weight > 0:
                        weight = weight / 10.0  # Convert to pounds if needed
                        total_weight += weight
        
        # Calculate weight of inventory items
        item_list = self.gff.get('ItemList', [])
        for item in item_list:
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
            if base_item_data:
                weight = field_mapper.get_field_value(base_item_data, 'weight', 0.0)
                if weight > 0:
                    weight = weight / 10.0  # Convert to pounds if needed
                    stack_size = item.get('StackSize', 1)
                    total_weight += weight * stack_size
        
        # Calculate carrying capacity using NWN2 rules
        strength = self.gff.get('Str', 10)
        light_load = strength * 3.3
        medium_load = strength * 6.6
        heavy_load = strength * 10
        
        # Determine encumbrance level
        if total_weight <= light_load:
            level = 'light'
        elif total_weight <= medium_load:
            level = 'medium'
        elif total_weight <= heavy_load:
            level = 'heavy'
        else:
            level = 'overloaded'
        
        return {
            'total_weight': total_weight,
            'light_load': light_load,
            'medium_load': medium_load,
            'heavy_load': heavy_load,
            'encumbrance_level': level
        }
    
    def get_inventory_summary(self) -> Dict[str, Any]:
        """Get summary of character's inventory using dynamic data"""
        item_list = self.gff.get('ItemList', [])
        
        summary = {
            'total_items': len(item_list),
            'equipped_items': {},
            'custom_items': [],
            'encumbrance': self.calculate_encumbrance()
        }
        
        # Check equipped items
        for slot, gff_slot in self.EQUIPMENT_SLOTS.items():
            item = self.gff.get(gff_slot)
            if item:
                base_item = item.get('BaseItem', 0)
                
                # Check if item exists in base items data
                base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
                is_custom = base_item_data is None
                
                summary['equipped_items'][slot] = {
                    'base_item': base_item,
                    'custom': is_custom
                }
                
                if is_custom:
                    summary['custom_items'].append({
                        'slot': slot,
                        'base_item': base_item
                    })
        
        return summary
    
    def get_equipment_bonuses(self) -> Dict[str, Dict[str, int]]:
        """
        Calculate all equipment bonuses for other managers to use
        
        Returns:
            Dict with bonus categories (ac, saves, attributes, skills, etc.)
        """
        bonuses = {
            'ac': {'armor': 0, 'shield': 0, 'deflection': 0, 'natural': 0},
            'saves': {'fortitude': 0, 'reflex': 0, 'will': 0},
            'attributes': {'Str': 0, 'Dex': 0, 'Con': 0, 'Int': 0, 'Wis': 0, 'Cha': 0},
            'skills': {},
            'combat': {'attack': 0, 'damage': 0},
            'misc': {}
        }
        
        # Check all equipped items
        for slot, gff_slot in self.EQUIPMENT_SLOTS.items():
            item = self.gff.get(gff_slot)
            if item:
                item_bonuses = self._calculate_item_bonuses(item)
                
                # Aggregate bonuses by type
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
    
    def _calculate_item_bonuses(self, item_data: Dict[str, Any]) -> Dict[str, Dict[str, int]]:
        """
        Calculate bonuses provided by a single item
        
        Args:
            item_data: Item data from GFF
            
        Returns:
            Dict with bonus categories and values
        """
        bonuses = {
            'ac': {},
            'saves': {},
            'attributes': {},
            'skills': {},
            'combat': {},
            'misc': {}
        }
        
        # Get base item data for default bonuses
        base_item = item_data.get('BaseItem', 0)
        base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
        
        if base_item_data:
            # Armor and shield AC bonuses
            ac_value = field_mapper.get_field_value(base_item_data, 'base_ac', 0)
            if ac_value > 0:
                item_type = field_mapper.get_field_value(base_item_data, 'base_item', 0)
                if item_type == 16:  # Armor
                    bonuses['ac']['armor'] = ac_value
                elif item_type == 29:  # Shield
                    bonuses['ac']['shield'] = ac_value
        
        # Parse item properties for additional bonuses
        properties = item_data.get('PropertiesList', [])
        for prop in properties:
            prop_bonuses = self._parse_item_property(prop)
            
            # Merge property bonuses
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
        """
        Parse a single item property for bonuses
        
        This is a simplified version - full implementation would need
        to parse all NWN2 item property types from itemprops.2da
        """
        bonuses = {
            'ac': {},
            'saves': {},
            'attributes': {},
            'skills': {},
            'combat': {},
            'misc': {}
        }
        
        property_name = property_data.get('PropertyName', 0)
        subtype = property_data.get('Subtype', 0)
        cost_table = property_data.get('CostTable', 0)
        cost_value = property_data.get('CostValue', 0)
        param1 = property_data.get('Param1', 0)
        param1_value = property_data.get('Param1Value', 0)
        
        # Common property types (would need full implementation)
        if property_name == 0:  # Ability Bonus
            ability_map = {0: 'Str', 1: 'Dex', 2: 'Con', 3: 'Int', 4: 'Wis', 5: 'Cha'}
            if subtype in ability_map:
                bonuses['attributes'][ability_map[subtype]] = cost_value
        
        elif property_name == 1:  # AC Bonus
            bonuses['ac']['deflection'] = cost_value
        
        elif property_name == 2:  # Attack Bonus
            bonuses['combat']['attack'] = cost_value
        
        elif property_name == 3:  # Saving Throw Bonus
            save_map = {0: 'fortitude', 1: 'reflex', 2: 'will'}
            if subtype in save_map:
                bonuses['saves'][save_map[subtype]] = cost_value
            elif subtype == 3:  # All saves
                for save_type in save_map.values():
                    bonuses['saves'][save_type] = cost_value
        
        # Add more property types as needed...
        
        return bonuses
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate inventory for corruption prevention only"""
        errors = []
        
        # Only check for data corruption issues
        # Check equipped items for valid IDs
        equipment_info = self.get_equipment_info()
        for slot, info in equipment_info.items():
            if info['is_custom']:
                # Custom items are allowed but noted
                logger.info(f"Custom item in {slot}: BaseItem {info['base_item']}")
        
        # Check inventory items for stack size corruption
        item_list = self.gff.get('ItemList', [])
        for idx, item in enumerate(item_list):
            stack_size = item.get('StackSize', 1)
            if stack_size < 0:
                errors.append(f"Inventory item {idx}: Invalid negative stack size {stack_size}")
            elif stack_size > 999:  # Reasonable max to prevent GFF issues
                errors.append(f"Inventory item {idx}: Stack size {stack_size} too large (max 999)")
        
        return len(errors) == 0, errors
    
    # Information utility methods for item queries (no restrictions)
    def get_all_weapons(self) -> List[Dict[str, Any]]:
        """Get all available weapons (no proficiency restrictions)"""
        all_weapons = []
        base_items = self.game_data_loader.get_table('baseitems')
        
        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue
                
            item_type = field_mapper.get_field_value(base_item_data, 'base_item', 0)
            
            # Check if it's a weapon (type < 60)
            if item_type < 60:
                weapon_name = field_mapper.get_field_value(base_item_data, 'label', f'Weapon {item_id}')
                proficiency_info = self.get_item_proficiency_info(base_item_data)
                
                all_weapons.append({
                    'id': item_id,
                    'name': weapon_name,
                    'type': item_type,
                    'weapon_type': field_mapper.get_field_value(base_item_data, 'weapon_type', 0),
                    'proficiency_info': proficiency_info
                })
        
        return all_weapons
    
    def get_all_armor(self) -> List[Dict[str, Any]]:
        """Get all available armor and shields (no proficiency restrictions)"""
        all_armor = []
        base_items = self.game_data_loader.get_table('baseitems')
        
        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue
                
            item_type = field_mapper.get_field_value(base_item_data, 'base_item', 0)
            
            # Check if it's armor (type 16) or shield (type 29)
            if item_type in [16, 29]:
                armor_name = field_mapper.get_field_value(base_item_data, 'label', f'Armor {item_id}')
                proficiency_info = self.get_item_proficiency_info(base_item_data)
                
                all_armor.append({
                    'id': item_id,
                    'name': armor_name,
                    'type': item_type,
                    'ac_type': field_mapper.get_field_value(base_item_data, 'ac_type', 0),
                    'ac_value': field_mapper.get_field_value(base_item_data, 'base_ac', 0),
                    'proficiency_info': proficiency_info
                })
        
        return all_armor
    
    def filter_items_by_type(self, item_type: int) -> List[Dict[str, Any]]:
        """Filter base items by type"""
        filtered_items = []
        base_items = self.game_data_loader.get_table('baseitems')
        
        for item_id, base_item_data in enumerate(base_items):
            if base_item_data is None:
                continue
                
            if field_mapper.get_field_value(base_item_data, 'base_item', 0) == item_type:
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
        
        # Check equipped items
        for slot, gff_slot in self.EQUIPMENT_SLOTS.items():
            item = self.gff.get(gff_slot)
            if item:
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
                
                if base_item_data is None:  # Custom item
                    custom_items.append({
                        'location': f'equipped_{slot}',
                        'base_item': base_item,
                        'item_data': item
                    })
        
        # Check inventory items
        item_list = self.gff.get('ItemList', [])
        for idx, item in enumerate(item_list):
            base_item = item.get('BaseItem', 0)
            base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
            
            if base_item_data is None:  # Custom item
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
        
        for slot, gff_slot in self.EQUIPMENT_SLOTS.items():
            item = self.gff.get(gff_slot)
            if item:
                base_item = item.get('BaseItem', 0)
                base_item_data = self.game_data_loader.get_by_id('baseitems', base_item)
                
                equipment_summary[slot] = {
                    'base_item': base_item,
                    'item_data': item,
                    'is_custom': base_item_data is None,
                    'name': field_mapper.get_field_value(base_item_data, 'label', f'Unknown Item {base_item}') if base_item_data else f'Custom Item {base_item}',
                    'bonuses': self._calculate_item_bonuses(item)
                }
            else:
                equipment_summary[slot] = None
        
        return equipment_summary