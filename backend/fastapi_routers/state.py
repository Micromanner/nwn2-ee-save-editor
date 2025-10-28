"""
State router - Character state aggregation endpoint
Provides comprehensive character state from all subsystem managers
"""

from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status
from loguru import logger
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep
)
# from fastapi_models.character_models import (...) - moved to lazy loading
router = APIRouter(tags=["state"])


@router.get("/characters/{character_id}/state")
def get_character_state(
    character_id: int,
    manager: CharacterManagerDep
):  # Return type removed for lazy loading
    """
    Get comprehensive character state with all subsystem information
    
    This aggregates data from all character managers:
    - Summary (name, level, race, alignment)
    - Classes and levels
    - Combat stats (AC, BAB, HP, saves)
    - Skills and ranks
    - Feats (including custom)
    - Spells known and prepared
    - Inventory and equipment
    - Ability scores and modifiers
    - Saving throws
    - Custom content detection
    - Campaign/quest info (if available)
    """
    
    try:
        # Use character manager method - no duplicated logic
        state_data = manager.get_character_state()
        
        # Ensure correct character ID is set (override manager default)
        if 'info' in state_data:
            state_data['info']['id'] = character_id
        
        # Lazy import to avoid circular dependencies
        from fastapi_models import CharacterState
        
        # Validate and convert to proper response model
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
    """
    Get basic character summary
    
    Lighter weight than full state - just name, level, race, classes, alignment
    """
    
    try:
        # Use character manager method - no duplicated logic
        summary_data = manager.get_character_summary()
        
        # Ensure we have an ID field for the frontend
        if 'id' not in summary_data:
            summary_data['id'] = character_id
        
        # Lazy import to avoid circular dependencies
        from fastapi_models import CharacterSummary
        
        # Validate and convert to proper response model
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
    """
    Validate character data for corruption or issues
    
    Returns validation status from all managers
    """
    
    try:
        # Use character manager method - no duplicated logic
        validation_result = manager.validate_character()
        
        # Lazy import to avoid circular dependencies
        from fastapi_models import ValidationResult
        
        # Validate and convert to proper response model
        return ValidationResult(**validation_result)
        
    except Exception as e:
        logger.error(f"Failed to validate character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate character: {str(e)}"
        )