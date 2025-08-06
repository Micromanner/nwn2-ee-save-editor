"""
Class ViewSet - All class-related endpoints
Handles class changes, levels, and prestige classes
"""

from rest_framework import viewsets, status
from rest_framework.decorators import action
from rest_framework.response import Response
from django.shortcuts import get_object_or_404
import logging

from character.models import Character
from character.factory import get_or_create_character_manager
from character.service_modules.class_categorizer import ClassCategorizer
from parsers.gff import GFFParser, GFFWriter
from parsers.savegame_handler import SaveGameHandler
from gamedata.middleware import get_character_manager, set_character_manager, clear_character_manager
from gamedata.loader import get_game_data_loader
from .base_character_view import BaseCharacterViewSet
from io import BytesIO
import os

logger = logging.getLogger(__name__)


class ClassViewSet(BaseCharacterViewSet):
    """
    ViewSet for class-related operations
    All endpoints are nested under /api/characters/{id}/classes/
    """
    
    @action(detail=False, methods=['post'], url_path='change')
    def change_class(self, request, character_pk=None):
        """
        Change character's class using the unified CharacterManager
        Returns all cascading changes (feats, spells, skills)
        """
        # Get parameters
        new_class_id = request.data.get('class_id')
        preserve_level = request.data.get('preserve_level', True)
        cheat_mode = request.data.get('cheat_mode', False)
        preview = request.data.get('preview', False)

        if not new_class_id:
            return Response(
                {'error': 'class_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )

        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager

            # Preview mode - don't save changes
            if preview:
                changes = manager.get_manager('class').change_class(
                    new_class_id, preserve_level, cheat_mode
                )

                # Get all cascading changes
                summary = {
                    'preview': True,
                    'class_change': changes,
                    'feat_summary': manager.get_manager('feat').get_feat_summary(),
                    'spell_summary': manager.get_manager('spell').get_spell_summary(),
                    'skill_summary': manager.get_manager('skill').get_skill_summary(),
                    'validation': manager.validate_changes(preview=True)
                }

                return Response(summary, status=status.HTTP_200_OK)

            # Execute the class change
            from django.db import transaction
            with transaction.atomic():
                changes = manager.get_manager('class').change_class(
                    new_class_id, preserve_level, cheat_mode
                )

                # Get all changes from all managers
                all_changes = {
                    'class_change': changes,
                    'feat_changes': manager.get_manager('feat').get_feat_summary(),
                    'spell_changes': manager.get_manager('spell').get_spell_summary(),
                    'skill_changes': manager.get_manager('skill').get_skill_summary(),
                    'equipment_validation': manager.get_manager('inventory').validate_all_equipment(),
                    'custom_content_preserved': len(manager.custom_content),
                    'transaction_summary': manager.export_changes(),
                    'has_unsaved_changes': session.has_unsaved_changes()
                }

                return Response(all_changes, status=status.HTTP_200_OK)

        except Exception as e:
            return self._handle_character_error(character_pk, e, "change_class")

    @action(detail=False, methods=['get'], url_path='categorized')
    def get_categorized_classes(self, request, character_pk=None):
        """
        Get all classes organized by type and focus for UI selection
        Data-driven categorization that works with mods
        
        Query parameters:
        - search: Filter classes by name
        - type: Filter by 'base' or 'prestige'
        - include_unplayable: Include NPC classes (default: false)
        """
        try:
            # Get query parameters
            search_query = request.query_params.get('search', '').strip()
            class_type_filter = request.query_params.get('type', '').lower()
            include_unplayable = request.query_params.get('include_unplayable', 'false').lower() == 'true'
            
            # Get game data loader
            game_data_loader = get_game_data_loader()
            if not game_data_loader:
                return Response(
                    {'error': 'Game data not available'},
                    status=status.HTTP_503_SERVICE_UNAVAILABLE
                )
            
            # Initialize categorizer
            categorizer = ClassCategorizer(game_data_loader)
            
            # Handle search mode
            if search_query:
                from character.services.class_categorizer import ClassType
                search_filter = None
                if class_type_filter == 'base':
                    search_filter = ClassType.BASE
                elif class_type_filter == 'prestige':
                    search_filter = ClassType.PRESTIGE
                
                search_results = categorizer.search_classes(search_query, search_filter)
                
                return Response({
                    'search_results': [self._serialize_class_info(class_info) for class_info in search_results],
                    'query': search_query,
                    'total_results': len(search_results)
                }, status=status.HTTP_200_OK)
            
            # Get full categorized classes
            categories = categorizer.get_categorized_classes(include_unplayable)
            
            # Apply type filter if specified
            if class_type_filter in ['base', 'prestige']:
                filtered_categories = {class_type_filter: categories[class_type_filter]}
            else:
                filtered_categories = categories
            
            # Serialize the data
            serialized_categories = {}
            for class_type, focus_groups in filtered_categories.items():
                serialized_categories[class_type] = {}
                for focus, class_list in focus_groups.items():
                    if class_list:  # Only include non-empty categories
                        serialized_categories[class_type][focus] = [
                            self._serialize_class_info(class_info) for class_info in class_list
                        ]
            
            # Get focus display info
            focus_info = categorizer.get_focus_display_info()
            
            response_data = {
                'categories': serialized_categories,
                'focus_info': focus_info,
                'total_classes': sum(
                    len(class_list) 
                    for focus_groups in filtered_categories.values() 
                    for class_list in focus_groups.values()
                ),
                'include_unplayable': include_unplayable
            }
            
            # Add character context if character_pk provided and character has prerequisites
            if character_pk:
                try:
                    character, manager = self._get_character_manager(character_pk)
                    response_data['character_context'] = self._get_character_class_context(manager, categorizer)
                except Exception as e:
                    logger.warning(f"Could not get character context: {e}")
                    # Continue without character context
            
            return Response(response_data, status=status.HTTP_200_OK)
            
        except Exception as e:
            logger.error(f"Error getting categorized classes: {e}")
            return Response(
                {'error': 'Failed to categorize classes', 'detail': str(e)},
                status=status.HTTP_500_INTERNAL_SERVER_ERROR
            )

    def _serialize_class_info(self, class_info) -> dict:
        """Serialize ClassInfo object for API response"""
        # Serialize parsed description if available
        parsed_desc = None
        if class_info.parsed_description:
            parsed_desc = {
                'title': class_info.parsed_description.title,
                'class_type': class_info.parsed_description.class_type,
                'summary': class_info.parsed_description.summary,
                'restrictions': class_info.parsed_description.restrictions,
                'requirements': class_info.parsed_description.requirements,
                'features': class_info.parsed_description.features,
                'abilities': class_info.parsed_description.abilities,
                'html': class_info.parsed_description.raw_html
            }
        
        return {
            'id': class_info.id,
            'name': class_info.name,
            'label': class_info.label,
            'type': class_info.class_type.value,
            'focus': class_info.focus.value,
            'max_level': class_info.max_level,
            'hit_die': class_info.hit_die,
            'skill_points': class_info.skill_points,
            'is_spellcaster': class_info.is_spellcaster,
            'has_arcane': class_info.has_arcane,
            'has_divine': class_info.has_divine,
            'primary_ability': class_info.primary_ability,
            'bab_progression': class_info.bab_progression,
            'alignment_restricted': class_info.alignment_restricted,
            'description': class_info.description,
            'parsed_description': parsed_desc,
            'prerequisites': class_info.prerequisites
        }

    def _get_character_class_context(self, manager, categorizer) -> dict:
        """Get character-specific class context (current classes, prerequisites, etc.)"""
        context = {}
        
        try:
            # Current classes
            class_manager = manager.get_manager('class')
            class_summary = class_manager.get_class_summary()
            context['current_classes'] = class_summary
            
            # Available prestige classes with requirement checking
            if hasattr(class_manager, 'get_prestige_class_options'):
                prestige_options = class_manager.get_prestige_class_options()
                context['prestige_requirements'] = prestige_options
            
            # Multiclass limitations
            context['can_multiclass'] = class_summary.get('can_multiclass', True)
            context['multiclass_slots_used'] = len(class_summary.get('classes', []))
            
        except Exception as e:
            logger.warning(f"Error getting character class context: {e}")
            context['error'] = str(e)
        
        return context

    @action(detail=False, methods=['get'], url_path='categorized-standalone')
    def get_categorized_classes_standalone(self, request):
        """
        Get all classes organized by type and focus (standalone, no character context needed)
        
        Query parameters:
        - search: Filter classes by name
        - type: Filter by 'base' or 'prestige'
        - include_unplayable: Include NPC classes (default: false)
        """
        try:
            # Get query parameters
            search_query = request.query_params.get('search', '').strip()
            class_type_filter = request.query_params.get('type', '').lower()
            include_unplayable = request.query_params.get('include_unplayable', 'false').lower() == 'true'
            
            # Get game data loader
            game_data_loader = get_game_data_loader()
            if not game_data_loader:
                return Response(
                    {'error': 'Game data not available'},
                    status=status.HTTP_503_SERVICE_UNAVAILABLE
                )
            
            # Initialize categorizer
            categorizer = ClassCategorizer(game_data_loader)
            
            # Handle search mode
            if search_query:
                from character.service_modules.class_categorizer import ClassType
                search_filter = None
                if class_type_filter == 'base':
                    search_filter = ClassType.BASE
                elif class_type_filter == 'prestige':
                    search_filter = ClassType.PRESTIGE
                
                search_results = categorizer.search_classes(search_query, search_filter)
                
                return Response({
                    'search_results': [self._serialize_class_info(class_info) for class_info in search_results],
                    'query': search_query,
                    'total_results': len(search_results)
                }, status=status.HTTP_200_OK)
            
            # Get full categorized classes
            categories = categorizer.get_categorized_classes(include_unplayable)
            
            # Apply type filter if specified
            if class_type_filter in ['base', 'prestige']:
                filtered_categories = {class_type_filter: categories[class_type_filter]}
            else:
                filtered_categories = categories
            
            # Serialize the data
            serialized_categories = {}
            for class_type, focus_groups in filtered_categories.items():
                serialized_categories[class_type] = {}
                for focus, class_list in focus_groups.items():
                    if class_list:  # Only include non-empty categories
                        serialized_categories[class_type][focus] = [
                            self._serialize_class_info(class_info) for class_info in class_list
                        ]
            
            # Get focus display info
            focus_info = categorizer.get_focus_display_info()
            
            response_data = {
                'categories': serialized_categories,
                'focus_info': focus_info,
                'total_classes': sum(
                    len(class_list) 
                    for focus_groups in filtered_categories.values() 
                    for class_list in focus_groups.values()
                ),
                'include_unplayable': include_unplayable
            }
            
            return Response(response_data, status=status.HTTP_200_OK)
            
        except Exception as e:
            logger.error(f"Error getting categorized classes: {e}")
            return Response(
                {'error': 'Failed to categorize classes', 'detail': str(e)},
                status=status.HTTP_500_INTERNAL_SERVER_ERROR
            )
    
    @action(detail=False, methods=['get'], url_path='state')
    def classes_state(self, request, character_pk=None):
        """
        Get character's current class information
        Returns class levels, progression, and multiclass status
        """
        try:
            character, manager = self._get_character_manager(character_pk)
            
            # Get class manager
            class_manager = manager.get_manager('class')
            
            # Get class summary
            class_summary = class_manager.get_class_summary()
            
            # Get detailed stats
            attack_bonuses = class_manager.get_attack_bonuses()
            total_saves = class_manager.calculate_total_saves()
            
            response_data = {
                **class_summary,
                'combat_stats': {
                    'base_attack_bonus': attack_bonuses['base_attack_bonus'],
                    'melee_attack_bonus': attack_bonuses['melee_attack_bonus'],
                    'ranged_attack_bonus': attack_bonuses['ranged_attack_bonus'],
                    'multiple_attacks': attack_bonuses['multiple_attacks'],
                    'fortitude_save': total_saves['fortitude'],
                    'reflex_save': total_saves['reflex'],
                    'will_save': total_saves['will'],
                    'base_fortitude': total_saves.get('base_fortitude', 0),
                    'base_reflex': total_saves.get('base_reflex', 0),
                    'base_will': total_saves.get('base_will', 0)
                }
            }
            
            return Response(response_data, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "classes_state")

    @action(detail=False, methods=['post'], url_path='level-up')
    def level_up(self, request, character_pk=None):
        """
        Add a level to a specific class (multiclassing or leveling up)
        """
        class_id = request.data.get('class_id')
        cheat_mode = request.data.get('cheat_mode', False)
        preview = request.data.get('preview', False)

        if not class_id:
            return Response(
                {'error': 'class_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )

        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            class_manager = manager.get_manager('class')
            
            if preview:
                # TODO: Generate level up preview
                return Response({
                    'preview': True,
                    'level_change': 1,
                    'stat_changes': {},
                    'features_gained': []
                }, status=status.HTTP_200_OK)
            
            # Execute level up
            changes = class_manager.add_class_level(class_id, cheat_mode)
            changes['has_unsaved_changes'] = session.has_unsaved_changes()
            
            return Response(changes, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "level_up")

    def get_class_features(self, request, character_pk=None, class_id=None):
        """
        Get detailed class features and progression for a specific class
        
        Query parameters:
        - max_level: Maximum level to show progression for (default: class max or 20)
        - include_spells: Include spell progression tables (default: true)
        - include_proficiencies: Include weapon/armor proficiencies (default: true)
        """
        try:
            if not class_id:
                return Response(
                    {'error': 'class_id is required'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            try:
                class_id = int(class_id)
            except ValueError:
                return Response(
                    {'error': 'class_id must be a valid integer'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            
            # Get query parameters
            max_level = int(request.query_params.get('max_level', 20))
            include_spells = request.query_params.get('include_spells', 'true').lower() == 'true'
            include_proficiencies = request.query_params.get('include_proficiencies', 'true').lower() == 'true'
            
            # Get game data loader
            game_data_loader = get_game_data_loader()
            if not game_data_loader:
                return Response(
                    {'error': 'Game data not available'},
                    status=status.HTTP_503_SERVICE_UNAVAILABLE
                )
            
            # Get class data
            class_data = game_data_loader.get_by_id('classes', class_id)
            if not class_data:
                return Response(
                    {'error': f'Class with ID {class_id} not found'},
                    status=status.HTTP_404_NOT_FOUND
                )
            
            # Get class manager if character is provided for personalized data
            class_manager = None
            if character_pk:
                try:
                    character, manager = self._get_character_manager(character_pk)
                    class_manager = manager.get_manager('class')
                except Exception:
                    # Continue without character context if it fails
                    pass
            
            # Build class progression data
            progression_data = self._build_class_progression(
                game_data_loader, class_data, class_id, max_level,
                include_spells, include_proficiencies, class_manager
            )
            
            return Response(progression_data, status=status.HTTP_200_OK)
            
        except Exception as e:
            logger.error(f"Error getting class features for class {class_id}: {e}")
            return Response(
                {'error': 'Failed to get class features', 'detail': str(e)},
                status=status.HTTP_500_INTERNAL_SERVER_ERROR
            )

    def _build_class_progression(self, game_data_loader, class_data, class_id: int, 
                                max_level: int, include_spells: bool, 
                                include_proficiencies: bool, class_manager=None) -> dict:
        """Build comprehensive class progression data"""
        from character.service_modules.class_categorizer import ClassCategorizer
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        
        field_mapper = FieldMappingUtility()
        categorizer = ClassCategorizer(game_data_loader)
        
        # Get basic class info
        class_name = field_mapper.get_field_value(class_data, 'Name', 'Unknown Class')
        hit_die = field_mapper.get_field_value(class_data, 'HitDie', '8')
        skill_points = field_mapper.get_field_value(class_data, 'SkillPointBase', '2')
        
        # Convert string values to integers
        try:
            hit_die = int(hit_die)
            skill_points = int(skill_points)
        except (ValueError, TypeError):
            hit_die = 8
            skill_points = 2
        
        # Get progression tables
        bab_table = field_mapper.get_field_value(class_data, 'AttackBonusTable', '')
        save_table = field_mapper.get_field_value(class_data, 'SavingThrowTable', '')
        
        # Build level-by-level progression
        level_progression = []
        for level in range(1, min(max_level + 1, 21)):  # Cap at 20 for most classes
            level_data = {
                'level': level,
                'hit_points': hit_die,  # Base HD, actual HP depends on CON
                'skill_points': skill_points,
                'base_attack_bonus': self._get_bab_for_level(game_data_loader, bab_table, level),
                'saves': self._get_saves_for_level(game_data_loader, save_table, level),
                'features': self._get_features_for_level(game_data_loader, class_id, level),
                'feats': self._get_feats_for_level(game_data_loader, class_id, level)
            }
            
            # Add spell progression if applicable
            if include_spells and self._is_spellcaster_class(class_data):
                level_data['spells'] = self._get_spell_progression_for_level(
                    game_data_loader, class_data, level
                )
            
            level_progression.append(level_data)
        
        # Build main response
        progression_data = {
            'class_id': class_id,
            'class_name': class_name,
            'basic_info': {
                'hit_die': hit_die,
                'skill_points_per_level': skill_points,
                'bab_progression': bab_table,
                'save_progression': save_table,
                'is_spellcaster': self._is_spellcaster_class(class_data),
                'spell_type': self._get_spell_type(class_data)
            },
            'level_progression': level_progression,
            'max_level_shown': max_level
        }
        
        # Add proficiencies if requested
        if include_proficiencies:
            progression_data['proficiencies'] = self._get_class_proficiencies(
                game_data_loader, class_data, class_id
            )
        
        # Add class description and features
        try:
            # Get class info from categorizer for rich description
            class_info = categorizer._create_class_info(class_data, class_id)
            if class_info and class_info.parsed_description:
                progression_data['description'] = {
                    'summary': class_info.parsed_description.summary,
                    'features': class_info.parsed_description.features,
                    'abilities': class_info.parsed_description.abilities,
                    'restrictions': class_info.parsed_description.restrictions
                }
        except Exception as e:
            logger.warning(f"Could not get class description: {e}")
        
        return progression_data

    def _get_bab_for_level(self, game_data_loader, bab_table: str, level: int) -> int:
        """Get BAB for specific level from progression table"""
        if not bab_table:
            return 0
            
        try:
            table_data = game_data_loader.get_table(bab_table.lower())
            if not table_data or level > len(table_data):
                return 0
            
            # BAB tables have 'BAB' column (note: uppercase)
            bab_value = table_data[level - 1].get('BAB', '0')
            return int(bab_value) if bab_value else 0
        except (ValueError, KeyError, IndexError, AttributeError):
            return 0

    def _get_saves_for_level(self, game_data_loader, save_table: str, level: int) -> dict:
        """Get saving throws for specific level"""
        if not save_table:
            return {'fortitude': 0, 'reflex': 0, 'will': 0}
            
        try:
            table_data = game_data_loader.get_table(save_table.lower())
            if not table_data or level > len(table_data):
                return {'fortitude': 0, 'reflex': 0, 'will': 0}
            
            row_data = table_data[level - 1]
            return {
                'fortitude': int(row_data.get('FortSave', '0') or 0),
                'reflex': int(row_data.get('RefSave', '0') or 0),
                'will': int(row_data.get('WillSave', '0') or 0)
            }
        except (ValueError, KeyError, IndexError, AttributeError):
            return {'fortitude': 0, 'reflex': 0, 'will': 0}

    def _get_features_for_level(self, game_data_loader, class_id: int, level: int) -> list:
        """Get class features gained at specific level using real class data"""
        features = []
        
        try:
            # Get categorized class data
            categorizer = ClassCategorizer(game_data_loader)
            class_data = game_data_loader.get_by_id('classes', class_id)
            if not class_data:
                return features
            
            # Try to get class info with parsed abilities
            categories = categorizer.get_categorized_classes(include_unplayable=True)
            class_info = None
            
            # Find the class in categorized data
            for class_type in ['base', 'prestige']:
                for focus_group in categories.get(class_type, {}).values():
                    for cls in focus_group:
                        if cls.id == class_id:
                            class_info = cls
                            break
                    if class_info:
                        break
                if class_info:
                    break
            
            # Extract level-specific abilities from parsed description
            if class_info and class_info.parsed_description and class_info.parsed_description.abilities:
                for ability_data in class_info.parsed_description.abilities:
                    if int(ability_data.get('level', 0)) == level:
                        ability_text = ability_data.get('ability', '')
                        # Split multiple abilities at level (comma or semicolon separated)
                        ability_names = [name.strip() for name in ability_text.replace(';', ',').split(',')]
                        
                        for ability_name in ability_names:
                            if ability_name:
                                features.append({
                                    'name': ability_name,
                                    'type': self._categorize_ability_type(ability_name),
                                    'description': f'Class ability gained at level {level}'
                                })
        
        except Exception as e:
            logger.warning(f"Could not get features for level {level} of class {class_id}: {e}")
        
        # Fallback to basic features if no specific data found
        if not features:
            if level == 1:
                features.append({
                    'name': 'Class Proficiencies',
                    'type': 'proficiency',
                    'description': 'Gained weapon and armor proficiencies'
                })
            
            # General feat progression
            if level % 3 == 0 and level > 1:
                features.append({
                    'name': 'Class Feature',
                    'type': 'ability',
                    'description': f'Class-specific ability at level {level}'
                })
        
        return features
    
    def _categorize_ability_type(self, ability_name: str) -> str:
        """Categorize ability type based on name"""
        ability_lower = ability_name.lower()
        
        if any(keyword in ability_lower for keyword in ['feat', 'focus', 'specialization']):
            return 'feat'
        elif any(keyword in ability_lower for keyword in ['spell', 'magic', 'arcane', 'divine']):
            return 'spell'
        elif any(keyword in ability_lower for keyword in ['attack', 'damage', 'weapon', 'armor']):
            return 'combat'
        elif any(keyword in ability_lower for keyword in ['skill', 'knowledge', 'craft']):
            return 'skill'
        else:
            return 'ability'

    def _get_feats_for_level(self, game_data_loader, class_id: int, level: int) -> list:
        """Get automatic feats gained at specific level (placeholder)"""
        # TODO: Implement actual feat lookup from class feat tables
        return []

    def _is_spellcaster_class(self, class_data) -> bool:
        """Check if class is a spellcaster"""
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        has_arcane = field_mapper.get_field_value(class_data, 'HasArcane', '0')
        has_divine = field_mapper.get_field_value(class_data, 'HasDivine', '0')
        
        return has_arcane == '1' or has_divine == '1'

    def _get_spell_type(self, class_data) -> str:
        """Get spell type (arcane/divine/none)"""
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        has_arcane = field_mapper.get_field_value(class_data, 'HasArcane', '0')
        has_divine = field_mapper.get_field_value(class_data, 'HasDivine', '0')
        
        if has_arcane == '1':
            return 'arcane'
        elif has_divine == '1':
            return 'divine'
        return 'none'

    def _get_spell_progression_for_level(self, game_data_loader, class_data, level: int) -> dict:
        """Get spell slots for specific level (placeholder)"""
        # TODO: Implement actual spell progression from spell tables
        return {
            'level_0': 0,
            'level_1': 0,
            'level_2': 0,
            'level_3': 0,
            'level_4': 0,
            'level_5': 0,
            'level_6': 0,
            'level_7': 0,
            'level_8': 0,
            'level_9': 0
        }

    def _get_class_proficiencies(self, game_data_loader, class_data, class_id: int) -> dict:
        """Get weapon and armor proficiencies from real class data"""
        proficiencies = {
            'weapons': [],
            'armor': [],
            'shields': False
        }
        
        try:
            # Get categorized class data with parsed descriptions
            categorizer = ClassCategorizer(game_data_loader)
            categories = categorizer.get_categorized_classes(include_unplayable=True)
            class_info = None
            
            # Find the class in categorized data
            for class_type in ['base', 'prestige']:
                for focus_group in categories.get(class_type, {}).values():
                    for cls in focus_group:
                        if cls.id == class_id:
                            class_info = cls
                            break
                    if class_info:
                        break
                if class_info:
                    break
            
            # Extract proficiencies from parsed description
            if class_info and class_info.parsed_description and class_info.parsed_description.features:
                features = class_info.parsed_description.features
                
                # Look for weapon proficiencies
                weapon_prof = features.get('weapon proficiencies', '')
                if weapon_prof:
                    weapon_prof_lower = weapon_prof.lower()
                    if 'all simple and martial' in weapon_prof_lower:
                        proficiencies['weapons'] = ['Simple', 'Martial']
                    elif 'martial' in weapon_prof_lower:
                        proficiencies['weapons'] = ['Simple', 'Martial'] 
                    elif 'simple' in weapon_prof_lower:
                        proficiencies['weapons'] = ['Simple']
                
                # Look for armor proficiencies
                armor_prof = features.get('armor proficiencies', '')
                if armor_prof:
                    armor_prof_lower = armor_prof.lower()
                    if 'all armor' in armor_prof_lower or 'heavy' in armor_prof_lower:
                        proficiencies['armor'] = ['Light', 'Medium', 'Heavy']
                        proficiencies['shields'] = True
                    elif 'medium' in armor_prof_lower:
                        proficiencies['armor'] = ['Light', 'Medium']
                        proficiencies['shields'] = True
                    elif 'light' in armor_prof_lower:
                        proficiencies['armor'] = ['Light']
                    
                    # Check for shields specifically
                    if 'shield' in armor_prof_lower:
                        proficiencies['shields'] = True
        
        except Exception as e:
            logger.warning(f"Could not get proficiencies for class {class_id}: {e}")
        
        # Fallback based on class focus if no parsed data available
        if not proficiencies['weapons'] and not proficiencies['armor']:
            from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
            field_mapper = FieldMappingUtility()
            
            has_arcane = field_mapper.get_field_value(class_data, 'HasArcane', '0') == '1'
            has_divine = field_mapper.get_field_value(class_data, 'HasDivine', '0') == '1'
            skill_points = int(field_mapper.get_field_value(class_data, 'SkillPointBase', '2'))
            hit_die = int(field_mapper.get_field_value(class_data, 'HitDie', '8'))
            
            if has_arcane:
                proficiencies['weapons'] = ['Simple']
                proficiencies['armor'] = []
            elif has_divine:
                proficiencies['weapons'] = ['Simple']
                proficiencies['armor'] = ['Light', 'Medium']
                proficiencies['shields'] = True
            elif hit_die >= 10:
                proficiencies['weapons'] = ['Simple', 'Martial']
                proficiencies['armor'] = ['Light', 'Medium', 'Heavy']
                proficiencies['shields'] = True
            else:
                proficiencies['weapons'] = ['Simple']
                proficiencies['armor'] = ['Light']
        
        return proficiencies

    # TODO: Add more class-related endpoints
    # - prestige_class_requirements
    # - etc.