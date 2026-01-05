"""FastAPI router for race-related operations."""

from fastapi import APIRouter, Depends, HTTPException, status
from typing import Optional
from loguru import logger

from .dependencies import get_character_manager, get_character_session, CharacterManagerDep, CharacterSessionDep

router = APIRouter(tags=["Race"])



@router.post("/characters/{character_id}/race/change")
def change_character_race(
    character_id: int,
    request,  
    char_session: CharacterSessionDep
):
    """Change character race with all associated effects."""
    from fastapi_models import RaceChangeRequest, RaceChangeResponse
    try:
        session = char_session
        manager = session.character_manager
        race_manager = manager.get_manager('race')
        
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
    """Get current race information."""
    try:
        from fastapi_models import CurrentRace
        
        race_manager = manager.get_manager('race')
        
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
    manager: CharacterManagerDep,
    subrace: Optional[str] = None
):
    """Validate if race change is allowed."""
    try:
        from fastapi_models import RaceValidationResponse
        
        race_manager = manager.get_manager('race')
        
        valid, errors = race_manager.validate_race_change(race_id, subrace or '')
        
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


@router.get("/characters/{character_id}/race/{race_id}/subraces")
def get_available_subraces_for_character(
    character_id: int,
    race_id: int,
    manager: CharacterManagerDep
):
    """Get available subraces for a specific race for this character."""
    try:
        from fastapi_models import AvailableSubracesResponse
        
        race_manager = manager.get_manager('race')
        
        available_subraces = race_manager.get_available_subraces(race_id)
        
        return AvailableSubracesResponse(
            race_id=race_id,
            subraces=available_subraces
        )
        
    except Exception as e:
        logger.error(f"Failed to get available subraces for character {character_id}, race {race_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get available subraces: {str(e)}"
        )


@router.get("/characters/{character_id}/race/subraces/validate")
def validate_subrace(
    character_id: int,
    race_id: int,
    subrace: str,
    manager: CharacterManagerDep
):
    """Validate if a specific subrace is compatible with a race."""
    try:
        from fastapi_models import SubraceValidationResponse
        
        race_manager = manager.get_manager('race')
        
        valid, errors = race_manager.validate_subrace(race_id, subrace)
        
        return SubraceValidationResponse(
            race_id=race_id,
            subrace=subrace,
            valid=valid,
            errors=errors
        )
        
    except Exception as e:
        logger.error(f"Failed to validate subrace for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to validate subrace: {str(e)}"
        )


@router.get("/characters/{character_id}/race/summary")
def get_race_summary(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get race summary with formatted ability modifier strings."""
    try:
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
    """Revert character to their original race."""
    try:
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