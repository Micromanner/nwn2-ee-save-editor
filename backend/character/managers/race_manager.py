"""
Data-Driven Race Manager - handles race changes and racial properties
Manages racial ability modifiers, size, speed, and racial feats using DynamicGameDataLoader
"""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, EventData
from dataclasses import dataclass
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

# Using global loguru logger


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
        self.game_rules_service = character_manager.rules_service
        
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
            race_data = self.game_rules_service.get_by_id('racialtypes', race_id)
            self._race_data_cache[race_id] = race_data
            return race_data
        except Exception as e:
            logger.warning(f"Could not load race data for ID {race_id}: {e}")
            return None
    
    def _get_subrace_data(self, subrace_name):
        """
        Get subrace data from racialsubtypes.2da by name
        
        Args:
            subrace_name: The subrace name to look up (string or int)
            
        Returns:
            Subrace data object or None if not found
        """
        # Handle both string and integer inputs
        if not subrace_name:
            return None
            
        # Convert to string if it's an integer
        if isinstance(subrace_name, int):
            if subrace_name == 0:
                return None  # 0 typically means no subrace
            subrace_name = str(subrace_name)
        
        # Handle string inputs
        if isinstance(subrace_name, str) and not subrace_name.strip():
            return None
            
        try:
            # Get all subrace data and find by name
            all_subraces_list = self.game_rules_service.get_table('racialsubtypes')
            for subrace_data in all_subraces_list:
                # Check both name and label fields
                if (field_mapper.get_field_value(subrace_data, 'subrace_name', '').lower() == subrace_name.lower() or
                    field_mapper.get_field_value(subrace_data, 'subrace_label', '').lower() == subrace_name.lower()):
                    return subrace_data
                    
        except Exception as e:
            logger.warning(f"Could not load subrace data for '{subrace_name}': {e}")
            
        return None
    
    def _get_subrace_name(self, subrace_input) -> str:
        """
        Convert subrace ID or name to standardized subrace name string
        
        Args:
            subrace_input: The subrace identifier (int ID or string name)
            
        Returns:
            Subrace name string, or empty string if not found/invalid
        """
        if not subrace_input:
            return ''
            
        # If it's already a string, return it
        if isinstance(subrace_input, str):
            return subrace_input.strip()
            
        # If it's an integer ID, look up the name
        if isinstance(subrace_input, int):
            if subrace_input == 0:
                return ''  # 0 typically means no subrace
                
            try:
                # Get all subrace data and find by ID (row index)
                all_subraces_list = self.game_rules_service.get_table('racialsubtypes')
                if subrace_input < len(all_subraces_list):
                    subrace_data = all_subraces_list[subrace_input]
                    # Get the subrace name from the data
                    return field_mapper.get_field_value(subrace_data, 'subrace_name', '')
                    
            except Exception as e:
                logger.warning(f"Could not load subrace name for ID {subrace_input}: {e}")
                
        return ''
    
    def get_available_subraces(self, race_id: int) -> List[Dict[str, Any]]:
        """
        Get list of available subraces for a given race
        
        Args:
            race_id: The base race ID
            
        Returns:
            List of dicts with subrace information
        """
        subraces = []
        
        try:
            all_subraces_list = self.game_rules_service.get_table('racialsubtypes')
            for subrace_id, subrace_data in enumerate(all_subraces_list):
                base_race = field_mapper.get_field_value(subrace_data, 'base_race', 0)
                if field_mapper._safe_int(base_race) == race_id:
                    # Check if it's player accessible
                    player_race = field_mapper.get_field_value(subrace_data, 'player_race', 1)
                    if field_mapper._safe_bool(player_race):
                        subraces.append({
                            'id': subrace_id,
                            'name': field_mapper.get_field_value(subrace_data, 'subrace_name', ''),
                            'label': field_mapper.get_field_value(subrace_data, 'subrace_label', ''),
                            'base_race': race_id
                        })
                        
        except Exception as e:
            logger.warning(f"Could not load subraces for race {race_id}: {e}")
            
        return subraces
    
    def validate_subrace(self, race_id: int, subrace_name) -> Tuple[bool, List[str]]:
        """
        Validate that a subrace is compatible with a race
        
        Args:
            race_id: The base race ID
            subrace_name: The subrace name (string or int)
            
        Returns:
            Tuple of (is_valid, error_messages)
        """
        errors = []
        
        # Handle different input types
        if not subrace_name:
            return True, errors  # Empty/None subrace is valid
            
        if isinstance(subrace_name, int) and subrace_name == 0:
            return True, errors  # 0 typically means no subrace
            
        if isinstance(subrace_name, str) and not subrace_name.strip():
            return True, errors  # Empty string subrace is valid
            
        subrace_data = self._get_subrace_data(subrace_name)
        if not subrace_data:
            errors.append(f"Unknown subrace: {subrace_name}")
            return False, errors
            
        # Check if subrace belongs to the base race
        base_race = field_mapper.get_field_value(subrace_data, 'base_race', 0)
        if field_mapper._safe_int(base_race) != race_id:
            errors.append(f"Subrace '{subrace_name}' does not belong to race ID {race_id}")
            return False, errors
            
        # Check if it's player accessible
        player_race = field_mapper.get_field_value(subrace_data, 'player_race', 1)
        if not field_mapper._safe_bool(player_race):
            errors.append(f"Subrace '{subrace_name}' is not available to players")
            return False, errors
            
        return True, errors
    
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
            
        # Validate subrace if provided
        if new_subrace:
            subrace_valid, subrace_errors = self.validate_subrace(new_race_id, new_subrace)
            if not subrace_valid:
                raise ValueError(f"Invalid subrace: {'; '.join(subrace_errors)}")
        
        new_subrace_data = self._get_subrace_data(new_subrace) if new_subrace else None
        
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
        
        # 3. Apply new racial ability modifiers (base race)
        self._apply_racial_modifiers(new_race, changes)
        
        # 3b. Apply subrace ability modifiers if subrace is specified
        if new_subrace_data:
            self._apply_subrace_modifiers(new_subrace_data, changes)
        
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
        
        # 6b. Handle subrace feats if subrace is specified
        if new_subrace_data:
            self._add_subrace_feats(new_subrace_data, changes)
        
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
        
        # Always use AttributeManager for proper cascading effects
        attr_manager = self.character_manager.get_manager('ability')
        
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr, 10)
                new_value = current + mod
                
                if attr_manager:
                    # Use AttributeManager to handle cascading effects
                    attr_manager.set_attribute(attr, new_value, validate=False)
                else:
                    # Fallback only if AttributeManager is not available
                    logger.warning(f"AttributeManager not available, setting {attr} directly")
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
    
    def _apply_subrace_modifiers(self, subrace_data: Any, changes: Dict[str, Any]):
        """Apply ability modifiers from subrace using dynamic data (stacks with base race)"""
        subrace_mods = field_mapper.get_ability_modifiers(subrace_data)
        logger.debug(f"Applying subrace modifiers: {subrace_mods}")
        
        # Always use AttributeManager for proper cascading effects
        attr_manager = self.character_manager.get_manager('ability')
        
        for attr, mod in subrace_mods.items():
            if mod != 0:
                current = self.gff.get(attr, 10)
                new_value = current + mod
                
                if attr_manager:
                    # Use AttributeManager to handle cascading effects
                    attr_manager.set_attribute(attr, new_value, validate=False)
                else:
                    # Fallback only if AttributeManager is not available
                    logger.warning(f"AttributeManager not available, setting {attr} directly")
                    self.gff.set(attr, new_value)
                
                changes['ability_changes'].append({
                    'attribute': attr,
                    'old_value': current,
                    'new_value': new_value,
                    'modifier_applied': mod,
                    'source': 'subrace'
                })
    
    def _add_subrace_feats(self, subrace_data: Any, changes: Dict[str, Any]):
        """Add feats granted by subrace"""
        # Try to get feats from feats table reference first
        feats_table = field_mapper.get_field_value(subrace_data, 'feats_table', '')
        
        subrace_feats = []
        if feats_table and feats_table != '****':
            # TODO: Load feats from referenced table when that functionality is available
            logger.debug(f"Subrace references feats table: {feats_table}")
        
        # Also try direct feat fields as fallback
        direct_feats = field_mapper.get_racial_feats(subrace_data)
        subrace_feats.extend(direct_feats)
        
        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager:
            for feat_id in subrace_feats:
                if not feat_manager.has_feat(feat_id):
                    feat_manager.add_feat(feat_id, source='subrace')
                    feat_info = feat_manager.get_feat_info(feat_id)
                    changes['feat_changes']['added'].append({
                        'id': feat_id,
                        'name': feat_info['name'],
                        'source': 'subrace'
                    })
    
    def _get_racial_feats(self, race_id: int) -> List[int]:
        """Get list of feats granted by a race using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if not race_data:
            logger.debug(f"_get_racial_feats: No race data for race_id={race_id}")
            return []
            
        feats = []
        
        # 1. Get direct feats from columns (usually FeatIndex)
        direct_feats = field_mapper.get_racial_feats(race_data)
        feats.extend(direct_feats)
        logger.debug(f"_get_racial_feats: Race {race_id} direct feats from columns: {direct_feats}")
        
        # 2. Get feats from referenced FeatsTable
        feats_table_name = field_mapper.get_field_value(race_data, 'feats_table')
        logger.debug(f"_get_racial_feats: Race {race_id} feats_table_name='{feats_table_name}'")
        
        if feats_table_name and feats_table_name != '****':
            try:
                table_data = self.game_rules_service.get_table(feats_table_name.lower())
                if table_data:
                    for row in table_data:
                        feat_id = field_mapper.get_field_value(row, 'feat_index', -1)
                        if feat_id is not None:
                            try:
                                val = int(feat_id)
                                if val >= 0:
                                    feats.append(val)
                            except (ValueError, TypeError):
                                pass
            except Exception as e:
                logger.warning(f"Error loading racial feat table {feats_table_name}: {e}")
        
        logger.info(f"_get_racial_feats: Race {race_id} returning feats: {feats}")
        return feats

    def get_all_racial_feats(self) -> List[int]:
        """
        Get all feats currently granted by race and subrace
        
        Returns:
            List of feat IDs
        """
        race_id = self.gff.get('Race', 0)
        logger.debug(f"get_all_racial_feats: Getting feats for race_id={race_id}")
        
        # Base racial feats (now includes table lookup)
        feats = set(self._get_racial_feats(race_id))
        logger.debug(f"get_all_racial_feats: Base racial feats: {feats}")
        
        subrace_raw = self.gff.get('Subrace', '')
        subrace_name = self._get_subrace_name(subrace_raw)
        logger.debug(f"get_all_racial_feats: Subrace='{subrace_name}'")
        
        if subrace_name:
            subrace_data = self._get_subrace_data(subrace_name)
            if subrace_data:
                # Get subrace feats
                direct_feats = field_mapper.get_racial_feats(subrace_data)
                feats.update(direct_feats)
                logger.debug(f"get_all_racial_feats: Subrace direct feats: {direct_feats}")
                
                # Check for feats table too
                feats_table_name = field_mapper.get_field_value(subrace_data, 'feats_table', '')
                if feats_table_name and feats_table_name != '****':
                    logger.debug(f"get_all_racial_feats: Subrace loading feats from table: {feats_table_name}")
                    try:
                        table_name_lower = feats_table_name.lower()
                        table_data = self.game_rules_service.get_table(table_name_lower)
                        logger.debug(f"get_all_racial_feats: get_table('{table_name_lower}') returned: {type(table_data).__name__}, len={len(table_data) if table_data else 'None'}")
                        if table_data:
                            logger.debug(f"get_all_racial_feats: Subrace feat table loaded with {len(table_data)} rows")
                            for row_idx, row in enumerate(table_data):
                                feat_id = field_mapper.get_field_value(row, 'feat_index', -1)
                                logger.debug(f"get_all_racial_feats: Subrace table row {row_idx}: feat_id={feat_id} (type={type(feat_id).__name__})")
                                if feat_id is not None:
                                    try:
                                        val = int(feat_id)
                                        if val >= 0:
                                            feats.update([val])
                                            logger.debug(f"get_all_racial_feats: Added subrace feat {val}")
                                    except (ValueError, TypeError) as e:
                                        logger.debug(f"get_all_racial_feats: Failed to convert subrace feat_id={feat_id}: {e}")
                        else:
                            logger.warning(f"get_all_racial_feats: Subrace feat table '{feats_table_name.lower()}' returned None/empty")
                    except Exception as e:
                        logger.warning(f"Error loading subrace feat table {feats_table_name}: {e}")

        result = list(feats)
        
        return result
    
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
        
        # Fallback: Default to 30ft
        logger.debug(f"No speed data found for race {race_id}, defaulting to 30")
        return 30
    
    def _get_race_name(self, race_id: int) -> str:
        """Get race name from dynamic data, resolving TLK strref for proper localized name"""
        race_data = self._get_race_data(race_id)
        if race_data:
            # First get the Name field value
            name_value = field_mapper.get_field_value(race_data, 'name')
            
            if name_value is not None:
                # Check if it's already a usable string (not a number/strref)
                if isinstance(name_value, str) and name_value.strip() and not name_value.isdigit():
                    return name_value
                
                # Try to resolve as strref
                strref = field_mapper._safe_int(name_value, 0)
                if strref > 0:
                    resolved_name = self.game_rules_service._loader.get_string(strref)
                    if resolved_name and not resolved_name.startswith('{StrRef:'):
                        return resolved_name
            
            # Fallback to label if name resolution fails
            label = field_mapper.get_field_value(race_data, 'label')
            if label and str(label).strip():
                return str(label)
        return f'Race_{race_id}'
    
    def get_race_name(self, race_id: int = None) -> str:
        """Public method to get race name (for character summary)"""
        if race_id is None:
            race_id = self.gff.get('Race', 0)
        return self._get_race_name(race_id)
    
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
            size_data = self.game_rules_service.get_by_id('creaturesize', size)
            if size_data:
                # Get label using field mapping utility for safe access
                label = field_mapper.get_field_value(size_data, 'label')
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
        """Get comprehensive racial properties using dynamic data, including subrace"""
        race_id = self.gff.get('Race', 0)
        subrace_raw = self.gff.get('Subrace', '')
        
        # Convert subrace to string if it's an integer
        subrace = self._get_subrace_name(subrace_raw) if subrace_raw else ''
        
        # Base race properties
        creature_size = self.get_creature_size()
        properties = {
            'race_id': race_id,
            'race_name': self._get_race_name(race_id),
            'subrace': subrace,
            'size': creature_size,
            'size_name': self._get_size_name(creature_size),
            'base_speed': self._get_base_speed(race_id),
            'ability_modifiers': self._get_racial_ability_modifiers(race_id),
            'racial_feats': self._get_racial_feats(race_id),
            'favored_class': self._get_favored_class(race_id)
        }
        
        # Add subrace properties if subrace is specified
        if subrace:
            subrace_data = self._get_subrace_data(subrace)
            if subrace_data:
                subrace_props = field_mapper.get_subrace_properties(subrace_data)
                
                # Combine ability modifiers (base race + subrace)
                combined_modifiers = properties['ability_modifiers'].copy()
                for attr, mod in subrace_props['ability_modifiers'].items():
                    combined_modifiers[attr] += mod
                
                properties.update({
                    'subrace_data': subrace_props,
                    'ability_modifiers': combined_modifiers,  # Combined modifiers
                    'base_race_modifiers': properties['ability_modifiers'],  # Keep separate for reference
                    'subrace_modifiers': subrace_props['ability_modifiers'],
                    'effective_character_level': subrace_props.get('ecl', 0),
                    'subrace_favored_class': subrace_props.get('favored_class', -1),
                    'available_subraces': self.get_available_subraces(race_id)
                })
                
                # Override favored class if subrace has one
                if subrace_props.get('has_favored_class') and subrace_props.get('favored_class', -1) >= 0:
                    properties['favored_class'] = subrace_props['favored_class']
        else:
            # No subrace, add available subraces list
            properties['available_subraces'] = self.get_available_subraces(race_id)
        
        return properties
    
    def _get_racial_ability_modifiers(self, race_id: int) -> Dict[str, int]:
        """Get racial ability modifiers from dynamic data using field mapping utility"""
        race_data = self._get_race_data(race_id)
        if race_data:
            return field_mapper.get_ability_modifiers(race_data)
        return {}

    def get_racial_modifier_deltas(self) -> Dict[str, int]:
        """Get the difference between Subrace and Base Race modifiers to determine effective bonuses to get the correct effective value: GFF Value + (Subrace Mod - Base Race Mod)"""
        attributes = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']
        deltas = {attr: 0 for attr in attributes}

        # 1. Get Base Race modifiers (Baked)
        race_id = self.gff.get('Race', 0)
        base_mods = self._get_racial_ability_modifiers(race_id)

        # 2. Get Subrace modifiers (Target/Dynamic)
        subrace_raw = self.gff.get('Subrace', '')
        if not subrace_raw:
            return deltas # No subrace, no delta needed
            
        subrace_name = self._get_subrace_name(subrace_raw)
        subrace_data = self._get_subrace_data(subrace_name)
        sub_mods = field_mapper.get_ability_modifiers(subrace_data) if subrace_data else {}

        # 3. Calculate Delta (Subrace - Base)
        for attr in attributes:
            base = base_mods.get(attr, 0)
            sub = sub_mods.get(attr, 0)
            deltas[attr] = sub - base
            
        return deltas

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
    
    def validate_race_change(self, new_race_id: int, new_subrace: str = '') -> Tuple[bool, List[str]]:
        """Validate race change - check for race ID existence and subrace compatibility"""
        errors = []
        
        # Check if race exists using dynamic data - this prevents crashes/corruption
        race_data = self._get_race_data(new_race_id)
        if not race_data:
            errors.append(f"Unknown race ID: {new_race_id}")
            return False, errors
        
        # Validate subrace if provided
        if new_subrace:
            subrace_valid, subrace_errors = self.validate_subrace(new_race_id, new_subrace)
            if not subrace_valid:
                errors.extend(subrace_errors)
        
        # Remove other game rule validations - users can change to any race that exists
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
            size_data = self.game_rules_service.get_by_id('creaturesize', size_id)
            if size_data:
                # Use field mapping utility for safe access to AC modifier column
                ac_mod = field_mapper.get_field_value(size_data, 'ac_attack_mod', 0)
                try:
                    return int(ac_mod)
                except (ValueError, TypeError):
                    return 0
        except Exception as e:
            logger.warning(f"Could not get size modifier for size {size_id}: {e}")
        
        return 0
    
    def get_base_speed(self, race_id: int = None) -> int:
        """
        Get base movement speed for a race

        Args:
            race_id: The race ID, if None uses current character race

        Returns:
            Base movement speed in feet
        """
        if race_id is None:
            race_id = self.gff.get('Race', 0)  # Remove hardcoded human race ID

        return self._get_base_speed(race_id)

    def get_creature_size(self) -> int:
        """
        Get creature size from character GFF

        Returns:
            Creature size ID (1=Tiny, 2-3=Small, 4=Medium, 5=Large, 6=Huge)
        """
        return self.gff.get('CreatureSize', 4)