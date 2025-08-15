"""
FastAPI Character Session Registry

Manages long-lived character editing sessions across multiple requests.
Each character gets ONE session that persists until explicitly closed.

This is the Django-free version for standalone FastAPI use.
"""

import logging
import threading
from typing import Dict, Optional, Tuple

from character.in_memory_save_manager import InMemoryCharacterSession
from .character_info import CharacterInfo, get_character_info

logger = logging.getLogger(__name__)

# Global registry of active character sessions
_character_sessions: Dict[str, InMemoryCharacterSession] = {}
_registry_lock = threading.Lock()

# Integer ID management for cleaner URLs
_next_session_id = 1
_id_to_path: Dict[int, str] = {}  # Maps integer IDs to file paths
_path_to_id: Dict[str, int] = {}  # Reverse mapping for lookups


def get_character_session(character_id: str) -> InMemoryCharacterSession:
    """
    Get or create a character editing session.
    
    This creates ONE session per character that persists across multiple requests
    until explicitly closed. All API calls for the same character will reuse
    the same session and managers.
    
    Args:
        character_id: Character ID (file path in standalone mode)
        
    Returns:
        InMemoryCharacterSession instance
        
    Raises:
        HTTPException: If character not found
        ValueError: If session creation fails
    """
    with _registry_lock:
        
        # Check if session already exists
        if character_id in _character_sessions:
            session = _character_sessions[character_id]
            if hasattr(session, 'character_manager') and session.character_manager:  # Verify session is still valid
                logger.debug(f"Reusing existing session for character {character_id}")
                return session
            else:
                # Session is invalid, remove it
                logger.warning(f"Found invalid session for character {character_id}, removing")
                _character_sessions.pop(character_id, None)
        
        # Get character info (this validates the character exists)
        character_info = get_character_info(character_id)
        
        if not character_info.is_savegame:
            raise ValueError("Individual .bic files not yet supported by in-memory system")
        
        try:
            logger.info(f"Creating new character session for character {character_id}")
            session = InMemoryCharacterSession(character_info.file_path, auto_load=True)
            
            if not session.character_manager:
                raise ValueError("Failed to load character data into memory")
            
            # Store in global registry
            _character_sessions[character_id] = session
            logger.info(f"Created and registered character session for character {character_id}")
            
            return session
            
        except Exception as e:
            logger.error(f"Failed to create character session: {e}", exc_info=True)
            raise ValueError(f"Unable to load character: {str(e)}")


def close_character_session(character_id: str) -> bool:
    """
    Close and cleanup a character editing session.
    
    Args:
        character_id: Character ID (file path in standalone mode)
        
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


def has_active_session(character_id: str) -> bool:
    """
    Check if character has an active session.
    
    Args:
        character_id: Character ID (file path in standalone mode)
        
    Returns:
        True if active session exists
    """
    with _registry_lock:
        return character_id in _character_sessions


def get_active_sessions() -> Dict[str, dict]:
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


def save_character_session(character_id: str, create_backup: bool = True) -> bool:
    """
    Save changes in a character session to disk.
    
    Args:
        character_id: Character ID (file path in standalone mode)
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


def register_character_path(file_path: str) -> int:
    """
    Register a file path and get an integer ID for it.
    
    Args:
        file_path: The full file path to the character/save
        
    Returns:
        Integer ID for this character
    """
    global _next_session_id
    
    with _registry_lock:
        # Check if path already has an ID
        if file_path in _path_to_id:
            return _path_to_id[file_path]
        
        # Generate new ID
        session_id = _next_session_id
        _next_session_id += 1
        
        # Store mappings
        _id_to_path[session_id] = file_path
        _path_to_id[file_path] = session_id
        
        logger.info(f"Registered character path {file_path} with ID {session_id}")
        return session_id


def get_path_from_id(session_id: int) -> Optional[str]:
    """
    Get the file path for a given integer ID.
    
    Args:
        session_id: Integer session ID
        
    Returns:
        File path or None if not found
    """
    return _id_to_path.get(session_id)


def get_id_from_path(file_path: str) -> Optional[int]:
    """
    Get the integer ID for a given file path.
    
    Args:
        file_path: Full file path
        
    Returns:
        Integer ID or None if not registered
    """
    return _path_to_id.get(file_path)