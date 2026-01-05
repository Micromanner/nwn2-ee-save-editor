"""Lightweight FastAPI dependencies using existing session registry."""
from __future__ import annotations

from typing import Annotated, TYPE_CHECKING
from fastapi import Depends, HTTPException, status
from loguru import logger

if TYPE_CHECKING:
    from character.character_manager import CharacterManager
    from character.in_memory_save_manager import InMemoryCharacterSession


def get_character_manager(character_id: int) -> "CharacterManager":
    """Get character manager using existing session registry (lazy imports)."""
    from gamedata.dynamic_loader.singleton import is_loader_ready
    from services.fastapi.session_registry import get_character_session, get_path_from_id
    from services.fastapi.exceptions import SystemNotReadyException
    
    if not is_loader_ready():
        logger.info(f"Request for character {character_id} but system not ready")
        raise SystemNotReadyException(50)
    
    
    file_path = get_path_from_id(character_id)
    if not file_path:
        logger.warning(f"Character {character_id} not found in session registry")
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Character {character_id} not found"
        )
    
    
    try:
        session = get_character_session(file_path)
        return session.character_manager
    except Exception as e:
        logger.error(f"Failed to get character manager for {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to load character: {str(e)}"
        )


def get_character_session(character_id: int) -> "InMemoryCharacterSession":
    """Get character session using existing session registry (lazy imports)."""
    from gamedata.dynamic_loader.singleton import is_loader_ready
    from services.fastapi.session_registry import get_character_session, get_path_from_id
    from services.fastapi.exceptions import SystemNotReadyException
    
    # System readiness check
    if not is_loader_ready():
        logger.info(f"Request for character {character_id} but system not ready")
        raise SystemNotReadyException(50)
    
    
    file_path = get_path_from_id(character_id)
    if not file_path:
        logger.warning(f"Character {character_id} not found in session registry")
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Character {character_id} not found"
        )
    
    
    try:
        return get_character_session(file_path)
    except Exception as e:
        logger.error(f"Failed to get character session for {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to load character: {str(e)}"
        )


def check_system_ready() -> None:
    """Check if system is ready to handle requests (lazy import)."""
    from gamedata.dynamic_loader.singleton import is_loader_ready
    from services.fastapi.exceptions import SystemNotReadyException
    
    if not is_loader_ready():
        logger.info("Request received but system not ready")
        raise SystemNotReadyException(50)


CharacterManagerDep = Annotated["CharacterManager", Depends(get_character_manager)]
CharacterSessionDep = Annotated["InMemoryCharacterSession", Depends(get_character_session)]
SystemReadyDep = Annotated[None, Depends(check_system_ready)]

get_character_session_dep = get_character_session
get_character_manager_dep = get_character_manager