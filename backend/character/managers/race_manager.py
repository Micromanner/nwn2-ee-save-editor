"""
Data-Driven Race Manager - handles race changes and racial properties
Manages racial ability modifiers, size, speed, and racial feats using DynamicGameDataLoader
"""

from typing import Dict, List, Tuple, Optional, Any
import logging
import time

from ..events import EventEmitter, EventType, EventData
from dataclasses import dataclass
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)


@dataclass
class RaceChangedEvent(EventData):
    """Event for race changes"""
    old_race_id: Optional[int]
    new_race_id: int
    old_subrace: Optional[str]
    new_subrace: Optional[str]
    
    def __post_init__(self):
        self.event_type = EventType.ALIGNMENT_CHANGED  # Using as placeholder for race change


class RaceManager(EventEmitter):
    """Data-Driven Race Manager using DynamicGameDataLoader"""
    
    def __init__(self, character_manager):
        """
        Initialize the RaceManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_data_loader = character_manager.game_data_loader
        
        # Cache for performance
        self._race_data_cache = {}
        
        # Cache original race for reverting
        self._original_race = {
            'race_id': self.gff.get('Race', 0),
            'subrace': self.gff.get('Subrace', ''),
            'attributes': self._get_base_attributes()
        }
    
    def _get_race_data(self, race_id: int):
        """
        Get race data from cache or load from game data
        
        Args:
            race_id: The race ID to get data for
            
        Returns:
            Race data object or None if not found
        """
        if race_id in self._race_data_cache:
            return self._race_data_cache[race_id]
        
        try:
            race_data = self.game_data_loader.get_by_id('racialtypes', race_id)
            self._race_data_cache[race_id] = race_data
            return race_data
        except Exception as e:
            logger.warning(f"Could not load race data for ID {race_id}: {e}")
            return None
    
    def change_race(self, new_race_id: int, new_subrace: str = '', 
                   preserve_feats: bool = True) -> Dict[str, Any]:
        """
        Change character's race
        
        Args:
            new_race_id: The new race ID
            new_subrace: Optional subrace string
            preserve_feats: Whether to keep non-racial feats
            
        Returns:
            Dict with all changes made
        """
        logger.info(f"Changing race to {new_race_id} (subrace: {new_subrace})")
        
        old_race_id = self.gff.get('Race', 0)
        old_subrace = self.gff.get('Subrace', '')
        
        # Get race data using dynamic loader
        new_race = self._get_race_data(new_race_id)
        if not new_race:
            raise ValueError(f"Unknown race ID: {new_race_id}")
        
        changes = {
            'old_race': {
                'id': old_race_id,
                'name': self._get_race_name(old_race_id),
                'subrace': old_subrace
            },
            'new_race': {
                'id': new_race_id,
                'name': getattr(new_race, 'label', f'Race_{new_race_id}'),
                'subrace': new_subrace
            },
            'ability_changes': [],
            'size_change': None,
            'speed_change': None,
            'feat_changes': {
                'removed': [],
                'added': []
            }
        }
        
        # 1. Remove old racial ability modifiers
        old_race = self._get_race_data(old_race_id)
        if old_race:
            self._remove_racial_modifiers(old_race, changes)
        
        # 2. Update race
        self.gff.set('Race', new_race_id)
        self.gff.set('Subrace', new_subrace)
        
        # Update Character model fields if available
        if hasattr(self.character_manager, 'character_model'):
            char = self.character_manager.character_model
            char.race_id = new_race_id
            char.race_name = getattr(new_race, 'label', '')
            char.subrace_id = 0  # NWN2 doesn't have subrace IDs
            char.subrace_name = new_subrace
        
        # 3. Apply new racial ability modifiers
        self._apply_racial_modifiers(new_race, changes)
        
        # 4. Update size
        old_size = self.gff.get('CreatureSize', 4)
        new_size = self._get_race_size(new_race_id)
        if old_size != new_size:
            self.gff.set('CreatureSize', new_size)
            changes['size_change'] = {
                'old': old_size,
                'new': new_size,
                'old_name': self._get_size_name(old_size),
                'new_name': self._get_size_name(new_size)
            }
        
        # 5. Update movement speed
        old_speed = self._get_base_speed(old_race_id)
        new_speed = self._get_base_speed(new_race_id)
        if old_speed != new_speed:
            changes['speed_change'] = {
                'old': old_speed,
                'new': new_speed
            }
        
        # 6. Handle racial feats
        if not preserve_feats:
            self._remove_racial_feats(old_race_id, changes)
        self._add_racial_feats(new_race_id, changes)
        
        # 7. Emit race changed event
        event = RaceChangedEvent(
            event_type=EventType.ALIGNMENT_CHANGED,  # Placeholder
            source_manager='race',
            timestamp=time.time(),
            old_race_id=old_race_id,
            new_race_id=new_race_id,
            old_subrace=old_subrace,
            new_subrace=new_subrace
        )
        self.character_manager.emit(event)
        
        return changes
    
    def _get_base_attributes(self) -> Dict[str, int]:
        """Get current base attributes"""
        return {
            'Str': self.gff.get('Str', 10),
            'Dex': self.gff.get('Dex', 10),
            'Con': self.gff.get('Con', 10),
            'Int': self.gff.get('Int', 10),
            'Wis': self.gff.get('Wis', 10),
            'Cha': self.gff.get('Cha', 10)
        }
    
    def _remove_racial_modifiers(self, old_race_data: Any, changes: Dict[str, Any]):
        """Remove ability modifiers from old race using dynamic data"""
        # Note: old_race_data parameter kept for API compatibility but not used
        # We get modifiers dynamically to ensure consistency
        race_id = self.gff.get('Race', 0)  # Current race before change
        racial_mods = self._get_racial_ability_modifiers(race_id)
        logger.debug(f"Removing racial modifiers for race {race_id}: {racial_mods}")
        
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr, 10)
                new_value = current - mod
                self.gff.set(attr, new_value)
                
                changes['ability_changes'].append({
                    'attribute': attr,
                    'old_value': current,
                    'new_value': new_value,
                    'modifier_removed': mod
                })
    
    def _apply_racial_modifiers(self, new_race_data: Any, changes: Dict[str, Any]):
        """Apply ability modifiers from new race using dynamic data"""
        # Note: new_race_data parameter kept for API compatibility but not used
        # We get modifiers dynamically to ensure consistency
        new_race_id = changes['new_race']['id']
        racial_mods = self._get_racial_ability_modifiers(new_race_id)
        logger.debug(f"Applying racial modifiers for race {new_race_id}: {racial_mods}")
        
        # Use AttributeManager if available for proper cascading effects
        attr_manager = self.character_manager.get_manager('attribute')
        
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr, 10)
                new_value = current + mod
                
                if attr_manager:
                    # Use AttributeManager to handle cascading effects
                    attr_manager.set_attribute(attr, new_value, validate=False)
                else:
                    # Direct set if no AttributeManager
                    self.gff.set(attr, new_value)
                
                changes['ability_changes'].append({
                    'attribute': attr,
                    'old_value': current,
                    'new_value': new_value,
                    'modifier_applied': mod
                })
    
    def _remove_racial_feats(self, race_id: int, changes: Dict[str, Any]):
        """Remove feats granted by old race"""
        # Get racial feat list from game data
        race_feats = self._get_racial_feats(race_id)
        
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager:
            for feat_id in race_feats:
                if feat_manager.has_feat(feat_id):
                    feat_manager.remove_feat(feat_id, force=True)
                    feat_info = feat_manager.get_feat_info(feat_id)
                    changes['feat_changes']['removed'].append({
                        'id': feat_id,
                        'name': feat_info['name']
                    })
    
    def _add_racial_feats(self, race_id: int, changes: Dict[str, Any]):
        """Add feats granted by new race"""
        race_feats = self._get_racial_feats(race_id)
        
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager:
            for feat_id in race_feats:
                if not feat_manager.has_feat(feat_id):
                    feat_manager.add_feat(feat_id, source='racial')
                    feat_info = feat_manager.get_feat_info(feat_id)
                    changes['feat_changes']['added'].append({
                        'id': feat_id,
                        'name': feat_info['name']
                    })
    
    def _get_racial_feats(self, race_id: int) -> List[int]:
        """Get list of feats granted by a race using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            return field_mapper.get_racial_feats(race_data)
        return []
    
    def _get_base_speed(self, race_id: int) -> int:
        """Get base movement speed from dynamic data using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            speed = field_mapper.get_field_value(race_data, 'movement_rate')
            if speed is not None:
                try:
                    return int(speed)
                except (ValueError, TypeError):
                    pass
        
        # Fallback: Small races typically have 20ft speed, others 30ft
        race_size = self._get_race_size(race_id)
        default_speed = 20 if race_size == 3 else 30  # 3 = Small size
        logger.debug(f"No speed data found for race {race_id}, defaulting to {default_speed}")
        return default_speed
    
    def _get_race_name(self, race_id: int) -> str:
        """Get race name from dynamic data using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            name = field_mapper.get_field_value(race_data, 'label')
            if name and str(name).strip():
                return str(name)
        return f'Race_{race_id}'
    
    def _get_race_size(self, race_id: int) -> int:
        """Get race size from dynamic data using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            size = field_mapper.get_field_value(race_data, 'creature_size')
            if size is not None:
                try:
                    return int(size)
                except (ValueError, TypeError):
                    pass
        
        # Default to Medium (4) if no size data found
        logger.debug(f"No size data found for race {race_id}, defaulting to Medium (4)")
        return 4
    
    def _get_size_name(self, size: int) -> str:
        """Get size category name from game data using enhanced DynamicGameDataLoader"""
        try:
            # Use enhanced get_by_id method which now handles creaturesize.2da offset mapping
            if hasattr(self.character_manager, 'game_data_loader'):
                size_data = self.character_manager.game_data_loader.get_by_id('creaturesize', size)
                if size_data:
                    # Get label from the dynamic data instance
                    label = getattr(size_data, 'LABEL', None)
                    if label and label != 'INVALID':
                        # Convert to proper case (SMALL -> Small)
                        return label.title()
        except Exception as e:
            logger.debug(f"Could not get size name from game data for size {size}: {e}")
        
        # Fallback to NWN2 creature size mapping
        size_names = {
            1: 'Tiny',      
            2: 'Small',     
            3: 'Small',     
            4: 'Medium',    
            5: 'Large',     
            6: 'Huge',      
        }
        
        return size_names.get(size, 'Unknown')
    
    def get_racial_properties(self) -> Dict[str, Any]:
        """Get comprehensive racial properties using dynamic data"""
        race_id = self.gff.get('Race', 0)
        subrace = self.gff.get('Subrace', '')
        
        properties = {
            'race_id': race_id,
            'race_name': self._get_race_name(race_id),
            'subrace': subrace,
            'size': self._get_race_size(race_id),
            'size_name': self._get_size_name(self._get_race_size(race_id)),
            'base_speed': self._get_base_speed(race_id),
            'ability_modifiers': self._get_racial_ability_modifiers(race_id),
            'racial_feats': self._get_racial_feats(race_id),
            'favored_class': self._get_favored_class(race_id)
        }
        
        return properties
    
    def _get_racial_ability_modifiers(self, race_id: int) -> Dict[str, int]:
        """Get racial ability modifiers from dynamic data using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            return field_mapper.get_ability_modifiers(race_data)
        
        # Return default values if no race data
        return {
            'Str': 0, 'Dex': 0, 'Con': 0, 
            'Int': 0, 'Wis': 0, 'Cha': 0
        }
    
    def _get_favored_class(self, race_id: int) -> Optional[int]:
        """Get favored class from dynamic data using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            value = field_mapper.get_field_value(race_data, 'favored_class')
            if value is not None:
                try:
                    return int(value)
                except (ValueError, TypeError):
                    pass
        return None
    
    def validate_race_change(self, new_race_id: int) -> Tuple[bool, List[str]]:
        """Validate race change - only check for race ID existence (crash prevention)"""
        errors = []
        
        # Check if race exists using dynamic data - this prevents crashes/corruption
        race_data = self._get_race_data(new_race_id)
        if not race_data:
            errors.append(f"Unknown race ID: {new_race_id}")
            return False, errors
        
        # Remove game rule validations - users can change to any race that exists
        # This includes non-player races if they exist in the data files
        
        return len(errors) == 0, errors
    
    def revert_to_original_race(self) -> Dict[str, Any]:
        """Revert character to original race"""
        return self.change_race(
            self._original_race['race_id'],
            self._original_race['subrace']
        )
    
    def get_race_summary(self) -> Dict[str, Any]:
        """Get summary of racial properties"""
        props = self.get_racial_properties()
        
        # Add readable modifier strings
        mod_strings = []
        for attr, mod in props['ability_modifiers'].items():
            if mod != 0:
                mod_strings.append(f"{attr} {mod:+d}")
        
        props['ability_modifier_string'] = ", ".join(mod_strings) if mod_strings else "None"
        
        return props
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current race configuration - only check for corruption prevention"""
        errors = []
        
        race_id = self.gff.get('Race', -1)
        
        # Check if race exists using dynamic data - prevents crashes/corruption
        race_data = self._get_race_data(race_id)
        if not race_data:
            errors.append(f"Invalid race ID: {race_id}")
        
        # Remove game rule validations like size matching - users can set any size
        # Size mismatches don't corrupt saves, they just create unusual characters
        
        return len(errors) == 0, errors
    
    def get_size_modifier(self, size_id: int) -> int:
        """
        Get AC/attack modifier for a creature size
        
        Args:
            size_id: The size ID from creaturesize.2da
            
        Returns:
            AC modifier for the size (positive = bonus, negative = penalty)
        """
        try:
            size_data = self.game_data_loader.get_by_id('creaturesize', size_id)
            if size_data:
                # ACATTACKMOD column has the AC modifier
                ac_mod = getattr(size_data, 'acattackmod', 0)
                try:
                    return int(ac_mod)
                except (ValueError, TypeError):
                    return 0
        except Exception as e:
            logger.warning(f"Could not get size modifier for size {size_id}: {e}")
        
        return 0