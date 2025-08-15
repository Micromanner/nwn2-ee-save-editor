"""
State router - Character state aggregation endpoint
Provides comprehensive character state from all subsystem managers
"""

import logging
from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status
from fastapi_routers.dependencies import (
    get_character_manager,
    CharacterManagerDep
)
from fastapi_models.character_models import (
    CharacterState,
    CharacterSummary,
    ValidationResult
)

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api", tags=["state"])


@router.get("/characters/{character_id}/state/", response_model=CharacterState)
def get_character_state(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
) -> CharacterState:
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
        
        # Validate and convert to proper response model
        return CharacterState(**state_data)
        
    except Exception as e:
        logger.error(f"Failed to get character state for {character_id}: {e}", exc_info=True)
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get character state: {str(e)}"
        )


@router.get("/characters/{character_id}/summary/", response_model=CharacterSummary)
def get_character_summary(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
) -> CharacterSummary:
    """
    Get basic character summary
    
    Lighter weight than full state - just name, level, race, classes, alignment
    """
    
    try:
        # Use character manager method - no duplicated logic
        summary_data = manager.get_character_summary()
        
        # Validate and convert to proper response model
        return CharacterSummary(**summary_data)
        
    except Exception as e:
        logger.error(f"Failed to get character summary for {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get character summary: {str(e)}"
        )


@router.get("/characters/{character_id}/validation/", response_model=ValidationResult)
def validate_character(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
) -> ValidationResult:
    """
    Validate character data for corruption or issues
    
    Returns validation status from all managers
    """
    
    try:
        # Use character manager method - no duplicated logic
        validation_result = manager.validate_character()
        
        # Validate and convert to proper response model
        return ValidationResult(**validation_result)
        
    except Exception as e:
        logger.error(f"Failed to validate character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate character: {str(e)}"
        )