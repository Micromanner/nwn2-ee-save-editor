"""
Alignment router - Character alignment (Law/Chaos, Good/Evil) endpoints
Handles D&D alignment system operations
"""

import logging
from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session_dep,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models.shared_models import (
    AlignmentResponse,
    AlignmentUpdateRequest,
    AlignmentShiftRequest,
    AlignmentShiftResponse
)

logger = logging.getLogger(__name__)
router = APIRouter(prefix="/api", tags=["alignment"])


@router.get("/characters/{character_id}/alignment/", response_model=AlignmentResponse)
def get_alignment(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
) -> AlignmentResponse:
    """
    Get character alignment
    
    Returns:
    - lawChaos: 0-100 scale (0=Chaotic, 50=Neutral, 100=Lawful)
    - goodEvil: 0-100 scale (0=Evil, 50=Neutral, 100=Good)
    - alignment_string: Human readable alignment (e.g., "Lawful Good", "Chaotic Neutral")
    """
    
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


@router.post("/characters/{character_id}/alignment/", response_model=AlignmentResponse)
def update_alignment(
    character_id: int,
    alignment_data: AlignmentUpdateRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
) -> AlignmentResponse:
    """
    Update character alignment
    
    Args:
        alignment_data: Dict with 'lawChaos' and/or 'goodEvil' values (0-100)
    
    Returns updated alignment with unsaved changes flag
    """
    character_info, session = char_session
    
    try:
        manager = session.character_manager
        state_manager = manager.get_manager('state')
        
        # Use CharacterStateManager - no duplicated logic
        result = state_manager.set_alignment(
            law_chaos=alignment_data.lawChaos,
            good_evil=alignment_data.goodEvil
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


@router.post("/characters/{character_id}/alignment/shift/", response_model=AlignmentShiftResponse)
def shift_alignment(
    character_id: int,
    shift_data: AlignmentShiftRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
) -> AlignmentShiftResponse:
    """
    Shift alignment by a relative amount
    
    Args:
        shift_data: Dict with 'lawChaosShift' and/or 'goodEvilShift' values
                   Positive shifts toward Law/Good, negative toward Chaos/Evil
    
    Returns updated alignment
    """
    character_info, session = char_session
    
    try:
        manager = session.character_manager
        state_manager = manager.get_manager('state')
        
        # Use CharacterStateManager - no duplicated logic
        result = state_manager.shift_alignment(
            law_chaos_shift=shift_data.lawChaosShift,
            good_evil_shift=shift_data.goodEvilShift
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


@router.get("/characters/{character_id}/alignment/history/")
def get_alignment_history(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
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