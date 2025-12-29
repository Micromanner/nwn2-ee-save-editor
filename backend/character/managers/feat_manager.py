"""Data-Driven Feat Manager - handles feat additions, removals, and protection Uses CharacterManager and DynamicGameDataLoader for all feat data access

TODO: Modded feat bonus parsing limitations:
  - Level-scaling bonuses show base value only (e.g., "+1 at level 1, +2 at level 4" just shows +1)
  - Persistent spell effects (SPELLID) aren't analyzed - we rely on description text parsing
"""

from typing import Dict, List, Set, Tuple, Optional, Any
from loguru import logger
import time
import os
import re

from ..events import (
    EventEmitter, EventType, EventData, 
    ClassChangedEvent, LevelGainedEvent, FeatChangedEvent
)
from gamedata.dynamic_loader.field_mapping_utility import field_mapper

USE_PREREQUISITE_GRAPH = os.environ.get('USE_PREREQUISITE_GRAPH', 'true').lower() == 'true'

_SAVE_PATTERNS = [
    (re.compile(r'([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+all\s+(?:saving\s+throws|saves)', re.IGNORECASE), 'universal'),
    (re.compile(r'([+-]\d+)\s+(?:to\s+)?all\s+(?:saving\s+throws|saves)', re.IGNORECASE), 'universal'),
    (re.compile(r'([+-]\d+)\s+(?:bonus\s+)?(?:to|on)\s+Fortitude\s+and\s+Will\s+(?:saving\s+throws|saves?)', re.IGNORECASE), 'fortitude_and_will'),
    (re.compile(r'([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+(?:all\s+)?Fortitude\s+(?:saving\s+throws|saves?)', re.IGNORECASE), 'fortitude'),
    (re.compile(r'([+-]\d+)\s+Fortitude\s+Save', re.IGNORECASE), 'fortitude'),
    (re.compile(r'([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+(?:all\s+)?Reflex\s+(?:saving\s+throws|saves?)', re.IGNORECASE), 'reflex'),
    (re.compile(r'([+-]\d+)\s+Reflex\s+Save', re.IGNORECASE), 'reflex'),
    (re.compile(r'([+-]\d+)\s+(?:\w+\s+)?bonus\s+(?:to|on)\s+(?:all\s+)?Will\s+(?:saving\s+throws|saves?)', re.IGNORECASE), 'will'),
    (re.compile(r'([+-]\d+)\s+(?:to|on)\s+all\s+Will\s+(?:saving\s+throws|saves?)', re.IGNORECASE), 'will'),
    (re.compile(r'([+-]\d+)\s+Will\s+Save', re.IGNORECASE), 'will'),
]

_AC_PATTERNS = [
    re.compile(r'\+(\d+)\s+(?:\w+\s+)?bonus\s+to\s+(?:Armor\s+Class|AC)', re.IGNORECASE),
    re.compile(r'\+(\d+)\s+(?:to\s+)?AC(?:\s|\.|\,)', re.IGNORECASE),
    re.compile(r'\+(\d+)\s+AC\s+bonus', re.IGNORECASE),
]

_AC_DODGE_PATTERN = re.compile(r'\+(\d+)')

_INITIATIVE_PATTERNS = [
    re.compile(r'\+(\d+)\s+(?:\w+\s+)?bonus\s+to\s+initiative', re.IGNORECASE),
    re.compile(r'\+(\d+)\s+(?:to\s+)?initiative', re.IGNORECASE),
]

_SAVE_CONDITIONAL_KEYWORDS = ('against', 'vs ', 'versus', 'to avoid', 'made to')
_AC_CONDITIONAL_KEYWORDS = ('against', 'vs ', 'versus', 'when ', 'while ', 'if ', 'when wielding', 'when wearing', 'when using', 'when fighting')


