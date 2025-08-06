"""
Save ViewSet - All saving throw related endpoints
Handles fortitude, reflex, will saves and resistances
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class SaveViewSet(BaseCharacterViewSet):
    """
    ViewSet for saving throw operations
    All endpoints are nested under /api/characters/{id}/saves/
    """
    
    @action(detail=False, methods=['get'], url_path='state')
    def saves_state(self, request, character_pk=None):
        """Get current saving throw bonuses for the saves editor"""
        try:
            character, manager = self._get_character_manager(character_pk)
            save_manager = manager.get_manager('save')
            
            state = {
                'save_summary': save_manager.get_save_summary(),
                'fortitude': {
                    'total': save_manager.calculate_fortitude_save(),
                    'base': save_manager._get_base_fortitude_save(),
                    'ability_modifier': save_manager._get_con_modifier(),
                    'misc_bonus': save_manager._get_misc_fortitude_bonus()
                },
                'reflex': {
                    'total': save_manager.calculate_reflex_save(),
                    'base': save_manager._get_base_reflex_save(),
                    'ability_modifier': save_manager._get_dex_modifier(),
                    'misc_bonus': save_manager._get_misc_reflex_bonus()
                },
                'will': {
                    'total': save_manager.calculate_will_save(),
                    'base': save_manager._get_base_will_save(),
                    'ability_modifier': save_manager._get_wis_modifier(),
                    'misc_bonus': save_manager._get_misc_will_bonus()
                }
            }
            
            return Response(state, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "saves_state")
    
    @action(detail=False, methods=['get'], url_path='breakdown')
    def save_breakdown(self, request, character_pk=None):
        """Get detailed breakdown of all saving throws"""
        try:
            character, manager = self._get_character_manager(character_pk)
            save_manager = manager.get_manager('save')
            
            breakdown = {
                'fortitude': save_manager.get_fortitude_breakdown(),
                'reflex': save_manager.get_reflex_breakdown(),
                'will': save_manager.get_will_breakdown()
            }
            
            return Response(breakdown, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "save_breakdown")
    
    @action(detail=False, methods=['post'], url_path='simulate')
    def simulate_save(self, request, character_pk=None):
        """Simulate a saving throw roll"""
        save_type = request.data.get('type')  # 'fortitude', 'reflex', or 'will'
        dc = request.data.get('dc', 15)
        
        if save_type not in ['fortitude', 'reflex', 'will']:
            return Response(
                {'error': 'Invalid save type. Must be fortitude, reflex, or will'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, manager = self._get_character_manager(character_pk)
            save_manager = manager.get_manager('save')
            
            result = save_manager.simulate_saving_throw(save_type, dc)
            
            return Response(result, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "simulate_save")
    
    @action(detail=False, methods=['get'], url_path='resistances')
    def damage_resistances(self, request, character_pk=None):
        """Get damage resistances and immunities"""
        try:
            character, manager = self._get_character_manager(character_pk)
            save_manager = manager.get_manager('save')
            
            resistances = {
                'damage_reduction': save_manager.get_damage_reduction(),
                'energy_resistances': save_manager.get_energy_resistances(),
                'damage_immunities': save_manager.get_damage_immunities(),
                'spell_resistance': save_manager.get_spell_resistance()
            }
            
            return Response(resistances, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "damage_resistances")
    
    @action(detail=False, methods=['post'], url_path='update')
    def update_save_bonuses(self, request, character_pk=None):
        """Update character's miscellaneous saving throw bonuses"""
        save_bonuses = request.data.get('save_bonuses', {})
        
        if not save_bonuses:
            return Response(
                {'error': 'save_bonuses field is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            save_manager = manager.get_manager('save')
            
            # Track all changes
            changes = []
            
            # Apply each save bonus change
            for save_type, new_value in save_bonuses.items():
                if save_type in ['fortitude', 'reflex', 'will']:
                    result = save_manager.set_misc_save_bonus(save_type, new_value)
                    changes.append(result)
            
            return Response({
                'changes': changes,
                'has_unsaved_changes': session.has_unsaved_changes()
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "update_save_bonuses")