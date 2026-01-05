"""Abilities router - All ability score related endpoints."""

from typing import Dict, Any, Optional
from fastapi import APIRouter, Depends, HTTPException, status, Path, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models import (
    AttributeState, AttributeChangeRequest, AttributeSetRequest,
    AttributeModifiersResponse
)

router = APIRouter()


@router.get("/characters/{character_id}/abilities")
def get_abilities(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character abilities and modifiers."""
    return get_abilities_state(character_id, manager)


@router.get("/characters/{character_id}/abilities/state")
def get_abilities_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current abilities and modifiers for the abilities editor."""
    try:
        if not hasattr(manager, 'get_abilities_summary'):
             raise RuntimeError("CharacterManager missing get_abilities_summary method")
             
        summary = manager.get_abilities_summary()
        return AttributeState(**summary)
        
    except Exception as e:
        logger.error(f"Failed to get abilities state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get abilities state: {str(e)}"
        )


@router.post("/characters/{character_id}/abilities/update")
def change_abilities(
    character_id: int,
    char_session: CharacterSessionDep,
    request: AttributeChangeRequest = Body(...)
):
    """Change character abilities (STR, DEX, CON, INT, WIS, CHA)."""
    session = char_session
    manager = session.character_manager
    
    try:
        ability_manager = manager.get_manager('ability')
        if not ability_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Ability manager not available"
            )

        changes = ability_manager.update_base_attributes(request.attributes)
        
        validation_errors = []
        cascading_effects = []
        successful_changes = []
        
        for change in changes:
            if change.get('error'):
                validation_errors.append(f"{change['attribute']}: {change['error']}")
            else:
                successful_changes.append(change)
                if 'all_cascading_effects' in change:
                    cascading_effects.extend(change['all_cascading_effects'])
        
        return {
            'success': len(validation_errors) == 0,
            'ability_changes': successful_changes,
            'cascading_effects': cascading_effects,
            'validation_errors': validation_errors,
            'saved': False,
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except Exception as e:
        logger.error(f"Failed to change abilities for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to change abilities: {str(e)}"
        )


@router.post("/characters/{character_id}/abilities/{ability_name}/set")
def set_attribute(
    character_id: int,
    char_session: CharacterSessionDep,
    ability_name: str = Path(..., description="Ability name (str, dex, con, int, wis, cha)"),
    request: AttributeSetRequest = Body(...)
):
    """Set a specific ability to a value."""
    session = char_session
    manager = session.character_manager
    
    try:
        ability_manager = manager.get_manager('ability')
        if not ability_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Ability manager not available"
            )
        
        result = ability_manager.set_attribute_by_name(ability_name, request.value)
        
        return {
            'success': True,
            'ability_change': result,
            'saved': False,
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except ValueError as e:
        logger.warning(f"Validation error setting ability {ability_name} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Validation error: {str(e)}"
        )
    except Exception as e:
        logger.error(f"Failed to set ability {ability_name} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to set ability: {str(e)}"
        )


@router.get("/characters/{character_id}/abilities/modifiers")
def get_modifiers(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed breakdown of all ability modifiers."""
    try:
        ability_manager = manager.get_manager('ability')
        if not ability_manager:
            raise HTTPException(
                status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                detail="Ability manager not available"
            )
        
        modifiers_data = {
            'base_modifiers': ability_manager.get_attribute_modifiers(),
            'racial_modifiers': ability_manager.get_racial_modifiers(),
            'enhancement_modifiers': ability_manager.get_enhancement_modifiers(),
            'item_modifiers': ability_manager.get_item_modifiers(),
            'temporary_modifiers': ability_manager.get_temporary_modifiers(),
            'total_modifiers': ability_manager.get_total_modifiers()
        }
        
        return AttributeModifiersResponse(**modifiers_data)
        
    except Exception as e:
        logger.error(f"Failed to get modifiers for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get modifiers: {str(e)}"
        )