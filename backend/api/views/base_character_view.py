"""
Base view class for character-related operations using in-memory save management.

This replaces the complex _get_character_and_manager method that exists in all views
with a simple in-memory approach.
"""

from rest_framework import viewsets, status
from rest_framework.response import Response
from django.shortcuts import get_object_or_404
import logging

from character.models import Character
from character.session_registry import get_character_session
from gamedata.dynamic_loader.singleton import is_loader_ready

logger = logging.getLogger(__name__)


class BaseCharacterViewSet(viewsets.ViewSet):
    """
    Base ViewSet for character operations using global session registry
    
    This uses a global registry to ensure one session per character across
    all requests, eliminating duplicate manager initialization.
    """
    
    def _check_system_ready(self):
        """
        Check if the system is ready to handle requests.
        
        Returns:
            tuple: (is_ready: bool, error_response: Optional[Response])
                   If not ready, returns appropriate error response to send
        """
        if not is_loader_ready():
            logger.info("Request received but DynamicGameDataLoader not ready yet")
            return False, Response(
                {
                    'error': 'System is still initializing, please try again in a few seconds',
                    'retry_after': 5
                },
                status=status.HTTP_503_SERVICE_UNAVAILABLE,
                headers={'Retry-After': '5'}
            )
        return True, None
    
    def _get_character_session(self, character_id):
        """
        Get character and in-memory session using global registry
        
        Args:
            character_id: Character database ID
            
        Returns:
            tuple: (character_model, character_session)
            
        Raises:
            Http404: If character not found
            ValueError: If save files can't be loaded
        """
        # Check if system is ready
        is_ready, error_response = self._check_system_ready()
        if not is_ready:
            raise ValueError("System not ready")
        
        try:
            # Get session from global registry (creates if needed)
            session = get_character_session(character_id)
            
            # Get character model for return tuple
            character = get_object_or_404(Character, pk=character_id)
            
            return character, session
            
        except Exception as e:
            logger.error(f"Failed to get character session: {e}", exc_info=True)
            raise ValueError(f"Unable to load character: {str(e)}")
    
    def _get_character_manager(self, character_id):
        """
        Get character manager for read operations
        
        Args:
            character_id: Character database ID
            
        Returns:
            tuple: (character_model, character_manager)
        """
        character, session = self._get_character_session(character_id)
        return character, session.character_manager
    
    def _handle_character_error(self, character_id, error, operation="operation"):
        """
        Standard error handling for character operations
        
        Args:
            character_id: Character ID for logging
            error: Exception that occurred
            operation: Operation being performed
            
        Returns:
            Response: Error response
        """
        logger.exception(f"Error in {operation} for character {character_id}: {str(error)}")
        
        # Check if system not ready
        if "system not ready" in str(error).lower():
            return Response(
                {
                    'error': 'System is still initializing, please try again in a few seconds',
                    'retry_after': 5
                },
                status=status.HTTP_503_SERVICE_UNAVAILABLE,
                headers={'Retry-After': '5'}
            )
        # Provide helpful error messages based on error type
        elif "not found" in str(error).lower():
            return Response(
                {'error': f'Character {character_id} not found or save files missing'},
                status=status.HTTP_404_NOT_FOUND
            )
        elif "not supported" in str(error):
            return Response(
                {'error': str(error)},
                status=status.HTTP_501_NOT_IMPLEMENTED
            )
        else:
            return Response(
                {'error': f'Failed to {operation}: {str(error)}'},
                status=status.HTTP_500_INTERNAL_SERVER_ERROR
            )