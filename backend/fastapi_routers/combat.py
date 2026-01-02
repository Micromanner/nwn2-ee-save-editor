"""
Combat router - Combat statistics endpoints
Handles BAB, AC, attack bonuses, damage, and combat statistics
"""

from typing import Dict, Any
from fastapi import APIRouter, Depends, HTTPException, status
from loguru import logger

from fastapi_routers.dependencies import (
    get_character_manager,
    get_character_session,
    CharacterManagerDep,
    CharacterSessionDep
)
# from fastapi_models import (...) - moved to lazy loading

router = APIRouter()


@router.get("/characters/{character_id}/combat/state")
def get_combat_state(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get current combat statistics for the combat editor"""
    from fastapi_models import CombatState
    
    try:
        combat_manager = manager.get_manager('combat')

        # Get data from combat manager methods
        combat_summary = combat_manager.get_combat_summary()
        armor_class = combat_manager.calculate_armor_class()

        # Validate data before proceeding
        if combat_summary is None:
            logger.error("combat_summary is None")
            raise ValueError("Combat summary returned None")
        if armor_class is None:
            logger.error("armor_class is None")
            raise ValueError("Armor class calculation returned None")

        # Extract nested data properly
        bab_info = combat_summary.get('bab_info', {})
        weapons_info = combat_summary.get('weapons', {})
        initiative_info = combat_summary.get('initiative', {})
        
        # Update combat_summary to match CombatSummary model
        if 'initiative' in combat_summary and isinstance(combat_summary['initiative'], dict):
            combat_summary['initiative_breakdown'] = combat_summary['initiative']
            # Keep the initiative int value if it exists in the dict
            if 'total' in combat_summary['initiative']:
                combat_summary['initiative'] = combat_summary['initiative']['total']
        
        return CombatState(
            summary=combat_summary,
            armor_class=armor_class,  # Pass dict directly, model handles it
            base_attack_bonus=bab_info,  # Pass dict directly
            attack_bonuses=combat_summary.get('attack_bonuses', {}),
            damage_bonuses=combat_summary.get('damage_bonuses', {}),
            equipped_weapons=weapons_info,  # Pass dict directly
            defensive_abilities=combat_summary.get('defensive_abilities', {}),
            combat_maneuvers=combat_summary.get('combat_maneuvers', {}),
            initiative=initiative_info  # Pass dict directly
        )
        
    except Exception as e:
        logger.error(f"Failed to get combat state for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get combat state: {str(e)}"
        )


@router.get("/characters/{character_id}/combat/bab")
def get_base_attack_bonus(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed base attack bonus breakdown"""
    from fastapi_models import BaseAttackBonusInfo
    
    try:
        combat_manager = manager.get_manager('combat')
        
        # Get attack bonuses and BAB info from combat summary
        attack_bonuses = combat_manager.get_attack_bonuses()
        combat_summary = combat_manager.get_combat_summary()
        
        # Combine data for the response
        bab_data = {
            'base_attack_bonus': attack_bonuses.get('base_attack_bonus', 0),
            'melee_attack_bonus': attack_bonuses.get('melee_attack_bonus', 0),
            'ranged_attack_bonus': attack_bonuses.get('ranged_attack_bonus', 0),
            'str_modifier': attack_bonuses.get('str_modifier', 0),
            'dex_modifier': attack_bonuses.get('dex_modifier', 0),
            'melee': attack_bonuses.get('melee'),
            'ranged': attack_bonuses.get('ranged'),
            **combat_summary.get('bab_info', {})
        }
        
        return bab_data
        
    except Exception as e:
        logger.error(f"Failed to get base attack bonus for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get base attack bonus: {str(e)}"
        )


@router.get("/characters/{character_id}/combat/ac")
def get_armor_class(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed armor class breakdown"""
    from fastapi_models import ArmorClassBreakdown
    
    try:
        combat_manager = manager.get_manager('combat')
        
        # Get AC data from combat manager
        ac_result = combat_manager.calculate_armor_class()
        
        # Pass dict directly, model will handle the structure
        return ac_result
        
    except Exception as e:
        logger.error(f"Failed to get armor class for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get armor class: {str(e)}"
        )


@router.get("/characters/{character_id}/combat/attacks")
def get_attack_bonuses(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed attack bonus breakdown"""
    
    try:
        combat_manager = manager.get_manager('combat')
        
        # Use combat manager method - no duplicated logic
        attack_bonuses = combat_manager.get_attack_bonuses()
        
        return attack_bonuses
        
    except Exception as e:
        logger.error(f"Failed to get attack bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get attack bonuses: {str(e)}"
        )


@router.post("/characters/{character_id}/combat/update-ac")
def update_natural_armor(
    character_id: int,
    char_session: CharacterSessionDep,
    request_data: Dict[str, Any]
):
    """Update character's natural armor bonus"""
    from fastapi_models import NaturalArmorUpdateRequest, NaturalArmorUpdateResponse
    session = char_session
    
    # Create NaturalArmorUpdateRequest from raw data
    natural_armor_data = NaturalArmorUpdateRequest(**request_data)
    
    try:
        manager = session.character_manager
        combat_manager = manager.get_manager('combat')
        
        # Use combat manager method - no duplicated logic
        old_value = manager.gff.get('NaturalAC', 0)
        result = combat_manager.update_natural_armor(natural_armor_data.natural_ac)
        
        return NaturalArmorUpdateResponse(
            field='NaturalAC',
            old_value=old_value,
            new_value=natural_armor_data.natural_ac,
            new_ac=result.get('new_ac', {}),
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to update natural armor for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update natural armor: {str(e)}"
        )


@router.post("/characters/{character_id}/combat/update-initiative")
def update_initiative_bonus(
    character_id: int,
    char_session: CharacterSessionDep,
    request_data: Dict[str, Any]
):
    """Update character's initiative misc bonus"""
    from fastapi_models import InitiativeBonusUpdateRequest, InitiativeBonusUpdateResponse
    session = char_session
    
    # Create InitiativeBonusUpdateRequest from raw data
    initiative_data = InitiativeBonusUpdateRequest(**request_data)
    
    try:
        manager = session.character_manager
        combat_manager = manager.get_manager('combat')
        
        # Use combat manager method - no duplicated logic
        old_value = manager.gff.get('initbonus', 0)
        result = combat_manager.update_initiative_bonus(initiative_data.initiative_bonus)
        
        return InitiativeBonusUpdateResponse(
            field='initbonus',
            old_value=old_value,
            new_value=initiative_data.initiative_bonus,
            new_initiative=result.get('new_initiative', {}),
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to update initiative bonus for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update initiative bonus: {str(e)}"
        )


@router.get("/characters/{character_id}/combat/damage")
def get_damage_bonuses(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get detailed damage bonus breakdown"""
    
    try:
        combat_manager = manager.get_manager('combat')
        
        # Use combat manager method - no duplicated logic
        damage_bonuses = combat_manager.get_damage_bonuses()
        
        return damage_bonuses
        
    except Exception as e:
        logger.error(f"Failed to get damage bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get damage bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/combat/weapons")
def get_equipped_weapons(
    character_id: int,
    manager: CharacterManagerDep
):
    """Get information about equipped weapons"""
    from fastapi_models import EquippedWeapons
    
    try:
        combat_manager = manager.get_manager('combat')
        
        # Get weapons info from combat manager
        weapons_info = combat_manager.get_equipped_weapons()
        
        # Pass dict directly, model will handle the structure
        return weapons_info
        
    except Exception as e:
        logger.error(f"Failed to get equipped weapons for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipped weapons: {str(e)}"
        )




@router.post("/characters/{character_id}/combat/update-hp")
def update_hit_points(
    character_id: int,
    char_session: CharacterSessionDep,
    request_data: Dict[str, Any]
):
    """Update hit points"""
    from fastapi_models import HitPointsUpdateRequest, HitPointsUpdateResponse
    
    try:
        session = char_session
        manager = session.character_manager
        combat_manager = manager.get_manager('combat')
        
        # Parse request
        hp_data = HitPointsUpdateRequest(**request_data)
        
        # Update HP
        result = combat_manager.update_hit_points(
            current=hp_data.current_hp,
            max_hp=hp_data.max_hp
        )
        
        return HitPointsUpdateResponse(
            success=True,
            current=result['current'],
            max=result['max'],
            temp=result['temp'],
            message="Hit points updated",
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to update hit points for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to update hit points: {str(e)}"
        )
