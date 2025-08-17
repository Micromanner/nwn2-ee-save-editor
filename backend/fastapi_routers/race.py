"""
FastAPI router for race-related operations.
Handles race changes and subraces.
"""

from fastapi import APIRouter, Depends, HTTPException, status
import logging
from typing import Optional

# from fastapi_models import (...) - moved to lazy loading
from .dependencies import get_character_manager, get_character_session, CharacterManagerDep, CharacterSessionDep

logger = logging.getLogger(__name__)

router = APIRouter(tags=["Race"])



@router.post("/characters/{character_id}/race/change")
def change_character_race(
    character_id: int,
    request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """
    Change character race with all associated effects.
    
    - **race_id**: New race ID from racialtypes.2da
    - **subrace**: Optional subrace string
    - **preserve_feats**: Whether to preserve existing feats (default: True)
    
    Returns cascading changes to attributes, feats, and other systems.
    """
    from fastapi_models import RaceChangeRequest, RaceChangeResponse
    try:
        session = char_session
        manager = session.character_manager
        race_manager = manager.get_manager('race')
        
        # Use race manager method - fix parameter names to match manager signature
        result = race_manager.change_race(
            request.race_id, 
            request.subrace or '', 
            request.preserve_feats
        )
        
        return RaceChangeResponse(
            success=True,
            old_race=result['old_race'],
            new_race=result['new_race'],
            ability_changes=result['ability_changes'],
            size_change=result['size_change'],
            speed_change=result['speed_change'],
            feat_changes=result['feat_changes'],
            cascading_changes=result,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to change race for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to change race: {str(e)}"
        )


@router.get("/characters/{character_id}/race/current")
def get_current_race(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get current race information.
    
    Returns racial properties and summary including:
    - Race ID and name
    - Subrace (if any)
    - Size category
    - Favored class
    - Ability modifiers
    - Racial feats
    """
    try:
        # Lazy imports for performance
        from fastapi_models import CurrentRace
        
        race_manager = manager.get_manager('race')
        
        # Use existing race manager method - get_racial_properties()
        racial_properties = race_manager.get_racial_properties()
        
        return CurrentRace(**racial_properties)
        
    except Exception as e:
        logger.error(f"Failed to get race for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get current race: {str(e)}"
        )


@router.get("/characters/{character_id}/race/{race_id}/validate")
def validate_race_change(
    character_id: int,
    race_id: int,
    manager: CharacterManagerDep
):
    """
    Validate if race change is allowed.
    
    Checks:
    - Race exists in racialtypes.2da
    - Character meets any special requirements
    - Compatibility with current classes
    
    Returns validation status and any errors.
    """
    try:
        # Lazy imports for performance
        from fastapi_models import RaceValidationResponse
        
        race_manager = manager.get_manager('race')
        
        # Use existing race manager validation method
        valid, errors = race_manager.validate_race_change(race_id)
        
        return RaceValidationResponse(
            valid=valid,
            errors=errors
        )
        
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except Exception as e:
        logger.error(f"Failed to validate race change for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate race change: {str(e)}"
        )


@router.get("/characters/{character_id}/race/summary")
def get_race_summary(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get race summary with formatted ability modifier strings.
    
    Returns racial properties with user-friendly formatting like:
    - "STR +2, DEX -1" for ability modifiers
    - Complete racial property details
    """
    try:
        # Lazy imports for performance
        from fastapi_models import RaceSummary
        
        race_manager = manager.get_manager('race')
        
        race_summary = race_manager.get_race_summary()
        
        return RaceSummary(**race_summary)
        
    except Exception as e:
        logger.error(f"Failed to get race summary for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get race summary: {str(e)}"
        )


@router.post("/characters/{character_id}/race/revert")
def revert_to_original_race(
    character_id: int,
    char_session: CharacterSessionDep
):
    """
    Revert character to their original race.
    
    Restores the race that was set when the character was first loaded.
    """
    try:
        # Lazy imports for performance
        from fastapi_models import RaceChangeResponse
        
        session = char_session
        manager = session.character_manager
        race_manager = manager.get_manager('race')
        
        result = race_manager.revert_to_original_race()
        
        return RaceChangeResponse(
            success=True,
            old_race=result['old_race'],
            new_race=result['new_race'],
            ability_changes=result['ability_changes'],
            size_change=result['size_change'],
            speed_change=result['speed_change'],
            feat_changes=result['feat_changes'],
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to revert race for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to revert race: {str(e)}"
        )