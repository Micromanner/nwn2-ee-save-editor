"""
Character Session Management ViewSet

Provides explicit lifecycle management for character editing sessions.
"""

from rest_framework import viewsets, status
from rest_framework.decorators import action
from rest_framework.response import Response
from django.shortcuts import get_object_or_404
import logging

from character.models import Character
from character.session_registry import (
    get_character_session, 
    close_character_session,
    has_active_session,
    get_active_sessions,
    save_character_session,
    get_session_stats
)

logger = logging.getLogger(__name__)


class CharacterSessionViewSet(viewsets.ViewSet):
    """
    ViewSet for managing character editing sessions
    All endpoints are nested under /api/characters/{id}/session/
    """
    
    @action(detail=False, methods=['post'], url_path='start')
    def start_session(self, request, character_pk=None):
        """
        Start a character editing session
        
        POST /api/characters/{id}/session/start/
        """
        try:
            # Verify character exists
            character = get_object_or_404(Character, pk=character_pk)
            
            # Check if session already exists
            if has_active_session(character_pk):
                logger.info(f"Session already exists for character {character_pk}")
                return Response({
                    'message': 'Session already active',
                    'character_id': character_pk,
                    'character_name': character.name
                }, status=status.HTTP_200_OK)
            
            # Create new session
            session = get_character_session(character_pk)
            
            # Get character summary for response
            summary = session.get_info() if hasattr(session, 'get_info') else {}
            
            logger.info(f"Started session for character {character_pk}")
            
            return Response({
                'message': 'Session started successfully',
                'character_id': character_pk,
                'character_name': character.name,
                'session_info': summary
            }, status=status.HTTP_201_CREATED)
            
        except Exception as e:
            logger.error(f"Failed to start session for character {character_pk}: {e}")
            return Response({
                'error': f'Failed to start session: {str(e)}'
            }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)
    
    @action(detail=False, methods=['delete'], url_path='stop')
    def stop_session(self, request, character_pk=None):
        """
        Stop a character editing session
        
        DELETE /api/characters/{id}/session/stop/
        """
        try:
            # Check if session exists
            if not has_active_session(character_pk):
                return Response({
                    'message': 'No active session to stop',
                    'character_id': character_pk
                }, status=status.HTTP_404_NOT_FOUND)
            
            # Check for unsaved changes
            check_unsaved = request.query_params.get('check_unsaved', 'true').lower() == 'true'
            if check_unsaved:
                session = get_character_session(character_pk)
                if session.has_unsaved_changes():
                    return Response({
                        'error': 'Session has unsaved changes',
                        'character_id': character_pk,
                        'has_unsaved_changes': True
                    }, status=status.HTTP_409_CONFLICT)
            
            # Close the session
            success = close_character_session(character_pk)
            
            if success:
                logger.info(f"Stopped session for character {character_pk}")
                return Response({
                    'message': 'Session stopped successfully',
                    'character_id': character_pk
                }, status=status.HTTP_200_OK)
            else:
                return Response({
                    'error': 'Failed to stop session',
                    'character_id': character_pk
                }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)
                
        except Exception as e:
            logger.error(f"Failed to stop session for character {character_pk}: {e}")
            return Response({
                'error': f'Failed to stop session: {str(e)}'
            }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)
    
    @action(detail=False, methods=['get'], url_path='status')
    def session_status(self, request, character_pk=None):
        """
        Get status of character editing session
        
        GET /api/characters/{id}/session/status/
        """
        try:
            character = get_object_or_404(Character, pk=character_pk)
            
            if not has_active_session(character_pk):
                return Response({
                    'active': False,
                    'character_id': character_pk,
                    'character_name': character.name
                }, status=status.HTTP_200_OK)
            
            # Get session info
            session = get_character_session(character_pk)
            
            return Response({
                'active': True,
                'character_id': character_pk,
                'character_name': character.name,
                'has_unsaved_changes': session.has_unsaved_changes(),
                'session_info': session.get_info() if hasattr(session, 'get_info') else {}
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            logger.error(f"Failed to get session status for character {character_pk}: {e}")
            return Response({
                'error': f'Failed to get session status: {str(e)}'
            }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)
    
    @action(detail=False, methods=['post'], url_path='save')
    def save_session(self, request, character_pk=None):
        """
        Save changes in character editing session
        
        POST /api/characters/{id}/session/save/
        """
        try:
            if not has_active_session(character_pk):
                return Response({
                    'error': 'No active session to save',
                    'character_id': character_pk
                }, status=status.HTTP_404_NOT_FOUND)
            
            # Get backup preference from request
            create_backup = request.data.get('create_backup', True)
            
            # Save the session
            success = save_character_session(character_pk, create_backup=create_backup)
            
            if success:
                logger.info(f"Saved session for character {character_pk}")
                return Response({
                    'message': 'Session saved successfully',
                    'character_id': character_pk,
                    'backup_created': create_backup
                }, status=status.HTTP_200_OK)
            else:
                return Response({
                    'error': 'Failed to save session',
                    'character_id': character_pk
                }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)
                
        except Exception as e:
            logger.error(f"Failed to save session for character {character_pk}: {e}")
            return Response({
                'error': f'Failed to save session: {str(e)}'
            }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)
    
    @action(detail=False, methods=['get'], url_path='list')
    def list_active_sessions(self, request):
        """
        List all active character sessions
        
        GET /api/characters/session/list/
        """
        try:
            active_sessions = get_active_sessions()
            stats = get_session_stats()
            
            return Response({
                'active_sessions': active_sessions,
                'statistics': stats
            }, status=status.HTTP_200_OK)
            
        except Exception as e:
            logger.error(f"Failed to list active sessions: {e}")
            return Response({
                'error': f'Failed to list sessions: {str(e)}'
            }, status=status.HTTP_500_INTERNAL_SERVER_ERROR)