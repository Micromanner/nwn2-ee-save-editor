"""State router - Aggregates character state from all subsystem managers."""

from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status
from loguru import logger
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep,
    CharacterSessionDep
)
router = APIRouter(tags=["state"])


@router.get("/characters/{character_id}/state")
def get_character_state(
    character_id: int,
    session: CharacterSessionDep
):  
    """Get comprehensive character state with all subsystem information."""
    
    try:
        manager = session.character_manager
        
        state_data = manager.get_character_state()
        
        if 'info' in state_data:
            state_data['info']['id'] = character_id
            
        state_data['has_unsaved_changes'] = session.has_unsaved_changes()
        
        from fastapi_models import CharacterState
        
        return CharacterState(**state_data)
        
    except Exception as e:
        logger.error(f"Failed to get character state for {character_id}: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get character state: {str(e)}"
        )


@router.get("/characters/{character_id}/summary")
def get_character_summary(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get basic character summary."""
    
    try:
        summary_data = manager.get_character_summary()
        
        if 'id' not in summary_data:
            summary_data['id'] = character_id
        
        from fastapi_models import CharacterSummary
        
        return CharacterSummary(**summary_data)
        
    except Exception as e:
        logger.error(f"Failed to get character summary for {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get character summary: {str(e)}"
        )


@router.get("/characters/{character_id}/validation")
def validate_character(
    character_id: int,
    manager: CharacterManagerDep
):
    """Validate character data for corruption or issues."""
    
    try:
        validation_result = manager.validate_character()
        
        from fastapi_models import ValidationResult
        
        return ValidationResult(**validation_result)
        
    except Exception as e:
        logger.error(f"Failed to validate character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate character: {str(e)}"
        )