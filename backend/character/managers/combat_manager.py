"""
Data-Driven Combat Manager - handles AC calculation, initiative, and combat statistics
Uses CharacterManager and DynamicGameDataLoader for all game data access
"""

from typing import Dict, List, Tuple, Optional, Any
import logging
import time

from ..events import EventEmitter, EventType, EventData
from ..custom_content import CustomContentDetector
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)


class CombatManager(EventEmitter):
    """
    Data-Driven Combat Manager
    Uses CharacterManager as hub for all character data access
    """
    
    def __init__(self, character_manager):
        """
        Initialize the CombatManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_data_loader = character_manager.game_data_loader
        
        # Cache for performance
        self._base_item_cache = {}
        self._feat_cache = {}
        self._class_cache = {}
        
        # BAB calculation cache to prevent redundant calculations during character loading
        self._bab_cache = None
        self._bab_cache_dirty = True
        
        # Field mapping utility for 2DA access
        self.field_mapper = field_mapper
        
        # Register for relevant events
        self._register_event_handlers()
    
    def _register_event_handlers(self):
        """Register handlers for events that affect combat stats"""
        self.character_manager.on(EventType.ATTRIBUTE_CHANGED, self._on_attribute_changed)
        self.character_manager.on(EventType.ITEM_EQUIPPED, self._on_item_equipped)
        self.character_manager.on(EventType.ITEM_UNEQUIPPED, self._on_item_unequipped)
    
    def calculate_armor_class(self) -> Dict[str, Any]:
        """
        Calculate total AC and all components
        
        Returns:
            Dict with total AC and breakdown of all components
        """
        # Base AC from game rules (D&D 3.5/NWN2 standard)
        # TODO: This should come from game data if configurable
        base_ac = 10  # Standard D&D base AC
        
        # Get DEX modifier
        dex_bonus = (self.gff.get('Dex', 10) - 10) // 2
        
        # Get armor and shield bonuses
        armor_bonus = 0
        shield_bonus = 0
        max_dex_bonus = 999  # No limit by default
        
        # Check equipped items
        chest_item = self._get_equipped_item('Chest')
        if chest_item:
            armor_bonus = self._get_item_ac_bonus(chest_item)
            max_dex_bonus = self._get_item_max_dex(chest_item)
        
        left_hand = self._get_equipped_item('LeftHand')
        if left_hand and self._is_shield(left_hand):
            shield_bonus = self._get_item_ac_bonus(left_hand)
        
        # Apply max dex bonus from armor
        effective_dex_bonus = min(dex_bonus, max_dex_bonus)
        
        # Get natural armor (from race, spells, etc.)
        if hasattr(self.character_manager, 'character_data') and self.character_manager.character_data:
            natural_armor = self.character_manager.character_data.get('NaturalAC', 0)
        else:
            natural_armor = self.gff.get('NaturalAC', 0)
        
        # Get dodge bonus (from feats like Dodge)
        dodge_bonus = self._calculate_dodge_bonus()
        
        # Get deflection bonus (from items/spells)
        deflection_bonus = 0  # Would come from items
        
        # Get size modifier from game data
        size = self.gff.get('CreatureSize', 4)  # Default Medium
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            size_modifier = race_manager.get_size_modifier(size)
        else:
            size_modifier = 0
        
        # Calculate total AC
        total_ac = (base_ac + armor_bonus + shield_bonus + effective_dex_bonus + 
                   natural_armor + dodge_bonus + deflection_bonus + size_modifier)
        
        # Calculate touch AC (ignores armor, shield, natural)
        touch_ac = base_ac + effective_dex_bonus + dodge_bonus + deflection_bonus + size_modifier
        
        # Calculate flat-footed AC (no DEX or dodge)
        flatfooted_ac = base_ac + armor_bonus + shield_bonus + natural_armor + deflection_bonus + size_modifier
        
        return {
            'total': total_ac,  # Add 'total' key for test compatibility
            'total_ac': total_ac,
            'touch_ac': touch_ac,
            'flatfooted_ac': flatfooted_ac,
            'components': {
                'base': base_ac,
                'armor': armor_bonus,
                'shield': shield_bonus,
                'dex': effective_dex_bonus,
                'natural': natural_armor,
                'dodge': dodge_bonus,
                'deflection': deflection_bonus,
                'size': size_modifier
            },
            'dex_bonus': dex_bonus,
            'max_dex_from_armor': max_dex_bonus,
            'armor_check_penalty': self._get_armor_check_penalty()
        }
    
    def calculate_initiative(self) -> Dict[str, Any]:
        """
        Calculate initiative modifier
        
        Returns:
            Dict with initiative bonus and components
        """
        # Base initiative is DEX modifier
        dex_mod = (self.gff.get('Dex', 10) - 10) // 2
        
        # Check for Improved Initiative feat
        improved_init = 4 if self._has_feat_by_name('ImprovedInitiative') else 0
        
        # Check for other initiative bonuses (items, etc.)
        # Try to get from character_data first, then fall back to GFF wrapper
        misc_bonus = self.character_manager.character_data.get('initbonus', 0)
        if misc_bonus == 0:
            # Fallback to GFF wrapper for backwards compatibility
            misc_bonus = self.gff.get('initbonus', 0)
        
        total_initiative = dex_mod + improved_init + misc_bonus
        
        return {
            'total': total_initiative,
            'dex_modifier': dex_mod,
            'improved_initiative': improved_init,
            'misc_bonus': misc_bonus
        }
    
    def calculate_combat_maneuver_bonus(self) -> Dict[str, Any]:
        """
        Calculate CMB (Combat Maneuver Bonus) for grapple, trip, etc.
        Note: This is more D&D 3.5/Pathfinder, NWN2 uses different system
        
        Returns:
            Dict with CMB and components
        """
        # Use our own BAB calculation
        bab = self.calculate_base_attack_bonus()
        
        # Get STR modifier
        str_mod = (self.gff.get('Str', 10) - 10) // 2
        
        # Get size modifier (opposite of AC size mod for CMB)
        size = self.gff.get('CreatureSize', 4)
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            size_mod = -race_manager.get_size_modifier(size)
        else:
            size_mod = 0
        
        total_cmb = bab + str_mod + size_mod
        
        return {
            'total': total_cmb,
            'base_attack_bonus': bab,
            'strength_modifier': str_mod,
            'size_modifier': size_mod
        }
    
    def get_damage_reduction(self) -> List[Dict[str, Any]]:
        """
        Get all damage reduction sources
        
        Returns:
            List of DR entries with amount and bypass type
        """
        dr_list = []
        
        # Check for Barbarian DR
        if self._has_class('Barbarian'):
            barb_level = self._get_class_level('Barbarian')
            if barb_level >= 7:
                dr_amount = 1 + (barb_level - 7) // 3
                dr_list.append({
                    'amount': dr_amount,
                    'bypass': '-',  # Cannot be bypassed
                    'source': 'Barbarian class'
                })
        
        # Would check for other sources (items, spells, etc.)
        
        return dr_list
    
    def _get_equipped_item(self, slot: str) -> Optional[Any]:
        """Get item equipped in specific slot from Django model"""
        # If we have access to the character model
        if hasattr(self.character_manager, 'character_model'):
            character = self.character_manager.character_model
            try:
                # Use Django ORM to get equipped item
                from character.models import CharacterItem
                item = character.items.filter(location=slot.upper()).first()
                return item
            except:
                pass
        
        # Fallback to GFF data if no model access
        equipped_items = self.gff.get('Equip_ItemList', [])
        
        slot_mapping = {
            'Head': 0,
            'Chest': 1,
            'Boots': 2,
            'Arms': 3,
            'RightHand': 4,
            'LeftHand': 5,
            'Cloak': 6,
            'LeftRing': 7,
            'RightRing': 8,
            'Neck': 9,
            'Belt': 10,
            'Arrows': 11,
            'Bullets': 12,
            'Bolts': 13,
            'Gloves': 15,
        }
        
        slot_id = slot_mapping.get(slot)
        if slot_id is None:
            return None
        
        for item in equipped_items:
            if item.get('Slot') == slot_id:
                return item
        
        return None
    
    def _get_item_ac_bonus(self, item) -> int:
        """Get AC bonus from an item"""
        # Handle Django model
        if hasattr(item, 'base_item_id'):
            # Django model properties are stored as JSONField
            if hasattr(item, 'properties') and item.properties:
                # Look for AC bonus in item properties
                for prop in item.properties:
                    if isinstance(prop, dict) and prop.get('type') == 'ac_bonus':
                        return prop.get('value', 0)
            
            # Use base item defaults if no AC property
            return self._get_base_item_ac(item.base_item_id)
        
        # Handle raw GFF data
        base_item = item.get('BaseItem', 0)
        
        # Use base item data to determine AC bonus
        base_item_data = self._get_base_item_data(base_item)
        if base_item_data:
            # Get AC value using field mapping utility
            ac_bonus = self.field_mapper.get_field_value(base_item_data, 'ac', 0)
            try:
                return int(ac_bonus) if ac_bonus else 0
            except (ValueError, TypeError):
                pass
        
        # Fallback: check if it has ArmorRulesType for old approach
        if 'ArmorRulesType' in item:
            return item.get('ArmorRulesType', 0) + 1
        
        return 0
    
    def _get_base_item_data(self, base_item_id: int):
        """
        Get base item data from cache or load from game data
        
        Args:
            base_item_id: The base item ID to get data for
            
        Returns:
            Base item data object or None if not found
        """
        if base_item_id in self._base_item_cache:
            return self._base_item_cache[base_item_id]
        
        try:
            base_item_data = self.game_data_loader.get_by_id('baseitems', base_item_id)
            self._base_item_cache[base_item_id] = base_item_data
            return base_item_data
        except Exception as e:
            logger.warning(f"Could not load base item data for ID {base_item_id}: {e}")
            return None

    def _get_base_item_ac(self, base_item_id: int) -> int:
        """Get default AC for base item type using dynamic game data"""
        base_item = self._get_base_item_data(base_item_id)
        if not base_item:
            return 0
        
        # Use AC value from baseitems.2da with proper field mapping
        ac_patterns = ['ac', 'AC', 'base_ac', 'BaseAC']
        ac_value = self.field_mapper.get_robust_field_value(
            base_item, ac_patterns, 'int', 0
        )
        if ac_value > 0:
            return ac_value
        
        return 0
    
    def _get_item_max_dex(self, item) -> int:
        """Get max DEX bonus allowed by armor using dynamic game data"""
        if not item:
            return 999  # No armor = no DEX limit
        
        # Handle Django model
        if hasattr(item, 'base_item_id'):
            base_item_id = item.base_item_id
        else:
            # Raw GFF data
            base_item_id = item.get('BaseItem', 0)
        
        # Get data from baseitems.2da
        base_item = self._get_base_item_data(base_item_id)
        if base_item:
            try:
                # Use field mapping utility for max DEX bonus
                max_dex_patterns = ['dex_bonus', 'DexBonus', 'max_dex_bonus', 'MaxDexBonus']
                max_dex_value = self.field_mapper.get_robust_field_value(
                    base_item, max_dex_patterns, 'int', 999
                )
                return max_dex_value
            except (ValueError, AttributeError, TypeError):
                pass
        
        return 999  # No limit if data not found
    
    def _is_shield(self, item) -> bool:
        """Check if item is a shield using baseitems.2da data"""
        # Get base item ID
        if hasattr(item, 'base_item_id'):
            base_item_id = item.base_item_id
        else:
            base_item_id = item.get('BaseItem', 0)
        
        # Get base item data
        base_item_data = self._get_base_item_data(base_item_id)
        if base_item_data:
            # Check if it's categorized as a shield
            # Shields typically have ItemClass = 'Shield' or similar
            item_class = self.field_mapper.get_field_value(base_item_data, 'item_class', '')
            if isinstance(item_class, str) and 'shield' in item_class.lower():
                return True
                
            # Alternative: check if it has shield-like properties
            weapon_type = self.field_mapper.get_field_value(base_item_data, 'weapon_type', 0)
            try:
                # Shield weapon types are typically specific values
                weapon_type_int = int(weapon_type) if weapon_type else 0
                # This would need to be validated against actual 2DA data
                # For now, use the old range as fallback
                return 63 <= base_item_id <= 68
            except (ValueError, TypeError):
                pass
        
        # Fallback to old range check if no data available
        return 63 <= base_item_id <= 68
    
    def _is_heavy_armor(self, base_item_data) -> bool:
        """Check if armor type reduces movement speed"""
        if not base_item_data:
            return False
            
        # Check armor properties that would reduce speed
        # Heavy armors typically have higher AC and lower max DEX
        max_dex = self.field_mapper.get_field_value(base_item_data, 'max_dex_bonus', 999)
        try:
            max_dex_int = int(max_dex) if max_dex else 999
            # Heavy armors typically have max DEX of 1 or less
            return max_dex_int <= 1
        except (ValueError, TypeError):
            pass
            
        # Alternative: check by armor name/label for heavy types
        label = self.field_mapper.get_field_value(base_item_data, 'label', '')
        if isinstance(label, str):
            heavy_keywords = ['chainmail', 'splint', 'banded', 'full plate', 'plate']
            return any(keyword in label.lower() for keyword in heavy_keywords)
            
        return False
    
    def _calculate_dodge_bonus(self) -> int:
        """Calculate dodge AC bonus from feats"""
        dodge_bonus = 0
        
        # Dodge feat
        if self._has_feat_by_name('Dodge'):
            dodge_bonus += 1
        
        # Mobility feat
        if self._has_feat_by_name('Mobility'):
            dodge_bonus += 4  # Only vs AoO, but simplified here
        
        return dodge_bonus
    
    def _get_armor_check_penalty(self) -> int:
        """Get armor check penalty for skills using dynamic game data"""
        chest_item = self._get_equipped_item('Chest')
        if not chest_item:
            return 0
        
        # Handle Django model vs raw GFF data
        if hasattr(chest_item, 'base_item_id'):
            base_item_id = chest_item.base_item_id
        else:
            base_item_id = chest_item.get('BaseItem', 0)
        
        # Get data from baseitems.2da
        base_item = self._get_base_item_data(base_item_id)
        if base_item:
            # Use field mapping utility for armor check penalty
            acp_patterns = ['armor_check_penalty', 'ArmorCheckPenalty', 'acp', 'ACP']
            acp_value = self.field_mapper.get_robust_field_value(
                base_item, acp_patterns, 'int', 0
            )
            return acp_value
        
        return 0  # No penalty if data not found
    
    def _get_feat_data(self, feat_id: int):
        """
        Get feat data from cache or load from game data
        
        Args:
            feat_id: The feat ID to get data for
            
        Returns:
            Feat data object or None if not found
        """
        if feat_id in self._feat_cache:
            return self._feat_cache[feat_id]
        
        try:
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            self._feat_cache[feat_id] = feat_data
            return feat_data
        except Exception as e:
            logger.warning(f"Could not load feat data for ID {feat_id}: {e}")
            return None

    def _has_feat_by_name(self, feat_label: str) -> bool:
        """Check if character has a feat by its label using FeatManager"""
        feat_manager = self.character_manager.get_manager('feat')
        return feat_manager.has_feat_by_name(feat_label) if feat_manager else False

    def _has_feat_by_id(self, feat_id: int) -> bool:
        """Check if character has a feat by its ID"""
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            if feat.get('Feat', -1) == feat_id:
                return True
        
        return False
    
    def _get_class_data(self, class_id: int):
        """
        Get class data from cache or load from game data
        
        Args:
            class_id: The class ID to get data for
            
        Returns:
            Class data object or None if not found
        """
        if class_id in self._class_cache:
            return self._class_cache[class_id]
        
        try:
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            self._class_cache[class_id] = class_data
            return class_data
        except Exception as e:
            logger.warning(f"Could not load class data for ID {class_id}: {e}")
            return None

    def _has_class(self, class_name: str) -> bool:
        """Check if character has levels in a class using ClassManager"""
        class_manager = self.character_manager.get_manager('class')
        return class_manager.has_class_by_name(class_name) if class_manager else False
    
    def _get_class_level(self, class_name: str) -> int:
        """Get level in a specific class using ClassManager"""
        class_manager = self.character_manager.get_manager('class')
        return class_manager.get_class_level_by_name(class_name) if class_manager else 0

    def _has_class_by_id(self, class_id: int) -> bool:
        """Check if character has levels in a class by ID"""
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            if class_info.get('Class', -1) == class_id:
                return True
        
        return False

    def _get_class_level_by_id(self, class_id: int) -> int:
        """Get level in a specific class by ID"""
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            if class_info.get('Class', -1) == class_id:
                return class_info.get('ClassLevel', 0)
        
        return 0
    
    def _on_attribute_changed(self, event: EventData):
        """Handle attribute changes that affect combat stats"""
        # DEX affects AC and initiative
        if hasattr(event, 'changes'):
            for change in event.changes:
                if change['attribute'] in ['Dex', 'Str']:
                    logger.info(f"Combat stats affected by {change['attribute']} change")
    
    def _on_item_equipped(self, event: EventData):
        """Handle item equip that affects AC"""
        logger.info("Item equipped - recalculating AC")
    
    def _on_item_unequipped(self, event: EventData):
        """Handle item unequip that affects AC"""
        logger.info("Item unequipped - recalculating AC")
    
    def get_combat_summary(self) -> Dict[str, Any]:
        """Get combat summary for CombatSummary model"""
        # Get basic data
        ac_data = self.calculate_armor_class()
        initiative_data = self.calculate_initiative()
        attack_bonuses = self.get_attack_bonuses()
        
        # Get hit points
        current_hp = self.gff.get('CurrentHitPoints', 0)
        max_hp = self.gff.get('MaxHitPoints', 0)
        temp_hp = self.gff.get('TempHitPoints', 0)
        
        # Get speed
        speed_data = self._get_movement_speed()
        
        # Get damage reduction summary
        dr_list = self.get_damage_reduction()
        dr_summary = None
        if dr_list:
            dr_parts = [f"{dr['amount']}/{dr['bypass']}" for dr in dr_list]
            dr_summary = ", ".join(dr_parts)
        
        # Get spell resistance
        spell_resistance = self.get_spell_resistance()
        
        # Return data that matches CombatSummary model
        return {
            'hit_points': current_hp,
            'max_hit_points': max_hp,
            'temporary_hit_points': temp_hp,
            'armor_class': ac_data.get('total_ac', 10),
            'touch_ac': ac_data.get('touch_ac', 10),
            'flat_footed_ac': ac_data.get('flatfooted_ac', 10),
            'base_attack_bonus': attack_bonuses.get('base_attack_bonus', 0),
            'initiative': initiative_data.get('total', 0),
            'speed': speed_data.get('current', 30),
            'damage_reduction': dr_summary,
            'spell_resistance': spell_resistance,
            'main_attack_bonus': attack_bonuses.get('melee_attack_bonus', 0),
            'main_damage': '1d8',  # Would get from equipped weapon
            'is_flat_footed': False,
            'is_flanked': False,
            'is_prone': False,
            'is_stunned': False,
            # Additional data for the router
            'bab_info': {
                'total_bab': attack_bonuses.get('base_attack_bonus', 0),
                'class_breakdown': {},  # Would implement
                'attack_sequence': self.get_attack_sequence(),
                'iterative_attacks': len(self.get_attack_sequence()) - 1,
                'progression_type': 'Average'  # Would calculate
            },
            'attack_bonuses': attack_bonuses,
            'damage_bonuses': self.get_damage_bonuses(),
            'weapons': self.get_equipped_weapons(),
            'defensive_abilities': {
                'damage_reduction': dr_list,
                'energy_resistance': {},
                'damage_immunity': [],
                'spell_resistance': spell_resistance,
                'concealment': 0,
                'fortification': 0,
                'evasion': False,
                'improved_evasion': False,
                'uncanny_dodge': False,
                'improved_uncanny_dodge': False
            },
            'combat_maneuvers': self.calculate_combat_maneuver_bonus(),
            'initiative': initiative_data
        }
    
    def _get_movement_speed(self) -> Dict[str, int]:
        """Get movement speed (affected by armor)"""
        # Get base speed from race data via RaceManager
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            race_id = self.gff.get('Race', 6)  # Default human if not found
            base_speed = race_manager.get_base_speed(race_id)
        else:
            # Fallback if RaceManager not available
            base_speed = 30
        
        # Check for heavy armor
        chest_item = self._get_equipped_item('Chest')
        if chest_item:
            # Handle Django model vs raw GFF data
            if hasattr(chest_item, 'base_item_id'):
                base_item = chest_item.base_item_id
            else:
                base_item = chest_item.get('BaseItem', 0)
            # Check if armor reduces speed using baseitems.2da
            base_item_data = self._get_base_item_data(base_item)
            if base_item_data and self._is_heavy_armor(base_item_data):
                base_speed = 20
        
        # Barbarian fast movement
        if self._has_class('Barbarian'):
            base_speed += 10
        
        # Monk fast movement
        if self._has_class('Monk'):
            monk_level = self._get_class_level('Monk')
            base_speed += (monk_level // 3) * 10
        
        return {
            'base': base_speed,
            'current': base_speed,
            'armor_penalty': base_speed < 30
        }
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate combat statistics for save file integrity only"""
        errors = []
        
        # Only check for values that would cause save corruption or loading failures
        ac_data = self.calculate_armor_class()
        if ac_data['total_ac'] < 0:
            errors.append("AC is negative - check for errors")
        
        # Removed game rule validation: "AC seems unusually high" - users should be free to set any AC
        # Keep only corruption prevention validations
        
        return len(errors) == 0, errors
    
    def get_armor_class(self) -> Dict[str, Any]:
        """Alias for calculate_armor_class for test compatibility"""
        return self.calculate_armor_class()
    
    def get_attack_bonuses(self) -> Dict[str, Any]:
        """
        Get attack bonuses (moved from ClassManager)
        
        Returns:
            Dict with melee and ranged attack bonuses and their components
        """
        # Use our own BAB calculation
        bab = self.calculate_base_attack_bonus()
        
        # Get ability modifiers
        str_mod = (self.gff.get('Str', 10) - 10) // 2
        dex_mod = (self.gff.get('Dex', 10) - 10) // 2
        
        # Get size modifier
        size = self.gff.get('CreatureSize', 4)  # Default Medium
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            size_modifier = race_manager.get_size_modifier(size)
        else:
            size_modifier = 0
        
        # Calculate melee attack bonus
        melee_attack = {
            'base': bab,
            'ability': str_mod,
            'size': size_modifier,
            'misc': 0,  # Would include enhancement bonuses, etc.
            'total': bab + str_mod + size_modifier
        }
        
        # Calculate ranged attack bonus
        ranged_attack = {
            'base': bab,
            'ability': dex_mod,
            'size': size_modifier,
            'misc': 0,
            'total': bab + dex_mod + size_modifier
        }
        
        return {
            'melee': melee_attack,
            'ranged': ranged_attack,
            'melee_attack_bonus': melee_attack['total'],  # Legacy compatibility
            'ranged_attack_bonus': ranged_attack['total'],  # Legacy compatibility
            'str_modifier': str_mod,
            'dex_modifier': dex_mod,
            'base_attack_bonus': bab
        }
    
    # Missing methods called by views - basic implementations
    
    def calculate_base_attack_bonus(self) -> int:
        """
        Calculate base attack bonus from all classes with caching to prevent redundant calculations.
        This is the authoritative BAB calculation method - moved from ClassManager.
        """
        # Use cache if available and not dirty
        if not self._bab_cache_dirty and self._bab_cache is not None:
            return self._bab_cache
        
        # Calculate BAB from all classes
        class_list = self.gff.get('ClassList', [])
        total_bab = 0
        
        for class_info in class_list:
            class_id = class_info.get('Class', 0)
            class_level = class_info.get('ClassLevel', 0)
            
            if class_level > 0:
                class_data = self.game_data_loader.get_by_id('classes', class_id)
                if class_data:
                    class_bab = self._calculate_class_bab(class_data, class_level)
                    total_bab += class_bab
        
        # Cache the result and mark as clean
        self._bab_cache = total_bab
        self._bab_cache_dirty = False
        
        # Only log once per character session, not on every calculation
        if total_bab != self.gff.get('BaseAttackBonus', 0):
            logger.info(f"Calculated total BAB: {total_bab}")
        
        return total_bab
    
    def get_base_attack_bonus(self) -> int:
        """Get base attack bonus - alias for calculate_base_attack_bonus for compatibility"""
        return self.calculate_base_attack_bonus()
        
    def _calculate_class_bab(self, class_data, level: int) -> int:
        """Calculate BAB for a single class and level (moved from ClassManager)"""
        # Use FieldMappingUtility for proper field access
        bab_table_name = self.field_mapper.get_field_value(class_data, 'attack_bonus_table', '')
        if not bab_table_name:
            class_label = self.field_mapper.get_field_value(class_data, 'label', 'Unknown')
            logger.warning(f"No BAB table found for class {class_label}")
            return 0
        
        # Cache BAB table data (convert to lowercase for lookup)
        bab_table_name_lower = bab_table_name.lower()
        if bab_table_name_lower not in self._class_cache:
            bab_table = self.game_data_loader.get_table(bab_table_name_lower)
            if bab_table:
                self._class_cache[bab_table_name_lower] = bab_table
            else:
                logger.warning(f"BAB table '{bab_table_name}' not found")
                return 0
        
        bab_table = self._class_cache[bab_table_name_lower]
        
        # Get BAB for level (level - 1 because tables are 0-indexed)
        level_idx = min(level - 1, 19)  # Cap at 20
        if level_idx < len(bab_table):
            bab_row = bab_table[level_idx]
            # Use FieldMappingUtility to get BAB value with proper field mapping
            bab_value = self.field_mapper.get_field_value(bab_row, 'bab', '0')
            return self.field_mapper._safe_int(bab_value, 0)
        
        return 0
        
    def invalidate_bab_cache(self):
        """Invalidate BAB cache when class data changes"""
        self._bab_cache_dirty = True
        
    def _get_dex_modifier(self) -> int:
        """Get dexterity modifier"""  
        return (self.gff.get('Dex', 10) - 10) // 2
    
    def _get_armor_bonus(self) -> int:
        """Get armor bonus to AC"""
        armor = self._get_equipped_item('Chest')
        if armor:
            return self._get_item_ac_bonus(armor)
        return 0
    
    def _get_shield_bonus(self) -> int:
        """Get shield bonus to AC"""
        shield = self._get_equipped_item('LeftHand')
        if shield and self._is_shield(shield):
            return self._get_item_ac_bonus(shield)
        return 0
    
    def _get_natural_armor_bonus(self) -> int:
        """Get natural armor bonus"""
        if hasattr(self.character_manager, 'character_data') and self.character_manager.character_data:
            return self.character_manager.character_data.get('NaturalAC', 0)
        return self.gff.get('NaturalAC', 0)
    
    def update_natural_armor(self, value: int) -> Dict[str, Any]:
        """
        Set natural armor bonus
        
        Args:
            value: The natural armor bonus to set
            
        Returns:
            Dict with old/new values and updated AC
        """
        old_value = self.character_manager.character_data.get('NaturalAC', 0)
        
        # Engine limit - NWN2 engine caps natural armor at 255
        # This is an engine constraint, not a game rule
        value = max(0, min(255, int(value)))
        
        # Update the character data
        self.character_manager.character_data['NaturalAC'] = value
        
        # Always update GFF wrapper as well (like save_manager pattern)
        self.gff.set('NaturalAC', value)
        
        # Also update the GFF element if available
        if hasattr(self.character_manager, '_gff_element') and self.character_manager._gff_element:
            self.character_manager._gff_element.set_field('NaturalAC', value)
        
        # Calculate new AC
        new_ac = self.calculate_armor_class()
        
        # Emit change event using STATE_CHANGED
        from ..events import EventType, EventData
        event_data = EventData(
            event_type=EventType.STATE_CHANGED,
            source_manager='combat_manager',
            timestamp=0
        )
        # Also emit with data dict for compatibility
        self.emit(EventType.STATE_CHANGED, {
            'field': 'NaturalAC',
            'old_value': old_value,
            'new_value': value,
            'new_ac': new_ac
        })
        
        return {
            'field': 'NaturalAC',
            'old_value': old_value,
            'new_value': value,
            'new_ac': new_ac
        }
    
    def update_initiative_bonus(self, value: int) -> Dict[str, Any]:
        """
        Set initiative misc bonus
        
        Args:
            value: The initiative misc bonus to set
            
        Returns:
            Dict with old/new values and updated initiative
        """
        old_value = self.character_manager.character_data.get('initbonus', 0)
        logger.info(f"DEBUG: old_value from character_data: {old_value}")
        
        # Engine limit - NWN2 engine uses BYTE for bonus fields (-128 to +127)
        # This is an engine constraint, not a game rule
        value = max(-128, min(127, int(value)))
        
        # Update the character data
        self.character_manager.character_data['initbonus'] = value
        
        # Always update GFF wrapper as well (like save_manager pattern)
        self.gff.set('initbonus', value)
        
        # Also update the GFF element if available
        if hasattr(self.character_manager, '_gff_element') and self.character_manager._gff_element:
            self.character_manager._gff_element.set_field('initbonus', value)
        
        # Calculate new initiative
        new_initiative = self.calculate_initiative()
        
        # Emit change event using STATE_CHANGED
        from ..events import EventType, EventData
        event_data = EventData(
            event_type=EventType.STATE_CHANGED,
            source_manager='combat_manager',
            timestamp=0
        )
        # Also emit with data dict for compatibility
        self.emit(EventType.STATE_CHANGED, {
            'field': 'initbonus',
            'old_value': old_value,
            'new_value': value,
            'new_initiative': new_initiative
        })
        
        return {
            'field': 'initbonus',
            'old_value': old_value,
            'new_value': value,
            'new_initiative': new_initiative
        }
    
    def calculate_melee_attack_bonus(self) -> int:
        """Calculate melee attack bonus"""
        attack_bonuses = self.get_attack_bonuses()
        return attack_bonuses.get('melee_attack_bonus', 0)
    
    def calculate_ranged_attack_bonus(self) -> int:
        """Calculate ranged attack bonus"""
        attack_bonuses = self.get_attack_bonuses()
        return attack_bonuses.get('ranged_attack_bonus', 0)
    
    def get_damage_bonuses(self) -> Dict[str, Any]:
        """
        Get damage bonuses for different weapon types
        
        Returns:
            Dict with damage bonuses by weapon type
        """
        # Get ability modifiers
        str_mod = (self.gff.get('Str', 10) - 10) // 2
        dex_mod = (self.gff.get('Dex', 10) - 10) // 2
        
        # Basic melee damage (STR modifier)
        melee_damage = {
            'base': 0,
            'ability': str_mod,
            'weapon_enhancement': 0,  # Would come from equipped weapon
            'weapon_specialization': 0,  # Would come from feats
            'misc': 0,
            'total': str_mod
        }
        
        # Basic ranged damage (usually no ability modifier)
        ranged_damage = {
            'base': 0,
            'ability': 0,  # Most ranged weapons don't add ability modifier
            'weapon_enhancement': 0,
            'weapon_specialization': 0,
            'misc': 0,
            'total': 0
        }
        
        # Two-handed weapons get 1.5x STR
        two_handed_damage = {
            'base': 0,
            'ability': int(str_mod * 1.5),
            'weapon_enhancement': 0,
            'weapon_specialization': 0,
            'misc': 0,
            'total': int(str_mod * 1.5)
        }
        
        return {
            'melee': melee_damage,
            'ranged': ranged_damage,
            'two_handed': two_handed_damage,
            'off_hand': {
                'base': 0,
                'ability': str_mod // 2,  # Half STR for off-hand
                'weapon_enhancement': 0,
                'weapon_specialization': 0,
                'misc': 0,
                'total': str_mod // 2
            }
        }
    
    def get_equipped_weapons(self) -> Dict[str, Any]:
        """
        Get information about currently equipped weapons
        
        Returns:
            Dict with equipped weapon information
        """
        main_hand = self._get_equipped_item('RightHand')
        off_hand = self._get_equipped_item('LeftHand')
        
        # Get main hand weapon info
        main_hand_info = None
        if main_hand:
            main_hand_info = {
                'name': 'Equipped Weapon',  # Would get from item data
                'base_item_id': main_hand.get('BaseItem', 0) if hasattr(main_hand, 'get') else getattr(main_hand, 'base_item_id', 0),
                'damage_dice': '1d8',  # Would get from baseitems.2da
                'threat_range': '20',
                'critical_multiplier': 2,
                'enhancement_bonus': 0,
                'weapon_type': 'Simple',
                'damage_type': 'Slashing',
                'size': 'Medium',
                'weight': 0.0,
                'two_handed': False,
                'finesseable': False,
                'properties': []
            }
        
        # Get off hand weapon info (if not shield)
        off_hand_info = None
        if off_hand and not self._is_shield(off_hand):
            off_hand_info = {
                'name': 'Off-hand Weapon',
                'base_item_id': off_hand.get('BaseItem', 0) if hasattr(off_hand, 'get') else getattr(off_hand, 'base_item_id', 0),
                'damage_dice': '1d6',
                'threat_range': '20',
                'critical_multiplier': 2,
                'enhancement_bonus': 0,
                'weapon_type': 'Simple',
                'damage_type': 'Slashing',
                'size': 'Medium',
                'weight': 0.0,
                'two_handed': False,
                'finesseable': False,
                'properties': []
            }
        
        # Unarmed strike is always available
        unarmed_strike = {
            'name': 'Unarmed Strike',
            'base_item_id': 0,  # Special case
            'damage_dice': '1d3',
            'threat_range': '20',
            'critical_multiplier': 2,
            'enhancement_bonus': 0,
            'weapon_type': 'Simple',
            'damage_type': 'Bludgeoning',
            'size': 'Medium',
            'weight': 0.0,
            'two_handed': False,
            'finesseable': False,
            'properties': []
        }
        
        return {
            'main_hand': main_hand_info,
            'off_hand': off_hand_info,
            'ranged': None,  # Would check for ranged weapon
            'ammunition': None,
            'unarmed_strike': unarmed_strike,
            'two_weapon_fighting': main_hand_info is not None and off_hand_info is not None,
            'weapon_finesse_active': False,  # Would check for feat
            'power_attack_active': False,
            'combat_expertise_active': False
        }
    
    def calculate_melee_damage_bonus(self) -> int:
        """Calculate melee damage bonus"""
        return (self.gff.get('Str', 10) - 10) // 2
    
    def calculate_ranged_damage_bonus(self) -> int:
        """Calculate ranged damage bonus"""
        return 0  # Usually no str bonus for ranged
    
    # Additional placeholder methods for combat views
    def get_class_bab_breakdown(self) -> Dict[str, int]:
        return {}
    
    def get_attack_sequence(self) -> List[int]:
        bab = self.calculate_base_attack_bonus()
        attacks = [bab]
        while bab > 5:
            bab -= 5
            attacks.append(bab)
        return attacks
    
    def get_iterative_attacks(self) -> List[Dict[str, Any]]:
        return []
    
    def _get_deflection_bonus(self) -> int:
        return 0
    
    def _get_size_ac_modifier(self) -> int:
        return 0
        
    def calculate_touch_ac(self) -> int:
        return 10 + self._get_dex_modifier() + self._get_size_ac_modifier()
    
    def calculate_flat_footed_ac(self) -> int:
        return 10 + self._get_armor_bonus() + self._get_shield_bonus() + self._get_natural_armor_bonus()
    
    def _get_str_modifier(self) -> int:
        return (self.gff.get('Str', 10) - 10) // 2
    
    def _get_size_attack_modifier(self) -> int:
        return 0
    
    def _get_weapon_focus_bonus(self) -> int:
        return 0
    
    def _get_misc_attack_bonuses(self) -> int:
        return 0
    
    def _get_weapon_specialization_bonus(self) -> int:
        return 0
    
    def _get_weapon_enhancement_bonus(self) -> int:
        return 0
    
    def _get_misc_damage_bonuses(self) -> int:
        return 0
    
    def get_main_hand_weapon(self) -> Optional[Dict[str, Any]]:
        return None
    
    def get_off_hand_weapon(self) -> Optional[Dict[str, Any]]:
        return None
    
    def get_ranged_weapon(self) -> Optional[Dict[str, Any]]:
        return None
    
    def is_two_weapon_fighting(self) -> bool:
        return False
    
    def can_use_weapon_finesse(self) -> bool:
        return False
    
    
    def get_spell_resistance(self) -> int:
        """
        Get spell resistance value
        
        Returns:
            Total spell resistance
        """
        # Base SR from race
        race_id = self.gff.get('Race', 0)
        race_data = self.game_data_loader.get_by_id('racialtypes', race_id)
        base_sr = 0
        
        if race_data:
            # Try different possible field names for spell resistance
            for field in ['spell_resistance', 'spellresistance', 'sr']:
                sr_value = getattr(race_data, field, 0)
                if sr_value:
                    try:
                        base_sr = int(sr_value)
                        break
                    except (ValueError, TypeError):
                        continue
        
        # TODO: Add SR from feats, items, class features
        return base_sr
    
    def get_concealment(self) -> int:
        return 0
    
    def get_miss_chance(self) -> int:
        return 0