"""
Class Categorization Service - provides data-driven categorization of NWN2 classes
Simplified data-driven approach using existing infrastructure
"""

from typing import Dict, List, Any, Optional
import logging
from dataclasses import dataclass
from enum import Enum
from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility

logger = logging.getLogger(__name__)


class ClassFocus(Enum):
    """Class focus categories based on gameplay role"""
    COMBAT = "combat"
    ARCANE_CASTER = "arcane_caster"
    DIVINE_CASTER = "divine_caster"
    SKILL_SPECIALIST = "skill_specialist"
    HYBRID = "hybrid"
    STEALTH_INFILTRATION = "stealth_infiltration"


class ClassType(Enum):
    """Primary class type categories"""
    BASE = "base"
    PRESTIGE = "prestige"
    NPC = "npc"


@dataclass
class ClassInfo:
    """Class information with frontend compatibility"""
    id: int
    name: str
    label: str
    class_type: ClassType
    focus: ClassFocus
    max_level: int
    hit_die: int
    skill_points: int
    is_spellcaster: bool
    has_arcane: bool
    has_divine: bool
    primary_ability: str
    bab_progression: str
    alignment_restricted: bool
    description: Optional[str] = None
    parsed_description: Optional[Any] = None
    prerequisites: Optional[Dict[str, Any]] = None


