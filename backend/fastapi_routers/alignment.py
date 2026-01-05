"""Alignment router for character Law/Chaos and Good/Evil axes."""

from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
router = APIRouter(tags=["alignment"])


@router.get("/characters/{character_id}/alignment")
def get_alignment(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character alignment values and string representation."""
    from fastapi_models.shared_models import AlignmentResponse
    
    try:
        identity_manager = manager.get_manager('identity')
        alignment_data = identity_manager.get_alignment()
        
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
    alignment_data: Dict[str, Any] = Body(...)
):
    """Update character alignment values."""
    from fastapi_models.shared_models import AlignmentUpdateRequest, AlignmentResponse
    session = char_session
    
    try:
        manager = session.character_manager
        identity_manager = manager.get_manager('identity')

        result = identity_manager.set_alignment(
            law_chaos=alignment_data.get('lawChaos'),
            good_evil=alignment_data.get('goodEvil')
        )
        
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
    shift_data: Dict[str, int] = Body(...)
):
    """Shift alignment by a relative amount."""
    from fastapi_models.shared_models import AlignmentShiftRequest, AlignmentShiftResponse
    session = char_session
    
    try:
        manager = session.character_manager
        identity_manager = manager.get_manager('identity')

        result = identity_manager.shift_alignment(
            law_chaos_shift=shift_data.get('lawChaosShift', 0),
            good_evil_shift=shift_data.get('goodEvilShift', 0)
        )
        
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
    """Get alignment shift history."""
    return {
        'history': [],
        'message': 'Alignment history tracking not yet implemented'
    }