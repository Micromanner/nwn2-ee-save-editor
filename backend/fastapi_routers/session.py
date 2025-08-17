"""
Session management router - Character session operations
"""

import logging
from datetime import datetime
from typing import List
from fastapi import APIRouter, Depends, HTTPException, status

from fastapi_core.session_registry import (
    get_character_session, 
    close_character_session, 
    get_active_sessions,
    has_active_session,
    get_path_from_id
)
# Removed character_info dependency - use actual session instead
from fastapi_routers.dependencies import check_system_ready
# Lazy import for startup performance - models loaded in background

logger = logging.getLogger(__name__)

router = APIRouter()


def _get_file_path(character_id: int) -> str:
    """Helper function to get file path from character ID - no duplicated logic"""
    file_path = get_path_from_id(character_id)
    if not file_path:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Character with ID {character_id} not found"
        )
    return file_path


def _get_character_name_from_session(session, character_id: int) -> str:
    """Helper function to get character name from existing session - no duplicate session creation"""
    try:
        if session and session.character_manager:
            summary = session.character_manager.get_character_summary()
            first_name = summary.get('first_name', '')
            last_name = summary.get('last_name', '')
            name = f"{first_name} {last_name}".strip()
            return name or f"Character {character_id}"
        return f"Character {character_id}"
    except Exception:
        return f"Character {character_id}"


@router.post("/characters/{character_id}/session/start")
def start_session(
    character_id: int,
    ready_check: None = Depends(check_system_ready)
):
    """Start or get existing character session"""
    try:
        # Lazy imports for performance
        from fastapi_models import SessionInfo, SessionStatus
        
        # Use helper functions - no duplicated logic
        file_path = _get_file_path(character_id)
        
        # Get or create session (session registry handles duplicates)
        session = get_character_session(file_path)
        
        # Get character name from the actual session (no duplicate session creation)
        character_name = _get_character_name_from_session(session, character_id)
        
        session_info = SessionInfo(
            session_id=str(character_id),
            character_id=character_id,
            character_name=character_name,
            character_file=file_path,
            started_at=datetime.now(),  # TODO: Add actual session start time tracking
            last_activity=datetime.now(),  # TODO: Add actual last activity tracking
            has_unsaved_changes=session.has_unsaved_changes(),
            changes_count=0  # TODO: Add actual changes count
        )
        
        return SessionStatus(
            active=True,
            session=session_info,
            total_sessions=1
        )
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to start session for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to start session: {str(e)}"
        )


@router.delete("/characters/{character_id}/session/stop")
def stop_session(character_id: int):
    """Stop character session"""
    try:
        # Use helper function - no duplicated logic
        file_path = _get_file_path(character_id)
        
        if not has_active_session(file_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"No active session for character {character_id}"
            )
        
        close_character_session(file_path)
        
        return {
            "status": "success",
            "message": f"Session for character {character_id} stopped"
        }
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to stop session for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to stop session: {str(e)}"
        )


@router.get("/characters/{character_id}/session/status")
def get_session_status(character_id: int):
    """Get session status for a character"""
    try:
        # Lazy imports for performance
        from fastapi_models import SessionInfo, SessionStatus
        
        # Use helper function - no duplicated logic
        try:
            file_path = _get_file_path(character_id)
        except HTTPException:
            return SessionStatus(active=False)
        
        if not has_active_session(file_path):
            return SessionStatus(active=False)
        
        session = get_character_session(file_path)
        character_name = _get_character_name_from_session(session, character_id)
        
        session_info = SessionInfo(
            session_id=str(character_id),
            character_id=character_id,
            character_name=character_name,
            character_file=file_path,
            started_at=datetime.now(),  # TODO: Add actual session start time tracking
            last_activity=datetime.now(),  # TODO: Add actual last activity tracking
            has_unsaved_changes=session.has_unsaved_changes(),
            changes_count=0  # TODO: Add actual changes count
        )
        
        return SessionStatus(
            active=True,
            session=session_info,
            total_sessions=1
        )
        
    except Exception as e:
        logger.error(f"Failed to get session status for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get session status: {str(e)}"
        )


@router.post("/characters/{character_id}/session/save")
def save_session(character_id: int):
    """Save character session to disk"""
    try:
        # Use helper function - no duplicated logic
        file_path = _get_file_path(character_id)
        
        if not has_active_session(file_path):
            raise HTTPException(
                status_code=status.HTTP_404_NOT_FOUND,
                detail=f"No active session for character {character_id}"
            )
        
        session = get_character_session(file_path)
        
        # Save the session (this should write to GFF files)
        save_result = session.save(create_backup=True)
        
        return {
            "status": "success",
            "message": f"Character {character_id} saved to disk",
            "saved": save_result,
            "has_unsaved_changes": session.has_unsaved_changes()
        }
        
    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to save session for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to save session: {str(e)}"
        )


@router.get("/characters/session/list")
def list_character_sessions():
    """List all active character sessions"""
    try:
        from fastapi_models import ActiveSessionsList, SessionInfo
        active_sessions = get_active_sessions()
        
        # Convert to SessionInfo objects - character name already extracted by get_active_sessions()
        session_infos = []
        for character_id, session_data in active_sessions.items():
            character_name = session_data.get('character_name', f'Character {character_id}')
            
            session_infos.append(SessionInfo(
                session_id=str(character_id),
                character_id=character_id,
                character_name=character_name,
                character_file=character_id,  # This should be the file path
                started_at=datetime.now(),  # TODO: Add actual session start time tracking
                last_activity=datetime.now(),  # TODO: Add actual last activity tracking
                has_unsaved_changes=session_data.get('has_unsaved_changes', False),
                changes_count=0  # TODO: Add actual changes count
            ))
        
        return ActiveSessionsList(
            sessions=session_infos,
            count=len(session_infos)
        )
        
    except Exception as e:
        logger.error(f"Failed to list active sessions: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to list active sessions: {str(e)}"
        )