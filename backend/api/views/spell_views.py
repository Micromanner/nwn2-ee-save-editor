"""
Spell ViewSet - All spell-related endpoints
Handles spellbooks, memorization, and spell management
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class SpellViewSet(BaseCharacterViewSet):
    """
    ViewSet for spell-related operations
    All endpoints are nested under /api/characters/{id}/spells/
    """
    
    @action(detail=False, methods=['get'], url_path='state')
    def spells_state(self, request, character_pk=None):
        """Get current spells and spellbook state for the spells editor"""
        try:
            character, manager = self._get_character_manager(character_pk)
            spell_manager = manager.get_manager('spell')
            
            # Get spellcasting classes
            spellcasting_classes = []
            for idx, class_info in enumerate(manager.character_data.get('ClassList', [])):
                class_id = class_info.get('Class', -1)
                if spell_manager.is_spellcaster(class_id):
                    spellcasting_classes.append({
                        'index': idx,
                        'class_id': class_id,
                        'class_name': spell_manager.get_class_name(class_id),
                        'caster_level': spell_manager.get_caster_level(idx),
                        'spell_type': 'prepared' if spell_manager.is_prepared_caster(class_id) else 'spontaneous'
                    })
            
            state = {
                'spellcasting_classes': spellcasting_classes,
                'spell_summary': spell_manager.get_spell_summary(),
                'memorized_spells': spell_manager.get_all_memorized_spells() if spellcasting_classes else []
            }
            
            # Add available spells for each level if requested
            if request.query_params.get('include_available') == 'true':
                state['available_by_level'] = {}
                for level in range(10):  # Spells 0-9
                    state['available_by_level'][level] = spell_manager.get_available_spells(level)
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "spells_state")
    
    @action(detail=False, methods=['get'], url_path='available')
    def available_spells(self, request, character_pk=None):
        """Get spells available for learning at a specific level"""
        spell_level = request.query_params.get('level')
        class_id = request.query_params.get('class_id')
        
        if spell_level is None:
            return Response(
                {'error': 'level parameter is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, manager = self._get_character_manager(character_pk)
            spell_manager = manager.get_manager('spell')
            
            # Get available spells using the implemented method
            available = spell_manager.get_available_spells(int(spell_level), int(class_id) if class_id else None)
            
            return Response({
                'spell_level': int(spell_level),
                'class_id': int(class_id) if class_id else None,
                'available_spells': available,
                'total': len(available)
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "available_spells")
    
    @action(detail=False, methods=['get'], url_path='all')
    def all_spells(self, request, character_pk=None):
        """Get all legitimate spells (filtered) for spell browsing"""
        try:
            character, manager = self._get_character_manager(character_pk)
            spell_manager = manager.get_manager('spell')
            
            # Get all spells by level
            all_spells = []
            for level in range(10):  # Levels 0-9
                level_spells = spell_manager.get_available_spells(level)
                for spell in level_spells:
                    spell['spell_level'] = level
                    all_spells.append(spell)
            
            # Remove duplicates (spells available at multiple levels)
            seen_ids = set()
            unique_spells = []
            for spell in all_spells:
                if spell['id'] not in seen_ids:
                    seen_ids.add(spell['id'])
                    unique_spells.append(spell)
            
            return Response({
                'spells': unique_spells,
                'count': len(unique_spells),
                'total_by_level': {str(level): len(spell_manager.get_available_spells(level)) for level in range(10)}
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "all_spells")
    
    @action(detail=False, methods=['post'], url_path='manage')
    def manage_spells(self, request, character_pk=None):
        """
        Add or remove spells from character's spellbook
        """
        action = request.data.get('action')  # 'add' or 'remove'
        spell_id = request.data.get('spell_id')
        class_index = request.data.get('class_index', 0)
        spell_level = request.data.get('spell_level')
        
        if not action or action not in ['add', 'remove']:
            return Response(
                {'error': 'action must be "add" or "remove"'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        if spell_id is None:
            return Response(
                {'error': 'spell_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            spell_manager = manager.get_manager('spell')
            
            # Get class ID from index
            class_list = manager.character_data.get('ClassList', [])
            if class_index >= len(class_list):
                return Response(
                    {'error': f'Invalid class index: {class_index}'},
                    status=status.HTTP_400_BAD_REQUEST
                )
            class_id = class_list[class_index].get('Class', -1)
            
            if action == 'add':
                # Determine spell level if not provided
                if spell_level is None:
                    spell_level = spell_manager.get_spell_level_for_class(spell_id, class_id)
                    if spell_level is None:
                        return Response(
                            {'error': 'Could not determine spell level for this class'},
                            status=status.HTTP_400_BAD_REQUEST
                        )
                
                # Add the spell
                spell_manager.add_known_spell(class_id, spell_level, spell_id)
                message = 'Spell added successfully'
            else:
                # Remove the spell
                if spell_level is None:
                    spell_level = spell_manager.get_spell_level_for_class(spell_id, class_id)
                    if spell_level is None:
                        return Response(
                            {'error': 'Could not determine spell level for this class'},
                            status=status.HTTP_400_BAD_REQUEST
                        )
                
                removed = spell_manager.remove_known_spell(class_id, spell_level, spell_id)
                if not removed:
                    return Response(
                        {'error': 'Spell not found in known spell list'},
                        status=status.HTTP_404_NOT_FOUND
                    )
                message = 'Spell removed successfully'
            
            # Get updated spell list
            spell_summary = spell_manager.get_spell_summary()
            
            return Response({
                'message': message,
                'spell_summary': spell_summary,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "manage_spells")