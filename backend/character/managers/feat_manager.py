"""
Data-Driven Feat Manager - handles feat additions, removals, and protection
Uses CharacterManager and DynamicGameDataLoader for all feat data access
"""

from typing import Dict, List, Set, Tuple, Optional, Any
import logging
import time

from ..events import (
    EventEmitter, EventType, EventData, 
    ClassChangedEvent, LevelGainedEvent, FeatChangedEvent
)
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)


class FeatManager(EventEmitter):
    """
    Data-Driven Feat Manager
    Uses CharacterManager as hub for all character data access
    """
    
    def __init__(self, character_manager):
        """
        Initialize the FeatManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.game_data_loader = character_manager.game_data_loader
        self.gff = character_manager.gff
        
        # Register for events
        self._register_event_handlers()
        
        # Cache for performance
        self._feat_cache = {}
        self._class_cache = {}  # Cache for class lookups
        self._protected_feats: Set[int] = set()
        self._update_protected_feats()
    
    def _register_event_handlers(self):
        """Register handlers for relevant events"""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self.on_level_gained)
    
    def _update_protected_feats(self):
        """Update the set of protected feat IDs"""
        self._protected_feats.clear()
        
        # Add all custom content feats
        for content_id, info in self.character_manager.custom_content.items():
            if info['type'] == 'feat' and info.get('protected', False):
                self._protected_feats.add(info['id'])
        
        # Use character manager's epithet feat detection
        epithet_feats = self.character_manager.detect_epithet_feats()
        self._protected_feats.update(epithet_feats)
        
        logger.debug(f"Protected feats updated: {len(self._protected_feats)} feats protected")
    
    def on_class_changed(self, event: ClassChangedEvent):
        """Handle class change event"""
        logger.info(f"FeatManager handling class change: {event.old_class_id} -> {event.new_class_id}")
        
        # Remove old class feats (except protected)
        if event.old_class_id is not None:
            self._remove_class_feats(event.old_class_id, event.level, event.preserve_feats)
        
        # Add new class feats
        self._add_class_feats(event.new_class_id, event.level)
    
    def on_level_gained(self, event: LevelGainedEvent):
        """Handle level gain event"""
        logger.info(f"FeatManager handling level gain: Class {event.class_id}, Level {event.new_level}")
        
        # Add feats for the new level using dynamic game data
        class_data = self.game_data_loader.get_by_id('classes', event.class_id)
        if class_data:
            # Use character manager's method to get class feats for level
            feats_at_level = self.character_manager.get_class_feats_for_level(
                class_data, event.new_level
            )
            
            for feat_info in feats_at_level:
                if feat_info['list_type'] == 0:  # Auto-granted
                    feat_id = feat_info['feat_id']
                    
                    # Check if this is a progression feat that replaces an older version
                    old_feat_id = self._check_feat_progression(feat_id, event.class_id)
                    if old_feat_id:
                        # Remove the old version first
                        logger.info(f"Progressing feat: {old_feat_id} -> {feat_id}")
                        self.remove_feat(old_feat_id, force=True)
                    
                    self.add_feat(feat_id, source='level')
    
    def add_feat(self, feat_id: int, source: str = 'manual') -> bool:
        """
        Add a feat to the character
        
        Args:
            feat_id: The feat ID to add
            source: Source of the feat ('class', 'level', 'manual')
            
        Returns:
            True if feat was added
        """
        # Check if already has feat
        if self.has_feat(feat_id):
            logger.debug(f"Character already has feat {feat_id}")
            return False
        
        # Validate prerequisites
        if source == 'manual':
            is_valid, errors = self.validate_feat_prerequisites(feat_id)
            if not is_valid:
                logger.warning(f"Cannot add feat {feat_id}: {errors}")
                return False
        
        # Add to feat list
        feat_list = self.gff.get('FeatList', [])
        feat_list.append({'Feat': feat_id})
        self.gff.set('FeatList', feat_list)
        
        # Emit event
        event = FeatChangedEvent(
            event_type=EventType.FEAT_ADDED,  # Will be overridden by __post_init__
            source_manager='feat',
            timestamp=time.time(),
            feat_id=feat_id,
            action='added',
            source=source
        )
        self.character_manager.emit(event)
        
        logger.info(f"Added feat {feat_id} from source: {source}")
        return True
    
    def remove_feat(self, feat_id: int, force: bool = False) -> bool:
        """
        Remove a feat from the character
        
        Args:
            feat_id: The feat ID to remove
            force: Force removal even if protected
            
        Returns:
            True if feat was removed
        """
        # Check protection
        if not force and self.is_feat_protected(feat_id):
            logger.warning(f"Cannot remove protected feat {feat_id}")
            return False
        
        # Find and remove feat
        feat_list = self.gff.get('FeatList', [])
        original_count = len(feat_list)
        feat_list = [f for f in feat_list if f.get('Feat') != feat_id]
        
        if len(feat_list) < original_count:
            self.gff.set('FeatList', feat_list)
            
            # Emit event
            event = FeatChangedEvent(
                event_type=EventType.FEAT_REMOVED,  # Will be overridden by __post_init__
                source_manager='feat',
                timestamp=time.time(),
                feat_id=feat_id,
                action='removed',
                source='manual'
            )
            self.character_manager.emit(event)
            
            logger.info(f"Removed feat {feat_id}")
            return True
        
        return False
    
    def has_feat(self, feat_id: int) -> bool:
        """Check if character has a specific feat"""
        feat_list = self.gff.get('FeatList', [])
        return any(f.get('Feat') == feat_id for f in feat_list)
    
    def is_feat_protected(self, feat_id: int) -> bool:
        """Check if a feat is protected from removal"""
        return feat_id in self._protected_feats
    
    def _check_feat_progression(self, new_feat_id: int, class_id: int) -> Optional[int]:
        """
        Check if a feat is part of a progression that replaces an older version
        
        Returns the old feat ID that should be removed, or None
        """
        # Get the new feat's data using dynamic game data
        new_feat = self.game_data_loader.get_by_id('feat', new_feat_id)
        if not new_feat:
            return None
            
        # Get label using dynamic data attributes
        new_label = getattr(new_feat, 'label', getattr(new_feat, 'name', ''))
        
        # Try to parse progression from feat name
        # Patterns: "Something2", "Something_2", "FEAT_SOMETHING_2"
        import re
        match = re.search(r'^(.*?)[\s_]?(\d+)$', new_label)
        if not match:
            return None
            
        base_name = match.group(1).rstrip('_')
        new_number = int(match.group(2))
        
        # Only consider it a progression if number is 2 or higher
        if new_number < 2:
            return None
            
        # Look for the previous version in character's feats
        feat_list = self.gff.get('FeatList', [])
        character_feat_ids = {f.get('Feat') for f in feat_list}
        
        # Search for feats with same base name but lower number
        for feat_id in character_feat_ids:
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            if not feat_data:
                continue
                
            label = getattr(feat_data, 'label', getattr(feat_data, 'name', ''))
            
            # Check if it's the same feat family
            if label.startswith(base_name):
                # Try to extract number
                old_match = re.search(r'^(.*?)[\s_]?(\d+)$', label)
                if old_match:
                    old_base = old_match.group(1).rstrip('_')
                    old_number = int(old_match.group(2))
                    
                    # If same base and lower number, this is what we replace
                    if old_base == base_name and old_number < new_number:
                        logger.info(f"Auto-detected progression: {label} -> {new_label}")
                        return feat_id
                        
                # Also check for base version without number (e.g., "BarbarianRage" -> "BarbarianRage2")
                elif label == base_name or label == base_name.rstrip('_'):
                    logger.info(f"Auto-detected progression: {label} -> {new_label}")
                    return feat_id
        
        return None
    
    def get_feat_info(self, feat_id: int, feat_data=None) -> Optional[Dict[str, Any]]:
        """
        Get detailed information about a feat
        
        Args:
            feat_id: ID of the feat
            feat_data: Optional pre-loaded feat data to avoid redundant lookups
        """
        # Check cache first
        if feat_id in self._feat_cache:
            return self._feat_cache[feat_id]
        
        # Use provided feat_data or get from dynamic game data loader
        if feat_data is None:
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
        
        if feat_data:
            # Use label as the primary name
            label = getattr(feat_data, 'label', getattr(feat_data, 'name', f'Feat_{feat_id}'))
            
            # Get prerequisites and check if character meets them
            prereqs = field_mapper.get_feat_prerequisites(feat_data)
            can_take, missing_reqs = self.validate_feat_prerequisites(feat_id, feat_data)
            
            # Get description
            description = field_mapper.get_field_value(feat_data, 'description', '')
            
            # Get icon reference
            icon = field_mapper.get_field_value(feat_data, 'icon', '')
            
            info = {
                'id': feat_id,
                'label': label,
                'name': label,  # Use label as name since it's more readable
                'type': getattr(feat_data, 'type', getattr(feat_data, 'feat_type', 0)),
                'protected': self.is_feat_protected(feat_id),
                'custom': self.character_manager.is_custom_content('feat', feat_id),
                'description': description,
                'icon': icon,
                'prerequisites': prereqs,
                'can_take': can_take,
                'missing_requirements': missing_reqs,
                'has_feat': self.has_feat(feat_id)
            }
            self._feat_cache[feat_id] = info
            return info
        
        # Unknown feat
        return {
            'id': feat_id,
            'label': f'Unknown_{feat_id}',
            'name': f'Unknown Feat {feat_id}',
            'type': 0,
            'protected': True,  # Protect unknown feats
            'custom': True,
            'description': 'Unknown or custom feat',
            'icon': '',
            'prerequisites': {'abilities': {}, 'feats': [], 'class': -1, 'level': 0, 'bab': 0, 'spell_level': 0},
            'can_take': False,
            'missing_requirements': ['Unknown feat - cannot validate'],
            'has_feat': self.has_feat(feat_id)
        }
    
    def validate_feat_prerequisites(self, feat_id: int, feat_data=None) -> Tuple[bool, List[str]]:
        """
        Validate if character meets prerequisites for a feat
        
        Args:
            feat_id: ID of the feat to validate
            feat_data: Optional pre-loaded feat data to avoid redundant lookups
        
        Returns:
            (is_valid, list_of_errors)
        """
        errors = []
        if feat_data is None:
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
        
        if not feat_data:
            return True, []  # Allow unknown feats (custom content)
        
        # Check prerequisites using field mapping utility
        prereqs = field_mapper.get_feat_prerequisites(feat_data)
        
        # Check ability score prerequisites
        for ability, min_score in prereqs['abilities'].items():
            if min_score > 0:
                current_score = self.gff.get(ability, 10)
                if current_score < min_score:
                    errors.append(f"Requires {ability.upper()} {min_score}")
        
        # Check feat prerequisites
        for prereq_feat_id in prereqs['feats']:
            if not self.has_feat(prereq_feat_id):
                # Check cache first to avoid redundant lookups
                if prereq_feat_id in self._feat_cache:
                    prereq_name = self._feat_cache[prereq_feat_id].get('label', f'Feat {prereq_feat_id}')
                else:
                    prereq_feat_data = self.game_data_loader.get_by_id('feat', prereq_feat_id)
                    if prereq_feat_data is None:
                        logger.warning(f"Prerequisite feat ID {prereq_feat_id} not found in feat table (for feat {feat_id})")
                    prereq_name = field_mapper.get_field_value(prereq_feat_data, 'label', f'Feat {prereq_feat_id}') if prereq_feat_data else f'Feat {prereq_feat_id}'
                errors.append(f"Requires {prereq_name}")
        
        # Check class requirements
        if prereqs['class'] >= 0:
            class_list = self.gff.get('ClassList', [])
            has_class = any(c.get('Class') == prereqs['class'] for c in class_list)
            if not has_class:
                # Check cache first to avoid redundant lookups
                class_id = prereqs['class']
                if class_id in self._class_cache:
                    class_name = self._class_cache[class_id]
                else:
                    class_data = self.game_data_loader.get_by_id('classes', class_id)
                    class_name = field_mapper.get_field_value(class_data, 'label', f'Class {class_id}') if class_data else f'Class {class_id}'
                    self._class_cache[class_id] = class_name
                errors.append(f"Requires {class_name} class")
        
        # Check level requirements
        if prereqs['level'] > 0:
            total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            if total_level < prereqs['level']:
                errors.append(f"Requires level {prereqs['level']}")
                
        # Check BAB requirements
        if prereqs['bab'] > 0:
            # TODO: Implement proper BAB calculation
            # For now, approximate based on total level and classes
            total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            estimated_bab = total_level  # Simplified - should calculate based on class BAB tables
            if estimated_bab < prereqs['bab']:
                errors.append(f"Requires BAB +{prereqs['bab']}")
                
        # Check spell level requirements
        if prereqs['spell_level'] > 0:
            # TODO: Implement spell level requirement validation
            pass
        
        return len(errors) == 0, errors
    
    def get_detailed_prerequisites(self, feat_id: int) -> Dict[str, Any]:
        """
        Get detailed prerequisite information in a user-friendly format
        
        Returns:
            Dict with detailed prerequisite breakdown
        """
        feat_data = self._get_feat_data_cached(feat_id)
        if not feat_data:
            return {'requirements': [], 'met': [], 'unmet': []}
        
        prereqs = field_mapper.get_feat_prerequisites(feat_data)
        detailed = {
            'requirements': [],
            'met': [],
            'unmet': []
        }
        
        # Ability score requirements
        for ability, min_score in prereqs['abilities'].items():
            if min_score > 0:
                current_score = self.gff.get(ability, 10)
                req_text = f"{ability.upper()} {min_score}+"
                detailed['requirements'].append({
                    'type': 'ability',
                    'description': req_text,
                    'required_value': min_score,
                    'current_value': current_score,
                    'met': current_score >= min_score
                })
                
                if current_score >= min_score:
                    detailed['met'].append(req_text)
                else:
                    detailed['unmet'].append(f"{req_text} (current: {current_score})")
        
        # Feat requirements
        for prereq_feat_id in prereqs['feats']:
            has_prereq = self.has_feat(prereq_feat_id)
            # Check cache first to avoid redundant lookups
            if prereq_feat_id in self._feat_cache:
                prereq_name = self._feat_cache[prereq_feat_id].get('label', f'Feat {prereq_feat_id}')
            else:
                prereq_feat_data = self.game_data_loader.get_by_id('feat', prereq_feat_id)
                prereq_name = field_mapper.get_field_value(prereq_feat_data, 'label', f'Feat {prereq_feat_id}') if prereq_feat_data else f'Feat {prereq_feat_id}'
            
            detailed['requirements'].append({
                'type': 'feat',
                'description': prereq_name,
                'feat_id': prereq_feat_id,
                'met': has_prereq
            })
            
            if has_prereq:
                detailed['met'].append(prereq_name)
            else:
                detailed['unmet'].append(prereq_name)
        
        # Class requirements
        if prereqs['class'] >= 0:
            class_list = self.gff.get('ClassList', [])
            has_class = any(c.get('Class') == prereqs['class'] for c in class_list)
            # Check cache first to avoid redundant lookups
            class_id = prereqs['class']
            if class_id in self._class_cache:
                class_name = self._class_cache[class_id]
            else:
                class_data = self.game_data_loader.get_by_id('classes', class_id)
                class_name = field_mapper.get_field_value(class_data, 'label', f'Class {class_id}') if class_data else f'Class {class_id}'
                self._class_cache[class_id] = class_name
            
            detailed['requirements'].append({
                'type': 'class',
                'description': f"{class_name} class",
                'class_id': prereqs['class'],
                'met': has_class
            })
            
            if has_class:
                detailed['met'].append(f"{class_name} class")
            else:
                detailed['unmet'].append(f"{class_name} class")
        
        # Level requirements
        if prereqs['level'] > 0:
            total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            req_text = f"Level {prereqs['level']}"
            
            detailed['requirements'].append({
                'type': 'level',
                'description': req_text,
                'required_value': prereqs['level'],
                'current_value': total_level,
                'met': total_level >= prereqs['level']
            })
            
            if total_level >= prereqs['level']:
                detailed['met'].append(req_text)
            else:
                detailed['unmet'].append(f"{req_text} (current: {total_level})")
        
        # BAB requirements
        if prereqs['bab'] > 0:
            total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            estimated_bab = total_level  # Simplified calculation
            req_text = f"BAB +{prereqs['bab']}"
            
            detailed['requirements'].append({
                'type': 'bab',
                'description': req_text,
                'required_value': prereqs['bab'],
                'current_value': estimated_bab,
                'met': estimated_bab >= prereqs['bab']
            })
            
            if estimated_bab >= prereqs['bab']:
                detailed['met'].append(req_text)
            else:
                detailed['unmet'].append(f"{req_text} (current: +{estimated_bab})")
        
        # Spell level requirements
        if prereqs['spell_level'] > 0:
            req_text = f"Able to cast {prereqs['spell_level']}th level spells"
            # TODO: Implement proper spell level checking
            detailed['requirements'].append({
                'type': 'spell_level',
                'description': req_text,
                'required_value': prereqs['spell_level'],
                'met': False  # Default to false until implemented
            })
            detailed['unmet'].append(req_text)
        
        return detailed
    
    def _remove_class_feats(self, class_id: int, level: int, preserve_list: List[int]):
        """Remove feats granted by a class"""
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            return
        
        # Collect all auto-granted feats for this class
        feats_to_remove = []
        for lvl in range(1, level + 1):
            feats_at_level = self.character_manager.get_class_feats_for_level(class_data, lvl)
            for feat_info in feats_at_level:
                if feat_info['list_type'] == 0:  # Auto-granted
                    feat_id = feat_info['feat_id']
                    # Skip if in preserve list or protected
                    if feat_id not in preserve_list and not self.is_feat_protected(feat_id):
                        feats_to_remove.append(feat_id)
        
        # Remove feats
        removed_count = 0
        for feat_id in feats_to_remove:
            if self.remove_feat(feat_id):
                removed_count += 1
        
        if removed_count > 0:
            class_name = getattr(class_data, 'label', getattr(class_data, 'name', f'Class {class_id}'))
            logger.info(f"Removed {removed_count} feats from {class_name}")
    
    def _add_class_feats(self, class_id: int, level: int):
        """Add feats granted by a class"""
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            return
        
        # Add all auto-granted feats for this class
        added_count = 0
        for lvl in range(1, level + 1):
            feats_at_level = self.character_manager.get_class_feats_for_level(class_data, lvl)
            for feat_info in feats_at_level:
                if feat_info['list_type'] == 0:  # Auto-granted
                    if self.add_feat(feat_info['feat_id'], source='class'):
                        added_count += 1
        
        if added_count > 0:
            class_name = getattr(class_data, 'label', getattr(class_data, 'name', f'Class {class_id}'))
            logger.info(f"Added {added_count} feats for {class_name}")
    
    def is_legitimate_feat(self, feat_data) -> bool:
        """
        Check if a feat is legitimate (not a dev/broken feat)
        
        Args:
            feat_data: The feat data object from dynamic game data
            
        Returns:
            True if feat is legitimate and should be shown to users
        """
        # Get label using field mapping utility
        label = field_mapper.get_field_value(feat_data, 'label', '')
        if not label:
            return False
        
        # Check for explicitly removed feats
        removed = field_mapper.get_field_value(feat_data, 'removed', '0')
        if removed == '1' or removed == 1:
            return False
        
        # Check for empty/placeholder labels
        if label.strip() in ['****', '']:
            return False
        
        # Check for deleted feat prefixes
        if label.startswith('DEL_') or label == 'DELETED':
            return False
        
        return True
    
    def get_legitimate_feats(self, feat_type: Optional[int] = None) -> List[Dict[str, Any]]:
        """
        Get list of all legitimate feats (filtered to remove dev/broken feats)
        
        Args:
            feat_type: Optional feat type filter
            
        Returns:
            List of feat info dicts for legitimate feats only
        """
        legitimate = []
        
        # Get all feats from dynamic game data
        all_feats = self.game_data_loader.get_table('feat')
        if not all_feats:
            return legitimate
        
        for row_index, feat_data in enumerate(all_feats):
            # Filter out illegitimate feats first
            if not self.is_legitimate_feat(feat_data):
                continue
                
            # Use proper row index as feat ID
            feat_id = getattr(feat_data, 'id', getattr(feat_data, 'row_index', row_index))
            
            # Skip if wrong type
            if feat_type is not None:
                data_type = getattr(feat_data, 'type', getattr(feat_data, 'feat_type', 0))
                if data_type != feat_type:
                    continue
            
            feat_info = self.get_feat_info(feat_id, feat_data)
            if feat_info:
                legitimate.append(feat_info)
        
        return legitimate

    def get_available_feats(self, feat_type: Optional[int] = None) -> List[Dict[str, Any]]:
        """
        Get list of feats available for selection - OPTIMIZED VERSION
        
        Args:
            feat_type: Optional feat type filter
            
        Returns:
            List of feat info dicts
        """
        available = []
        
        # Get all feats from dynamic game data
        all_feats = self.game_data_loader.get_table('feat')
        if not all_feats:
            return available
        
        # Get character's current feats once to avoid repeated has_feat() calls
        current_feat_ids = set()
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            current_feat_ids.add(feat.get('Feat', -1))
        
        # Count for logging
        total_checked = 0
        prereq_checked = 0
        
        for row_index, feat_data in enumerate(all_feats):
            # Filter out illegitimate feats first (fast check)
            if not self.is_legitimate_feat(feat_data):
                continue
                
            # Use proper row index as feat ID
            feat_id = getattr(feat_data, 'id', getattr(feat_data, 'row_index', row_index))
            
            # Skip if already has feat (now O(1) lookup)
            if feat_id in current_feat_ids:
                continue
            
            # Skip if wrong type (fast check)
            if feat_type is not None:
                data_type = getattr(feat_data, 'type', getattr(feat_data, 'feat_type', 0))
                if data_type != feat_type:
                    continue
            
            total_checked += 1
            
            # Check prerequisites - this is the expensive part, do it last
            is_valid, _ = self.validate_feat_prerequisites(feat_id, feat_data)
            prereq_checked += 1
            
            if is_valid:
                available.append(self.get_feat_info(feat_id, feat_data))
        
        logger.debug(f"get_available_feats: Checked {prereq_checked} prerequisites out of {total_checked} candidates from {len(all_feats)} total feats")
        
        return available
    
    def get_feat_category(self, feat_data) -> str:
        """
        Determine the category of a feat based on its properties
        
        Args:
            feat_data: Feat data object from 2DA
            
        Returns:
            Category string: 'general', 'combat', 'metamagic', 'item_creation', 
                           'divine', 'epic', 'class', 'racial', etc.
        """
        # Check for Epic feats first (highest priority)
        min_level = getattr(feat_data, 'MinLevel', getattr(feat_data, 'minlevel', 0))
        if min_level and int(min_level) >= 21:
            return 'epic'
        
        # Check feat type
        feat_type = getattr(feat_data, 'Type', getattr(feat_data, 'type', 0))
        if feat_type:
            feat_type = int(feat_type)
            if feat_type == 0:
                return 'general'
            elif feat_type == 1:
                return 'combat'
            elif feat_type == 2:
                return 'metamagic'
            elif feat_type == 3:
                return 'item_creation'
            elif feat_type == 4:
                return 'divine'
        
        # Check for class-specific feats
        label = getattr(feat_data, 'LABEL', getattr(feat_data, 'label', ''))
        if label:
            label_lower = label.lower()
            # Class-specific feat patterns
            if any(cls in label_lower for cls in ['barbarian', 'bard', 'cleric', 'druid', 
                                                   'fighter', 'monk', 'paladin', 'ranger', 
                                                   'rogue', 'sorcerer', 'wizard', 'warlock']):
                return 'class'
            
            # Racial feat patterns  
            if any(race in label_lower for race in ['human', 'elf', 'dwarf', 'halfling',
                                                     'gnome', 'orc', 'tiefling', 'aasimar']):
                return 'racial'
        
        # Default to general
        return 'general'
    
    def get_feat_subcategory(self, feat_data, category: str) -> str:
        """
        Get subcategory for class or racial feats
        
        Args:
            feat_data: Feat data object
            category: Main category
            
        Returns:
            Subcategory string or empty string
        """
        if category != 'class' and category != 'racial':
            return ''
            
        label = getattr(feat_data, 'LABEL', getattr(feat_data, 'label', ''))
        if not label:
            return ''
            
        label_lower = label.lower()
        
        if category == 'class':
            # Check for specific class names
            classes = ['barbarian', 'bard', 'cleric', 'druid', 'fighter', 
                      'monk', 'paladin', 'ranger', 'rogue', 'sorcerer', 
                      'wizard', 'warlock']
            for cls in classes:
                if cls in label_lower:
                    return cls
                    
        elif category == 'racial':
            # Check for specific race names
            races = ['human', 'elf', 'dwarf', 'halfling', 'gnome', 
                    'orc', 'tiefling', 'aasimar']
            for race in races:
                if race in label_lower:
                    return race
        
        return ''
    
    def get_legitimate_feats_by_category(self, category: str = '', subcategory: str = '', 
                                         feat_type: Optional[int] = None) -> List[Dict[str, Any]]:
        """
        Get legitimate feats filtered by category (no prerequisite checking)
        
        Args:
            category: Category filter (general, combat, class, etc.)
            subcategory: Subcategory filter (for class/racial)
            feat_type: Optional feat type filter
            
        Returns:
            List of feat info dicts
        """
        legitimate = []
        
        # Get all feats from dynamic game data
        all_feats = self.game_data_loader.get_table('feat')
        if not all_feats:
            return legitimate
        
        for row_index, feat_data in enumerate(all_feats):
            # Filter out illegitimate feats first
            if not self.is_legitimate_feat(feat_data):
                continue
                
            # Check category if specified
            if category:
                feat_category = self.get_feat_category(feat_data)
                if feat_category != category:
                    continue
                    
                # Check subcategory if specified
                if subcategory and category in ['class', 'racial']:
                    feat_subcategory = self.get_feat_subcategory(feat_data, category)
                    if feat_subcategory != subcategory:
                        continue
            
            # Use proper row index as feat ID
            feat_id = getattr(feat_data, 'id', getattr(feat_data, 'row_index', row_index))
            
            # Skip if wrong type
            if feat_type is not None:
                data_type = getattr(feat_data, 'type', getattr(feat_data, 'feat_type', 0))
                if data_type != feat_type:
                    continue
            
            feat_info = self.get_feat_info(feat_id, feat_data)
            if feat_info:
                legitimate.append(feat_info)
        
        return legitimate
    
    def get_available_feats_by_category(self, category: str = '', subcategory: str = '',
                                        feat_type: Optional[int] = None) -> List[Dict[str, Any]]:
        """
        Get available feats filtered by category (WITH prerequisite checking)
        
        Args:
            category: Category filter (general, combat, class, etc.)
            subcategory: Subcategory filter (for class/racial)
            feat_type: Optional feat type filter
            
        Returns:
            List of feat info dicts that character can actually take
        """
        available = []
        
        # First get legitimate feats by category (no prereq checking)
        category_feats = self.get_legitimate_feats_by_category(category, subcategory, feat_type)
        
        # Get character's current feats once to avoid repeated has_feat() calls
        current_feat_ids = set()
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            current_feat_ids.add(feat.get('Feat', -1))
        
        # Now check prerequisites for each feat
        for feat_info in category_feats:
            feat_id = feat_info['id']
            
            # Skip if already has feat
            if feat_id in current_feat_ids:
                continue
            
            # Check prerequisites
            is_valid, _ = self.validate_feat_prerequisites(feat_id)
            
            if is_valid:
                available.append(feat_info)
        
        return available
    
    def get_feat_summary(self) -> Dict[str, Any]:
        """Get summary of character's feats"""
        feat_list = self.gff.get('FeatList', [])
        
        categorized = {
            'total': len(feat_list),
            'protected': [],
            'class_feats': [],
            'general_feats': [],
            'custom_feats': []
        }
        
        for feat in feat_list:
            feat_id = feat.get('Feat', 0)
            feat_info = self.get_feat_info(feat_id)
            
            if feat_info['protected']:
                categorized['protected'].append(feat_info)
            
            if feat_info['custom']:
                categorized['custom_feats'].append(feat_info)
            elif feat_info['type'] == 1:  # General feat
                categorized['general_feats'].append(feat_info)
            else:
                categorized['class_feats'].append(feat_info)
        
        return categorized
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current feat configuration"""
        errors = []
        feat_list = self.gff.get('FeatList', [])
        
        # Check for duplicate feats
        feat_ids = [f.get('Feat', 0) for f in feat_list]
        if len(feat_ids) != len(set(feat_ids)):
            errors.append("Duplicate feats detected")
        
        # Validate each feat's prerequisites
        for feat in feat_list:
            feat_id = feat.get('Feat', 0)
            is_valid, feat_errors = self.validate_feat_prerequisites(feat_id)
            if not is_valid:
                feat_name = self.get_feat_info(feat_id)['name']
                errors.extend([f"{feat_name}: {e}" for e in feat_errors])
        
        return len(errors) == 0, errors
    
    def get_all_feats(self) -> List[Dict[str, Any]]:
        """
        Get all character feats (not just summary)
        
        Returns:
            List of detailed feat information
        """
        feats = []
        feat_list = self.gff.get('FeatList', [])
        
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id >= 0:
                feat_info = self.get_feat_info(feat_id)
                if feat_info:
                    # Add additional metadata
                    feat_info['is_protected'] = self.is_feat_protected(feat_id)
                    feat_info['source'] = feat_entry.get('Source', 'unknown')
                    feat_info['uses_remaining'] = feat_entry.get('Uses', -1)
                    feats.append(feat_info)
        
        return feats
    
    def batch_add_feats(self, feat_ids: List[int]) -> List[Dict[str, Any]]:
        """
        Add multiple feats at once
        
        Args:
            feat_ids: List of feat IDs to add
            
        Returns:
            List of results for each feat
        """
        results = []
        
        # Start transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            for feat_id in feat_ids:
                try:
                    success = self.add_feat(feat_id)
                    feat_info = self.get_feat_info(feat_id)
                    results.append({
                        'feat_id': feat_id,
                        'name': feat_info['name'] if feat_info else f'Feat {feat_id}',
                        'success': success,
                        'error': None
                    })
                except Exception as e:
                    results.append({
                        'feat_id': feat_id,
                        'name': f'Feat {feat_id}',
                        'success': False,
                        'error': str(e)
                    })
            
            self.character_manager.commit_transaction()
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            raise
        
        return results
    
    def get_feat_chains(self) -> Dict[str, List[Dict[str, Any]]]:
        """
        Get related feat progressions (e.g., Weapon Focus -> Weapon Specialization)
        
        Returns:
            Dict mapping chain names to feat lists
        """
        chains = {}
        
        # Define common feat chains
        chain_definitions = {
            'weapon_focus': {
                'pattern': ['weapon_focus', 'weapon_specialization', 'greater_weapon_focus', 'greater_weapon_specialization'],
                'name': 'Weapon Mastery'
            },
            'power_attack': {
                'pattern': ['power_attack', 'cleave', 'great_cleave'],
                'name': 'Power Attack Chain'
            },
            'combat_expertise': {
                'pattern': ['combat_expertise', 'improved_disarm', 'improved_trip'],
                'name': 'Combat Expertise Chain'
            },
            'dodge': {
                'pattern': ['dodge', 'mobility', 'spring_attack'],
                'name': 'Dodge Chain'
            },
            'toughness': {
                'pattern': ['toughness', 'improved_toughness', 'epic_toughness'],
                'name': 'Toughness Chain'
            }
        }
        
        # Get all feats
        feat_table = self.game_data_loader.get_table('feat')
        if not feat_table:
            return chains
        
        # Check each chain
        for chain_key, chain_def in chain_definitions.items():
            chain_feats = []
            
            for feat_pattern in chain_def['pattern']:
                # Find feats matching pattern
                for feat_data in feat_table:
                    feat_label = field_mapper.get_field_value(feat_data, 'label', '').lower()
                    if feat_pattern in feat_label:
                        feat_id = field_mapper.get_field_value(feat_data, 'id', -1)
                        if feat_id >= 0:
                            has_feat = self.has_feat(feat_id)
                            can_take, _ = self.validate_feat_prerequisites(feat_id)
                            
                            chain_feats.append({
                                'id': feat_id,
                                'name': field_mapper.get_field_value(feat_data, 'label', f'Feat {feat_id}'),
                                'has_feat': has_feat,
                                'can_take': can_take and not has_feat,
                                'position': chain_def['pattern'].index(feat_pattern)
                            })
            
            if chain_feats:
                # Sort by position in chain
                chain_feats.sort(key=lambda x: x['position'])
                chains[chain_def['name']] = chain_feats
        
        return chains
    
    def swap_feat(self, old_feat_id: int, new_feat_id: int) -> bool:
        """
        Replace a feat with another (useful for retraining)
        
        Args:
            old_feat_id: Feat to remove
            new_feat_id: Feat to add
            
        Returns:
            True if successful
        """
        # Check if old feat can be removed
        if self.is_feat_protected(old_feat_id):
            raise ValueError(f"Cannot swap protected feat {old_feat_id}")
        
        # Check if new feat can be taken
        can_take, errors = self.validate_feat_prerequisites(new_feat_id)
        if not can_take:
            raise ValueError(f"Cannot take new feat: {'; '.join(errors)}")
        
        # Start transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            # Remove old feat
            if not self.remove_feat(old_feat_id):
                raise ValueError("Failed to remove old feat")
            
            # Add new feat
            if not self.add_feat(new_feat_id, source='swap'):
                raise ValueError("Failed to add new feat")
            
            self.character_manager.commit_transaction()
            
            # Log the swap
            old_info = self.get_feat_info(old_feat_id)
            new_info = self.get_feat_info(new_feat_id)
            logger.info(f"Swapped feat {old_info['name']} for {new_info['name']}")
            
            return True
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            logger.error(f"Failed to swap feats: {e}")
            raise
    
    def can_take_feat(self, feat_id: int) -> Tuple[bool, str]:
        """
        Public method to check if feat can be taken
        
        Args:
            feat_id: The feat ID to check
            
        Returns:
            (can_take, reason) tuple
        """
        # Check if already has the feat
        if self.has_feat(feat_id):
            return False, "Already has this feat"
        
        # Check prerequisites
        can_take, errors = self.validate_feat_prerequisites(feat_id)
        
        if can_take:
            return True, "All requirements met"
        else:
            return False, "; ".join(errors)
    
    def get_feat_uses(self, feat_id: int) -> Optional[int]:
        """
        Get remaining uses for a feat (for feats with limited uses/day)
        
        Args:
            feat_id: The feat ID
            
        Returns:
            Number of uses remaining, or None if unlimited
        """
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            if feat.get('Feat') == feat_id:
                uses = feat.get('Uses', -1)
                return uses if uses >= 0 else None
        
        return None
    
    def set_feat_uses(self, feat_id: int, uses: int) -> bool:
        """
        Set remaining uses for a feat
        
        Args:
            feat_id: The feat ID
            uses: Number of uses to set
            
        Returns:
            True if successful
        """
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            if feat.get('Feat') == feat_id:
                feat['Uses'] = uses
                return True
        
        return False
    
    def get_bonus_feats_available(self) -> int:
        """
        Get number of bonus feats available to select
        
        Returns:
            Number of unallocated bonus feats
        """
        # This is complex as it depends on class levels and feat selections
        # For now, return a placeholder
        # TODO: Implement proper bonus feat tracking
        return 0
    
    def get_feat_categories(self) -> Dict[str, List[int]]:
        """
        Get feats organized by category
        
        Returns:
            Dict mapping category names to feat ID lists
        """
        from collections import defaultdict
        categories = defaultdict(list)
        feat_list = self.gff.get('FeatList', [])
        
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id >= 0:
                feat_data = self.game_data_loader.get_by_id('feat', feat_id)
                if feat_data:
                    # Get category from feat data
                    category = field_mapper.get_field_value(feat_data, 'categories', 'General')
                    if not category:
                        category = 'General'
                    
                    categories[category].append(feat_id)
        
        return dict(categories)