"""
Data-Driven Feat Manager - handles feat additions, removals, and protection
Uses CharacterManager and DynamicGameDataLoader for all feat data access
"""

from typing import Dict, List, Set, Tuple, Optional, Any
import logging
import time
import os

from ..events import (
    EventEmitter, EventType, EventData, 
    ClassChangedEvent, LevelGainedEvent, FeatChangedEvent
)
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

logger = logging.getLogger(__name__)

# Check if prerequisite graph is enabled
USE_PREREQUISITE_GRAPH = os.environ.get('USE_PREREQUISITE_GRAPH', 'true').lower() == 'true'


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
        
        # Separate caches for performance optimization
        # Display cache: Static feat info (name, description, icon) - rarely invalidated
        self._display_cache = {}
        # Validation cache: Character-specific validation results - frequently invalidated
        self._validation_cache = {}
        # Legacy cache for backward compatibility (will be phased out)
        self._feat_cache = {}
        self._class_cache = {}  # Cache for class lookups
        self._protected_feats: Set[int] = set()
        self._update_protected_feats()
        
        # Get prerequisite graph if available
        self._prerequisite_graph = None
        if USE_PREREQUISITE_GRAPH:
            try:
                from .prerequisite_graph import get_prerequisite_graph
                self._prerequisite_graph = get_prerequisite_graph()
                if self._prerequisite_graph:
                    logger.info("FeatManager using PrerequisiteGraph for fast validation")
                else:
                    logger.info("PrerequisiteGraph not available yet, using standard validation")
            except Exception as e:
                logger.warning(f"Failed to get PrerequisiteGraph: {e}")
                self._prerequisite_graph = None
    
    def _register_event_handlers(self):
        """Register handlers for relevant events"""
        self.character_manager.on(EventType.CLASS_CHANGED, self.on_class_changed)
        self.character_manager.on(EventType.LEVEL_GAINED, self.on_level_gained)
        # Cache invalidation for prerequisite changes
        self.character_manager.on(EventType.ATTRIBUTE_CHANGED, self.on_attribute_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self.on_feat_changed)
        self.character_manager.on(EventType.FEAT_REMOVED, self.on_feat_changed)
        self.character_manager.on(EventType.SKILL_UPDATED, self.on_skill_changed)
    
    def _get_content_manager(self):
        """Get the ContentManager from CharacterManager"""
        return self.character_manager.get_manager('content')
    
    def _update_protected_feats(self):
        """Update the set of protected feat IDs"""
        self._protected_feats.clear()
        
        # Add all custom content feats
        for content_id, info in self.character_manager.custom_content.items():
            if info['type'] == 'feat' and info.get('protected', False):
                self._protected_feats.add(info['id'])
        
        # Use our own epithet feat detection
        epithet_feats = self.detect_epithet_feats()
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
        
        # Invalidate validation cache since class levels affect prerequisites
        self.invalidate_validation_cache()
    
    def on_level_gained(self, event: LevelGainedEvent):
        """Handle level gain event"""
        logger.info(f"FeatManager handling level gain: Class {event.class_id}, Level {event.new_level}")
        
        # Add feats for the new level using dynamic game data
        class_data = self.game_data_loader.get_by_id('classes', event.class_id)
        if class_data:
            # Use class manager's method to get class feats for level
            class_manager = self.character_manager.get_manager('class')
            if class_manager:
                feats_at_level = class_manager.get_class_feats_for_level(
                    class_data, event.new_level
                )
            else:
                feats_at_level = []
            
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
        
        # Invalidate validation cache since character level affects prerequisites
        self.invalidate_validation_cache()
    
    def on_attribute_changed(self, event: EventData):
        """Handle attribute change event - invalidate cache for prerequisite checking"""
        logger.debug("FeatManager handling attribute change, invalidating validation cache")
        self.invalidate_validation_cache()
    
    def on_feat_changed(self, event: 'FeatChangedEvent'):
        """Handle feat added/removed event - invalidate cache since feats can be prerequisites"""
        logger.debug(
            f"FeatManager handling feat change (feat {event.feat_id} {event.action}), "
            "invalidating validation cache"
        )
        self.invalidate_validation_cache()
    
    def on_skill_changed(self, event: EventData):
        """Handle skill change event - invalidate cache for skill rank prerequisites"""
        logger.debug("FeatManager handling skill change, invalidating validation cache")
        self.invalidate_validation_cache()
    
    def add_feat(self, feat_id: int, source: str = 'manual') -> bool:
        """
        Add a feat to the character
        
        Args:
            feat_id: The feat ID to add
            source: Source of the feat ('class', 'level', 'manual')
            
        Returns:
            True if feat was added
        """
        # Check if already has feat (duplicate prevention - corruption prevention)
        if self.has_feat(feat_id):
            logger.debug(f"Character already has feat {feat_id}")
            return False
        
        # Check if feat ID exists (corruption prevention)
        feat_data = self.game_data_loader.get_by_id('feat', feat_id)
        if not feat_data and feat_id >= 0:  # Allow custom/unknown feats with negative IDs
            logger.warning(f"Feat ID {feat_id} not found in feat table")
            # Still allow it - might be custom content
        
        # NOTE: Prerequisite validation removed per validation cleanup plan
        # Users can now add any feat regardless of prerequisites
        # Prerequisites are still available for informational display via get_detailed_prerequisites()
        
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
    
    def has_feat_by_name(self, feat_label: str) -> bool:
        """
        Check if character has a feat by its label
        
        Args:
            feat_label: The feat label to check
            
        Returns:
            True if character has the feat
        """
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            if feat_data:
                label = getattr(feat_data, 'label', '')
                if label == feat_label:
                    return True
        
        return False
    
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
    
    def get_feat_info(self, feat_id: int, feat_data=None, skip_validation: bool = False) -> Optional[Dict[str, Any]]:
        """
        Get detailed information about a feat
        
        Args:
            feat_id: ID of the feat
            feat_data: Optional pre-loaded feat data to avoid redundant lookups
            skip_validation: If True, skip expensive prerequisite validation (for display only)
        """
        # If skip_validation is True, use the fast display method
        if skip_validation:
            return self.get_feat_info_display(feat_id, feat_data)
        
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
            can_take, missing_reqs = self.get_feat_prerequisites_info(feat_id, feat_data)
            
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
                'custom': self._get_content_manager().is_custom_content('feat', feat_id) if self._get_content_manager() else False,
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
    
    def get_feat_info_display(self, feat_id: int, feat_data=None) -> Optional[Dict[str, Any]]:
        """
        Get feat information for DISPLAY only - no prerequisite validation.
        10-100x faster than get_feat_info() since it skips expensive validation.
        
        Args:
            feat_id: ID of the feat
            feat_data: Optional pre-loaded feat data to avoid redundant lookups
            
        Returns:
            Dict with feat display information (no can_take or missing_requirements)
        """
        # Check display cache first
        if feat_id in self._display_cache:
            cached = self._display_cache[feat_id].copy()
            # Add current has_feat status (not cached as it can change)
            cached['has_feat'] = self.has_feat(feat_id)
            return cached
        
        # Use provided feat_data or get from dynamic game data loader
        if feat_data is None:
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
        
        if feat_data:
            # Use label as the primary name
            label = getattr(feat_data, 'label', getattr(feat_data, 'name', f'Feat_{feat_id}'))
            
            # Get static prerequisites (for display only, not validation)
            prereqs = field_mapper.get_feat_prerequisites(feat_data)
            
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
                'custom': self._get_content_manager().is_custom_content('feat', feat_id) if self._get_content_manager() else False,
                'description': description,
                'icon': icon,
                'prerequisites': prereqs,
                # NO validation - these fields are omitted for performance:
                # 'can_take': NOT CALCULATED
                # 'missing_requirements': NOT CALCULATED
            }
            
            # Cache the static parts (everything except has_feat)
            self._display_cache[feat_id] = info.copy()
            
            # Add current has_feat status
            info['has_feat'] = self.has_feat(feat_id)
            return info
        
        # Unknown feat - return minimal info
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
            'has_feat': self.has_feat(feat_id)
        }
    
    def get_feat_summary_fast(self) -> Dict[str, Any]:
        """
        Fast version of get_feat_summary() for display only.
        Uses get_feat_info_display() instead of get_feat_info() to skip validation.
        
        Returns:
            Dict with categorized feats (no validation data)
        """
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
            # Use FAST display method instead of full validation
            feat_info = self.get_feat_info_display(feat_id)
            
            if feat_info['protected']:
                categorized['protected'].append(feat_info)
            
            if feat_info['custom']:
                categorized['custom_feats'].append(feat_info)
            elif feat_info['type'] == 1:  # General feat
                categorized['general_feats'].append(feat_info)
            else:
                categorized['class_feats'].append(feat_info)
        
        return categorized
    
    def invalidate_validation_cache(self):
        """
        Clear the validation cache when character state changes.
        This should be called when abilities, levels, or feats change.
        
        Note: Does NOT clear display cache or game data cache (those are static).
        """
        self._validation_cache.clear()
        # Also clear legacy feat cache validation data
        for feat_id in self._feat_cache:
            if 'can_take' in self._feat_cache[feat_id]:
                del self._feat_cache[feat_id]['can_take']
            if 'missing_requirements' in self._feat_cache[feat_id]:
                del self._feat_cache[feat_id]['missing_requirements']
        logger.debug("Feat validation cache invalidated due to character state change")
    
    def get_feat_prerequisites_info_batch(self, feat_ids: List[int]) -> Dict[int, Tuple[bool, List[str]]]:
        """
        Get prerequisite information for multiple feats at once (INFORMATIONAL ONLY)\n        NOTE: This method provides information for UI display but does NOT block feat selection.\n        Users can add any feat regardless of the prerequisites shown here.
        
        Args:
            feat_ids: List of feat IDs to check
            
        Returns:
            Dictionary mapping feat_id to (meets_requirements, list_of_missing_requirements)
        """
        # Try to use PrerequisiteGraph for fast batch validation if available
        if self._prerequisite_graph and self._prerequisite_graph.is_built:
            # Prepare character data for graph validation
            character_feats = set()
            feat_list = self.gff.get('FeatList', [])
            for feat in feat_list:
                character_feats.add(feat.get('Feat', -1))
            
            # Get character data for non-feat prerequisites
            class_list = self.gff.get('ClassList', [])
            character_data = {
                'Str': self.gff.get('Str', 10),
                'Dex': self.gff.get('Dex', 10),
                'Con': self.gff.get('Con', 10),
                'Int': self.gff.get('Int', 10),
                'Wis': self.gff.get('Wis', 10),
                'Cha': self.gff.get('Cha', 10),
                'classes': set(c.get('Class') for c in class_list),
                'level': sum(c.get('ClassLevel', 0) for c in class_list),
                'bab': self.character_manager.get_manager('combat').get_base_attack_bonus() if hasattr(self.character_manager, 'get_manager') else 0
            }
            
            # Use graph for fast batch validation
            return self._prerequisite_graph.validate_batch_fast(
                feat_ids, character_feats, character_data
            )
        
        # Fallback to standard batch validation
        results = {}
        
        # Pre-load all feat data at once
        feat_data_map = {}
        for feat_id in feat_ids:
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            if feat_data:
                feat_data_map[feat_id] = feat_data
        
        # Pre-load character data once
        char_abilities = {
            'Str': self.gff.get('Str', 10),
            'Dex': self.gff.get('Dex', 10),
            'Con': self.gff.get('Con', 10),
            'Int': self.gff.get('Int', 10),
            'Wis': self.gff.get('Wis', 10),
            'Cha': self.gff.get('Cha', 10)
        }
        
        # Get character's current feats once
        current_feat_ids = set()
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            current_feat_ids.add(feat.get('Feat', -1))
        
        # Get character's classes once
        class_list = self.gff.get('ClassList', [])
        char_classes = set(c.get('Class') for c in class_list)
        
        # Get character level and BAB once
        char_level = sum(c.get('ClassLevel', 0) for c in class_list)
        char_bab = self.character_manager.get_manager('combat').get_base_attack_bonus()
        
        # Process each feat
        for feat_id in feat_ids:
            if feat_id not in feat_data_map:
                # Allow unknown feats (custom content)
                results[feat_id] = (True, [])
                continue
            
            feat_data = feat_data_map[feat_id]
            errors = []
            
            # Check prerequisites using field mapping utility
            prereqs = field_mapper.get_feat_prerequisites(feat_data)
            
            # Check ability score prerequisites
            for ability, min_score in prereqs['abilities'].items():
                if min_score > 0:
                    current_score = char_abilities.get(ability, 10)
                    if current_score < min_score:
                        errors.append(f"Requires {ability.upper()} {min_score}")
            
            # Check feat prerequisites
            for prereq_feat_id in prereqs['feats']:
                if prereq_feat_id not in current_feat_ids:
                    # Get prereq feat name from cache or lookup
                    if prereq_feat_id in self._feat_cache:
                        prereq_name = self._feat_cache[prereq_feat_id].get('label', f'Feat {prereq_feat_id}')
                    else:
                        prereq_feat_data = self.game_data_loader.get_by_id('feat', prereq_feat_id)
                        prereq_name = field_mapper.get_field_value(prereq_feat_data, 'label', f'Feat {prereq_feat_id}') if prereq_feat_data else f'Feat {prereq_feat_id}'
                        # Cache for future use
                        if prereq_feat_data:
                            self._feat_cache[prereq_feat_id] = {'label': prereq_name}
                    errors.append(f"Requires {prereq_name}")
            
            # Check class requirements
            if prereqs['class'] >= 0 and prereqs['class'] not in char_classes:
                # Get class name from cache or lookup
                class_id = prereqs['class']
                if class_id in self._class_cache:
                    class_name = self._class_cache[class_id]
                else:
                    class_data = self.game_data_loader.get_by_id('classes', class_id)
                    class_name = field_mapper.get_field_value(class_data, 'label', f'Class {class_id}') if class_data else f'Class {class_id}'
                    self._class_cache[class_id] = class_name
                errors.append(f"Requires {class_name} class")
            
            # Check level requirements
            if prereqs['level'] > 0 and char_level < prereqs['level']:
                errors.append(f"Requires character level {prereqs['level']}")
            
            # Check BAB requirements
            if prereqs['bab'] > 0 and char_bab < prereqs['bab']:
                errors.append(f"Requires base attack bonus +{prereqs['bab']}")
            
            # Check spell level requirements
            if prereqs['spell_level'] > 0:
                # This would require checking spellcasting capabilities
                # For now, we'll skip this check
                pass
            
            results[feat_id] = (len(errors) == 0, errors)
        
        return results
    
    def get_feat_prerequisites_info(self, feat_id: int, feat_data=None) -> Tuple[bool, List[str]]:
        """
        Get prerequisite information for a feat (INFORMATIONAL ONLY)
        NOTE: This method provides information for UI display but does NOT block feat selection.
        Users can add any feat regardless of the prerequisites shown here.
        
        Args:
            feat_id: ID of the feat to check
            feat_data: Optional pre-loaded feat data to avoid redundant lookups
        
        Returns:
            (meets_requirements, list_of_missing_requirements)
        """
        # Try to use PrerequisiteGraph for fast validation if available
        if self._prerequisite_graph and self._prerequisite_graph.is_built:
            # Prepare character data for graph validation
            character_feats = set()
            feat_list = self.gff.get('FeatList', [])
            for feat in feat_list:
                character_feats.add(feat.get('Feat', -1))
            
            # Get character data for non-feat prerequisites
            class_list = self.gff.get('ClassList', [])
            character_data = {
                'Str': self.gff.get('Str', 10),
                'Dex': self.gff.get('Dex', 10),
                'Con': self.gff.get('Con', 10),
                'Int': self.gff.get('Int', 10),
                'Wis': self.gff.get('Wis', 10),
                'Cha': self.gff.get('Cha', 10),
                'classes': set(c.get('Class') for c in class_list),
                'level': sum(c.get('ClassLevel', 0) for c in class_list),
                'bab': self.character_manager.get_manager('combat').get_base_attack_bonus() if hasattr(self.character_manager, 'get_manager') else 0
            }
            
            # Use graph for fast validation
            return self._prerequisite_graph.validate_feat_prerequisites_fast(
                feat_id, character_feats, character_data
            )
        
        # Fallback to standard validation
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
        Get detailed prerequisite information in a user-friendly format (INFORMATIONAL ONLY)\n        NOTE: This provides prerequisite information for UI display but does NOT block feat selection.\n        Users can add any feat regardless of the prerequisites shown here.
        
        Returns:
            Dict with detailed prerequisite breakdown
        """
        feat_data = self.game_data_loader.get_by_id('feat', feat_id)
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
        class_manager = self.character_manager.get_manager('class')
        for lvl in range(1, level + 1):
            if class_manager:
                feats_at_level = class_manager.get_class_feats_for_level(class_data, lvl)
            else:
                feats_at_level = []
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
        class_manager = self.character_manager.get_manager('class')
        for lvl in range(1, level + 1):
            if class_manager:
                feats_at_level = class_manager.get_class_feats_for_level(class_data, lvl)
            else:
                feats_at_level = []
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
            
            # Use fast display method - skip validation for performance
            feat_info = self.get_feat_info(feat_id, feat_data, skip_validation=True)
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
            is_valid, _ = self.get_feat_prerequisites_info(feat_id, feat_data)
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
            
            # Use fast display method - skip validation for performance
            feat_info = self.get_feat_info(feat_id, feat_data, skip_validation=True)
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
        
        # Filter out feats the character already has and collect IDs for batch validation
        feats_to_validate = []
        feat_info_map = {}
        for feat_info in category_feats:
            feat_id = feat_info['id']
            
            # Skip if already has feat
            if feat_id in current_feat_ids:
                continue
            
            feats_to_validate.append(feat_id)
            feat_info_map[feat_id] = feat_info
        
        # Batch validate all prerequisites at once
        if feats_to_validate:
            validation_results = self.get_feat_prerequisites_info_batch(feats_to_validate)
            
            # Add only valid feats to available list
            for feat_id, (is_valid, _) in validation_results.items():
                if is_valid:
                    available.append(feat_info_map[feat_id])
        
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
        """Validate current feat configuration for corruption prevention only"""
        errors = []
        feat_list = self.gff.get('FeatList', [])
        
        # Check for duplicate feats (corruption prevention)
        feat_ids = [f.get('Feat', 0) for f in feat_list]
        if len(feat_ids) != len(set(feat_ids)):
            errors.append("Duplicate feats detected - this can cause save corruption")
        
        # Check for invalid feat IDs (corruption prevention)
        for feat in feat_list:
            feat_id = feat.get('Feat', 0)
            if feat_id < 0:  # Negative IDs might be custom content, allow them
                continue
            
            # Check if feat exists in the data (prevents crash on load)
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            if not feat_data:
                errors.append(f"Feat ID {feat_id} not found in feat table - may cause load errors")
        
        # NOTE: Prerequisite validation removed per validation cleanup plan
        # Users can now have any feat regardless of prerequisites
        
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
                            can_take, _ = self.get_feat_prerequisites_info(feat_id)
                            
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
        
        # NOTE: Prerequisite validation removed per validation cleanup plan
        # Users can now swap to any feat regardless of prerequisites
        
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
        can_take, errors = self.get_feat_prerequisites_info(feat_id)
        
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
    
    def get_feat_categories_fast(self) -> Dict[str, List[Dict[str, Any]]]:
        """
        Get feats organized by category using fast display methods (no validation).
        Performance-optimized version that skips prerequisite checking.
        
        Returns:
            Dict mapping category names to lists of feat info (display only)
        """
        from collections import defaultdict
        categories = defaultdict(list)
        feat_list = self.gff.get('FeatList', [])
        
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id >= 0:
                # Use fast display method instead of full get_feat_info
                feat_info = self.get_feat_info_display(feat_id)
                if feat_info:
                    # Determine category based on feat type
                    feat_category = self.get_feat_category_by_type(feat_info.get('type', 0))
                    categories[feat_category].append(feat_info)
        
        return dict(categories)
    
    def get_feat_category_by_type(self, feat_type: int) -> str:
        """
        Get category name based on feat type number.
        Helper method for fast categorization.
        
        Args:
            feat_type: The feat type number
            
        Returns:
            Category name string
        """
        if feat_type == 1:
            return 'General'
        elif feat_type == 2:
            return 'Combat'
        elif feat_type == 8:
            return 'Metamagic'
        elif feat_type == 16:
            return 'Divine'
        elif feat_type == 32:
            return 'Epic'
        elif feat_type == 64:
            return 'Class'
        else:
            return 'General'
    
    def detect_epithet_feats(self) -> Set[int]:
        """
        Detect epithet feats (special story/custom feats that should be protected)
        
        Returns:
            Set of feat IDs that are epithet feats
        """
        epithet_feats = set()
        
        # Get all feats from character
        feat_list = self.gff.get('FeatList', [])
        
        # Get vanilla feat data
        feat_table = self.game_data_loader.get_table('feat')
        if not feat_table:
            logger.warning("Feat table not found, cannot detect epithet feats")
            return epithet_feats
        
        # Build set of all vanilla feat IDs (using row indices which correspond to feat IDs)
        vanilla_feat_ids = set(range(len(feat_table)))
        
        # Check each character feat
        for feat_entry in feat_list:
            if isinstance(feat_entry, dict):
                feat_id = feat_entry.get('Feat', -1)
                
                # Check if this is a non-vanilla feat or has special properties
                if feat_id not in vanilla_feat_ids:
                    # This is a custom/mod feat
                    epithet_feats.add(feat_id)
                else:
                    # Check if it's an epithet feat by looking at properties
                    feat_data = self.game_data_loader.get_by_id('feat', feat_id)
                    if feat_data:
                        # Epithet feats often have specific naming patterns or categories
                        label = (getattr(feat_data, 'label', '') or '').lower()
                        category = (getattr(feat_data, 'categories', '') or '').lower()
                        
                        # Common patterns for epithet/story feats
                        epithet_patterns = [
                            'epithet', 'story', 'history', 'background',
                            'blessing', 'curse', 'gift', 'legacy'
                        ]
                        
                        for pattern in epithet_patterns:
                            if pattern in label or pattern in category:
                                epithet_feats.add(feat_id)
                                break
        
        logger.info(f"Detected {len(epithet_feats)} epithet feats: {epithet_feats}")
        return epithet_feats