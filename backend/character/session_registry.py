"""
Global Character Session Registry

Manages long-lived character editing sessions across multiple requests.
Each character gets ONE session that persists until explicitly closed.
"""

import logging
import threading
from typing import Dict, Optional, Tuple
from django.shortcuts import get_object_or_404

from .models import Character
from .in_memory_save_manager import InMemoryCharacterSession

logger = logging.getLogger(__name__)

# Global registry of active character sessions
_character_sessions: Dict[int, InMemoryCharacterSession] = {}
_registry_lock = threading.Lock()


def get_character_session(character_id: int) -> InMemoryCharacterSession:
    """
    Get or create a character editing session.
    
    This creates ONE session per character that persists across multiple requests
    until explicitly closed. All API calls for the same character will reuse
    the same session and managers.
    
    Args:
        character_id: Character database ID
        
    Returns:
        InMemoryCharacterSession instance
        
    Raises:
        Http404: If character not found
        ValueError: If session creation fails
    """
    with _registry_lock:
        # Check if session already exists
        if character_id in _character_sessions:
            session = _character_sessions[character_id]
            if session.character_manager:  # Verify session is still valid
                logger.debug(f"Reusing existing session for character {character_id}")
                return session
            else:
                # Session is invalid, remove it
                logger.warning(f"Found invalid session for character {character_id}, removing")
                _character_sessions.pop(character_id, None)
        
        # Create new session
        character = get_object_or_404(Character, pk=character_id)
        
        if not character.is_savegame:
            raise ValueError("Individual .bic files not yet supported by in-memory system")
        
        try:
            logger.info(f"Creating new character session for character {character_id}")
            session = InMemoryCharacterSession(character.file_path, auto_load=True)
            
            if not session.character_manager:
                raise ValueError("Failed to load character data into memory")
            
            # Store in global registry
            _character_sessions[character_id] = session
            logger.info(f"Created and registered character session for character {character_id}")
            
            return session
            
        except Exception as e:
            logger.error(f"Failed to create character session: {e}", exc_info=True)
            raise ValueError(f"Unable to load character: {str(e)}")


def close_character_session(character_id: int) -> bool:
    """
    Close and cleanup a character editing session.
    
    Args:
        character_id: Character database ID
        
    Returns:
        True if session was closed, False if no session existed
    """
    with _registry_lock:
        session = _character_sessions.pop(character_id, None)
        if session:
            try:
                session.close()
                logger.info(f"Closed character session for character {character_id}")
                return True
            except Exception as e:
                logger.error(f"Error closing session for character {character_id}: {e}")
                return False
        else:
            logger.debug(f"No session to close for character {character_id}")
            return False


def has_active_session(character_id: int) -> bool:
    """
    Check if character has an active session.
    
    Args:
        character_id: Character database ID
        
    Returns:
        True if active session exists
    """
    with _registry_lock:
        return character_id in _character_sessions


def get_active_sessions() -> Dict[int, dict]:
    """
    Get information about all active sessions.
    
    Returns:
        Dict mapping character_id to session info
    """
    with _registry_lock:
        info = {}
        for character_id, session in _character_sessions.items():
            try:
                info[character_id] = {
                    'has_unsaved_changes': session.has_unsaved_changes(),
                    'character_name': session.character_manager.get_character_summary().get('name', 'Unknown') if session.character_manager else 'Unknown'
                }
            except Exception as e:
                logger.warning(f"Error getting info for session {character_id}: {e}")
                info[character_id] = {'error': str(e)}
        return info


def save_character_session(character_id: int, create_backup: bool = True) -> bool:
    """
    Save changes in a character session to disk.
    
    Args:
        character_id: Character database ID
        create_backup: Whether to create backup before saving
        
    Returns:
        True if saved successfully, False otherwise
    """
    with _registry_lock:
        session = _character_sessions.get(character_id)
        if not session:
            logger.error(f"No active session for character {character_id}")
            return False
        
        try:
            return session.save(create_backup=create_backup)
        except Exception as e:
            logger.error(f"Failed to save session for character {character_id}: {e}")
            return False


def cleanup_all_sessions():
    """
    Close all active sessions. Used for testing or shutdown.
    """
    with _registry_lock:
        session_ids = list(_character_sessions.keys())
        for character_id in session_ids:
            try:
                close_character_session(character_id)
            except Exception as e:
                logger.error(f"Error during cleanup of session {character_id}: {e}")
        
        logger.info(f"Cleaned up {len(session_ids)} character sessions")


def get_session_stats() -> dict:
    """
    Get statistics about the session registry.
    
    Returns:
        Dict with session statistics
    """
    with _registry_lock:
        total_sessions = len(_character_sessions)
        sessions_with_changes = 0
        
        for session in _character_sessions.values():
            try:
                if session.has_unsaved_changes():
                    sessions_with_changes += 1
            except:
                pass
        
        return {
            'total_active_sessions': total_sessions,
            'sessions_with_unsaved_changes': sessions_with_changes,
            'character_ids': list(_character_sessions.keys())
        }