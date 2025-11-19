"""
Ability Manager - handles character abilities (Strength, Dexterity, etc.)
Manages base ability scores, racial modifiers, and ability score improvements
"""

from typing import Dict, List, Optional, Any, Tuple
from loguru import logger
import time

from ..events import EventEmitter, EventType, EventData, ClassChangedEvent, LevelGainedEvent

# Using global loguru logger


class AbilityManager(EventEmitter):
    """Manages character ability scores"""
    
    # Core attributes
    ATTRIBUTES = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']
    
    # Standardized mapping between readable names and GFF field names
    ABILITY_MAPPING = {
        'strength': 'Str',
        'dexterity': 'Dex', 
        'constitution': 'Con',
        'intelligence': 'Int',
        'wisdom': 'Wis',
        'charisma': 'Cha'
    }
    # Note: Point-buy validation has been removed to allow user freedom
    # Users can set attributes freely within engine limits (3-50)
    
    def __init__(self, character_manager):
        """
        Initialize the AbilityManager

        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_rules_service = character_manager.rules_service

        # Request-level caching to prevent redundant calculations
        self._attributes_cache: Dict[str, Optional[Dict[str, int]]] = {
            'with_equipment': None,
            'without_equipment': None
        }

        # Register for events
        self.character_manager.on(EventType.CLASS_CHANGED, self._on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self._on_level_gained)
    
    def get_attributes(self, include_equipment: bool = True) -> Dict[str, int]:
        """
        Get all character attributes with request-level caching

        Args:
            include_equipment: Include equipment bonuses in the result

        Returns:
            Dict mapping attribute names to values
        """
        cache_key = 'with_equipment' if include_equipment else 'without_equipment'

        if self._attributes_cache[cache_key] is not None:
            return self._attributes_cache[cache_key].copy()

        attributes = {}
        for attr in self.ATTRIBUTES:
            default_value = self._get_default_attribute_value(attr)
            base_value = self.gff.get(attr, default_value)
            attributes[attr] = base_value
            logger.debug(f"AbilityManager.get_attributes: {attr} base = {base_value}")

        if include_equipment:
            inventory_manager = self.character_manager.get_manager('inventory')
            if inventory_manager:
                equipment_bonuses = inventory_manager.get_equipment_bonuses()
                attributes_bonuses = equipment_bonuses.get('attributes', {}) or {}
                for attr in self.ATTRIBUTES:
                    equipment_bonus = attributes_bonuses.get(attr, 0) if attributes_bonuses else 0
                    if equipment_bonus > 0:
                        attributes[attr] += equipment_bonus
                        logger.debug(f"AbilityManager.get_attributes: {attr} with equipment = {attributes[attr]} (+{equipment_bonus})")

        self._attributes_cache[cache_key] = attributes.copy()
        return attributes

    def _invalidate_attributes_cache(self):
        """Invalidate the attributes cache when data changes"""
        self._attributes_cache = {
            'with_equipment': None,
            'without_equipment': None
        }

    def _get_default_attribute_value(self, attribute: str) -> int:
        """Get default attribute value from game rules or fallback to 10"""
        # Try to get default from game rules, fallback to standard D&D default
        try:
            # Look for default ability scores in game data
            defaults = self.game_rules_service.get_by_id('defaults', 0)
            if defaults:
                from gamedata.dynamic_loader.field_mapping_utility import field_mapper
                default_val = field_mapper.get_field_value(defaults, f'default_{attribute.lower()}', 10)
                return int(default_val)
        except Exception:
            pass
        
        # Standard D&D default
        return 10
    
    def get_attribute_modifiers(self) -> Dict[str, int]:
        """
        Calculate attribute modifiers from BASE scores only (D&D rules: (score - 10) / 2)

        Returns:
            Dict mapping attribute names to base modifiers
        """
        attributes = self.get_attributes(include_equipment=False)
        modifiers = {}
        for attr, value in attributes.items():
            modifiers[attr] = (value - 10) // 2
        return modifiers
    
    def validate_attribute_value(self, attribute: str, value: int, context: str = "") -> Dict[str, Any]:
        """
        Validate an attribute value for corruption prevention only
        
        CRITICAL: Maintains 3-50 range validation as this is a base game engine limit
        that prevents bugs. All other game rule validations have been removed.
        
        Args:
            attribute: Attribute name (Str, Dex, etc.)
            value: Value to validate
            context: Additional context for error messages
            
        Returns:
            Dict with validation results
        """
        result = {
            'valid': True,
            'attribute': attribute,
            'value': value,
            'context': context,
            'errors': [],
            'warnings': [],
            'corrected_value': value
        }
        
        if attribute not in self.ATTRIBUTES:
            result['valid'] = False
            result['errors'].append(f"Unknown attribute '{attribute}'. Valid attributes: {', '.join(self.ATTRIBUTES)}")
            return result
        
        # CRITICAL: Keep 3-50 range - this is a base game engine limit to prevent bugs
        if value < 3:
            result['valid'] = False
            result['errors'].append(f"{attribute} cannot be less than 3 (got {value}). This is a base game engine limit.")
            result['corrected_value'] = 3
        elif value > 50:
            result['valid'] = False
            result['errors'].append(f"{attribute} cannot be greater than 50 (got {value}). This is a base game engine limit.")
            result['corrected_value'] = 50
        
        # Keep corruption prevention for extreme values
        elif value > 100:
            result['valid'] = False
            result['errors'].append(f"{attribute} value {value} is unreasonably high (max 100). This likely indicates corrupted data.")
            result['corrected_value'] = 50
        
        return result
    
    def validate_all_attributes(self, attributes: Dict[str, int], context: str = "") -> Dict[str, Any]:
        """
        Validate all attributes for corruption prevention only
        
        Args:
            attributes: Dict mapping attribute names to values
            context: Additional context for error messages
            
        Returns:
            Dict with overall validation results
        """
        results = {
            'valid': True,
            'context': context,
            'attributes': {},
            'total_errors': 0,
            'total_warnings': 0,
            'summary': []
        }
        
        for attr, value in attributes.items():
            attr_result = self.validate_attribute_value(attr, value, context)
            results['attributes'][attr] = attr_result
            
            if not attr_result['valid']:
                results['valid'] = False
                results['total_errors'] += len(attr_result['errors'])
            
            results['total_warnings'] += len(attr_result['warnings'])
        
        # Generate summary
        if results['total_errors'] > 0:
            results['summary'].append(f"Found {results['total_errors']} validation errors")
        if results['total_warnings'] > 0:
            results['summary'].append(f"Found {results['total_warnings']} warnings")
        if results['valid']:
            results['summary'].append("All attributes pass validation")
        
        return results

    def set_attribute(self, attribute: str, value: int, validate: bool = True) -> Dict[str, Any]:
        """
        Set a character attribute
        
        Args:
            attribute: Attribute name (Str, Dex, etc.)
            value: New value (typically 3-18 before racial modifiers)
            validate: Whether to validate the change
            
        Returns:
            Dict with change details
        """
        if attribute not in self.ATTRIBUTES:
            raise ValueError(f"Invalid attribute: {attribute}")
        
        old_value = self.gff.get(attribute, 10)
        
        if validate:
            validation_result = self.validate_attribute_value(attribute, value, "set_attribute")
            if not validation_result['valid']:
                error_details = "; ".join(validation_result['errors'])
                raise ValueError(f"Invalid {attribute} value: {error_details}")
            
            # Log warnings if any
            for warning in validation_result['warnings']:
                logger.warning(f"Attribute warning: {warning}")
        
        # Update the attribute
        self.gff.set(attribute, value)

        # Invalidate cache
        self._invalidate_attributes_cache()

        # Calculate new modifier
        old_modifier = (old_value - 10) // 2
        new_modifier = (value - 10) // 2
        
        change = {
            'attribute': attribute,
            'old_value': old_value,
            'new_value': value,
            'old_modifier': old_modifier,
            'new_modifier': new_modifier
        }
        
        # Handle cascading effects
        cascading_changes = []
        
        # If STR changed, update combat modifiers
        if attribute == 'Str':
            combat_change = self._update_str_combat_modifiers(old_modifier, new_modifier)
            if combat_change:
                cascading_changes.append(combat_change)
        
        # If DEX changed, update AC, combat components, and reflex save
        elif attribute == 'Dex':
            ac_change = self._update_ac_components(old_modifier, new_modifier)
            if ac_change:
                cascading_changes.append(ac_change)
            combat_change = self._update_dex_combat_modifiers(old_modifier, new_modifier)
            if combat_change:
                cascading_changes.append(combat_change)
            save_change = self._update_saving_throw('reflex', old_modifier, new_modifier)
            if save_change:
                cascading_changes.append(save_change)
        
        # If CON changed, recalculate HP and fortitude save
        elif attribute == 'Con':
            hp_change = self._recalculate_hit_points(old_modifier, new_modifier)
            if hp_change:
                cascading_changes.append(hp_change)
            save_change = self._update_saving_throw('fortitude', old_modifier, new_modifier)
            if save_change:
                cascading_changes.append(save_change)
        
        # If INT/WIS/CHA changed, note spell-related changes
        elif attribute in ['Int', 'Wis', 'Cha']:
            spell_change = self._update_spell_components(attribute, old_modifier, new_modifier)
            if spell_change:
                cascading_changes.append(spell_change)
            # WIS also affects Will save
            if attribute == 'Wis':
                save_change = self._update_saving_throw('will', old_modifier, new_modifier)
                if save_change:
                    cascading_changes.append(save_change)
        
        # Emit event
        event = EventData(
            event_type=EventType.ATTRIBUTE_CHANGED,
            source_manager='attribute',
            timestamp=time.time()
        )
        # Add extra data to the event
        event.changes = [change]
        event.cascading_changes = cascading_changes
        self.character_manager.emit(event)
        
        logger.info(f"Set {attribute} from {old_value} to {value}")
        return change
    
    def set_all_attributes(self, attributes: Dict[str, int], validate: bool = True) -> List[Dict[str, Any]]:
        """
        Set multiple attributes at once with cascading effects
        
        Args:
            attributes: Dict mapping attribute names to values (both standard names and GFF field names supported)
            validate: Whether to validate the changes
            
        Returns:
            List of change details with cascading effects
        """
        changes = []
        all_cascading_changes = []
        
        # Convert standard ability names to GFF field names if needed
        converted_attributes = {}
        for attr, value in attributes.items():
            # Check if this is a standard ability name (strength, dexterity, etc.) 
            gff_field = self.ABILITY_MAPPING.get(attr.lower())
            if gff_field:
                converted_attributes[gff_field] = value
            elif attr in self.ATTRIBUTES:
                # Already a GFF field name (Str, Dex, etc.)
                converted_attributes[attr] = value
            else:
                logger.warning(f"Unknown attribute name: {attr}")
                continue
        
        # Apply all attribute changes
        for attr, value in converted_attributes.items():
            try:
                change = self.set_attribute(attr, value, validate)
                changes.append(change)
                
                # Collect any cascading effects from this change
                if hasattr(change, 'cascading_effects') and change.cascading_effects:
                    all_cascading_changes.extend(change.cascading_effects)
                    
            except Exception as e:
                logger.error(f"Failed to set attribute {attr} to {value}: {e}")
                # Add error to changes list for reporting
                changes.append({
                    'attribute': attr,
                    'old_value': self.gff.get(attr, 10),
                    'new_value': value,
                    'error': str(e),
                    'success': False
                })
        
        # If we collected cascading effects, add them to the last successful change
        if all_cascading_changes and changes:
            # Find the last successful change and add cascading effects
            for change in reversed(changes):
                if change.get('success', True):  # Default to True for backward compatibility
                    change['all_cascading_effects'] = all_cascading_changes
                    break
        
        return changes
    
    def apply_racial_modifiers(self, race_id: int) -> List[Dict[str, Any]]:
        """
        Apply racial attribute modifiers
        
        Args:
            race_id: The race ID
            
        Returns:
            List of changes made
        """
        race = self.game_rules_service.get_by_id('racialtypes', race_id)
        if not race:
            logger.warning(f"Unknown race ID: {race_id}")
            return []
        
        changes = []
        
        # Apply racial modifiers using field mapping utility for safe access
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        racial_mods = field_mapper.get_ability_modifiers(race)
        
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr, 10)
                new_value = current + mod
                change = self.set_attribute(attr, new_value, validate=False)
                change['racial_modifier'] = mod
                changes.append(change)
        
        return changes
    
    def get_total_attribute_points(self) -> int:
        """
        Get the total of all BASE attribute values (for point-buy calculation)
        Note: NWN2 doesn't use point buy system in game data

        Returns:
            Sum of all base attribute values
        """
        attributes = self.get_attributes(include_equipment=False)
        return sum(attributes.values())
    
    def apply_ability_increase(self, attribute: str) -> Dict[str, Any]:
        """
        Apply an ability score increase (typically at 4th, 8th, 12th, etc. levels)
        
        Args:
            attribute: The attribute to increase
            
        Returns:
            Change details
        """
        if attribute not in self.ATTRIBUTES:
            raise ValueError(f"Invalid attribute: {attribute}")
        
        current = self.gff.get(attribute, 10)
        new_value = current + 1
        
        change = self.set_attribute(attribute, new_value, validate=False)
        change['reason'] = 'ability_increase'
        
        return change
    
    def _on_class_changed(self, event: ClassChangedEvent):
        """Handle class change events"""
        logger.info(f"🔥 AbilityManager handling class change to {event.new_class_id}, level {event.level}")
        logger.info(f"🔥 Event details: old_class={event.old_class_id}, new_class={event.new_class_id}, level={event.level}")

        # Invalidate cache
        self._invalidate_attributes_cache()

        # When level changes (especially reductions), adjust level-up bonuses
        logger.info(f"🔥 Calling _adjust_level_up_bonuses_for_level({event.level})")
        self._adjust_level_up_bonuses_for_level(event.level)
    
    def _on_level_gained(self, event: LevelGainedEvent):
        """Handle level gained events"""
        # Invalidate cache
        self._invalidate_attributes_cache()

        # Check if this level grants an ability increase
        if event.new_level % 4 == 0:
            logger.info(f"Level {event.new_level} grants ability score increase")
            # Note: The actual increase would be handled by user choice
    
    def get_hit_points(self) -> Dict[str, int]:
        """
        Get character hit points
        
        Returns:
            Dict with current and max hit points
        """
        current_hp = self.gff.get('CurrentHitPoints', 0)
        max_hp = self.gff.get('MaxHitPoints', 0)
        return {
            'current': current_hp,
            'max': max_hp
        }
    
    def get_saving_throw_modifiers(self) -> Dict[str, int]:
        """
        Get attribute modifiers for saving throws
        
        Returns:
            Dict with Fortitude (Con), Reflex (Dex), Will (Wis) modifiers
        """
        # Note: In NWN2, saving throw attributes are standard D&D rules
        # but the actual base values come from class tables
        modifiers = self.get_attribute_modifiers()
        return {
            'fortitude': modifiers['Con'],
            'reflex': modifiers['Dex'],
            'will': modifiers['Wis']
        }
    
    def get_skill_modifiers(self) -> Dict[int, int]:
        """
        Get attribute modifiers for skills
        
        Returns:
            Dict mapping skill IDs to their attribute modifiers
        """
        modifiers = self.get_attribute_modifiers()
        skill_mods = {}
        
        # Get all skills and their governing attributes from game rules service using field mapping
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        skills_table = self.game_rules_service.get_table('skills')
        if skills_table:
            for skill in skills_table:
                skill_id = field_mapper.get_field_value(skill, 'id', -1)
                if skill_id != -1:
                    key_ability = field_mapper.get_field_value(skill, 'key_ability', '')
                    # key_ability is the attribute name (STR, DEX, etc.)
                    if key_ability:
                        # Convert from NWN2 format (STR, DEX) to our format (Str, Dex)
                        attr_name = key_ability.capitalize()
                        if attr_name in modifiers:
                            skill_mods[skill_id] = modifiers[attr_name]
        
        return skill_mods
    
    def _recalculate_hit_points(self, old_con_mod: int, new_con_mod: int) -> Optional[Dict[str, Any]]:
        """
        Recalculate hit points when Constitution changes
        
        Args:
            old_con_mod: Old Constitution modifier
            new_con_mod: New Constitution modifier
            
        Returns:
            Dict with HP changes or None if no change
        """
        # Get character level
        class_list = self.gff.get('ClassList', [])
        total_level = sum(
            int(cls.get('ClassLevel', 0)) if cls.get('ClassLevel', 0) else 0
            for cls in class_list 
            if isinstance(cls, dict)
        )
        
        if total_level == 0:
            return None
        
        # Calculate HP difference
        # Each level gets CON modifier to HP
        con_mod_diff = new_con_mod - old_con_mod
        hp_change = total_level * con_mod_diff
        
        if hp_change == 0:
            return None
        
        # Get current HP values
        current_hp = self.gff.get('CurrentHitPoints', 0)
        max_hp = self.gff.get('MaxHitPoints', 0)
        
        # Calculate new values
        new_max_hp = max_hp + hp_change
        new_current_hp = current_hp + hp_change
        
        # Ensure current HP doesn't exceed max or go below 1
        new_current_hp = max(1, min(new_current_hp, new_max_hp))
        
        # Update GFF
        self.gff.set('MaxHitPoints', new_max_hp)
        self.gff.set('CurrentHitPoints', new_current_hp)
        self.gff.set('HitPoints', new_max_hp)  # Legacy field
        
        logger.info(f"Recalculated HP: Max {max_hp} -> {new_max_hp}, Current {current_hp} -> {new_current_hp}")
        
        return {
            'type': 'hp_recalculation',
            'reason': 'constitution_change',
            'old_con_modifier': old_con_mod,
            'new_con_modifier': new_con_mod,
            'level': total_level,
            'hp_change_per_level': con_mod_diff,
            'total_hp_change': hp_change,
            'old_max_hp': max_hp,
            'new_max_hp': new_max_hp,
            'old_current_hp': current_hp,
            'new_current_hp': new_current_hp
        }
    
    def get_encumbrance_limits(self) -> Dict[str, Any]:
        """
        Get encumbrance limits based on Strength using game rules
        
        Returns:
            Dict with encumbrance thresholds and current status
        """
        strength_raw = self.gff.get('Str', 10)
        strength = int(strength_raw) if strength_raw is not None else 10
        
        # Try to get encumbrance data from game rules service using field mapping
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        try:
            encumbrance_data = self.game_rules_service.get_by_id('encumbrance', strength)
            
            if encumbrance_data:
                normal_capacity = int(field_mapper.get_field_value(encumbrance_data, 'normal', strength * 10))
                heavy_threshold = int(field_mapper.get_field_value(encumbrance_data, 'heavy', strength * 20))
            else:
                # Use D&D 3.5 standard calculation if no table
                normal_capacity = strength * 10  # Light load
                heavy_threshold = strength * 20  # Heavy load
        except:
            # Fallback calculation based on D&D 3.5 rules
            normal_capacity = strength * 10
            heavy_threshold = strength * 20
        
        # Calculate medium threshold (typically 2/3 of heavy)
        medium_threshold = int(heavy_threshold * 0.67)
        
        return {
            'strength': strength,
            'normal_capacity': normal_capacity,
            'medium_load': medium_threshold,
            'heavy_load': heavy_threshold,
            'current_weight': 0  # Would need to calculate from inventory
        }
    
    def _update_ac_components(self, old_dex_mod: int, new_dex_mod: int) -> Optional[Dict[str, Any]]:
        """
        Update AC components when Dexterity changes
        Note: This updates the DEX component only - total AC calculation happens elsewhere
        
        Args:
            old_dex_mod: Old Dexterity modifier
            new_dex_mod: New Dexterity modifier
            
        Returns:
            Dict with AC component changes or None
        """
        if old_dex_mod == new_dex_mod:
            return None
        
        # In NWN2, AC is calculated from multiple sources
        # We just track the DEX modifier change here
        # The actual AC calculation considers armor max dex bonus
        
        return {
            'type': 'ac_component_update',
            'reason': 'dexterity_change',
            'old_dex_modifier': old_dex_mod,
            'new_dex_modifier': new_dex_mod,
            'dex_ac_change': new_dex_mod - old_dex_mod,
            'note': 'Total AC change depends on armor max dex bonus'
        }
    
    def _update_spell_components(self, attribute: str, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """
        Update spell-related components when INT/WIS/CHA changes
        
        Args:
            attribute: Which attribute changed (Int, Wis, or Cha)
            old_mod: Old attribute modifier
            new_mod: New attribute modifier
            
        Returns:
            Dict with spell component changes or None
        """
        if old_mod == new_mod:
            return None
        
        # Get character's known spells and determine which ones use this attribute
        affected_spells = self._get_spells_using_attribute(attribute)
        
        if not affected_spells:
            return None
        
        # Calculate changes
        dc_change = new_mod - old_mod
        
        # Bonus spells per day (for each spell level 1-9)
        bonus_spells = {}
        for spell_level in range(1, 10):
            if new_mod >= spell_level:
                # Formula: (ability_mod - spell_level + 1) / 4 + 1
                old_bonus = max(0, (old_mod - spell_level + 1) // 4 + 1) if old_mod >= spell_level else 0
                new_bonus = max(0, (new_mod - spell_level + 1) // 4 + 1)
                if old_bonus != new_bonus:
                    bonus_spells[spell_level] = {
                        'old': old_bonus,
                        'new': new_bonus,
                        'change': new_bonus - old_bonus
                    }
        
        # Get unique affected classes
        affected_classes = list(set(spell_info['class_name'] for spell_info in affected_spells.values()))
        
        result = {
            'type': 'spell_component_update',
            'reason': f'{attribute.lower()}_change',
            'attribute': attribute,
            'old_modifier': old_mod,
            'new_modifier': new_mod,
            'affected_classes': affected_classes,
            'affected_spells': affected_spells,
            'spell_dc_change': dc_change,
            'bonus_spells': bonus_spells
        }
        
        # Special case for CHA and Turn Undead (Cleric/Paladin)
        if attribute == 'Cha':
            turn_undead_classes = [cls for cls in affected_classes if cls.lower() in ['cleric', 'paladin']]
            if turn_undead_classes:
                # Turn undead uses = 3 + CHA modifier
                old_turns = 3 + old_mod
                new_turns = 3 + new_mod
                result['turn_undead'] = {
                    'old_uses': max(0, old_turns),
                    'new_uses': max(0, new_turns),
                    'change': new_turns - old_turns,
                    'classes': turn_undead_classes
                }
        
        return result
    
    def _get_spells_using_attribute(self, attribute: str) -> Dict[int, Dict[str, Any]]:
        """
        Get all spells known by the character that use the specified attribute
        
        Args:
            attribute: The attribute (Int, Wis, or Cha)
            
        Returns:
            Dict mapping spell IDs to their casting information
        """
        affected_spells = {}
        
        # Map attribute to class types
        # Note: Wiz_Sorc represents both Wizard (Int) and Sorcerer (Cha)
        # We need to check character's actual classes to determine which applies
        attribute_to_classes = {
            'Int': ['Wiz_Sorc'],  # Wizard spells
            'Wis': ['Cleric', 'Druid', 'Ranger'],  # Divine casters
            'Cha': ['Wiz_Sorc', 'Bard', 'Paladin', 'Warlock']  # Sorcerer, Bard, Paladin, Warlock
        }
        
        target_class_types = attribute_to_classes.get(attribute, [])
        if not target_class_types:
            return affected_spells
        
        # Get character's spell lists
        for spell_level in range(10):  # Spell levels 0-9
            spell_list_key = f'KnownList{spell_level}'
            known_spells = self.gff.get(spell_list_key, [])
            
            for spell_entry in known_spells:
                if not isinstance(spell_entry, dict):
                    continue
                    
                spell_id = spell_entry.get('Spell', -1)
                if spell_id == -1:
                    continue
                
                # Check if this spell uses the specified attribute
                spell_info = self._get_spell_casting_info(spell_id, attribute, target_class_types)
                if spell_info:
                    affected_spells[spell_id] = spell_info
        
        # Also check memorized spells
        for spell_level in range(1, 10):  # Memorized spells are levels 1-9
            mem_list_key = f'SpellLvlMem{spell_level}'
            mem_data = self.gff.get(mem_list_key, [])
            
            for mem_entry in mem_data:
                if not isinstance(mem_entry, dict):
                    continue
                    
                memorized_list = mem_entry.get('MemorizedList', [])
                for mem_spell in memorized_list:
                    if not isinstance(mem_spell, dict):
                        continue
                        
                    spell_id = mem_spell.get('Spell', -1)
                    if spell_id == -1:
                        continue
                    
                    # Check if this spell uses the specified attribute
                    spell_info = self._get_spell_casting_info(spell_id, attribute, target_class_types)
                    if spell_info:
                        affected_spells[spell_id] = spell_info
        
        return affected_spells
    
    def _get_spell_casting_info(self, spell_id: int, attribute: str, target_class_types: List[str]) -> Optional[Dict[str, Any]]:
        """
        Determine if a spell uses the specified attribute and get casting info
        
        Args:
            spell_id: The spell ID to check
            attribute: The attribute being checked (Int, Wis, Cha)
            target_class_types: List of class types that use this attribute
            
        Returns:
            Dict with casting info if spell uses this attribute, None otherwise
        """
        # Get spell data from spells table using game rules service with field mapping
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        try:
            spell_data = self.game_rules_service.get_by_id('spells', spell_id)
            if not spell_data:
                return None
            
            # Check which classes can cast this spell and at what level
            for class_type in target_class_types:
                # Get spell level for this class type using field mapping
                spell_level_str = field_mapper.get_field_value(spell_data, class_type.lower(), '')
                if spell_level_str and str(spell_level_str).strip() and str(spell_level_str) != '****':
                    try:
                        spell_level = int(spell_level_str)
                        # Found a match - this spell uses this attribute for this class
                        return {
                            'spell_id': spell_id,
                            'spell_label': field_mapper.get_field_value(spell_data, 'label', f'Spell_{spell_id}'),
                            'class_type': class_type,
                            'class_name': self._class_type_to_name(class_type),
                            'spell_level': spell_level,
                            'casting_attribute': attribute
                        }
                    except (ValueError, TypeError):
                        continue
            
            return None
            
        except Exception as e:
            logger.warning(f"Error checking spell {spell_id}: {e}")
            return None
    
    def _class_type_to_name(self, class_type: str) -> str:
        """
        Convert class type from spells.2da to readable name
        
        Args:
            class_type: Class type from spells.2da (e.g., 'Wiz_Sorc', 'Cleric')
            
        Returns:
            Readable class name
        """
        # Try to get class data for better naming
        try:
            # Handle special case for combined types
            if class_type == 'Wiz_Sorc':
                return 'Wizard/Sorcerer'
            
            # Try to find matching class by label using field mapping
            from gamedata.dynamic_loader.field_mapping_utility import field_mapper
            classes = self.game_rules_service.get_table('classes')
            if classes:
                for class_data in classes:
                    label = field_mapper.get_field_value(class_data, 'label', '')
                    if label.lower() == class_type.lower():
                        # Return the proper class name
                        name = field_mapper.get_field_value(class_data, 'name', label)
                        return name if name else label
        except Exception as e:
            logger.warning(f"Could not find class name for type {class_type}: {e}")
        
        # Return original if no match found
        return class_type
    
    def _update_str_combat_modifiers(self, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """
        Update combat modifiers when Strength changes
        
        Args:
            old_mod: Old Strength modifier
            new_mod: New Strength modifier
            
        Returns:
            Dict with combat modifier changes or None
        """
        if old_mod == new_mod:
            return None
        
        modifier_change = new_mod - old_mod
        
        # Check if character has Weapon Finesse
        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')
        
        result = {
            'type': 'combat_modifiers_update',
            'reason': 'strength_change',
            'old_str_modifier': old_mod,
            'new_str_modifier': new_mod,
            'melee_damage_bonus_change': modifier_change,  # Always uses STR
            'note': 'STR modifier applies to melee damage'
        }
        
        if has_weapon_finesse:
            result['melee_attack_bonus_change'] = 0  # No change for finesse weapons
            result['note'] += ' (Weapon Finesse: DEX used for finesse weapon attacks)'
        else:
            result['melee_attack_bonus_change'] = modifier_change
            result['note'] += ' and melee attack rolls'
        
        return result
    
    def _update_dex_combat_modifiers(self, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """
        Update combat modifiers when Dexterity changes
        
        Args:
            old_mod: Old Dexterity modifier  
            new_mod: New Dexterity modifier
            
        Returns:
            Dict with combat modifier changes or None
        """
        if old_mod == new_mod:
            return None
        
        modifier_change = new_mod - old_mod
        
        # Check if character has Weapon Finesse
        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')
        
        result = {
            'type': 'combat_modifiers_update',
            'reason': 'dexterity_change',
            'old_dex_modifier': old_mod,
            'new_dex_modifier': new_mod,
            'ranged_attack_bonus_change': modifier_change,
            'initiative_bonus_change': modifier_change,
            'note': 'DEX modifier applies to ranged attack rolls and initiative'
        }
        
        if has_weapon_finesse:
            result['finesse_melee_attack_bonus_change'] = modifier_change
            result['note'] += ' (Weapon Finesse: also applies to finesse weapon melee attacks)'
        
        return result
    
    def _has_feat_by_name(self, feat_label: str) -> bool:
        """
        Check if character has a feat by its label name
        
        Args:
            feat_label: The label of the feat to check for
            
        Returns:
            True if character has the feat
        """
        feat_manager = self.character_manager.get_manager('feat')
        return feat_manager.has_feat_by_name(feat_label) if feat_manager else False
    
    def _update_saving_throw(self, save_type: str, old_mod: int, new_mod: int) -> Optional[Dict[str, Any]]:
        """
        Update saving throw when its governing attribute changes
        
        Args:
            save_type: Type of save ('fortitude', 'reflex', or 'will')
            old_mod: Old attribute modifier
            new_mod: New attribute modifier
            
        Returns:
            Dict with save changes or None
        """
        if old_mod == new_mod:
            return None
        
        modifier_change = new_mod - old_mod
        
        return {
            'type': 'saving_throw_update',
            'save_type': save_type,
            'old_modifier': old_mod,
            'new_modifier': new_mod,
            'save_bonus_change': modifier_change,
            'note': f'{save_type.capitalize()} save {"improved" if modifier_change > 0 else "reduced"} by {abs(modifier_change)}'
        }
    
    def get_attribute(self, attribute: str) -> int:
        """
        Get a single attribute value
        
        Args:
            attribute: Attribute name (Str, Dex, etc.)
            
        Returns:
            Attribute value
        """
        if attribute not in self.ATTRIBUTES:
            raise ValueError(f"Invalid attribute: {attribute}")
        
        return self.gff.get(attribute, 10)
    
    def get_ability_score(self, ability_name: str, default: int = 10) -> int:
        """
        Get ability score using standard name mapping
        
        Args:
            ability_name: Standard ability name (strength, dexterity, etc.)
            default: Default value if not found
            
        Returns:
            Ability score value
        """
        gff_field = self.ABILITY_MAPPING.get(ability_name.lower())
        if gff_field:
            return self.gff.get(gff_field, default)
        return default
    
    def get_ability_scores(self) -> Dict[str, int]:
        """Get all BASE ability scores using standard names (without equipment)"""
        gff_attributes = self.get_attributes(include_equipment=False)
        return {
            ability: gff_attributes[gff_field]
            for ability, gff_field in self.ABILITY_MAPPING.items()
        }
    
    def set_ability_score(self, ability_name: str, value: int):
        """
        Set ability score using standard name mapping
        
        Args:
            ability_name: Standard ability name (strength, dexterity, etc.)
            value: New ability score value
        """
        gff_field = self.ABILITY_MAPPING.get(ability_name.lower())
        if gff_field:
            self.set_attribute(gff_field, value)
        else:
            raise ValueError(f"Unknown ability name: {ability_name}")
    
    def get_attribute_modifier(self, attribute: str) -> int:
        """
        Get modifier for a single attribute
        
        Args:
            attribute: Attribute name (Str, Dex, etc.)
            
        Returns:
            Attribute modifier
        """
        value = self.get_attribute(attribute)
        return (value - 10) // 2
    
    def roll_attribute_check(self, attribute: str) -> Dict[str, Any]:
        """
        Roll d20 + attribute modifier (simulated)
        
        Args:
            attribute: Attribute name (Str, Dex, etc.)
            
        Returns:
            Dict with roll result and breakdown
        """
        import random
        
        modifier = self.get_attribute_modifier(attribute)
        roll = random.randint(1, 20)
        total = roll + modifier
        
        return {
            'attribute': attribute,
            'roll': roll,
            'modifier': modifier,
            'total': total,
            'critical': roll == 20,
            'fumble': roll == 1
        }
    
    def get_attribute_dependencies(self) -> Dict[str, List[str]]:
        """
        Get what game mechanics depend on each attribute
        
        Returns:
            Dict mapping attributes to list of dependencies
        """
        return {
            'Str': [
                'Melee attack rolls',
                'Melee damage rolls',
                'Carrying capacity',
                'Some skill checks (Climb, Jump, Swim)',
                'Combat maneuvers (Trip, Bull Rush, etc.)'
            ],
            'Dex': [
                'Ranged attack rolls',
                'AC bonus (limited by armor)',
                'Reflex saves',
                'Initiative',
                'Some skill checks (Hide, Move Silently, etc.)',
                'Weapon Finesse melee attacks'
            ],
            'Con': [
                'Hit points per level',
                'Fortitude saves',
                'Concentration checks'
            ],
            'Int': [
                'Skill points per level',
                'Wizard spell slots',
                'Some skill checks (Appraise, Craft, etc.)',
                'Combat Expertise AC bonus'
            ],
            'Wis': [
                'Will saves',
                'Cleric/Druid/Ranger spell slots',
                'Some skill checks (Listen, Spot, etc.)',
                'Monk AC bonus'
            ],
            'Cha': [
                'Sorcerer/Bard/Paladin spell slots',
                'Turn Undead uses',
                'Some skill checks (Bluff, Diplomacy, etc.)',
                'Paladin saving throw bonus'
            ]
        }
    
    def preview_attribute_change(self, attribute: str, new_value: int) -> Dict[str, Any]:
        """
        Preview cascading effects of changing an attribute
        
        Args:
            attribute: Attribute name
            new_value: Proposed new value
            
        Returns:
            Dict with all effects that would occur
        """
        current_value = self.get_attribute(attribute)
        current_modifier = (current_value - 10) // 2
        new_modifier = (new_value - 10) // 2
        modifier_change = new_modifier - current_modifier
        
        effects = {
            'attribute': attribute,
            'current_value': current_value,
            'new_value': new_value,
            'current_modifier': current_modifier,
            'new_modifier': new_modifier,
            'modifier_change': modifier_change,
            'effects': []
        }
        
        if modifier_change == 0:
            effects['effects'].append({'type': 'none', 'description': 'No mechanical changes'})
            return effects
        
        # Check specific attribute effects
        if attribute == 'Str':
            effects['effects'].extend([
                {
                    'type': 'combat',
                    'description': f'Melee attack bonus: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                },
                {
                    'type': 'combat',
                    'description': f'Melee damage bonus: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                },
                {
                    'type': 'encumbrance',
                    'description': 'Carrying capacity will change'
                }
            ])
        
        elif attribute == 'Dex':
            effects['effects'].extend([
                {
                    'type': 'defense',
                    'description': f'AC bonus: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)} (if not limited by armor)'
                },
                {
                    'type': 'combat',
                    'description': f'Ranged attack bonus: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                },
                {
                    'type': 'save',
                    'description': f'Reflex save: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                },
                {
                    'type': 'initiative',
                    'description': f'Initiative: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                }
            ])
        
        elif attribute == 'Con':
            total_level = sum(int(c.get('ClassLevel', 0)) if c.get('ClassLevel', 0) else 0 for c in self.gff.get('ClassList', []))
            hp_change = modifier_change * total_level
            effects['effects'].extend([
                {
                    'type': 'hitpoints',
                    'description': f'Hit points: {"++" if hp_change > 0 else "--"}{abs(hp_change)}'
                },
                {
                    'type': 'save',
                    'description': f'Fortitude save: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                }
            ])
        
        elif attribute == 'Int':
            effects['effects'].extend([
                {
                    'type': 'skills',
                    'description': f'Skill points per level: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                }
            ])
            
            # Check for wizard spells
            class_manager = self.character_manager.get_manager('class')
            if class_manager and class_manager.has_class_by_name('wizard'):
                effects['effects'].append({
                    'type': 'spells',
                    'description': 'Wizard spell slots and DCs will change'
                })
        
        elif attribute == 'Wis':
            effects['effects'].extend([
                {
                    'type': 'save',
                    'description': f'Will save: {"++" if modifier_change > 0 else "--"}{abs(modifier_change)}'
                }
            ])
            
            # Check for divine casters
            for class_name in ['cleric', 'druid', 'ranger', 'paladin']:
                if class_manager and class_manager.has_class_by_name(class_name):
                    effects['effects'].append({
                        'type': 'spells',
                        'description': f'{class_name.capitalize()} spell slots and DCs will change'
                    })
                    break
        
        elif attribute == 'Cha':
            # Check for charisma casters
            for class_name in ['sorcerer', 'bard', 'paladin', 'warlock']:
                if class_manager and class_manager.has_class_by_name(class_name):
                    effects['effects'].append({
                        'type': 'spells',
                        'description': f'{class_name.capitalize()} spell slots and DCs will change'
                    })
                    break
        
        # Add skill effects
        skill_effects = self._get_affected_skills(attribute, modifier_change)
        if skill_effects:
            effects['effects'].append({
                'type': 'skills',
                'description': f'{len(skill_effects)} skills affected',
                'details': skill_effects
            })
        
        return effects
    
    def _get_affected_skills(self, attribute: str, modifier_change: int) -> List[str]:
        """Get list of skills affected by attribute change"""
        # Map attributes to their associated skills
        attribute_skills = {
            'Str': ['Climb', 'Jump', 'Swim'],
            'Dex': ['Balance', 'Escape Artist', 'Hide', 'Move Silently', 'Open Lock', 'Sleight of Hand', 'Tumble'],
            'Con': ['Concentration'],
            'Int': ['Appraise', 'Craft', 'Decipher Script', 'Disable Device', 'Forgery', 'Knowledge', 'Search', 'Spellcraft'],
            'Wis': ['Heal', 'Listen', 'Profession', 'Sense Motive', 'Spot', 'Survival'],
            'Cha': ['Bluff', 'Diplomacy', 'Disguise', 'Gather Information', 'Handle Animal', 'Intimidate', 'Perform', 'Use Magic Device']
        }
        
        skills = attribute_skills.get(attribute, [])
        return [f"{skill}: {'+' if modifier_change > 0 else ''}{modifier_change}" for skill in skills]
    
    # Missing methods called by views
    def get_all_modifiers(self) -> Dict[str, int]:
        """Get all ability modifiers using uppercase GFF field names (STR, DEX, etc.)"""
        return {
            gff_field.upper(): (self.gff.get(gff_field, 10) - 10) // 2
            for gff_field in self.ATTRIBUTES
        }

    def get_racial_modifiers(self) -> Dict[str, int]:
        """Get combined racial + subrace attribute modifiers"""
        race_id = self.gff.get('Race', 0)
        subrace = self.gff.get('Subrace', '')
        
        race = self.game_rules_service.get_by_id('racialtypes', race_id)
        if not race:
            logger.warning(f"Unknown race ID: {race_id}")
            return {attr: 0 for attr in self.ATTRIBUTES}
        
        # Get base racial modifiers using field mapping utility for safe access
        from gamedata.dynamic_loader.field_mapping_utility import field_mapper
        racial_mods = field_mapper.get_ability_modifiers(race)
        
        # Get subrace modifiers if subrace is specified
        subrace_mods = {attr: 0 for attr in self.ATTRIBUTES}
        if subrace:
            race_manager = self.character_manager.get_manager('race')
            if race_manager:
                subrace_data = race_manager._get_subrace_data(subrace)
                if subrace_data:
                    subrace_mods = field_mapper.get_ability_modifiers(subrace_data)
                    logger.debug(f"Subrace {subrace} modifiers: {subrace_mods}")
        
        # Combine base race and subrace modifiers
        result = {attr: 0 for attr in self.ATTRIBUTES}
        for attr in self.ATTRIBUTES:
            result[attr] = racial_mods.get(attr, 0) + subrace_mods.get(attr, 0)
        
        logger.debug(f"Combined racial modifiers (race {race_id} + subrace '{subrace}'): {result}")
        return result
    
    def get_item_modifiers(self) -> Dict[str, int]:
        """Get attribute modifiers from equipped items"""
        inventory_manager = self.character_manager.get_manager('inventory')
        if not inventory_manager:
            return {attr: 0 for attr in self.ATTRIBUTES}
        
        equipment_bonuses = inventory_manager.get_equipment_bonuses()
        attribute_bonuses = equipment_bonuses.get('attributes', {})
        
        # Ensure all attributes are present with default 0
        result = {attr: 0 for attr in self.ATTRIBUTES}
        result.update(attribute_bonuses)
        
        return result
    
    def get_enhancement_modifiers(self) -> Dict[str, int]:
        """Get enhancement modifiers (temporary or magical bonuses)"""
        # TODO: Implement enhancement modifiers from spells/effects
        return {attr: 0 for attr in self.ATTRIBUTES}
    
    def get_temporary_modifiers(self) -> Dict[str, int]:
        """Get temporary modifiers (spells, effects, etc.)"""
        # TODO: Implement temporary modifiers from active effects
        return {attr: 0 for attr in self.ATTRIBUTES}
    
    def get_level_up_modifiers(self) -> Dict[str, int]:
        """Get level-up attribute bonuses (every 4 levels)"""
        # In NWN2, every 4 levels you get a +1 to an attribute of your choice
        # These bonuses are stored in the character's level-up history
        
        # Get total character level
        class_list = self.gff.get('ClassList', [])
        total_level = sum(
            cls.get('ClassLevel', 0) 
            for cls in class_list 
            if isinstance(cls, dict)
        )
        
        # Calculate how many ability score increases the character should have
        ability_increases_available = total_level // 4
        
        # Check for level-up choices in LevelUpList or similar structure
        levelup_bonuses = {attr: 0 for attr in self.ATTRIBUTES}

        # Read level-up data from LvlStatList (NWN2's per-level data structure)
        # Ability increases are stored in LvlStatAbility field as an index (0-5)
        # 0=Str, 1=Dex, 2=Con, 3=Int, 4=Wis, 5=Cha
        lvl_stat_list = self.gff.get('LvlStatList', [])

        logger.debug(f"Found {len(lvl_stat_list)} levels in LvlStatList")

        # Map ability index to attribute name
        ABILITY_INDEX_MAP = {
            0: 'Str',
            1: 'Dex',
            2: 'Con',
            3: 'Int',
            4: 'Wis',
            5: 'Cha'
        }

        # Parse LvlStatAbility from each level
        for level_idx, level_entry in enumerate(lvl_stat_list):
            if not isinstance(level_entry, dict):
                continue

            # Check if this level has an ability increase
            if 'LvlStatAbility' in level_entry:
                ability_index = level_entry['LvlStatAbility']
                if ability_index in ABILITY_INDEX_MAP:
                    attr_name = ABILITY_INDEX_MAP[ability_index]
                    levelup_bonuses[attr_name] += 1
                    logger.debug(f"Level {level_idx + 1}: {attr_name} +1 (LvlStatAbility={ability_index})")
                else:
                    logger.warning(f"Level {level_idx + 1}: Invalid LvlStatAbility index {ability_index}")

        logger.debug(f"Total ability bonuses from LvlStatList: {levelup_bonuses}")
        logger.debug(f"Sum of bonuses: {sum(levelup_bonuses.values())} (expected: {ability_increases_available})")

        # If no explicit level-up data found, try to infer from character progression
        # This is a fallback for characters created outside our editor
        if all(bonus == 0 for bonus in levelup_bonuses.values()) and ability_increases_available > 0:
            logger.info(f"🔥 No explicit level-up data found, attempting to infer {ability_increases_available} ability increases")
            
            # For imported characters, we need to reverse-engineer the level-up bonuses
            # by comparing current attributes to expected base values
            base_attrs = self.get_attributes(include_equipment=False)
            racial_mods = self.get_racial_modifiers()
            
            # Try to determine starting ability scores by working backwards
            # Standard NWN2 point-buy range is typically 8-18 before racial modifiers
            # We'll assume the character started with reasonable stats and infer increases
            
            # Calculate what the "base" scores might have been (before level-up bonuses)
            inferred_bonuses = {attr: 0 for attr in self.ATTRIBUTES}
            total_inferred_increases = 0
            
            for attr in self.ATTRIBUTES:
                current_score = base_attrs[attr]
                racial_mod = racial_mods[attr]
                
                # Estimate the starting score (current - racial - estimated level bonuses)
                # We'll distribute the available increases to best explain the current scores
                estimated_starting = current_score - racial_mod
                
                # If the score seems too high for a reasonable starting value, 
                # assume some level-up bonuses were applied to this attribute
                if estimated_starting > 18:  # Very high starting score, likely has bonuses
                    bonus_estimate = min(ability_increases_available, estimated_starting - 16)
                    inferred_bonuses[attr] = bonus_estimate
                    total_inferred_increases += bonus_estimate
                elif estimated_starting > 16:  # Moderately high, possible bonus
                    bonus_estimate = min(ability_increases_available - total_inferred_increases, 1)
                    inferred_bonuses[attr] = bonus_estimate
                    total_inferred_increases += bonus_estimate
                
                # Stop if we've allocated all available increases
                if total_inferred_increases >= ability_increases_available:
                    break
            
            # If we couldn't account for all increases, put remaining in primary stats
            remaining = ability_increases_available - total_inferred_increases
            if remaining > 0:
                # Prioritize STR, DEX, CON as commonly boosted stats
                for attr in ['Str', 'Dex', 'Con', 'Wis', 'Int', 'Cha']:
                    if remaining > 0 and inferred_bonuses[attr] < 2:  # Don't over-allocate
                        additional = min(remaining, 1)
                        inferred_bonuses[attr] += additional
                        remaining -= additional
            
            # Create synthetic LevelUpList entries for these inferred bonuses
            self._create_synthetic_levelup_entries(inferred_bonuses, total_level)
            
            logger.info(f"Inferred level-up bonuses for imported character: {inferred_bonuses}")
            levelup_bonuses = inferred_bonuses
            
        return levelup_bonuses
    
    def _create_synthetic_levelup_entries(self, inferred_bonuses: Dict[str, int], total_level: int):
        """
        Create synthetic LevelUpList entries for imported characters without level-up history
        
        Args:
            inferred_bonuses: Dict of attribute bonuses to create entries for
            total_level: Total character level
        """
        levelup_list = self.gff.get('LevelUpList', [])
        
        # Distribute the bonuses across the appropriate levels (every 4 levels)
        bonus_levels = [4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60]  # Up to level 60
        available_levels = [level for level in bonus_levels if level <= total_level]
        
        # Create entries for each bonus level, distributing the inferred bonuses
        remaining_bonuses = inferred_bonuses.copy()
        
        for level in available_levels:
            # Check if we already have an entry for this level
            existing_entry = None
            for entry in levelup_list:
                if isinstance(entry, dict) and entry.get('Level') == level:
                    existing_entry = entry
                    break
            
            if existing_entry:
                continue  # Skip if we already have this level
            
            # Find an attribute that still needs bonuses allocated
            chosen_attr = None
            for attr, remaining in remaining_bonuses.items():
                if remaining > 0:
                    chosen_attr = attr
                    break
            
            if chosen_attr:
                # Create synthetic entry
                new_entry = {
                    'Level': level,
                    f'{chosen_attr}Gain': 1,  # Standard +1 bonus
                    'Source': 'imported_character_inference'  # Mark as inferred
                }
                levelup_list.append(new_entry)
                remaining_bonuses[chosen_attr] -= 1
                
                logger.debug(f"Created synthetic level-up entry: Level {level}, +1 {chosen_attr}")
        
        # Update the GFF with the new entries
        self.gff.set('LevelUpList', levelup_list)
        logger.info(f"Created {len(available_levels)} synthetic level-up entries for imported character")
    
    def _adjust_level_up_bonuses_for_level(self, new_total_level: int):
        """
        Adjust level-up bonuses to match the new character level
        This properly removes attribute bonuses from levels that are no longer valid
        
        Args:
            new_total_level: The new total character level
        """
        logger.info(f"🔥 _adjust_level_up_bonuses_for_level called with new_total_level={new_total_level}")
        
        # Get current level-up list
        levelup_list = self.gff.get('LevelUpList', [])
        logger.info(f"🔥 Current LevelUpList: {levelup_list}")
        
        # Track which attribute bonuses need to be removed
        bonuses_to_remove = {attr: 0 for attr in self.ATTRIBUTES}
        valid_levelup_entries = []
        
        for levelup_entry in levelup_list:
            if not isinstance(levelup_entry, dict):
                continue
                
            # Check the level this entry was gained at
            entry_level = levelup_entry.get('Level', 0)
            
            # Only keep entries from levels we still have
            if entry_level <= new_total_level:
                valid_levelup_entries.append(levelup_entry)
            else:
                # This entry is from a level we no longer have - track what bonuses to remove
                logger.info(f"Removing level-up entry from level {entry_level} (new max level: {new_total_level})")
                
                # Count ability increases in this entry that need to be removed
                for attr in self.ATTRIBUTES:
                    for field_name in [f'{attr}Gain', f'Ability{attr}', attr]:
                        if field_name in levelup_entry:
                            bonus_amount = levelup_entry.get(field_name, 0)
                            if bonus_amount > 0:
                                bonuses_to_remove[attr] += bonus_amount
                                logger.debug(f"Will remove +{bonus_amount} {attr} from level {entry_level}")
        
        # Actually remove the bonuses from character attributes
        if any(bonus > 0 for bonus in bonuses_to_remove.values()):
            logger.info(f"Removing level-up bonuses: {bonuses_to_remove}")
            
            # Get current base attributes (these include level-up bonuses)
            current_attrs = self.get_attributes(include_equipment=False)
            
            # Remove the bonuses from the base attributes
            for attr in self.ATTRIBUTES:
                if bonuses_to_remove[attr] > 0:
                    old_value = current_attrs[attr]
                    new_value = max(3, old_value - bonuses_to_remove[attr])  # Min 3 ability score
                    
                    # Update the GFF directly
                    self.gff.set(attr, new_value)
                    logger.info(f"Reduced {attr}: {old_value} -> {new_value} (-{bonuses_to_remove[attr]})")
            
            # Update the LevelUpList to only include valid entries
            self.gff.set('LevelUpList', valid_levelup_entries)
            logger.info(f"Adjusted level-up entries: {len(levelup_list)} -> {len(valid_levelup_entries)} entries")
            
            # Emit event to notify other managers of attribute changes
            event = EventData(
                event_type=EventType.ATTRIBUTE_CHANGED,
                source_manager='ability',
                timestamp=time.time()
            )
            # Add level reduction context
            event.level_reduction = True
            event.new_level = new_total_level
            event.removed_bonuses = bonuses_to_remove
            event.removed_entries = len(levelup_list) - len(valid_levelup_entries)
            self.character_manager.emit(event)
        elif len(valid_levelup_entries) != len(levelup_list):
            # No bonuses to remove, but still update the list
            self.gff.set('LevelUpList', valid_levelup_entries)
            logger.info(f"Updated level-up entries: {len(levelup_list)} -> {len(valid_levelup_entries)} entries")
    
    def get_total_modifiers(self) -> Dict[str, int]:
        """Get total effective modifiers from all sources"""
        base_attrs = self.get_attributes(include_equipment=False)
        racial_mods = self.get_racial_modifiers()
        item_mods = self.get_item_modifiers()
        enhancement_mods = self.get_enhancement_modifiers()
        temp_mods = self.get_temporary_modifiers()
        # Note: Level-up bonuses are already included in base_attrs from GFF

        total_modifiers = {}
        for attr in self.ATTRIBUTES:
            # Calculate effective ability score
            effective_score = (
                base_attrs[attr] +
                racial_mods[attr] +
                item_mods[attr] +
                enhancement_mods[attr] +
                temp_mods[attr]
            )
            # Convert to D&D modifier
            total_modifiers[attr] = (effective_score - 10) // 2

        return total_modifiers
    
    def get_effective_attributes(self) -> Dict[str, int]:
        """Get effective attribute scores (what the game actually uses)"""
        base_attrs = self.get_attributes(include_equipment=False)
        racial_mods = self.get_racial_modifiers()
        item_mods = self.get_item_modifiers()
        enhancement_mods = self.get_enhancement_modifiers()
        temp_mods = self.get_temporary_modifiers()
        # Note: Level-up bonuses are already included in base_attrs from GFF

        effective_attrs = {}
        for attr in self.ATTRIBUTES:
            effective_attrs[attr] = (
                base_attrs[attr] +
                racial_mods[attr] +
                item_mods[attr] +
                enhancement_mods[attr] +
                temp_mods[attr]
            )

        return effective_attrs
    
    def calculate_point_buy_total(self) -> int:
        """Calculate total point buy cost for current attributes (informational only)"""
        # Standard NWN2 point buy costs (28-point buy system)
        # This is now purely informational - no validation is performed
        POINT_BUY_COSTS = {
            8: 0, 9: 1, 10: 2, 11: 3, 12: 4, 13: 5, 14: 6,
            15: 8, 16: 10, 17: 13, 18: 16
        }
        
        total_cost = 0
        base_attributes = self.get_attributes(include_equipment=False)  # Get base only for point-buy
        
        for attr in self.ATTRIBUTES:
            value = base_attributes.get(attr, 10)
            # Calculate cost for any value, even outside normal ranges
            if value <= 8:
                cost = 0
            elif value >= 18:
                cost = 16
            else:
                cost = POINT_BUY_COSTS.get(value, 0)
            total_cost += cost
        
        return total_cost
    
    def get_character_age(self) -> int:
        """
        Get character age
        
        Returns:
            Character age in years
        """
        return self.gff.get('Age', 18)  # Default adult age
    
    def get_character_background(self) -> str:
        """
        Get character background/biography
        
        Returns:
            Character background text
        """
        bio = self.gff.get('Description', {})
        
        # Handle localized string structure
        if isinstance(bio, dict) and 'substrings' in bio:
            substrings = bio.get('substrings', [])
            if substrings and isinstance(substrings[0], dict):
                return substrings[0].get('string', '')
        elif isinstance(bio, str):
            return bio
        
        return ''
    
    def get_experience_points(self) -> int:
        """
        Get current experience points
        
        Returns:
            Current XP value
        """
        return self.gff.get('Experience', 0)
    
    def _get_character_name(self) -> str:
        """Extract character name from localized string structure"""
        first_name = self.gff.get('FirstName', {})
        last_name = self.gff.get('LastName', {})
        
        # Handle localized string structure
        if isinstance(first_name, dict) and 'substrings' in first_name:
            first = first_name.get('substrings', [{}])[0].get('string', '')
        else:
            first = str(first_name)
            
        if isinstance(last_name, dict) and 'substrings' in last_name:
            last = last_name.get('substrings', [{}])[0].get('string', '')
        else:
            last = str(last_name)
            
        full_name = f"{first} {last}".strip()
        return full_name if full_name and full_name != " " else ""
    
    def get_character_name(self) -> str:
        """Public method to get character name (for character summary)"""
        return self._get_character_name()
    
    def validate(self) -> Tuple[bool, List[str]]:
        """
        Validate all ability scores for corruption prevention only
        
        Returns:
            Tuple of (is_valid, list_of_errors)
        """
        errors = []
        
        # Validate ability scores - keep engine limits to prevent corruption
        for ability_name in self.ABILITY_MAPPING.keys():
            value = self.get_ability_score(ability_name)
                
            if value < 3 or value > 50:  # Engine limits - prevents GFF corruption
                errors.append(f"{ability_name.title()} must be between 3 and 50 (engine limit, got {value})")
        
        return len(errors) == 0, errors
    
    
    