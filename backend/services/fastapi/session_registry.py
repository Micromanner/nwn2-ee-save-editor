"""FastAPI Character Session Registry managing long-lived editing sessions."""

import threading
from typing import Dict, Optional, Tuple, TYPE_CHECKING
from loguru import logger

# Type hints only - no runtime imports to avoid heavy loading
if TYPE_CHECKING:
    from character.in_memory_save_manager import InMemoryCharacterSession
    from .character_info import CharacterInfo


_character_sessions: Dict[str, object] = {}
_registry_lock = threading.Lock()

_next_session_id = 1
_id_to_path: Dict[int, str] = {}
_path_to_id: Dict[str, int] = {}


def get_character_session(character_id: str) -> "InMemoryCharacterSession":
    """Get or create a character editing session, persisting across requests."""
    from character.in_memory_save_manager import InMemoryCharacterSession
    
    with _registry_lock:
        
        if character_id in _character_sessions:
            session = _character_sessions[character_id]
            if hasattr(session, 'character_manager') and session.character_manager:  # Verify session is still valid
                logger.debug(f"Reusing existing session for character {character_id}")
                return session
            else:
                # Session is invalid, remove it
                logger.warning(f"Found invalid session for character {character_id}, removing")
                _character_sessions.pop(character_id, None)
        
        # Let InMemoryCharacterSession handle all validation (better error messages)
        try:
            logger.info(f"Creating new character session for character {character_id}")
            session = InMemoryCharacterSession(character_id, auto_load=True)
            
            if not session.character_manager:
                raise ValueError("Failed to load character data into memory")
            
            _character_sessions[character_id] = session
            logger.info(f"Created and registered character session for character {character_id}")
            
            return session
            
        except Exception as e:
            logger.error(f"Failed to create character session: {e}")
            raise ValueError(f"Unable to load character: {str(e)}")


def close_character_session(character_id: str) -> bool:
    """Close and cleanup a character editing session."""
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
    """Check if character has an active session."""
    with _registry_lock:
        return character_id in _character_sessions


def get_active_sessions() -> Dict[str, dict]:
    """Get information about all active sessions."""
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
    """Save changes in a character session to disk."""
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
    """Close all active sessions. Used for testing or shutdown."""
    with _registry_lock:
        session_ids = list(_character_sessions.keys())
        for character_id in session_ids:
            try:
                close_character_session(character_id)
            except Exception as e:
                logger.error(f"Error during cleanup of session {character_id}: {e}")
        
        logger.info(f"Cleaned up {len(session_ids)} character sessions")


def get_session_stats() -> dict:
    """Get statistics about the session registry."""
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
    """Register a file path and get an integer ID for it."""
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
    """Get the file path for a given integer ID."""
    return _id_to_path.get(session_id)


def get_id_from_path(file_path: str) -> Optional[int]:
    """Get the integer ID for a given file path."""
    return _path_to_id.get(file_path)