"""FastAPI router for saving throw operations."""

from fastapi import APIRouter, Depends, HTTPException, status
from typing import Dict, Any
from loguru import logger

from .dependencies import get_character_manager, get_character_session, CharacterManagerDep, CharacterSessionDep


router = APIRouter(tags=["Saves"])


@router.get("/characters/{character_id}/saves/summary")
def get_save_summary(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get saving throw summary."""
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
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
    """Get detailed saving throw breakdown."""
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
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
    """Get individual save totals."""
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
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
    """Check if a saving throw would succeed."""
    from fastapi_models.save_models import SaveCheckRequest
    
    save_check_data = SaveCheckRequest(**request_data)
    
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        check_result = save_manager.check_save(
            save_check_data.save_type, 
            save_check_data.dc, 
            save_check_data.modifier, 
            save_check_data.take_20
        )
        
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
    """Add temporary save modifier."""
    from fastapi_models.save_models import TemporaryModifierRequest
    
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
    """Remove temporary save modifier."""
    from fastapi_models.save_models import TemporaryModifierRequest
    
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
    """Clear all temporary save modifiers."""
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
    """Set miscellaneous saving throw bonus."""
    from fastapi_models.save_models import MiscSaveBonusRequest
    
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
    """Get racial save bonuses."""
    try:
        save_manager = manager.get_manager('save')
        
        if not save_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Save manager not available"
            )
        
        racial_saves = save_manager.get_racial_saves(race_id)
        
        return racial_saves
        
    except Exception as e:
        logger.error(f"Failed to get racial saves for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get racial saves: {str(e)}"
        )