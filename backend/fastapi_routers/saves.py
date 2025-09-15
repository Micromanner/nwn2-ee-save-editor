"""
FastAPI router for saving throw operations.
Handles fortitude, reflex, will saves and resistances.
"""

from fastapi import APIRouter, Depends, HTTPException, status
import logging
from typing import Dict, Any

from .dependencies import get_character_manager, get_character_session, CharacterManagerDep, CharacterSessionDep
# from fastapi_models import (...) - moved to lazy loading

logger = logging.getLogger(__name__)

router = APIRouter(tags=["Saves"])


@router.get("/characters/{character_id}/saves/summary")
def get_save_summary(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get saving throw summary using existing SaveManager method.
    
    Returns:
    - Fortitude, reflex, will save totals and breakdowns
    - Save conditions and immunities
    """
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        save_summary = save_manager.get_save_summary()
        
        return save_summary
        
    except Exception as e:
        logger.error(f"Failed to get save summary for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save summary: {str(e)}"
        )


@router.get("/characters/{character_id}/saves/breakdown")
def get_save_breakdown(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get detailed saving throw breakdown using SaveManager method.
    
    Returns:
    - Complete breakdown of all saves with components
    """
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        save_breakdown = save_manager.calculate_saving_throws()
        
        return save_breakdown
        
    except Exception as e:
        logger.error(f"Failed to get save breakdown for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save breakdown: {str(e)}"
        )


@router.get("/characters/{character_id}/saves/totals")
def get_save_totals(
    character_id: int,
    manager: CharacterManagerDep
):
    """
    Get individual save totals using SaveManager methods.
    
    Returns:
    - Individual fortitude, reflex, will totals
    """
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use individual SaveManager methods
        totals = {
            'fortitude': save_manager.calculate_fortitude_save(),
            'reflex': save_manager.calculate_reflex_save(),
            'will': save_manager.calculate_will_save()
        }
        
        return totals
        
    except Exception as e:
        logger.error(f"Failed to get save totals for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save totals: {str(e)}"
        )


@router.post("/characters/{character_id}/saves/check")
def check_save(
    character_id: int,
    manager: CharacterManagerDep,
    request_data: Dict[str, Any]
):
    """
    Check if a saving throw would succeed using SaveManager method.
    
    Args:
        request: Save check parameters
    
    Returns:
        Save check result with success probability and details
    """
    from fastapi_models.save_models import SaveCheckRequest
    
    # Create SaveCheckRequest from raw data
    save_check_data = SaveCheckRequest(**request_data)
    
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        check_result = save_manager.check_save(
            save_check_data.save_type, 
            save_check_data.dc, 
            save_check_data.modifier, 
            save_check_data.take_20
        )
        
        # Ensure success_chance is present for non-take_20 checks
        if not save_check_data.take_20 and 'success_chance' not in check_result:
            check_result['success_chance'] = None
        
        return check_result
        
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except Exception as e:
        logger.error(f"Failed to check save for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to check save: {str(e)}"
        )


@router.post("/characters/{character_id}/saves/temporary-modifier")
def add_temporary_modifier(
    character_id: int,
    char_session: CharacterSessionDep,
    request_data: Dict[str, Any]
):
    """
    Add temporary save modifier using SaveManager method.
    
    Args:
        request: Temporary modifier parameters
    """
    from fastapi_models.save_models import TemporaryModifierRequest
    
    # Create TemporaryModifierRequest from raw data
    temp_modifier_data = TemporaryModifierRequest(**request_data)
    
    try:
        session = char_session
        if not session or not session.character_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Character session not available"
            )
        save_manager = session.character_manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        save_manager.add_temporary_modifier(temp_modifier_data.save_type, temp_modifier_data.modifier, temp_modifier_data.duration)
        
        return {
            "success": True,
            "message": f"Added {temp_modifier_data.modifier:+d} temporary {temp_modifier_data.save_type} save modifier",
            "save_type": temp_modifier_data.save_type,
            "modifier": temp_modifier_data.modifier,
            "duration": temp_modifier_data.duration
        }
        
    except Exception as e:
        logger.error(f"Failed to add temporary modifier for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to add temporary modifier: {str(e)}"
        )


@router.delete("/characters/{character_id}/saves/temporary-modifier")
def remove_temporary_modifier(
    character_id: int,
    char_session: CharacterSessionDep,
    request_data: Dict[str, Any]
):
    """
    Remove temporary save modifier using SaveManager method.
    
    Args:
        request: Temporary modifier parameters (duration ignored for removal)
    """
    from fastapi_models.save_models import TemporaryModifierRequest
    
    # Create TemporaryModifierRequest from raw data
    temp_modifier_data = TemporaryModifierRequest(**request_data)
    
    try:
        session = char_session
        if not session or not session.character_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Character session not available"
            )
        save_manager = session.character_manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        save_manager.remove_temporary_modifier(temp_modifier_data.save_type, temp_modifier_data.modifier)
        
        return {
            "success": True,
            "message": f"Removed {temp_modifier_data.modifier:+d} temporary {temp_modifier_data.save_type} save modifier",
            "save_type": temp_modifier_data.save_type,
            "modifier": temp_modifier_data.modifier
        }
        
    except Exception as e:
        logger.error(f"Failed to remove temporary modifier for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to remove temporary modifier: {str(e)}"
        )


@router.delete("/characters/{character_id}/saves/temporary-modifiers")
def clear_temporary_modifiers(
    character_id: int,
    char_session: CharacterSessionDep
):
    """
    Clear all temporary save modifiers using SaveManager method.
    """
    try:
        session = char_session
        if not session or not session.character_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Character session not available"
            )
        save_manager = session.character_manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        save_manager.clear_temporary_modifiers()
        
        return {
            "success": True,
            "message": "Cleared all temporary save modifiers"
        }
        
    except Exception as e:
        logger.error(f"Failed to clear temporary modifiers for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to clear temporary modifiers: {str(e)}"
        )


@router.post("/characters/{character_id}/saves/misc-bonus")
def set_misc_save_bonus(
    character_id: int,
    char_session: CharacterSessionDep,
    request_data: Dict[str, Any]
):
    """
    Set miscellaneous saving throw bonus using SaveManager method.
    
    Args:
        request: Misc save bonus parameters
    """
    from fastapi_models.save_models import MiscSaveBonusRequest
    
    # Create MiscSaveBonusRequest from raw data
    misc_bonus_data = MiscSaveBonusRequest(**request_data)
    
    try:
        session = char_session
        if not session or not session.character_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Character session not available"
            )
        save_manager = session.character_manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        result = save_manager.set_misc_save_bonus(misc_bonus_data.save_type, misc_bonus_data.value)
        
        return result
        
    except ValueError as e:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=str(e)
        )
    except Exception as e:
        logger.error(f"Failed to set misc save bonus for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to set misc save bonus: {str(e)}"
        )


@router.get("/characters/{character_id}/saves/racial/{race_id}")
def get_racial_saves(
    character_id: int,
    race_id: int,
    manager: CharacterManagerDep
):
    """
    Get racial save bonuses using SaveManager method.
    
    Args:
        race_id: The race ID to get saves for
    
    Returns:
        Dict with fortitude, reflex, will save bonuses
    """
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        # Use existing SaveManager method
        racial_saves = save_manager.get_racial_saves(race_id)
        
        return racial_saves
        
    except Exception as e:
        logger.error(f"Failed to get racial saves for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get racial saves: {str(e)}"
        )