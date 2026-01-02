"""
Abilities router - All ability score related endpoints
Handles the six abilities: STR, DEX, CON, INT, WIS, CHA and their modifiers
"""

from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status, Path, Body
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
# from fastapi_models import (...) - moved to lazy loading

router = APIRouter()


@router.get("/characters/{character_id}/abilities")
def get_abilities(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get character abilities and modifiers"""
    # Redirect to the existing abilities state endpoint
    return get_abilities_state(character_id, manager)


@router.get("/characters/{character_id}/abilities/state")
def get_abilities_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current abilities and modifiers for the abilities editor"""
    from fastapi_models import AttributeState
    
    try:
        ability_manager = manager.get_manager('ability')
        
        # Get all data from the ability manager - no duplicated logic
        gff_abilities = ability_manager.get_attributes(include_equipment=False)
        level_up_mods = ability_manager.get_level_up_modifiers()

        # Base attributes = GFF value - level_up bonuses (so user edits the starting value)
        base_abilities = {
            attr: gff_abilities[attr] - level_up_mods.get(attr, 0)
            for attr in gff_abilities
        }
        effective_abilities = ability_manager.get_effective_attributes()


        # Build complete state using manager methods only
        try:
            hit_points_data = ability_manager.get_hit_points()
            logger.info(f"Hit points retrieved: {hit_points_data}")
            # Structure hit_points as expected by frontend
            derived_stats = {
                'hit_points': {
                    'current': hit_points_data['current'],
                    'maximum': hit_points_data['max']  # frontend expects 'maximum', not 'max'
                }
            }
        except Exception as e:
            logger.error(f"Failed to get hit points: {e}")
            derived_stats = {
                'hit_points': {
                    'current': 0,
                    'maximum': 0
                }
            }
        
        # Get combat stats including armor class and initiative
        try:
            combat_manager = manager.get_manager('combat')
            if combat_manager:
                armor_class_data = combat_manager.calculate_armor_class()
                initiative_data = combat_manager.calculate_initiative()
                combat_stats = {
                    'armor_class': armor_class_data,
                    'initiative': initiative_data
                }
                logger.info(f"Combat stats retrieved: AC={armor_class_data}, Initiative={initiative_data}")
            else:
                combat_stats = {}
                logger.warning("Combat manager not available")
        except Exception as e:
            logger.error(f"Failed to get combat stats: {e}")
            combat_stats = {}

        # Get saving throws
        try:
            save_manager = manager.get_manager('save')
            if save_manager:
                saving_throws = save_manager.calculate_saving_throws()
                logger.info(f"Saving throws retrieved: {saving_throws}")
            else:
                saving_throws = {}
                logger.warning("Save manager not available")
        except Exception as e:
            logger.error(f"Failed to get saving throws: {e}")
            saving_throws = {}

        # Log what we're sending to frontend
        logger.info(f"Sending to frontend - base_attributes: {base_abilities}")
        logger.info(f"Sending to frontend - effective_attributes: {effective_abilities}")

        state_data = {
            'base_attributes': base_abilities,
            'effective_attributes': effective_abilities,
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
            'derived_stats': derived_stats,
            'combat_stats': combat_stats,
            'saving_throws': saving_throws,
            'encumbrance_limits': ability_manager.get_encumbrance_limits(),
            'saving_throw_modifiers': ability_manager.get_saving_throw_modifiers(),
            'skill_modifiers': ability_manager.get_skill_modifiers(),
            'attribute_dependencies': ability_manager.get_attribute_dependencies(),
            'biography': {
                'name': ability_manager.get_character_name(),
                'age': ability_manager.get_character_age(),
                'background': ability_manager.get_character_background(),
                'experience_points': ability_manager.get_experience_points()
            },
            'point_summary': ability_manager.get_ability_points_summary()
        }
        
        logger.info(f"About to create AttributeState with keys: {list(state_data.keys())}")
        try:
            result = AttributeState(**state_data)
            logger.info("AttributeState created successfully")
            return result
        except Exception as e:
            logger.error(f"Failed to create AttributeState: {e}")
            logger.error(f"State data hit_points: {state_data.get('hit_points')}")
            raise
        
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
    request_data: Dict[str, Any]
):
    """
    Change character abilities (STR, DEX, CON, INT, WIS, CHA)
    Handles cascading effects like HP, saves, and combat bonuses
    """
    from fastapi_models import AttributeChangeRequest
    session = char_session
    
    # Create AttributeChangeRequest from raw data
    abilities_data = AttributeChangeRequest(**request_data)
    
    try:
        manager = session.character_manager
        ability_manager = manager.get_manager('ability')

        # User sends base values (starting scores), but GFF stores base + level_up_modifiers
        # So we need to add level_up_modifiers back before storing
        level_up_mods = ability_manager.get_level_up_modifiers()
        gff_values = {
            attr: value + level_up_mods.get(attr, 0)
            for attr, value in abilities_data.attributes.items()
        }

        # Use manager's set_all_attributes method - no duplicated logic
        changes = ability_manager.set_all_attributes(gff_values, abilities_data.should_validate)
        
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
            'ability_changes': successful_changes,
            'cascading_effects': cascading_effects,
            'validation_errors': validation_errors,
            'saved': False,  # Changes kept in memory only
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
    request_data: Dict[str, Any] = Body(...)
):
    """Set a specific ability to a value"""
    from fastapi_models import AttributeSetRequest
    session = char_session
    
    # Create AttributeSetRequest from raw data
    ability_data = AttributeSetRequest(**request_data)
    
    try:
        manager = session.character_manager
        ability_manager = manager.get_manager('ability')
        
        # Use manager's ABILITY_MAPPING - no duplicated logic
        gff_field = ability_manager.ABILITY_MAPPING.get(ability_name.lower())
        if not gff_field:
            # Try direct mapping if not in standard names
            if ability_name in ability_manager.ATTRIBUTES:
                gff_field = ability_name
            else:
                raise HTTPException(
                    status_code=status.HTTP_400_BAD_REQUEST,
                    detail=f"Invalid ability: {ability_name}"
                )
        
        # Use manager's set_attribute method - no duplicated logic
        result = ability_manager.set_attribute(gff_field, ability_data.value, ability_data.should_validate)
        
        return {
            'success': True,
            'ability_change': result,
            'saved': False,  # Changes kept in memory only
            'has_unsaved_changes': session.has_unsaved_changes()
        }
        
    except ValueError as e:
        # Validation errors should be 400 Bad Request
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
    """Get detailed breakdown of all ability modifiers"""
    from fastapi_models import AttributeModifiersResponse
    
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