class ClassCategorizer:
    """
    Simplified data-driven class categorization service
    Uses existing game data loader infrastructure
    """
    
    def __init__(self, game_data_loader):
        """
        Initialize the categorizer
        
        Args:
            game_data_loader: GameDataLoader instance for accessing class data
        """
        self.game_data_loader = game_data_loader
        self.field_mapper = FieldMappingUtility()
        self._cache = {}
        self._categories_cache = None
    
    def _get_field_value(self, class_data, field_name: str, default=None):
        """Get field value using field mapping utility with fallback"""
        return self.field_mapper.get_field_value(class_data, field_name, default)
    
    def _determine_class_focus(self, class_data) -> ClassFocus:
        """
        Determine class focus using simple data-driven rules
        """
        # Use field mapping utility like other managers
        has_arcane = self._get_field_value(class_data, 'has_arcane', '0') == '1'
        has_divine = self._get_field_value(class_data, 'has_divine', '0') == '1'
        skill_points = int(self._get_field_value(class_data, 'skill_point_base', '2'))
        hit_die = int(self._get_field_value(class_data, 'hit_die', '8'))
        
        # Simple categorization logic
        if has_arcane:
            return ClassFocus.ARCANE_CASTER
        elif has_divine:
            return ClassFocus.DIVINE_CASTER
        elif skill_points >= 6:
            return ClassFocus.SKILL_SPECIALIST
        elif hit_die >= 10:
            return ClassFocus.COMBAT
        else:
            return ClassFocus.HYBRID
    
    def _determine_class_type(self, class_data) -> ClassType:
        """
        Determine if class is base, prestige, or NPC
        """
        # Get both playerclass and max_level for proper classification
        player_class = getattr(class_data, 'playerclass', 1)
        max_level_raw = self._get_field_value(class_data, 'max_level', '0')
        
        try:
            max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
        except (ValueError, TypeError):
            max_level = 0
        
        # True NPC/Creature classes: playerclass=0 AND MaxLevel=0
        # These are creature types like Beast, Dragon, Undead, etc.
        player_class_is_zero = (str(player_class) == '0' or player_class == 0)
        if player_class_is_zero and max_level == 0:
            return ClassType.NPC
        
        # Player classes (including prestige classes with playerclass=0 but MaxLevel > 0)
        # Prestige classes have level limits, base classes don't
        return ClassType.PRESTIGE if max_level > 0 else ClassType.BASE
        
    def get_categorized_classes(self, include_unplayable: bool = False) -> Dict[str, Dict[str, List[ClassInfo]]]:
        """
        Get all classes organized by type and focus
        Simplified version using existing data patterns
        """
        if self._categories_cache is not None:
            return self._categories_cache
            
        logger.info("Categorizing classes from game data")
        
        # Get classes table using existing loader
        classes_table = self.game_data_loader.get_table('classes')
        if not classes_table:
            logger.error("Could not load classes table")
            return {}
        
        # Initialize categories
        categories = {
            ClassType.BASE.value: {},
            ClassType.PRESTIGE.value: {},
            ClassType.NPC.value: {}
        }
        
        for class_type in categories.keys():
            for focus in ClassFocus:
                categories[class_type][focus.value] = []
        
        # Process each class
        processed_count = 0
        for i, class_data in enumerate(classes_table):
            try:
                # Simple class info extraction like ClassManager does
                class_info = self._create_simple_class_info(class_data, i)
                
                if class_info is None:
                    continue
                
                # Simple filtering for junk entries
                if self._is_placeholder_class(class_data):
                    continue
                
                # Skip unplayable classes only if explicitly requested
                # (but NPC classes are now their own category, so we include them by default)
                if not include_unplayable and class_info.class_type == ClassType.NPC:
                    continue
                
                # Add to appropriate category
                categories[class_info.class_type.value][class_info.focus.value].append(class_info)
                processed_count += 1
                
            except Exception as e:
                logger.warning(f"Error categorizing class {i}: {e}")
                continue
        
        logger.info(f"Successfully categorized {processed_count} classes")
        
        # Sort by name
        for class_type in categories.values():
            for focus_classes in class_type.values():
                focus_classes.sort(key=lambda c: c.name.lower())
        
        self._categories_cache = categories
        return categories
    
    def _create_simple_class_info(self, class_data, class_id: int) -> Optional[ClassInfo]:
        """
        Create ClassInfo using simple field mapping like other managers
        """
        try:
            # Get name safely like ClassManager does
            name = getattr(class_data, 'name', None)
            label = getattr(class_data, 'label', None) or f'Class{class_id}'
            
            # Use field mapping utility for all attribute access
            hit_die = int(self._get_field_value(class_data, 'hit_die', '8'))
            skill_points = int(self._get_field_value(class_data, 'skill_point_base', '2'))
            has_arcane = self._get_field_value(class_data, 'has_arcane', '0') == '1'
            has_divine = self._get_field_value(class_data, 'has_divine', '0') == '1'
            primary_ability = self._get_field_value(class_data, 'primary_ability', 'STR')
            bab_progression = self._get_field_value(class_data, 'attack_bonus_table', 'CLS_ATK_2')
            
            # Alignment restrictions
            align_restrict_raw = self._get_field_value(class_data, 'align_restrict', '0x00')
            try:
                if isinstance(align_restrict_raw, str) and align_restrict_raw.startswith('0x'):
                    align_restrict = int(align_restrict_raw, 16)
                else:
                    align_restrict = int(align_restrict_raw) if align_restrict_raw else 0
            except (ValueError, TypeError):
                align_restrict = 0
            
            # Determine type and focus
            class_type = self._determine_class_type(class_data)
            focus = self._determine_class_focus(class_data)
            
            # Max level
            max_level_raw = self._get_field_value(class_data, 'max_level', '0')
            try:
                max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
            except (ValueError, TypeError):
                max_level = 0
            
            # Get description if available
            description = self._get_field_value(class_data, 'description', None)
            if description and isinstance(description, (int, str)) and str(description).isdigit():
                try:
                    # Try to resolve string reference
                    str_ref = int(description)
                    if hasattr(self.game_data_loader, 'get_string'):
                        resolved_desc = self.game_data_loader.get_string(str_ref)
                        if resolved_desc and not resolved_desc.startswith('StrRef:'):
                            description = resolved_desc
                        else:
                            description = None
                    else:
                        description = None
                except (ValueError, TypeError):
                    description = None
            
            # Use label as fallback if name is 0 or invalid
            display_name = str(name or label)
            if str(name) == '0' or name == 0:
                display_name = str(label)
            
            return ClassInfo(
                id=class_id,
                name=display_name,
                label=str(label),
                class_type=class_type,
                focus=focus,
                max_level=max_level,
                hit_die=hit_die,
                skill_points=skill_points,
                is_spellcaster=has_arcane or has_divine,
                has_arcane=has_arcane,
                has_divine=has_divine,
                primary_ability=primary_ability,
                bab_progression=bab_progression,
                alignment_restricted=align_restrict > 0,
                description=description,
                parsed_description=None,  # Keep None for now - can add basic parsing later if needed
                prerequisites=None  # Keep None for now - can add basic prereq lookup later if needed
            )
            
        except Exception as e:
            logger.warning(f"Error creating class info for class {class_id}: {e}")
            return None
    
    def _is_placeholder_class(self, class_data) -> bool:
        """Check if class is a placeholder/padding entry"""
        name = str(getattr(class_data, 'name', '')).lower()
        label = str(getattr(class_data, 'label', '')).lower()
        
        return (name in ['padding', '****', '', 'none'] or 
                label in ['padding', '****', '', 'none'] or
                (name.isdigit() and label == 'padding'))
    
    def get_base_classes_by_focus(self, focus: ClassFocus) -> List[ClassInfo]:
        """Get base classes for a specific focus"""
        categories = self.get_categorized_classes()
        return categories[ClassType.BASE.value].get(focus.value, [])
    
    def get_prestige_classes_by_focus(self, focus: ClassFocus) -> List[ClassInfo]:
        """Get prestige classes for a specific focus"""
        categories = self.get_categorized_classes()
        return categories[ClassType.PRESTIGE.value].get(focus.value, [])
    
    def get_class_info(self, class_id: int) -> Optional[ClassInfo]:
        """Get detailed info for a specific class"""
        if class_id in self._cache:
            return self._cache[class_id]
        
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            return None
        
        try:
            class_info = self._create_simple_class_info(class_data, class_id)
            if class_info:
                self._cache[class_id] = class_info
            return class_info
        except Exception as e:
            logger.error(f"Error extracting info for class {class_id}: {e}")
            return None
    
    def search_classes(self, query: str, class_type: Optional[ClassType] = None) -> List[ClassInfo]:
        """Search classes by name/label"""
        query = query.lower().strip()
        if not query:
            return []
        
        categories = self.get_categorized_classes()
        results = []
        
        # Search in specified type or all types
        types_to_search = [class_type.value] if class_type else [ClassType.BASE.value, ClassType.PRESTIGE.value]
        
        for type_name in types_to_search:
            for focus_classes in categories[type_name].values():
                for class_info in focus_classes:
                    if (query in class_info.name.lower() or 
                        query in class_info.label.lower()):
                        results.append(class_info)
        
        return sorted(results, key=lambda c: c.name.lower())
    
    def get_focus_display_info(self) -> Dict[str, Dict[str, str]]:
        """Get display information for focus categories"""
        return {
            ClassFocus.COMBAT.value: {
                'name': 'Combat',
                'description': 'Warriors and martial specialists'
            },
            ClassFocus.ARCANE_CASTER.value: {
                'name': 'Arcane Caster',
                'description': 'Wizards, sorcerers, and arcane magic users'
            },
            ClassFocus.DIVINE_CASTER.value: {
                'name': 'Divine Caster',
                'description': 'Clerics, druids, and divine magic users'
            },
            ClassFocus.SKILL_SPECIALIST.value: {
                'name': 'Skill Specialist',
                'description': 'Rogues, bards, and skill-focused classes'
            },
            ClassFocus.HYBRID.value: {
                'name': 'Hybrid',
                'description': 'Multi-role classes and unique specialists'
            },
            ClassFocus.STEALTH_INFILTRATION.value: {
                'name': 'Stealth & Infiltration',
                'description': 'Assassins, spies, and shadow specialists'
            }
        }
    
    def clear_cache(self):
        """Clear categorization cache"""
        self._cache.clear()
        self._categories_cache = None
        logger.info("Class categorization cache cleared")