class FeatManager(EventEmitter):
    """Data-Driven Feat Manager Uses CharacterManager as hub for all character data access"""
    
    def __init__(self, character_manager):
        """Initialize FeatManager with character_manager reference"""
        super().__init__()
        self.character_manager = character_manager
        self.game_rules_service = character_manager.rules_service
        self.gff = character_manager.gff

        self._register_event_handlers()

        self._display_cache = {}
        self._validation_cache = {}
        self._feat_cache = {}
        self._class_cache = {}
        self._protected_feats: Set[int] = set()

        logger.warning(f"FeatManager CREATED - instance id: {id(self)}")

        self._domain_feats_cache: Optional[Set[int]] = None
        self._domain_feat_map_cache: Optional[Dict[int, List[Dict]]] = None
        self._has_feat_set: Optional[Set[int]] = None
        self._has_class_set: Optional[Set[int]] = None

        self._save_bonuses_cache: Optional[Dict[str, int]] = None
        self._ac_bonuses_cache: Optional[Dict[str, int]] = None
        self._initiative_bonus_cache: Optional[int] = None

        self._update_protected_feats()

        self._prerequisite_graph = None
        if USE_PREREQUISITE_GRAPH:
            try:
                from .prerequisite_graph import get_prerequisite_graph
                self._prerequisite_graph = get_prerequisite_graph(rules_service=self.game_rules_service)
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
        self.character_manager.on(EventType.ATTRIBUTE_CHANGED, self.on_attribute_changed)
        self.character_manager.on(EventType.FEAT_ADDED, self.on_feat_changed)
        self.character_manager.on(EventType.FEAT_REMOVED, self.on_feat_changed)
        self.character_manager.on(EventType.SKILL_UPDATED, self.on_skill_changed)
    
    def _get_content_manager(self):
        """Get the ContentManager from CharacterManager"""
        return self.character_manager.get_manager('content')
    
    def _update_protected_feats(self):
        """Update the set of protected feat IDs (excludes domain feats - they're removable)"""
        self._protected_feats.clear()

        for content_id, info in self.character_manager.custom_content.items():
            if info['type'] == 'feat' and info.get('protected', False):
                self._protected_feats.add(info['id'])

        epithet_feats = self.detect_epithet_feats()

        # Remove domain feats from protected set (domains are changeable)
        domain_feat_ids = self.get_all_domain_feat_ids()
        epithet_feats = epithet_feats - domain_feat_ids

        self._protected_feats.update(epithet_feats)

        logger.debug(f"Protected feats updated: {len(self._protected_feats)} feats protected (domain feats excluded)")
    
    def on_class_changed(self, event: ClassChangedEvent):
        """Handle class change event"""
        logger.info(f"FeatManager handling class change: {event.old_class_id} -> {event.new_class_id}")

        if event.old_class_id is not None:
            self._remove_class_feats(event.old_class_id, event.level, event.preserve_feats)

        self._add_class_feats(event.new_class_id, event.level)
        self.invalidate_validation_cache()
    
    def on_level_gained(self, event: LevelGainedEvent):
        """Handle level gain event"""
        logger.info(f"FeatManager handling level gain: Class {event.class_id}, Level {event.new_level}")

        class_data = self.game_rules_service.get_by_id('classes', event.class_id)
        if class_data:
            class_manager = self.character_manager.get_manager('class')
            if class_manager:
                feats_at_level = class_manager.get_class_feats_for_level(
                    class_data, event.new_level
                )
            else:
                feats_at_level = []

            for feat_info in feats_at_level:
                if feat_info['list_type'] == 0:
                    feat_id = feat_info['feat_id']

                    old_feat_id = self._check_feat_progression(feat_id, event.class_id)
                    if old_feat_id:
                        logger.info(f"Progressing feat: {old_feat_id} -> {feat_id}")
                        self.remove_feat(old_feat_id, force=True)

                    self.add_feat(feat_id, source='level')

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
        self._save_bonuses_cache = None
        self._ac_bonuses_cache = None
        self._initiative_bonus_cache = None
    
    def on_skill_changed(self, event: EventData):
        """Handle skill change event - invalidate cache for skill rank prerequisites"""
        logger.debug("FeatManager handling skill change, invalidating validation cache")
        self.invalidate_validation_cache()
    
    def add_feat(self, feat_id: int, source: str = 'manual') -> bool:
        """Add feat to character, cascading to all domain feats if adding a domain epithet feat"""
        if self.has_feat(feat_id):
            logger.debug(f"Character already has feat {feat_id}")
            return False

        # Check if this is a domain epithet feat - if so, add the entire domain
        if source != 'domain' and self.is_domain_epithet_feat(feat_id):
            domain_feat_map = self._build_domain_feat_map()
            if feat_id in domain_feat_map:
                # Get the domain ID
                domain_info = domain_feat_map[feat_id][0]
                domain_id = domain_info['domain_id']
                logger.info(f"Adding domain epithet feat {feat_id}, will cascade to all domain {domain_id} feats")

                # Use add_domain to cascade addition
                self.add_domain(domain_id)
                return True

        feat_data = self.game_rules_service.get_by_id('feat', feat_id)
        if not feat_data and feat_id >= 0:
            logger.warning(f"Feat ID {feat_id} not found in feat table")

        feat_list = self.gff.get('FeatList', [])
        feat_list.append({'Feat': feat_id})
        self.gff.set('FeatList', feat_list)

        # Invalidate cached set
        self._has_feat_set = None

        # Sync to level up history
        class_manager = self.character_manager.get_manager('class')
        if class_manager:
            class_manager.record_feat_change(feat_id, True)


        event = FeatChangedEvent(
            event_type=EventType.FEAT_ADDED,
            source_manager='feat',
            timestamp=time.time(),
            feat_id=feat_id,
            action='added',
            source=source
        )
        self.character_manager.emit(event)

        logger.info(f"Added feat {feat_id} from source: {source}")
        return True

    def add_feat_with_prerequisites(self, feat_id: int, auto_add_prerequisites: bool = True, source: str = 'manual') -> Tuple[bool, List[Dict[str, Any]]]:
        """Add feat, optionally auto-adding missing prerequisites and ability scores. Returns (success, list_of_changes)"""
        auto_changes = []

        if auto_add_prerequisites:
            can_take, missing_info = self.get_feat_prerequisites_info(feat_id)

            if not can_take:
                logger.info(f"Feat {feat_id} missing prerequisites. Attempting to auto-add prerequisites.")

                feat_data = self.game_rules_service.get_by_id('feat', feat_id)
                if feat_data:
                    prereqs = field_mapper.get_feat_prerequisites(feat_data)

                    ability_manager = self.character_manager.get_manager('ability')
                    for ability, min_score in prereqs['abilities'].items():
                        if min_score > 0:
                            current_score = self.gff.get(ability, 10)
                            if current_score < min_score:
                                logger.info(f"Auto-increasing {ability} from {current_score} to {min_score}")
                                ability_manager.set_attribute(ability, min_score, validate=False)
                                auto_changes.append({
                                    'type': 'ability',
                                    'ability': ability,
                                    'old_value': current_score,
                                    'new_value': min_score,
                                    'label': f'{ability.upper()} increased to {min_score}'
                                })

                    for prereq_feat_id in prereqs['feats']:
                        if not self.has_feat(prereq_feat_id):
                            prereq_success, nested_auto_added = self.add_feat_with_prerequisites(
                                prereq_feat_id,
                                auto_add_prerequisites=True,
                                source='auto_prerequisite'
                            )

                            if prereq_success:
                                prereq_info = self.get_feat_info(prereq_feat_id) or {
                                    'id': prereq_feat_id,
                                    'name': f'Feat {prereq_feat_id}',
                                    'label': f'Feat {prereq_feat_id}'
                                }
                                prereq_info['type'] = 'feat'
                                auto_changes.append(prereq_info)
                                auto_changes.extend(nested_auto_added)

                                logger.debug(f"Auto-added prerequisite feat {prereq_feat_id}")

        success = self.add_feat(feat_id, source=source)

        return success, auto_changes

    def remove_feat(self, feat_id: int, force: bool = False, skip_cascade: bool = False) -> bool:
        """Remove feat, cascading to all domain feats if removing a domain epithet feat"""
        if not force and self.is_feat_protected(feat_id):
            logger.warning(f"Cannot remove protected feat {feat_id}")
            return False

        # Check if this is a domain epithet feat - if so, remove the entire domain
        if not skip_cascade and self.is_domain_epithet_feat(feat_id):
            domain_feat_map = self._build_domain_feat_map()
            if feat_id in domain_feat_map:
                # Get the domain ID
                domain_info = domain_feat_map[feat_id][0]
                domain_id = domain_info['domain_id']
                logger.info(f"Removing domain epithet feat {feat_id}, will cascade to all domain {domain_id} feats")

                # Use remove_domain to cascade removal
                self.remove_domain(domain_id)
                return True

        feat_list = self.gff.get('FeatList', [])
        original_count = len(feat_list)
        feat_list = [f for f in feat_list if f.get('Feat') != feat_id]

        if len(feat_list) < original_count:
            self.gff.set('FeatList', feat_list)

            # Invalidate cached set
            self._has_feat_set = None

            # Sync to level up history
            class_manager = self.character_manager.get_manager('class')
            if class_manager:
                class_manager.record_feat_change(feat_id, False)


            event = FeatChangedEvent(
                event_type=EventType.FEAT_REMOVED,
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
        """Check if character has a specific feat - O(1) lookup using cached set"""
        if self._has_feat_set is None:
            feat_list = self.gff.get('FeatList', [])
            self._has_feat_set = {f.get('Feat') for f in feat_list if 'Feat' in f}
        return feat_id in self._has_feat_set
    
    def has_feat_by_name(self, feat_label: str) -> bool:
        """Check if character has a feat by its label"""
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)
            if feat_data:
                label = field_mapper.get_field_value(feat_data, 'label', '')
                if label == feat_label:
                    return True
        
        return False
    
    def is_feat_protected(self, feat_id: int) -> bool:
        """Check if a feat is protected from removal"""
        return feat_id in self._protected_feats
    
    def _check_feat_progression(self, new_feat_id: int, class_id: int) -> Optional[int]:
        """Check if feat is part of a progression chain, returning old feat ID to remove or None"""
        new_feat = self.game_rules_service.get_by_id('feat', new_feat_id)
        if not new_feat:
            return None

        new_label = field_mapper.get_field_value(new_feat, 'label', '')

        import re
        match = re.search(r'^(.*?)[\s_]?(\d+)$', new_label)
        if not match:
            return None

        base_name = match.group(1).rstrip('_')
        new_number = int(match.group(2))

        if new_number < 2:
            return None

        feat_list = self.gff.get('FeatList', [])
        character_feat_ids = {f.get('Feat') for f in feat_list}

        for feat_id in character_feat_ids:
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)
            if not feat_data:
                continue

            label = field_mapper.get_field_value(feat_data, 'label', '')

            if label.startswith(base_name):
                old_match = re.search(r'^(.*?)[\s_]?(\d+)$', label)
                if old_match:
                    old_base = old_match.group(1).rstrip('_')
                    old_number = int(old_match.group(2))

                    if old_base == base_name and old_number < new_number:
                        logger.info(f"Auto-detected progression: {label} -> {new_label}")
                        return feat_id

                elif label == base_name or label == base_name.rstrip('_'):
                    logger.info(f"Auto-detected progression: {label} -> {new_label}")
                    return feat_id

        return None
    
    def get_feat_info(self, feat_id: int, feat_data=None, skip_validation: bool = False) -> Optional[Dict[str, Any]]:
        """Get detailed feat information with prerequisite validation (use skip_validation=True for display only)"""
        # If skip_validation is True, use the fast display method
        if skip_validation:
            return self.get_feat_info_display(feat_id, feat_data)

        if feat_id in self._feat_cache:
            return self._feat_cache[feat_id]

        if feat_data is None:
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)

        if feat_data:
            label_raw = field_mapper.get_field_value(feat_data, 'label', f'Feat_{feat_id}')
            if isinstance(label_raw, int) and label_raw > 0:
                label = self.game_rules_service._loader.get_string(label_raw) or f'Feat_{feat_id}'
            else:
                label = str(label_raw) if label_raw else f'Feat_{feat_id}'

            prereqs = field_mapper.get_feat_prerequisites(feat_data)
            can_take, missing_reqs = self.get_feat_prerequisites_info(feat_id, feat_data)

            desc_raw = field_mapper.get_field_value(feat_data, 'description', '')
            if isinstance(desc_raw, int) and desc_raw > 0:
                description = self.game_rules_service._loader.get_string(desc_raw) or ''
            else:
                description = str(desc_raw) if desc_raw else ''

            icon = field_mapper.get_field_value(feat_data, 'icon', '')

            feat_type = self._parse_feat_type(feat_data)

            category = self.get_feat_category_by_type(feat_type)
            if self.is_domain_epithet_feat(feat_id):
                category = 'Domain'
                feat_type = 8192
            elif 'BACKGROUND' in label.upper():
                category = 'Background'
                feat_type = 128

            info = {
                'id': feat_id,
                'label': label,
                'name': label,
                'type': feat_type,
                'category': category,
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

        return {
            'id': feat_id,
            'label': f'Unknown_{feat_id}',
            'name': f'Unknown Feat {feat_id}',
            'type': 0,
            'category': 'General',
            'protected': True,
            'custom': True,
            'description': 'Unknown or custom feat',
            'icon': '',
            'prerequisites': {'abilities': {}, 'feats': [], 'class': -1, 'level': 0, 'bab': 0, 'spell_level': 0},
            'can_take': False,
            'missing_requirements': ['Unknown feat - cannot validate'],
            'has_feat': self.has_feat(feat_id)
        }
    
    def get_feat_info_display(self, feat_id: int, feat_data=None) -> Optional[Dict[str, Any]]:
        """Get feat information for display only without prerequisite validation, 10-100x faster than get_feat_info."""
        if feat_id in self._display_cache:
            cached = self._display_cache[feat_id].copy()
            cached['has_feat'] = self.has_feat(feat_id)
            return cached

        if feat_data is None:
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)

        if feat_data:
            fields = field_mapper.get_feat_fields_batch(feat_data)

            prereqs = {
                'abilities': {
                    'Str': field_mapper._safe_int(fields.get('prereq_str', 0)),
                    'Dex': field_mapper._safe_int(fields.get('prereq_dex', 0)),
                    'Con': field_mapper._safe_int(fields.get('prereq_con', 0)),
                    'Int': field_mapper._safe_int(fields.get('prereq_int', 0)),
                    'Wis': field_mapper._safe_int(fields.get('prereq_wis', 0)),
                    'Cha': field_mapper._safe_int(fields.get('prereq_cha', 0)),
                },
                'feats': [
                    f for f in [
                        field_mapper._safe_int(fields.get('prereq_feat1', 0)),
                        field_mapper._safe_int(fields.get('prereq_feat2', 0))
                    ] if f > 0
                ],
                'class': field_mapper._safe_int(fields.get('required_class', -1)),
                'level': field_mapper._safe_int(fields.get('min_level', 0)),
                'bab': field_mapper._safe_int(fields.get('prereq_bab', 0)),
                'spell_level': field_mapper._safe_int(fields.get('prereq_spell_level', 0))
            }

            feat_strref = field_mapper._safe_int(fields.get('feat_name_strref', 0), 0)
            label = self.game_rules_service._loader.get_string(feat_strref) if feat_strref > 0 else f'Feat_{feat_id}'
            label = self._strip_nwn2_tags(label)

            engine_label = str(fields.get('label', ''))

            desc_raw = fields.get('description', '')
            if isinstance(desc_raw, int) and desc_raw > 0:
                description = self.game_rules_service._loader.get_string(desc_raw) or ''
            else:
                description = str(desc_raw) if desc_raw else ''
            description = self._strip_nwn2_tags(description)

            feat_type = self._parse_feat_type(feat_data)

            category = self.get_feat_category_by_type(feat_type)
            if self.is_domain_epithet_feat(feat_id):
                category = 'Domain'
                feat_type = 8192
            elif 'BACKGROUND' in engine_label.upper():
                category = 'Background'
                feat_type = 128

            info = {
                'id': feat_id,
                'label': label,
                'name': label,
                'type': feat_type,
                'category': category,
                'protected': self.is_feat_protected(feat_id),
                'custom': self._get_content_manager().is_custom_content('feat', feat_id) if self._get_content_manager() else False,
                'description': description,
                'icon': fields.get('icon', ''),
                'prerequisites': prereqs,
            }

            self._display_cache[feat_id] = info.copy()

            info['has_feat'] = self.has_feat(feat_id)
            return info

        return {
            'id': feat_id,
            'label': f'Unknown_{feat_id}',
            'name': f'Unknown Feat {feat_id}',
            'type': 0,
            'category': 'General',
            'protected': True,
            'custom': True,
            'description': 'Unknown or custom feat',
            'icon': '',
            'prerequisites': {'abilities': {}, 'feats': [], 'class': -1, 'level': 0, 'bab': 0, 'spell_level': 0},
            'has_feat': self.has_feat(feat_id)
        }
    
    def get_feat_summary_fast(self) -> Dict[str, Any]:
        """Get feat summary for display only using fast methods that skip validation."""
        feat_list = self.gff.get('FeatList', [])
        
        categorized = {
            'total': len(feat_list),
            'protected': [],
            'class_feats': [],
            'general_feats': [],
            'custom_feats': [],
            'background_feats': [],
            'domain_feats': []
        }
        
        for feat in feat_list:
            feat_id = feat.get('Feat', 0)
            feat_info = self.get_feat_info_display(feat_id)

            if feat_info['protected']:
                categorized['protected'].append(feat_info)

            category = feat_info.get('category', 'General')
            if category == 'Domain':
                categorized['domain_feats'].append(feat_info)
            elif category == 'Background':
                categorized['background_feats'].append(feat_info)
            elif feat_info['custom']:
                categorized['custom_feats'].append(feat_info)
            elif feat_info['type'] & 1:
                categorized['general_feats'].append(feat_info)
            else:
                categorized['class_feats'].append(feat_info)
        
        return categorized
    
    def invalidate_validation_cache(self):
        """Clear validation cache when character state changes."""
        self._validation_cache.clear()
        self._display_cache.clear()
        self._has_feat_set = None
        self._has_class_set = None
        for feat_id in self._feat_cache:
            if 'can_take' in self._feat_cache[feat_id]:
                del self._feat_cache[feat_id]['can_take']
            if 'missing_requirements' in self._feat_cache[feat_id]:
                del self._feat_cache[feat_id]['missing_requirements']
        logger.debug("Feat validation cache invalidated due to character state change")
    
    def _get_character_validation_data(self) -> dict:
        """Build character data dict for Rust prerequisite validation."""
        class_list = self.gff.get('ClassList', [])
        return {
            'strength': self.gff.get('Str', 10),
            'dexterity': self.gff.get('Dex', 10),
            'constitution': self.gff.get('Con', 10),
            'intelligence': self.gff.get('Int', 10),
            'wisdom': self.gff.get('Wis', 10),
            'charisma': self.gff.get('Cha', 10),
            'classes': set(c.get('Class') for c in class_list),
            'level': sum(c.get('ClassLevel', 0) for c in class_list),
            'bab': self.character_manager.get_manager('combat').get_base_attack_bonus() if self.character_manager.get_manager('combat') else 0
        }

    def _resolve_prerequisite_errors(self, errors: List[str]) -> List[str]:
        """Resolve 'Requires Feat 123' error strings to 'Requires Power Attack' format."""
        resolved_errors = []
        for error in errors:
            if error.startswith("Requires Feat "):
                try:
                    feat_id_str = error.replace("Requires Feat ", "")
                    prereq_feat_id = int(feat_id_str)
                    prereq_info = self.get_feat_info_display(prereq_feat_id)
                    if prereq_info:
                        prereq_name = prereq_info.get('name', f'Feat {prereq_feat_id}')
                    else:
                        prereq_name = f'Feat {prereq_feat_id}'
                    resolved_errors.append(f"Requires {prereq_name}")
                except (ValueError, AttributeError):
                    resolved_errors.append(error)
            else:
                resolved_errors.append(error)
        return resolved_errors

    def get_feat_prerequisites_info_batch(self, feat_ids: List[int]) -> Dict[int, Tuple[bool, List[str]]]:
        """Get prerequisite information for multiple feats at once for UI display only."""
        if not self._prerequisite_graph or not self._prerequisite_graph.is_built:
            logger.error("PrerequisiteGraph not available for batch validation")
            return {feat_id: (False, ["Validation unavailable"]) for feat_id in feat_ids}

        character_feats = set()
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            character_feats.add(feat.get('Feat', -1))

        character_data = self._get_character_validation_data()

        batch_results = self._prerequisite_graph.validate_batch_fast(
            feat_ids, character_feats, character_data
        )

        resolved_batch_results = {}
        for feat_id, (can_take, errors) in batch_results.items():
            resolved_errors = self._resolve_prerequisite_errors(errors)
            resolved_batch_results[feat_id] = (can_take, resolved_errors)

        return resolved_batch_results
    
    def get_feat_prerequisites_info(self, feat_id: int, feat_data=None) -> Tuple[bool, List[str]]:
        """Get prerequisite information for a feat using Rust PrerequisiteGraph for UI display only."""
        if not self._prerequisite_graph or not self._prerequisite_graph.is_built:
            logger.critical("PrerequisiteGraph not available! Cannot validate feat prerequisites.")
            return True, []

        character_feats = set()
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            character_feats.add(feat.get('Feat', -1))

        character_data = self._get_character_validation_data()

        can_take, errors = self._prerequisite_graph.validate_feat_prerequisites_fast(
            feat_id, character_feats, character_data
        )

        resolved_errors = self._resolve_prerequisite_errors(errors)
        return can_take, resolved_errors
    
    def get_detailed_prerequisites(self, feat_id: int) -> Dict[str, Any]:
        """Get detailed prerequisite information in a user-friendly format for UI display only."""
        feat_data = self.game_rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return {'requirements': [], 'met': [], 'unmet': []}
        
        prereqs = field_mapper.get_feat_prerequisites(feat_data)
        detailed = {
            'requirements': [],
            'met': [],
            'unmet': []
        }

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

        for prereq_feat_id in prereqs['feats']:
            has_prereq = self.has_feat(prereq_feat_id)
            if prereq_feat_id in self._feat_cache:
                prereq_name_raw = self._feat_cache[prereq_feat_id].get('label', f'Feat {prereq_feat_id}')
            else:
                prereq_feat_data = self.game_rules_service.get_by_id('feat', prereq_feat_id)
                prereq_name_raw = field_mapper.get_field_value(prereq_feat_data, 'label', f'Feat {prereq_feat_id}') if prereq_feat_data else f'Feat {prereq_feat_id}'

            if isinstance(prereq_name_raw, int) and prereq_name_raw > 0:
                prereq_name = self.game_rules_service._loader.get_string(prereq_name_raw) or f'Feat {prereq_feat_id}'
            else:
                prereq_name = str(prereq_name_raw) if prereq_name_raw else f'Feat {prereq_feat_id}'

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

        if prereqs['class'] >= 0:
            class_list = self.gff.get('ClassList', [])
            has_class = any(c.get('Class') == prereqs['class'] for c in class_list)
            class_id = prereqs['class']
            if class_id in self._class_cache:
                class_name = self._class_cache[class_id]
            else:
                class_data = self.game_rules_service.get_by_id('classes', class_id)
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

        if prereqs['bab'] > 0:
            total_level = sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', []))
            estimated_bab = total_level
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

        if prereqs['spell_level'] > 0:
            req_text = f"Able to cast {prereqs['spell_level']}th level spells"
            detailed['requirements'].append({
                'type': 'spell_level',
                'description': req_text,
                'required_value': prereqs['spell_level'],
                'met': False
            })
            detailed['unmet'].append(req_text)
        
        return detailed
    
    def _remove_class_feats(self, class_id: int, level: int, preserve_list: List[int]):
        """Remove feats granted by a class using conservative approach with special cleric domain handling."""
        class_data = self.game_rules_service.get_by_id('classes', class_id)
        if not class_data:
            return
        
        feats_to_remove = []
        class_manager = self.character_manager.get_manager('class')

        for lvl in range(1, level + 1):
            if class_manager:
                feats_at_level = class_manager.get_class_feats_for_level(class_data, lvl)
            else:
                feats_at_level = []
            for feat_info in feats_at_level:
                if feat_info['list_type'] == 0:
                    feat_id = feat_info['feat_id']
                    if feat_id not in preserve_list and not self.is_feat_protected(feat_id):
                        feats_to_remove.append(feat_id)

        feat_list = self.gff.get('FeatList', [])
        current_class_ids = set()
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            if class_entry.get('Class') != class_id:
                current_class_ids.add(class_entry.get('Class'))
        
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id in preserve_list or self.is_feat_protected(feat_id):
                continue

            if self._is_class_specific_feat(feat_id, class_id, current_class_ids):
                if feat_id not in feats_to_remove:
                    feats_to_remove.append(feat_id)

        # SPECIAL CLERIC HANDLING - Remove domain feats if losing all cleric levels
        class_name = field_mapper.get_field_value(class_data, 'label', f'Class {class_id}')
        is_cleric = class_name.lower() == 'cleric'

        domain_feats_removed = 0
        if is_cleric:
            remaining_cleric_levels = 0
            for class_entry in class_list:
                if class_entry.get('Class') == class_id:
                    continue
                entry_class_data = self.game_rules_service.get_by_id('classes', class_entry.get('Class'))
                if entry_class_data:
                    entry_class_name = field_mapper.get_field_value(entry_class_data, 'label', '').lower()
                    if entry_class_name == 'cleric':
                        remaining_cleric_levels += class_entry.get('ClassLevel', 0)

            if remaining_cleric_levels == 0:
                logger.info(f"Removing all domain feats due to complete loss of cleric class")
                domain_feats_removed = self.remove_all_domain_feats()
            else:
                logger.debug(f"Preserving domain feats - character retains {remaining_cleric_levels} cleric levels")

        removed_count = 0
        for feat_id in feats_to_remove:
            if self.remove_feat(feat_id, force=False):
                removed_count += 1
        
        if removed_count > 0 or domain_feats_removed > 0:
            total_removed = removed_count + domain_feats_removed
            logger.info(f"Removed {total_removed} feats from {class_name} ({removed_count} class feats, {domain_feats_removed} domain feats)")
    
    def _get_feats_gained_during_class_levels(self, class_id: int) -> List[int]:
        """Get all feats gained during levels of a specific class using LvlStatList."""
        feats_from_class = []
        
        # Get the level progression data
        lvl_stat_list = self.gff.get('LvlStatList', [])
        
        for level_data in lvl_stat_list:
            if level_data.get('LvlStatClass') == class_id:
                # This level was taken in the specified class
                feat_list = level_data.get('FeatList', [])
                for feat_entry in feat_list:
                    feat_id = feat_entry.get('Feat')
                    if feat_id is not None:
                        feats_from_class.append(feat_id)
        
        return feats_from_class
    
    def _get_all_class_feats_all_levels(self, class_id: int) -> Set[int]:
        """Get all feats that a class can grant across all its levels."""
        all_class_feats = set()
        
        try:
            class_data = self.game_rules_service.get_by_id('classes', class_id)
            if not class_data:
                return all_class_feats
            
            # Get the class feat table
            feat_table_name = field_mapper.get_field_value(class_data, 'feats_table', None)
            if feat_table_name:
                feat_table = self.game_rules_service.get_table(feat_table_name.lower())
                if feat_table:
                    for feat_entry in feat_table:
                        feat_id = field_mapper._safe_int(
                            field_mapper.get_field_value(feat_entry, 'feat_index', -1)
                        )
                        if feat_id >= 0:
                            all_class_feats.add(feat_id)
            
        except Exception as e:
            logger.warning(f"Error getting all feats for class {class_id}: {e}")
        
        return all_class_feats
    
    def _should_preserve_feat_level_based(self, feat_id: int, removed_class_id: int, remaining_class_ids: set) -> bool:
        """Check if a feat should be preserved because other remaining classes can also grant it."""
        # Check if any remaining class can grant this feat
        for class_id in remaining_class_ids:
            class_feats = self._get_all_class_feats_all_levels(class_id)
            if feat_id in class_feats:
                logger.debug(f"Preserving feat {feat_id} - also granted by class {class_id}")
                return True
        
        return False
    
    def _is_class_specific_feat(self, feat_id: int, removed_class_id: int, remaining_class_ids: set) -> bool:
        """Check if a feat should be removed using level-based analysis from LvlStatList."""
        # Step 1: Check if this feat was actually gained during removed class levels
        feats_from_removed_class = self._get_feats_gained_during_class_levels(removed_class_id)
        if feat_id not in feats_from_removed_class:
            # This feat was NOT gained during removed class levels, so keep it
            return False
        
        # Step 2: Check if any remaining class can also grant this feat
        if self._should_preserve_feat_level_based(feat_id, removed_class_id, remaining_class_ids):
            # Another class can grant this feat, so preserve it
            return False
        
        # Step 3: This feat was gained during removed class levels AND no other class grants it
        feat_data = self.game_rules_service.get_by_id('feat', feat_id)
        feat_label = field_mapper.get_field_value(feat_data, 'label', f'Feat_{feat_id}') if feat_data else f'Feat_{feat_id}'
        removed_class_data = self.game_rules_service.get_by_id('classes', removed_class_id)
        class_name = field_mapper.get_field_value(removed_class_data, 'label', f'Class_{removed_class_id}') if removed_class_data else f'Class_{removed_class_id}'
        
        logger.debug(f"Removing level-based feat {feat_id} ({feat_label}) - gained during {class_name} levels and not available from remaining classes")
        return True
    
    def _is_feat_from_class_table(self, feat_id: int, class_id: int) -> bool:
        """Check if a feat is granted by a specific class's feat table using hybrid lookup approach."""
        try:
            class_data = self.game_rules_service.get_by_id('classes', class_id)
            if not class_data:
                return False
            
            # Check if this class has a feat table using field mapper
            feat_table_name = field_mapper.get_field_value(class_data, 'feats_table', None)
            if not feat_table_name:
                return False
            
            # Direct table lookup approach (more reliable)
            try:
                # Load the feat table directly
                feat_table = self.game_rules_service.get_table(feat_table_name.lower())
                if feat_table:
                    for feat_entry in feat_table:
                        # Use field mapper to get feat index with proper field name mapping
                        entry_feat_id = field_mapper._safe_int(
                            field_mapper.get_field_value(feat_entry, 'feat_index', -1)
                        )
                        if entry_feat_id == feat_id:
                            logger.debug(f"Found feat {feat_id} in class {class_id} feat table {feat_table_name}")
                            return True
            except Exception as e:
                logger.debug(f"Direct table lookup failed for {feat_table_name}: {e}")
            
            # Fallback: Use class manager's method (level-by-level)
            class_manager = self.character_manager.get_manager('class')
            if class_manager:
                for level in range(1, 21):  # Check levels 1-20
                    feats_at_level = class_manager.get_class_feats_for_level(class_data, level)
                    for feat_info in feats_at_level:
                        if feat_info.get('feat_id') == feat_id:
                            logger.debug(f"Found feat {feat_id} in class {class_id} feat table at level {level}")
                            return True
            
            return False
            
        except Exception as e:
            logger.warning(f"Error checking feat {feat_id} for class {class_id}: {e}")
            return False  # Conservative: if we can't determine, don't remove
    
    def _add_class_feats(self, class_id: int, level: int):
        """Add feats granted by a class with special cleric domain handling."""
        class_data = self.game_rules_service.get_by_id('classes', class_id)
        if not class_data:
            return

        added_count = 0
        class_manager = self.character_manager.get_manager('class')
        for lvl in range(1, level + 1):
            if class_manager:
                feats_at_level = class_manager.get_class_feats_for_level(class_data, lvl)
            else:
                feats_at_level = []
            for feat_info in feats_at_level:
                if feat_info['list_type'] == 0:
                    if self.add_feat(feat_info['feat_id'], source='class'):
                        added_count += 1

        # SPECIAL CLERIC HANDLING - Add default domain feats if first cleric level
        class_name = field_mapper.get_field_value(class_data, 'label', f'Class {class_id}')
        is_cleric = class_name.lower() == 'cleric'

        domain_feats_added = 0
        if is_cleric:
            existing_domain_feats = self.get_character_domain_feats()

            if not existing_domain_feats:
                available_domains = self.get_available_domains()
                if available_domains and len(available_domains) >= 2:
                    default_domain_ids = {available_domains[0]['id'], available_domains[1]['id']}
                    domain_feats_added = self.add_domain_feats_for_domains(default_domain_ids)

                    domain_names = [d['name'] for d in available_domains[:2]]
                    logger.info(f"Added default domains for new cleric: {domain_names}")
                else:
                    logger.warning("Could not add default domains - insufficient available domains")
            else:
                logger.debug(f"Character already has domain feats, not adding defaults")
        
        if added_count > 0 or domain_feats_added > 0:
            total_added = added_count + domain_feats_added
            logger.info(f"Added {total_added} feats for {class_name} ({added_count} class feats, {domain_feats_added} domain feats)")
    
    def is_legitimate_feat(self, feat_data) -> bool:
        """Check if a feat is legitimate and should be shown to users."""
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
    
    def get_legitimate_feats(
        self,
        feat_type: Optional[int] = None,
        search: Optional[str] = None,
        page: int = 1,
        limit: int = 50
    ) -> Dict[str, Any]:
        """Get list of all legitimate feats with complete filtering and pagination."""
        import time
        time_filtering = 0
        time_search = 0
        time_get_feat_info = 0

        all_feats = self.game_rules_service.get_table('feat')
        if not all_feats:
            logger.warning("get_legitimate_feats: No feats table found")
            return self._build_pagination_response([], page, limit, 0)

        logger.debug(f"get_legitimate_feats: Processing {len(all_feats)} feats (feat_type={feat_type}, search={search}, page={page}, limit={limit})")
        filtered_count = 0

        # Phase 1: Fast filtering to get feat IDs only (no display cache building)
        legitimate_feat_ids = []
        for row_index, feat_data in enumerate(all_feats):
            t0 = time.perf_counter()

            # Filter out illegitimate feats
            if not self.is_legitimate_feat(feat_data):
                filtered_count += 1
                time_filtering += time.perf_counter() - t0
                continue

            # Use proper row index as feat ID
            feat_id = getattr(feat_data, 'id', getattr(feat_data, 'row_index', row_index))

            # Filter by feat type (bitwise AND for multiple type flags)
            if feat_type is not None:
                data_type = field_mapper.get_field_value(feat_data, 'type', 0)

                data_type_int = 0
                if isinstance(data_type, str):
                    data_type_upper = data_type.upper()
                    if 'GENERAL' in data_type_upper:
                        data_type_int = 1
                    elif 'PROFICIENCY' in data_type_upper:
                        data_type_int = 2
                    elif 'SKILLNSAVE' in data_type_upper or 'SKILL' in data_type_upper:
                        data_type_int = 4
                    elif 'METAMAGIC' in data_type_upper:
                        data_type_int = 8
                    elif 'DIVINE' in data_type_upper:
                        data_type_int = 16
                    elif 'EPIC' in data_type_upper:
                        data_type_int = 32
                    elif 'CLASSABILITY' in data_type_upper:
                        data_type_int = 64
                    elif 'BACKGROUND' in data_type_upper:
                        data_type_int = 128
                    elif 'SPELLCASTING' in data_type_upper:
                        data_type_int = 256
                    elif 'HISTORY' in data_type_upper:
                        data_type_int = 512
                    elif 'HERITAGE' in data_type_upper:
                        data_type_int = 1024
                    elif 'ITEMCREATION' in data_type_upper or 'ITEM' in data_type_upper:
                        data_type_int = 2048
                    elif 'RACIALABILITY' in data_type_upper or 'RACIAL' in data_type_upper:
                        data_type_int = 4096
                    else:
                        data_type_int = 1
                else:
                    try:
                        data_type_int = int(data_type) if data_type else 0
                        if data_type_int not in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096]:
                            data_type_int = 1
                    except (ValueError, TypeError):
                        data_type_int = 1

                # Override type for domains and backgrounds
                if self.is_domain_epithet_feat(feat_id):
                    data_type_int = 8192
                else:
                    label = field_mapper.get_field_value(feat_data, 'label', '')
                    if 'BACKGROUND' in str(label).upper():
                        data_type_int = 128

                if not (data_type_int & feat_type):
                    time_filtering += time.perf_counter() - t0
                    continue

            time_filtering += time.perf_counter() - t0
            legitimate_feat_ids.append((feat_id, feat_data))

        # Phase 2: Apply search BEFORE pagination (critical for correct results)
        if search:
            t_search = time.perf_counter()
            search_lower = search.lower()
            filtered_feat_ids = []

            for feat_id, feat_data in legitimate_feat_ids:
                # Get name from label reference
                label_ref = field_mapper.get_field_value(feat_data, 'feat')
                if isinstance(label_ref, int):
                    name = self.game_rules_service._loader.get_string(label_ref) if label_ref else ''
                else:
                    name = str(label_ref) if label_ref else ''

                # Get description
                desc_ref = field_mapper.get_field_value(feat_data, 'description')
                if isinstance(desc_ref, int):
                    description = self.game_rules_service._loader.get_string(desc_ref) if desc_ref else ''
                else:
                    description = str(desc_ref) if desc_ref else ''

                # Search in name or description
                if search_lower in name.lower() or search_lower in description.lower():
                    filtered_feat_ids.append((feat_id, feat_data))

            legitimate_feat_ids = filtered_feat_ids
            time_search = time.perf_counter() - t_search
            logger.debug(f"Search '{search}' filtered to {len(legitimate_feat_ids)} feats")

        total_count = len(legitimate_feat_ids)

        # Phase 3: Apply pagination BEFORE building display cache
        start_idx = (page - 1) * limit
        end_idx = start_idx + limit
        feats_to_load = legitimate_feat_ids[start_idx:end_idx]
        logger.info(f"Pagination: loading {len(feats_to_load)} of {total_count} feats (page {page})")

        # Phase 4: Build display cache ONLY for requested feats
        legitimate = []
        for feat_id, feat_data in feats_to_load:
            t1 = time.perf_counter()
            feat_info = self.get_feat_info(feat_id, feat_data, skip_validation=True)
            time_get_feat_info += time.perf_counter() - t1
            if feat_info:
                legitimate.append(feat_info)

        logger.info(f"get_legitimate_feats: Returned {len(legitimate)} legitimate feats (filtered out {filtered_count}, total valid: {total_count})")
        logger.info(f"get_legitimate_feats TIMING: filtering={time_filtering:.3f}s, search={time_search:.3f}s, get_feat_info={time_get_feat_info:.3f}s")
        logger.info(f"get_legitimate_feats: Display cache size: {len(self._display_cache)} entries")

        return self._build_pagination_response(legitimate, page, limit, total_count)

    def _build_pagination_response(self, feats: List[Dict[str, Any]], page: int, limit: int, total: int) -> Dict[str, Any]:
        """Build standardized pagination response structure with metadata."""
        pages = (total + limit - 1) // limit if total > 0 else 1

        return {
            'feats': feats,
            'pagination': {
                'page': page,
                'limit': limit,
                'total': total,
                'pages': pages,
                'has_next': page < pages,
                'has_previous': page > 1
            }
        }

    def get_available_feats(self, feat_type: Optional[int] = None) -> List[Dict[str, Any]]:
        """Get list of feats available for selection with prerequisite validation."""
        available = []
        
        # Get all feats from dynamic game data
        all_feats = self.game_rules_service.get_table('feat')
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
                data_type_int = self._parse_feat_type(feat_data)
                if not (data_type_int & feat_type):
                    continue
            
            total_checked += 1
            
            # Check prerequisites - this is the expensive part, do it last
            is_valid, _ = self.get_feat_prerequisites_info(feat_id, feat_data)
            prereq_checked += 1
            
            if is_valid:
                available.append(self.get_feat_info(feat_id, feat_data))
        
        logger.debug(f"get_available_feats: Checked {prereq_checked} prerequisites out of {total_checked} candidates from {len(all_feats)} total feats")
        
        return available
    
    def get_feat_summary(self) -> Dict[str, Any]:
        """Get summary of character's feats"""
        feat_list = self.gff.get('FeatList', [])
        
        categorized = {
            'total': len(feat_list),
            'protected': [],
            'class_feats': [],
            'general_feats': [],
            'custom_feats': [],
            'background_feats': [],
            'domain_feats': []
        }
        
        for feat in feat_list:
            feat_id = feat.get('Feat', 0)
            feat_info = self.get_feat_info(feat_id)
            
            if feat_info['protected']:
                categorized['protected'].append(feat_info)
            
            if feat_info['custom']:
                categorized['custom_feats'].append(feat_info)
            elif feat_info['type'] == 1:
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
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)
            if not feat_data:
                errors.append(f"Feat ID {feat_id} not found in feat table - may cause load errors")
        
        # NOTE: Prerequisite validation removed per validation cleanup plan
        # Users can now have any feat regardless of prerequisites
        
        return len(errors) == 0, errors
    
    def get_all_feats(self) -> List[Dict[str, Any]]:
        """Get all character feats with detailed information."""
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
        """Add multiple feats at once and return results for each."""
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

    def swap_feat(self, old_feat_id: int, new_feat_id: int) -> bool:
        """Replace a feat with another for retraining purposes."""
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
        """Check if feat can be taken and return reason."""
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
        """Get remaining uses for a feat with limited uses per day."""
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            if feat.get('Feat') == feat_id:
                uses = feat.get('Uses', -1)
                return uses if uses >= 0 else None
        
        return None
    
    def set_feat_uses(self, feat_id: int, uses: int) -> bool:
        """Set remaining uses for a feat."""
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            if feat.get('Feat') == feat_id:
                feat['Uses'] = uses
                return True
        
        return False
    
    def get_bonus_feats_available(self) -> int:
        """Get number of unallocated bonus feats available to select."""
        # This is complex as it depends on class levels and feat selections
        # For now, return a placeholder
        # TODO: Implement proper bonus feat tracking
        return 0
    
    def get_feat_categories(self) -> Dict[str, List[int]]:
        """Get feats organized by category."""
        from collections import defaultdict
        categories = defaultdict(list)
        feat_list = self.gff.get('FeatList', [])
        
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id >= 0:
                feat_data = self.game_rules_service.get_by_id('feat', feat_id)
                if feat_data:
                    # Get category from feat data
                    category = field_mapper.get_field_value(feat_data, 'categories', 'General')
                    if not category:
                        category = 'General'
                    
                    categories[category].append(feat_id)
        
        return dict(categories)
    
    def get_feat_categories_fast(self) -> Dict[str, List[Dict[str, Any]]]:
        """Get feats organized by category using fast display methods without validation."""
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
        """Get category name based on feat type number."""
        if feat_type == 1:
            return 'General'
        elif feat_type == 2:
            return 'Proficiency'
        elif feat_type == 4:
            return 'Skill/Save'
        elif feat_type == 8:
            return 'Metamagic'
        elif feat_type == 16:
            return 'Divine'
        elif feat_type == 32:
            return 'Epic'
        elif feat_type == 64:
            return 'Class'
        elif feat_type == 128:
            return 'Background'
        elif feat_type == 256:
            return 'Spellcasting'
        elif feat_type == 512:
            return 'History'
        elif feat_type == 1024:
            return 'Heritage'
        elif feat_type == 2048:
            return 'Item Creation'
        elif feat_type == 4096:
            return 'Racial'
        else:
            return 'General'

    def _strip_nwn2_tags(self, text: str) -> str:
        """Strip NWN2 markup tags like <i>, <color=Gold>, etc. from text."""
        import re
        if not text:
            return text
        text = re.sub(r'</?[a-zA-Z][^>]*>', '', text)
        return text.strip()

    def _parse_feat_type(self, feat_data) -> int:
        """Parse numeric feat type from feat data by checking DESCRIPTION and FeatCategory fields."""
        import re

        description = field_mapper.get_field_value(feat_data, 'description', '')
        feat_category = field_mapper.get_field_value(feat_data, 'type', '')

        if isinstance(feat_category, str):
            feat_category_upper = feat_category.upper()

            if 'GENERAL' in feat_category_upper:
                return 1
            elif 'PROFICIENCY' in feat_category_upper:
                return 2
            elif 'SKILLNSAVE' in feat_category_upper or 'SKILL' in feat_category_upper:
                return 4
            elif 'METAMAGIC' in feat_category_upper:
                return 8
            elif 'DIVINE' in feat_category_upper:
                return 16
            elif 'EPIC' in feat_category_upper:
                return 32
            elif 'CLASSABILITY' in feat_category_upper:
                return 64
            elif 'BACKGROUND' in feat_category_upper:
                return 128
            elif 'SPELLCASTING' in feat_category_upper:
                return 256
            elif 'HISTORY' in feat_category_upper:
                return 512
            elif 'HERITAGE' in feat_category_upper:
                return 1024
            elif 'ITEMCREATION' in feat_category_upper or 'ITEM' in feat_category_upper:
                return 2048
            elif 'RACIALABILITY' in feat_category_upper or 'RACIAL' in feat_category_upper:
                return 4096

        try:
            feat_type_int = int(feat_category) if feat_category else 0
            if feat_type_int in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096]:
                return feat_type_int
        except (ValueError, TypeError):
            pass

        if description:
            match = re.search(r'Type of Feat:\s*(\w+)', description, re.IGNORECASE)
            if match:
                feat_type_str = match.group(1).lower()

                if feat_type_str == 'combat':
                    return 2
                elif feat_type_str == 'metamagic':
                    return 8
                elif feat_type_str == 'epic':
                    return 32
                elif feat_type_str == 'class':
                    return 64
                elif feat_type_str == 'background':
                    return 128
                elif feat_type_str == 'special':
                    return 16

        return 1
    
    def detect_epithet_feats(self) -> Set[int]:
        """Detect epithet feats like special story or custom feats that should be protected."""
        epithet_feats = set()
        
        # Get all feats from character
        feat_list = self.gff.get('FeatList', [])
        
        # Get vanilla feat data
        feat_table = self.game_rules_service.get_table('feat')
        if not feat_table:
            logger.warning("Feat table not found, cannot detect epithet feats")
            return epithet_feats
        
        # Build set of all vanilla feat IDs (using row indices which correspond to feat IDs)
        try:
            vanilla_feat_ids = set(range(len(feat_table)))
        except (TypeError, AttributeError):
            # Handle case where feat_table is a Mock or not a proper list
            logger.debug("Feat table is not a proper list, cannot detect epithet feats")
            return epithet_feats
        
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
                    feat_data = self.game_rules_service.get_by_id('feat', feat_id)
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
    
    # ===== DOMAIN FEAT MANAGEMENT =====
    
    def _build_domain_feat_map(self) -> Dict[int, List[Dict]]:
        """Build mapping of feat_id to list of domains that grant it."""
        if self._domain_feat_map_cache is not None:
            return self._domain_feat_map_cache
        
        domain_feat_map = {}
        
        # Get domains data
        domains_table = self.game_rules_service.get_table('domains')
        if not domains_table:
            logger.warning("Domains table not found, cannot build domain feat map")
            self._domain_feat_map_cache = {}
            return self._domain_feat_map_cache
        
        # Process each domain
        for domain_id, domain_data in enumerate(domains_table):
            try:
                # Get domain name
                domain_name = field_mapper.get_field_value(domain_data, 'label', f'Domain_{domain_id}')
                
                # Check each feat type
                for feat_type in ['GrantedFeat', 'CastableFeat', 'EpithetFeat']:
                    feat_id = field_mapper.get_field_value(domain_data, feat_type, None)
                    
                    # Validate feat ID
                    if feat_id and str(feat_id).strip() not in ['', '****', '-1', '0']:
                        try:
                            feat_id_int = int(feat_id)
                            if feat_id_int > 0:
                                # Add to map
                                if feat_id_int not in domain_feat_map:
                                    domain_feat_map[feat_id_int] = []
                                
                                domain_feat_map[feat_id_int].append({
                                    'domain_id': domain_id,
                                    'domain_name': domain_name,
                                    'feat_type': feat_type
                                })
                                
                        except (ValueError, TypeError):
                            continue
                            
            except Exception as e:
                logger.warning(f"Error processing domain {domain_id}: {e}")
                continue
        
        logger.info(f"Built domain feat map with {len(domain_feat_map)} domain feats")
        self._domain_feat_map_cache = domain_feat_map
        return self._domain_feat_map_cache
    
    def get_all_domain_feat_ids(self) -> Set[int]:
        """Get set of all feat IDs that are domain feats."""
        if self._domain_feats_cache is not None:
            return self._domain_feats_cache
        
        domain_feat_map = self._build_domain_feat_map()
        self._domain_feats_cache = set(domain_feat_map.keys())
        
        logger.debug(f"Found {len(self._domain_feats_cache)} total domain feats")
        return self._domain_feats_cache
    
    def is_domain_feat(self, feat_id: int) -> bool:
        """Check if a feat is a domain feat from domains.2da."""
        domain_feats = self.get_all_domain_feat_ids()
        return feat_id in domain_feats

    def is_domain_epithet_feat(self, feat_id: int) -> bool:
        """Check if a feat is a domain epithet feat, the selectable domain marker."""
        domain_feat_map = self._build_domain_feat_map()
        if feat_id not in domain_feat_map:
            return False

        # Check if this feat is listed as EpithetFeat for any domain
        for domain_info in domain_feat_map[feat_id]:
            if domain_info.get('feat_type') == 'EpithetFeat':
                return True
        return False
    
    def get_character_domain_feats(self) -> Set[int]:
        """Get domain feats that the character currently has."""
        character_feats = set()
        feat_list = self.gff.get('FeatList', [])
        
        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat', -1)
            if feat_id > 0:
                character_feats.add(feat_id)
        
        # Filter to only domain feats
        domain_feats = self.get_all_domain_feat_ids()
        return character_feats.intersection(domain_feats)
    
    def get_character_active_domains(self) -> Set[int]:
        """Determine which domains are active based on character's domain feats."""
        character_domain_feats = self.get_character_domain_feats()
        domain_feat_map = self._build_domain_feat_map()
        
        active_domains = set()
        
        for feat_id in character_domain_feats:
            if feat_id in domain_feat_map:
                for domain_info in domain_feat_map[feat_id]:
                    active_domains.add(domain_info['domain_id'])
        
        logger.debug(f"Character has active domains: {active_domains}")
        return active_domains
    
    def get_domain_feats_for_domains(self, domain_ids: Set[int]) -> Set[int]:
        """Get all feat IDs associated with specific domains."""
        domain_feat_map = self._build_domain_feat_map()
        domain_feats = set()
        
        for feat_id, domain_list in domain_feat_map.items():
            for domain_info in domain_list:
                if domain_info['domain_id'] in domain_ids:
                    domain_feats.add(feat_id)
                    break
        
        return domain_feats
    
    def add_domain(self, domain_id: int) -> Dict[str, Any]:
        """Add a domain to the character by granting all associated feats."""
        from ..events import DomainChangedEvent
        import time

        domains_table = self.game_rules_service.get_table('domains')
        if not domains_table or domain_id >= len(domains_table):
            raise ValueError(f"Invalid domain ID: {domain_id}")

        domain_data = domains_table[domain_id]
        domain_name = field_mapper.get_field_value(domain_data, 'label', f'Domain_{domain_id}')

        added_feats = []

        # Get all feat types for this domain
        feat_types = ['GrantedFeat', 'CastableFeat', 'EpithetFeat']

        for feat_type in feat_types:
            feat_id = field_mapper.get_field_value(domain_data, feat_type, None)

            if feat_id and str(feat_id).strip() not in ['', '****', '-1', '0']:
                try:
                    feat_id_int = int(feat_id)
                    if feat_id_int > 0:
                        if not self.has_feat(feat_id_int):
                            success = self.add_feat(feat_id_int, source='domain')
                            if success:
                                feat_info = self.get_feat_info_display(feat_id_int)
                                added_feats.append({
                                    'feat_id': feat_id_int,
                                    'feat_name': feat_info.get('name', f'Feat_{feat_id_int}'),
                                    'feat_type': feat_type
                                })
                                logger.info(f"Added {feat_type} feat {feat_id_int} ({feat_info.get('name')}) for domain {domain_name}")
                        else:
                            logger.debug(f"Character already has {feat_type} feat {feat_id_int} for domain {domain_name}")
                except (ValueError, TypeError) as e:
                    logger.warning(f"Invalid feat ID in domain {domain_id} {feat_type}: {feat_id} - {e}")

        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            class_id = class_entry.get('Class')
            class_data = self.game_rules_service.get_by_id('classes', class_id)

            has_domains = (
                field_mapper.get_field_value(class_data, 'HasDomains', '') == '1' or
                field_mapper.get_field_value(class_data, 'MaxDomains', '0') != '0'
            )

            if has_domains:
                domain1 = class_entry.get('Domain1', -1)
                domain2 = class_entry.get('Domain2', -1)

                if domain1 == -1:
                    class_entry['Domain1'] = domain_id
                    logger.info(f"Set Domain1 to {domain_id} ({domain_name})")
                    break
                elif domain2 == -1:
                    class_entry['Domain2'] = domain_id
                    logger.info(f"Set Domain2 to {domain_id} ({domain_name})")
                    break
                else:
                    raise ValueError(f"Character already has 2 domains (Domain1={domain1}, Domain2={domain2})")

        self.gff.set('ClassList', class_list)

        from ..events import EventType

        event = DomainChangedEvent(
            event_type=EventType.DOMAIN_ADDED,
            source_manager='FeatManager',
            timestamp=time.time(),
            domain_id=domain_id,
            domain_name=domain_name,
            action='added',
            feats_affected=added_feats
        )
        self.character_manager.emit(event)

        return {
            'domain_id': domain_id,
            'domain_name': domain_name,
            'added_feats': added_feats,
            'total_feats_added': len(added_feats)
        }

    def remove_domain(self, domain_id: int) -> Dict[str, Any]:
        """Remove a domain from the character by removing all associated feats."""
        from ..events import DomainChangedEvent
        import time

        domains_table = self.game_rules_service.get_table('domains')
        if not domains_table or domain_id >= len(domains_table):
            raise ValueError(f"Invalid domain ID: {domain_id}")

        domain_data = domains_table[domain_id]
        domain_name = field_mapper.get_field_value(domain_data, 'label', f'Domain_{domain_id}')

        removed_feats = []

        # Get all feat types for this domain
        feat_types = ['GrantedFeat', 'CastableFeat', 'EpithetFeat']

        for feat_type in feat_types:
            feat_id = field_mapper.get_field_value(domain_data, feat_type, None)

            if feat_id and str(feat_id).strip() not in ['', '****', '-1', '0']:
                try:
                    feat_id_int = int(feat_id)
                    if feat_id_int > 0:
                        if self.has_feat(feat_id_int):
                            success = self.remove_feat(feat_id_int, skip_cascade=True)
                            if success:
                                removed_feats.append({
                                    'feat_id': feat_id_int,
                                    'feat_type': feat_type
                                })
                                logger.info(f"Removed {feat_type} feat {feat_id_int} for domain {domain_name}")
                except (ValueError, TypeError) as e:
                    logger.warning(f"Invalid feat ID in domain {domain_id} {feat_type}: {feat_id} - {e}")

        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            domain1 = class_entry.get('Domain1', -1)
            domain2 = class_entry.get('Domain2', -1)

            if domain1 == domain_id:
                class_entry['Domain1'] = -1
                logger.info(f"Cleared Domain1 (was {domain_id} - {domain_name})")
            elif domain2 == domain_id:
                class_entry['Domain2'] = -1
                logger.info(f"Cleared Domain2 (was {domain_id} - {domain_name})")

        self.gff.set('ClassList', class_list)

        from ..events import EventType

        event = DomainChangedEvent(
            event_type=EventType.DOMAIN_REMOVED,
            source_manager='FeatManager',
            timestamp=time.time(),
            domain_id=domain_id,
            domain_name=domain_name,
            action='removed',
            feats_affected=removed_feats
        )
        self.character_manager.emit(event)

        return {
            'domain_id': domain_id,
            'domain_name': domain_name,
            'removed_feats': removed_feats,
            'total_feats_removed': len(removed_feats)
        }

    def invalidate_domain_caches(self):
        """Clear domain feat caches when game data changes"""
        self._domain_feats_cache = None
        self._domain_feat_map_cache = None
        logger.debug("Domain feat caches invalidated")
    
    def remove_all_domain_feats(self) -> int:
        """Remove all domain feats from character for cleric to non-cleric transition."""
        character_domain_feats = self.get_character_domain_feats()
        removed_count = 0
        
        logger.info(f"Removing {len(character_domain_feats)} domain feats for cleric class loss")
        
        for feat_id in character_domain_feats:
            # Force removal since these are domain feats being properly removed
            if self.remove_feat(feat_id, force=True):
                removed_count += 1
                logger.debug(f"Removed domain feat {feat_id}")
            else:
                logger.warning(f"Failed to remove domain feat {feat_id}")
        
        logger.info(f"Successfully removed {removed_count} domain feats")
        return removed_count
    
    def remove_domain_feats_for_domains(self, domain_ids: Set[int]) -> int:
        """Remove domain feats for specific domains when changing cleric domains."""
        if not domain_ids:
            return 0
        
        feats_to_remove = self.get_domain_feats_for_domains(domain_ids)
        character_feats = self.get_character_domain_feats()
        
        # Only remove feats the character actually has
        feats_to_remove = feats_to_remove.intersection(character_feats)
        
        removed_count = 0
        logger.info(f"Removing {len(feats_to_remove)} feats for domains {domain_ids}")
        
        for feat_id in feats_to_remove:
            if self.remove_feat(feat_id, force=True):
                removed_count += 1
                logger.debug(f"Removed domain feat {feat_id} for domain change")
            else:
                logger.warning(f"Failed to remove domain feat {feat_id}")
        
        logger.info(f"Successfully removed {removed_count} domain feats for domain change")
        return removed_count
    
    def add_domain_feats_for_domains(self, domain_ids: Set[int]) -> int:
        """Add domain feats for specific domains when gaining cleric class or changing domains."""
        if not domain_ids:
            return 0
        
        feats_to_add = self.get_domain_feats_for_domains(domain_ids)
        character_feats = self.get_character_domain_feats()
        
        # Only add feats the character doesn't already have
        feats_to_add = feats_to_add - character_feats
        
        added_count = 0
        logger.info(f"Adding {len(feats_to_add)} feats for domains {domain_ids}")
        
        for feat_id in feats_to_add:
            if self.add_feat(feat_id, source='domain'):
                added_count += 1
                logger.debug(f"Added domain feat {feat_id}")
            else:
                logger.warning(f"Failed to add domain feat {feat_id}")
        
        logger.info(f"Successfully added {added_count} domain feats")
        return added_count
    
    def change_cleric_domains(self, old_domain_ids: Set[int], new_domain_ids: Set[int]) -> Dict[str, int]:
        """Change cleric domains by removing old domain feats and adding new ones."""
        logger.info(f"Changing cleric domains from {old_domain_ids} to {new_domain_ids}")
        
        # Determine which domains to remove and add
        domains_to_remove = old_domain_ids - new_domain_ids
        domains_to_add = new_domain_ids - old_domain_ids
        
        removed_count = 0
        added_count = 0
        
        # Remove feats for old domains
        if domains_to_remove:
            removed_count = self.remove_domain_feats_for_domains(domains_to_remove)
        
        # Add feats for new domains
        if domains_to_add:
            added_count = self.add_domain_feats_for_domains(domains_to_add)
        
        result = {
            'removed': removed_count,
            'added': added_count,
            'domains_removed': domains_to_remove,
            'domains_added': domains_to_add
        }
        
        logger.info(f"Domain change complete: {result}")
        return result
    
    def get_available_domains(self) -> List[Dict[str, Any]]:
        """Get list of available domains for cleric selection."""
        domains_table = self.game_rules_service.get_table('domains')
        if not domains_table:
            return []
        
        available_domains = []
        
        for domain_id, domain_data in enumerate(domains_table):
            try:
                # Skip empty domains
                if not hasattr(domain_data, 'Label') and not hasattr(domain_data, 'label'):
                    continue
                
                domain_name = field_mapper.get_field_value(domain_data, 'label', f'Domain_{domain_id}')
                domain_desc = field_mapper.get_field_value(domain_data, 'description', '')
                
                # Skip domains with empty names
                if not domain_name or domain_name.strip() in ['', '****']:
                    continue
                
                available_domains.append({
                    'id': domain_id,
                    'name': domain_name,
                    'description': domain_desc,
                    'granted_feat': field_mapper.get_field_value(domain_data, 'GrantedFeat', None),
                    'castable_feat': field_mapper.get_field_value(domain_data, 'CastableFeat', None),
                    'epithet_feat': field_mapper.get_field_value(domain_data, 'EpithetFeat', None)
                })
                
            except Exception as e:
                logger.debug(f"Skipping domain {domain_id}: {e}")
                continue
        
        logger.debug(f"Found {len(available_domains)} available domains")
        return available_domains

    def get_save_bonuses(self) -> Dict[str, int]:
        """Calculate total save bonuses from all character feats by parsing feat descriptions."""
        if self._save_bonuses_cache is not None:
            return self._save_bonuses_cache.copy()

        bonuses = {'fortitude': 0, 'reflex': 0, 'will': 0}
        feat_list = self.gff.get('FeatList', [])

        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat') if isinstance(feat_entry, dict) else feat_entry
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)
            if not feat_data:
                continue

            label = (getattr(feat_data, 'LABEL', '') or '').lower()
            desc_raw = getattr(feat_data, 'DESCRIPTION', '')
            if isinstance(desc_raw, int) and desc_raw > 0:
                description = self.game_rules_service._loader.get_string(desc_raw) or ''
            else:
                description = str(desc_raw) if desc_raw else ''
            description_lower = description.lower()

            if any(kw in description_lower for kw in _SAVE_CONDITIONAL_KEYWORDS):
                continue

            for pattern, save_type in _SAVE_PATTERNS:
                match = pattern.search(description)
                if match:
                    bonus_value = int(match.group(1))
                    if save_type == 'universal':
                        bonuses['fortitude'] += bonus_value
                        bonuses['reflex'] += bonus_value
                        bonuses['will'] += bonus_value
                    elif save_type == 'fortitude_and_will':
                        bonuses['fortitude'] += bonus_value
                        bonuses['will'] += bonus_value
                    else:
                        bonuses[save_type] += bonus_value
                    break

        self._save_bonuses_cache = bonuses
        return bonuses.copy()

    def get_ac_bonuses(self) -> Dict[str, int]:
        """Calculate all AC bonuses from feats by parsing feat descriptions."""
        if self._ac_bonuses_cache is not None:
            return self._ac_bonuses_cache.copy()

        bonuses = {'dodge': 0, 'misc': 0}
        feat_list = self.gff.get('FeatList', [])

        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat') if isinstance(feat_entry, dict) else feat_entry
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)
            if not feat_data:
                continue

            label = (getattr(feat_data, 'LABEL', '') or '').lower()
            desc_raw = getattr(feat_data, 'DESCRIPTION', '')
            if isinstance(desc_raw, int) and desc_raw > 0:
                description = self.game_rules_service._loader.get_string(desc_raw) or ''
            else:
                description = str(desc_raw) if desc_raw else ''
            description_lower = description.lower()

            if any(kw in description_lower for kw in _AC_CONDITIONAL_KEYWORDS):
                continue

            if 'dodge' in label or 'mobility' in label:
                match = _AC_DODGE_PATTERN.search(description)
                if match:
                    bonuses['dodge'] += int(match.group(1))
                elif 'dodge' in label:
                    bonuses['dodge'] += 1
                continue

            for pattern in _AC_PATTERNS:
                match = pattern.search(description)
                if match:
                    bonuses['misc'] += int(match.group(1))
                    break

        self._ac_bonuses_cache = bonuses
        return bonuses.copy()

    def get_initiative_bonus(self) -> int:
        """Calculate initiative bonus from feats by parsing feat descriptions."""
        if self._initiative_bonus_cache is not None:
            return self._initiative_bonus_cache

        bonus = 0
        feat_list = self.gff.get('FeatList', [])

        for feat_entry in feat_list:
            feat_id = feat_entry.get('Feat') if isinstance(feat_entry, dict) else feat_entry
            feat_data = self.game_rules_service.get_by_id('feat', feat_id)
            if not feat_data:
                continue

            label = (getattr(feat_data, 'LABEL', '') or '').lower()
            desc_raw = getattr(feat_data, 'DESCRIPTION', '')
            if isinstance(desc_raw, int) and desc_raw > 0:
                description = self.game_rules_service._loader.get_string(desc_raw) or ''
            else:
                description = str(desc_raw) if desc_raw else ''

            if 'improvedinitiative' in label.replace('_', '').replace(' ', ''):
                bonus += 4
                continue

            for pattern in _INITIATIVE_PATTERNS:
                match = pattern.search(description)
                if match:
                    bonus += int(match.group(1))
                    break

        self._initiative_bonus_cache = bonus
        return bonus