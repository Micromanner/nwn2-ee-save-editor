"""
Abilities router - All ability score related endpoints
Handles the six ability scores: STR, DEX, CON, INT, WIS, CHA and their modifiers
"""

import logging
from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status, Path

from fastapi_routers.dependencies import (
    get_character_manager, 
    get_character_session_dep,
    CharacterManagerDep,
    CharacterSessionDep
)
from fastapi_models import (
    AttributeState,
    AttributeChangeRequest,
    AttributeSetRequest,
    AttributeModifiersResponse
)

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/characters/{character_id}/attributes/state/", response_model=AttributeState)
def get_attributes_state(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get current attributes and modifiers for the attributes editor"""
    
    try:
        ability_manager = manager.get_manager('ability')
        
        # Get all data from the ability manager - no duplicated logic
        base_attrs = ability_manager.get_attributes()
        effective_attrs = ability_manager.get_effective_attributes()
        
        # Build complete state using manager methods only
        state_data = {
            'base_attributes': base_attrs,
            'effective_attributes': effective_attrs,
            'attribute_modifiers': ability_manager.get_attribute_modifiers(),
            'detailed_modifiers': {
                'base_modifiers': ability_manager.get_attribute_modifiers(),
                'racial_modifiers': ability_manager.get_racial_modifiers(),
                'item_modifiers': ability_manager.get_item_modifiers(),
                'enhancement_modifiers': ability_manager.get_enhancement_modifiers(),
                'temporary_modifiers': ability_manager.get_temporary_modifiers(),
                'level_up_modifiers': ability_manager.get_level_up_modifiers(),
                'total_modifiers': ability_manager.get_total_modifiers()
            },
            'point_buy_cost': ability_manager.calculate_point_buy_total(),
            'encumbrance_limits': ability_manager.get_encumbrance_limits(),
            'saving_throw_modifiers': ability_manager.get_saving_throw_modifiers(),
            'skill_modifiers': ability_manager.get_skill_modifiers(),
            'attribute_dependencies': ability_manager.get_attribute_dependencies(),
            'biography': {
                'name': ability_manager.get_character_name(),
                'age': ability_manager.get_character_age(),
                'background': ability_manager.get_character_background(),
                'experience_points': ability_manager.get_experience_points()
            }
        }
        
        return AttributeState(**state_data)
        
    except Exception as e:
        logger.error(f"Failed to get attributes state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get attributes state: {str(e)}"
        )


@router.post("/characters/{character_id}/attributes/update/")
def change_attributes(
    character_id: int,
    attributes_data: AttributeChangeRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """
    Change character attributes (STR, DEX, CON, INT, WIS, CHA)
    Handles cascading effects like HP, saves, and combat bonuses
    """
    character, session = char_session
    
    try:
        manager = session.character_manager
        ability_manager = manager.get_manager('ability')
        
        # Use manager's set_all_attributes method - no duplicated logic
        changes = ability_manager.set_all_attributes(attributes_data.attributes, attributes_data.validate)
        
        # Check for any validation errors in changes
        validation_errors = []
        cascading_effects = []
        successful_changes = []
        
        for change in changes:
            if change.get('error'):
                validation_errors.append(f"{change['attribute']}: {change['error']}")
            else:
                successful_changes.append(change)
                # Collect cascading effects
                if 'all_cascading_effects' in change:
                    cascading_effects.extend(change['all_cascading_effects'])
        
        return {
            'success': len(validation_errors) == 0,
            'attribute_changes': successful_changes,
            'cascading_effects': cascading_effects,
            'validation_errors': validation_errors,
            'saved': False,  # Changes kept in memory only
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except Exception as e:
        logger.error(f"Failed to change attributes for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to change attributes: {str(e)}"
        )


@router.post("/characters/{character_id}/attributes/{attribute_name}/set/")
def set_attribute(
    character_id: int,
    attribute_name: str = Path(..., description="Attribute name (str, dex, con, int, wis, cha)"),
    attribute_data: AttributeSetRequest = ...,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Set a specific attribute to a value"""
    character, session = char_session
    
    try:
        manager = session.character_manager
        ability_manager = manager.get_manager('ability')
        
        # Use manager's ABILITY_MAPPING - no duplicated logic
        gff_field = ability_manager.ABILITY_MAPPING.get(attribute_name.lower())
        if not gff_field:
            # Try direct mapping if not in standard names
            if attribute_name in ability_manager.ATTRIBUTES:
                gff_field = attribute_name
            else:
                raise HTTPException(
                    status_code=status.HTTP_400_BAD_REQUEST,
                    detail=f"Invalid attribute: {attribute_name}"
                )
        
        # Use manager's set_attribute method - no duplicated logic
        result = ability_manager.set_attribute(gff_field, attribute_data.value, attribute_data.validate)
        
        return {
            'success': True,
            'attribute_change': result,
            'saved': False,  # Changes kept in memory only
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except ValueError as e:
        # Validation errors should be 400 Bad Request
        logger.warning(f"Validation error setting attribute {attribute_name} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Validation error: {str(e)}"
        )
    except Exception as e:
        logger.error(f"Failed to set attribute {attribute_name} for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to set attribute: {str(e)}"
        )




@router.get("/characters/{character_id}/attributes/modifiers/", response_model=AttributeModifiersResponse)
def get_modifiers(
    character_id: int,
    manager: CharacterManagerDep = Depends(get_character_manager)
):
    """Get detailed breakdown of all attribute modifiers"""
    
    try:
        ability_manager = manager.get_manager('ability')
        
        # Use manager methods only - no duplicated logic
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