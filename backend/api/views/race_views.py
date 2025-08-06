"""
Race ViewSet - All race-related endpoints
Handles race changes and subraces
"""

from rest_framework import status
from rest_framework.decorators import action
from rest_framework.response import Response
import logging

from .base_character_view import BaseCharacterViewSet

logger = logging.getLogger(__name__)


class RaceViewSet(BaseCharacterViewSet):
    """
    ViewSet for race-related operations
    All endpoints are nested under /api/characters/{id}/race/
    """
    @action(detail=False, methods=['post'], url_path='change')
    def change_race(self, request, character_pk=None):
        """
        Change character race with all associated effects
        """
        new_race_id = request.data.get('race_id')
        new_subrace = request.data.get('subrace', '')
        preserve_feats = request.data.get('preserve_feats', True)
        
        if new_race_id is None:
            return Response(
                {'error': 'race_id is required'},
                status=status.HTTP_400_BAD_REQUEST
            )
        
        try:
            character, session = self._get_character_session(character_pk)
            manager = session.character_manager
            
            # Change race
            race_manager = manager.get_manager('race')
            changes = race_manager.change_race(new_race_id, new_subrace, preserve_feats)
            
            changes['has_unsaved_changes'] = session.has_unsaved_changes()
            return Response(changes, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "change_race")
    
    @action(detail=False, methods=['get'], url_path='current')
    def current_race(self, request, character_pk=None):
        """Get current race information"""
        try:
            character, manager = self._get_character_manager(character_pk)
            race_manager = manager.get_manager('race')
            
            race_properties = race_manager.get_racial_properties()
            race_summary = race_manager.get_race_summary()
            
            return Response({
                'race_properties': race_properties,
                'race_summary': race_summary
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            return self._handle_character_error(character_pk, e, "current_race")
    
    @action(detail=True, methods=['get'], url_path='validate')
    def validate_race_change(self, request, character_pk=None, pk=None):
        """Validate if race change is allowed"""
        try:
            character, manager = self._get_character_manager(character_pk)
            race_manager = manager.get_manager('race')
            
            race_id = int(pk)
            is_valid, errors = race_manager.validate_race_change(race_id)
            
            return Response({
                'race_id': race_id,
                'valid': is_valid,
                'errors': errors
            }, status=status.HTTP_200_OK)
            
        except ValueError:
            return Response(
                {'error': 'Invalid race ID'},
                status=status.HTTP_400_BAD_REQUEST
            )
        except Exception as e:
            return self._handle_character_error(character_pk, e, "validate_race_change")