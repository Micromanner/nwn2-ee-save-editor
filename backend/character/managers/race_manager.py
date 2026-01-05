"""Race Manager - handles race changes and racial properties."""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, EventData
from dataclasses import dataclass
from gamedata.dynamic_loader.field_mapping_utility import field_mapper


@dataclass
class RaceChangedEvent(EventData):
    """Event data for race changes."""
    old_race_id: Optional[int]
    new_race_id: int
    old_subrace: Optional[str]
    new_subrace: Optional[str]
    
    def __post_init__(self):
        self.event_type = EventType.ALIGNMENT_CHANGED  # TODO: Add proper RACE_CHANGED event


class RaceManager(EventEmitter):
    """Manages race changes and racial property lookups."""
    
    def __init__(self, character_manager):
        """Initialize RaceManager with parent CharacterManager."""
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_rules_service = character_manager.rules_service
        
        self._race_data_cache = {}
        
        self._original_race = {
            'race_id': self.gff.get('Race'),
            'subrace': self.gff.get('Subrace'),
            'attributes': self._get_base_attributes()
        }
    
    def _get_race_data(self, race_id: int):
        """Get race data from cache or game data."""
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
        """Get subrace data from racialsubtypes.2da by name."""
        if not subrace_name:
            return None
            
        if isinstance(subrace_name, int):
            if subrace_name == 0:
                return None
            subrace_name = str(subrace_name)
        
        if isinstance(subrace_name, str) and not subrace_name.strip():
            return None
            
        try:
            all_subraces_list = self.game_rules_service.get_table('racialsubtypes')
            for subrace_data in all_subraces_list:
                if (field_mapper.get_field_value(subrace_data, 'subrace_name', '').lower() == subrace_name.lower() or
                    field_mapper.get_field_value(subrace_data, 'subrace_label', '').lower() == subrace_name.lower()):
                    return subrace_data
        except Exception as e:
            logger.warning(f"Could not load subrace data for '{subrace_name}': {e}")
            
        return None
    
    def _get_subrace_name(self, subrace_input) -> str:
        """Convert subrace ID or name to standardized subrace name string."""
        if not subrace_input:
            return ''
            
        if isinstance(subrace_input, str):
            return subrace_input.strip()
            
        if isinstance(subrace_input, int):
            if subrace_input == 0:
                return ''
                
            try:
                all_subraces_list = self.game_rules_service.get_table('racialsubtypes')
                if subrace_input < len(all_subraces_list):
                    subrace_data = all_subraces_list[subrace_input]
                    return field_mapper.get_field_value(subrace_data, 'subrace_name', '')
            except Exception as e:
                logger.warning(f"Could not load subrace name for ID {subrace_input}: {e}")
                
        return ''
    
    def get_available_subraces(self, race_id: int) -> List[Dict[str, Any]]:
        """Get list of available subraces for a given race."""
        subraces = []
        
        try:
            all_subraces_list = self.game_rules_service.get_table('racialsubtypes')
            for subrace_id, subrace_data in enumerate(all_subraces_list):
                base_race = field_mapper.get_field_value(subrace_data, 'base_race', 0)
                if field_mapper._safe_int(base_race) == race_id:
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
        """Validate that a subrace is compatible with a race."""
        errors = []
        
        if not subrace_name:
            return True, errors
            
        if isinstance(subrace_name, int) and subrace_name == 0:
            return True, errors
            
        if isinstance(subrace_name, str) and not subrace_name.strip():
            return True, errors
            
        subrace_data = self._get_subrace_data(subrace_name)
        if not subrace_data:
            errors.append(f"Unknown subrace: {subrace_name}")
            return False, errors
            
        base_race = field_mapper.get_field_value(subrace_data, 'base_race', 0)
        if field_mapper._safe_int(base_race) != race_id:
            errors.append(f"Subrace '{subrace_name}' does not belong to race ID {race_id}")
            return False, errors
            
        player_race = field_mapper.get_field_value(subrace_data, 'player_race', 1)
        if not field_mapper._safe_bool(player_race):
            errors.append(f"Subrace '{subrace_name}' is not available to players")
            return False, errors
            
        return True, errors
    
    def change_race(self, new_race_id: int, new_subrace: str = '', 
                   preserve_feats: bool = True) -> Dict[str, Any]:
        """Change character's race and apply all associated changes."""
        logger.info(f"Changing race to {new_race_id} (subrace: {new_subrace})")
        
        old_race_id = self.gff.get('Race')
        old_subrace = self.gff.get('Subrace')
        
        new_race = self._get_race_data(new_race_id)
        if not new_race:
            raise ValueError(f"Unknown race ID: {new_race_id}")
            
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
        
        old_race = self._get_race_data(old_race_id)
        if old_race:
            self._remove_racial_modifiers(changes)
        
        self.gff.set('Race', new_race_id)
        self.gff.set('Subrace', new_subrace)
        
        self._apply_racial_modifiers(new_race_id, changes)
        
        if new_subrace_data:
            self._apply_subrace_modifiers(new_subrace_data, changes)
        
        old_size = self.gff.get('CreatureSize')
        new_size = self._get_race_size(new_race_id)
        if old_size != new_size:
            self.gff.set('CreatureSize', new_size)
            changes['size_change'] = {
                'old': old_size,
                'new': new_size,
                'old_name': self._get_size_name(old_size),
                'new_name': self._get_size_name(new_size)
            }
        
        old_speed = self._get_base_speed(old_race_id)
        new_speed = self._get_base_speed(new_race_id)
        if old_speed != new_speed:
            changes['speed_change'] = {
                'old': old_speed,
                'new': new_speed
            }
        
        if not preserve_feats:
            self._remove_racial_feats(old_race_id, changes)
        self._add_racial_feats(new_race_id, changes)
        
        if new_subrace_data:
            self._add_subrace_feats(new_subrace_data, changes)
        
        event = RaceChangedEvent(
            event_type=EventType.ALIGNMENT_CHANGED,
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
        """Get current base attributes from GFF."""
        return {
            'Str': self.gff.get('Str'),
            'Dex': self.gff.get('Dex'),
            'Con': self.gff.get('Con'),
            'Int': self.gff.get('Int'),
            'Wis': self.gff.get('Wis'),
            'Cha': self.gff.get('Cha')
        }
    
    def _remove_racial_modifiers(self, changes: Dict[str, Any]):
        """Remove ability modifiers from old race."""
        race_id = self.gff.get('Race')
        racial_mods = self._get_racial_ability_modifiers(race_id)
        logger.debug(f"Removing racial modifiers for race {race_id}: {racial_mods}")
        
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr)
                new_value = current - mod
                self.gff.set(attr, new_value)
                
                changes['ability_changes'].append({
                    'attribute': attr,
                    'old_value': current,
                    'new_value': new_value,
                    'modifier_removed': mod
                })
    
    def _apply_racial_modifiers(self, new_race_id: int, changes: Dict[str, Any]):
        """Apply ability modifiers from new race."""
        racial_mods = self._get_racial_ability_modifiers(new_race_id)
        logger.debug(f"Applying racial modifiers for race {new_race_id}: {racial_mods}")
        
        attr_manager = self.character_manager.get_manager('ability')
        if not attr_manager:
            raise RuntimeError("AttributeManager not available")
        
        for attr, mod in racial_mods.items():
            if mod != 0:
                current = self.gff.get(attr)
                new_value = current + mod
                attr_manager.set_attribute(attr, new_value, validate=False)
                
                changes['ability_changes'].append({
                    'attribute': attr,
                    'old_value': current,
                    'new_value': new_value,
                    'modifier_applied': mod
                })
    
    def _remove_racial_feats(self, race_id: int, changes: Dict[str, Any]):
        """Remove feats granted by old race."""
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
        """Add feats granted by new race."""
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
        """Apply ability modifiers from subrace (stacks with base race)."""
        subrace_mods = field_mapper.get_ability_modifiers(subrace_data)
        logger.debug(f"Applying subrace modifiers: {subrace_mods}")
        
        attr_manager = self.character_manager.get_manager('ability')
        if not attr_manager:
            raise RuntimeError("AttributeManager not available")
        
        for attr, mod in subrace_mods.items():
            if mod != 0:
                current = self.gff.get(attr)
                new_value = current + mod
                attr_manager.set_attribute(attr, new_value, validate=False)
                
                changes['ability_changes'].append({
                    'attribute': attr,
                    'old_value': current,
                    'new_value': new_value,
                    'modifier_applied': mod,
                    'source': 'subrace'
                })
    
    def _add_subrace_feats(self, subrace_data: Any, changes: Dict[str, Any]):
        """Add feats granted by subrace."""
        feats_table = field_mapper.get_field_value(subrace_data, 'feats_table', '')
        
        subrace_feats = []
        if feats_table and feats_table != '****':
            logger.debug(f"Subrace references feats table: {feats_table}")
        
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
        """Get list of feats granted by a race."""
        race_data = self._get_race_data(race_id)
        if not race_data:
            logger.debug(f"No race data for race_id={race_id}")
            return []
            
        feats = []
        
        direct_feats = field_mapper.get_racial_feats(race_data)
        feats.extend(direct_feats)
        logger.debug(f"Race {race_id} direct feats: {direct_feats}")
        
        feats_table_name = field_mapper.get_field_value(race_data, 'feats_table')
        logger.debug(f"Race {race_id} feats_table_name='{feats_table_name}'")
        
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
        
        logger.info(f"Race {race_id} returning feats: {feats}")
        return feats

    def get_all_racial_feats(self) -> List[int]:
        """Get all feats currently granted by race and subrace."""
        race_id = self.gff.get('Race')
        logger.debug(f"Getting feats for race_id={race_id}")
        
        feats = set(self._get_racial_feats(race_id))
        logger.debug(f"Base racial feats: {feats}")
        
        subrace_raw = self.gff.get('Subrace')
        subrace_name = self._get_subrace_name(subrace_raw) if subrace_raw else ''
        logger.debug(f"Subrace='{subrace_name}'")
        
        if subrace_name:
            subrace_data = self._get_subrace_data(subrace_name)
            if subrace_data:
                direct_feats = field_mapper.get_racial_feats(subrace_data)
                feats.update(direct_feats)
                logger.debug(f"Subrace direct feats: {direct_feats}")
                
                feats_table_name = field_mapper.get_field_value(subrace_data, 'feats_table', '')
                if feats_table_name and feats_table_name != '****':
                    logger.debug(f"Subrace loading feats from table: {feats_table_name}")
                    try:
                        table_name_lower = feats_table_name.lower()
                        table_data = self.game_rules_service.get_table(table_name_lower)
                        logger.debug(f"get_table('{table_name_lower}') returned: {type(table_data).__name__}, len={len(table_data) if table_data else 'None'}")
                        if table_data:
                            logger.debug(f"Subrace feat table loaded with {len(table_data)} rows")
                            for row_idx, row in enumerate(table_data):
                                feat_id = field_mapper.get_field_value(row, 'feat_index', -1)
                                logger.debug(f"Subrace table row {row_idx}: feat_id={feat_id} (type={type(feat_id).__name__})")
                                if feat_id is not None:
                                    try:
                                        val = int(feat_id)
                                        if val >= 0:
                                            feats.update([val])
                                            logger.debug(f"Added subrace feat {val}")
                                    except (ValueError, TypeError) as e:
                                        logger.debug(f"Failed to convert subrace feat_id={feat_id}: {e}")
                        else:
                            logger.warning(f"Subrace feat table '{feats_table_name.lower()}' returned None/empty")
                    except Exception as e:
                        logger.warning(f"Error loading subrace feat table {feats_table_name}: {e}")

        return list(feats)
    
    def _get_base_speed(self, race_id: int) -> int:
        """Get base movement speed for a race."""
        race_data = self._get_race_data(race_id)
        if not race_data:
            raise ValueError(f"No race data for race_id={race_id}")
            
        speed = field_mapper.get_field_value(race_data, 'movement_rate')
        if speed is None:
            raise ValueError(f"No movement_rate for race_id={race_id}")
        
        return int(speed)
    
    def _get_race_name(self, race_id: int) -> str:
        """Get race name, resolving TLK strref for localized name."""
        race_data = self._get_race_data(race_id)
        if race_data:
            name_value = field_mapper.get_field_value(race_data, 'name')
            
            if name_value is not None:
                if isinstance(name_value, str) and name_value.strip() and not name_value.isdigit():
                    return name_value
                
                strref = field_mapper._safe_int(name_value, 0)
                if strref > 0:
                    resolved_name = self.game_rules_service._loader.get_string(strref)
                    if resolved_name and not resolved_name.startswith('{StrRef:'):
                        return resolved_name
            
            label = field_mapper.get_field_value(race_data, 'label')
            if label and str(label).strip():
                return str(label)
                
        return f'Race_{race_id}'
    
    def get_race_name(self, race_id: int = None) -> str:
        """Public method to get race name."""
        if race_id is None:
            race_id = self.gff.get('Race')
        return self._get_race_name(race_id)
    
    def _get_race_size(self, race_id: int) -> int:
        """Get creature size for a race."""
        race_data = self._get_race_data(race_id)
        if not race_data:
            raise ValueError(f"No race data for race_id={race_id}")
            
        size = field_mapper.get_field_value(race_data, 'creature_size')
        if size is None:
            raise ValueError(f"No creature_size for race_id={race_id}")
            
        return int(size)
    
    def _get_size_name(self, size: int) -> str:
        """Get size category name from game data."""
        try:
            size_data = self.game_rules_service.get_by_id('creaturesize', size)
            if size_data:
                label = field_mapper.get_field_value(size_data, 'label')
                if label and label != 'INVALID':
                    return label.title()
        except Exception as e:
            logger.debug(f"Could not get size name for size {size}: {e}")
        
        return f'Size_{size}'
    
    def get_racial_properties(self) -> Dict[str, Any]:
        """Get comprehensive racial properties including subrace."""
        race_id = self.gff.get('Race')
        subrace_raw = self.gff.get('Subrace')
        
        subrace = self._get_subrace_name(subrace_raw) if subrace_raw else ''
        
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
        
        if subrace:
            subrace_data = self._get_subrace_data(subrace)
            if subrace_data:
                subrace_props = field_mapper.get_subrace_properties(subrace_data)
                
                combined_modifiers = properties['ability_modifiers'].copy()
                for attr, mod in subrace_props['ability_modifiers'].items():
                    combined_modifiers[attr] += mod
                
                properties.update({
                    'subrace_data': subrace_props,
                    'ability_modifiers': combined_modifiers,
                    'base_race_modifiers': properties['ability_modifiers'],
                    'subrace_modifiers': subrace_props['ability_modifiers'],
                    'effective_character_level': subrace_props.get('ecl', 0),
                    'subrace_favored_class': subrace_props.get('favored_class', -1),
                    'available_subraces': self.get_available_subraces(race_id)
                })
                
                if subrace_props.get('has_favored_class') and subrace_props.get('favored_class', -1) >= 0:
                    properties['favored_class'] = subrace_props['favored_class']
        else:
            properties['available_subraces'] = self.get_available_subraces(race_id)
        
        return properties
    
    def _get_racial_ability_modifiers(self, race_id: int) -> Dict[str, int]:
        """Get racial ability modifiers from game data."""
        race_data = self._get_race_data(race_id)
        if race_data:
            return field_mapper.get_ability_modifiers(race_data)
        return {}

    def get_racial_modifier_deltas(self) -> Dict[str, int]:
        """Get difference between subrace and base race modifiers."""
        attributes = ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']
        deltas = {attr: 0 for attr in attributes}

        race_id = self.gff.get('Race')
        base_mods = self._get_racial_ability_modifiers(race_id)

        subrace_raw = self.gff.get('Subrace')
        if not subrace_raw:
            return deltas
            
        subrace_name = self._get_subrace_name(subrace_raw)
        subrace_data = self._get_subrace_data(subrace_name)
        sub_mods = field_mapper.get_ability_modifiers(subrace_data) if subrace_data else {}

        for attr in attributes:
            base = base_mods.get(attr, 0)
            sub = sub_mods.get(attr, 0)
            deltas[attr] = sub - base
            
        return deltas

    def _get_favored_class(self, race_id: int) -> Optional[int]:
        """Get favored class for a race."""
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
        """Validate race change for race ID existence and subrace compatibility."""
        errors = []
        
        race_data = self._get_race_data(new_race_id)
        if not race_data:
            errors.append(f"Unknown race ID: {new_race_id}")
            return False, errors
        
        if new_subrace:
            subrace_valid, subrace_errors = self.validate_subrace(new_race_id, new_subrace)
            if not subrace_valid:
                errors.extend(subrace_errors)
        
        return len(errors) == 0, errors
    
    def revert_to_original_race(self) -> Dict[str, Any]:
        """Revert character to original race."""
        return self.change_race(
            self._original_race['race_id'],
            self._original_race['subrace']
        )
    
    def get_race_summary(self) -> Dict[str, Any]:
        """Get summary of racial properties with readable modifier strings."""
        props = self.get_racial_properties()
        
        mod_strings = []
        for attr, mod in props['ability_modifiers'].items():
            if mod != 0:
                mod_strings.append(f"{attr} {mod:+d}")
        
        props['ability_modifier_string'] = ", ".join(mod_strings) if mod_strings else "None"
        
        return props
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current race configuration for corruption prevention."""
        errors = []
        
        race_id = self.gff.get('Race')
        if race_id is None:
            errors.append("Race field missing from GFF")
            return False, errors
        
        race_data = self._get_race_data(race_id)
        if not race_data:
            errors.append(f"Invalid race ID: {race_id}")
        
        return len(errors) == 0, errors
    
    def get_size_modifier(self, size_id: int) -> int:
        """Get AC/attack modifier for a creature size."""
        try:
            size_data = self.game_rules_service.get_by_id('creaturesize', size_id)
            if size_data:
                ac_mod = field_mapper.get_field_value(size_data, 'ac_attack_mod', 0)
                try:
                    return int(ac_mod)
                except (ValueError, TypeError):
                    return 0
        except Exception as e:
            logger.warning(f"Could not get size modifier for size {size_id}: {e}")
        
        return 0
    
    def get_base_speed(self, race_id: int = None) -> int:
        """Get base movement speed for a race."""
        if race_id is None:
            race_id = self.gff.get('Race')

        return self._get_base_speed(race_id)

    def get_creature_size(self) -> int:
        """Get creature size from character GFF."""
        size = self.gff.get('CreatureSize')
        if size is None:
            raise ValueError("CreatureSize field missing from GFF")
        return size