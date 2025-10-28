"""
Alignment router - Character alignment (Law/Chaos, Good/Evil) endpoints
Handles D&D alignment system operations
"""

from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
# from fastapi_models.shared_models import (...) - moved to lazy loading
router = APIRouter(tags=["alignment"])


@router.get("/characters/{character_id}/alignment")
def get_alignment(
    character_id: int,
    manager: CharacterManagerDep
):  # Return type removed for lazy loading
    """
    Get character alignment
    
    Returns:
    - lawChaos: 0-100 scale (0=Chaotic, 50=Neutral, 100=Lawful)
    - goodEvil: 0-100 scale (0=Evil, 50=Neutral, 100=Good)
    - alignment_string: Human readable alignment (e.g., "Lawful Good", "Chaotic Neutral")
    """
    from fastapi_models.shared_models import AlignmentResponse
    
    try:
        # Use CharacterStateManager - no duplicated logic
        state_manager = manager.get_manager('state')
        alignment_data = state_manager.get_alignment()
        
        # Validate and convert to proper response model
        return AlignmentResponse(**alignment_data)
        
    except Exception as e:
        logger.error(f"Failed to get alignment for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get alignment: {str(e)}"
        )


@router.post("/characters/{character_id}/alignment")
def update_alignment(
    character_id: int,
    char_session: CharacterSessionDep,
    alignment_data = Body(...)  # Request body parameter
):  # Return type removed for lazy loading
    """
    Update character alignment
    
    Args:
        alignment_data: Dict with 'lawChaos' and/or 'goodEvil' values (0-100)
    
    Returns updated alignment with unsaved changes flag
    """
    from fastapi_models.shared_models import AlignmentUpdateRequest, AlignmentResponse
    session = char_session
    
    try:
        manager = session.character_manager
        state_manager = manager.get_manager('state')
        
        # Use CharacterStateManager - no duplicated logic
        result = state_manager.set_alignment(
            law_chaos=alignment_data.get('lawChaos'),
            good_evil=alignment_data.get('goodEvil')
        )
        
        # Add unsaved changes flag and validate response
        result['has_unsaved_changes'] = session.has_unsaved_changes()
        return AlignmentResponse(**result)
        
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except Exception as e:
        logger.error(f"Failed to update alignment for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update alignment: {str(e)}"
        )


@router.post("/characters/{character_id}/alignment/shift")
def shift_alignment(
    character_id: int,
    char_session: CharacterSessionDep,
    shift_data = Body(...)  # Request body parameter
):  # Return type removed for lazy loading
    """
    Shift alignment by a relative amount
    
    Args:
        shift_data: Dict with 'lawChaosShift' and/or 'goodEvilShift' values
                   Positive shifts toward Law/Good, negative toward Chaos/Evil
    
    Returns updated alignment
    """
    from fastapi_models.shared_models import AlignmentShiftRequest, AlignmentShiftResponse
    session = char_session
    
    try:
        manager = session.character_manager
        state_manager = manager.get_manager('state')
        
        # Use CharacterStateManager - no duplicated logic
        result = state_manager.shift_alignment(
            law_chaos_shift=shift_data.get('lawChaosShift', 0),
            good_evil_shift=shift_data.get('goodEvilShift', 0)
        )
        
        # Add unsaved changes flag and validate response
        result['has_unsaved_changes'] = session.has_unsaved_changes()
        return AlignmentShiftResponse(**result)
        
    except Exception as e:
        logger.error(f"Failed to shift alignment for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to shift alignment: {str(e)}"
        )


@router.get("/characters/{character_id}/alignment/history")
def get_alignment_history(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get alignment shift history (if tracked)
    
    Note: This would need to be implemented with event tracking
    Currently returns empty history
    """
    
    # TODO: Implement alignment history tracking via events
    return {
        'history': [],
        'message': 'Alignment history tracking not yet implemented'
    }