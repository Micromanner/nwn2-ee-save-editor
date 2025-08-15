"""
Inventory router - Complete inventory management endpoints
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
from fastapi_models.inventory_models import (
    EquipItemRequest,
    EquipItemResponse,
    UnequipItemRequest,
    UnequipItemResponse,
    AddToInventoryRequest,
    AddToInventoryResponse,
    RemoveFromInventoryRequest,
    RemoveFromInventoryResponse,
    EquipmentInfoResponse,
    EquipmentBonusesResponse,
    AllWeaponsResponse,
    AllArmorResponse,
    CustomItemsResponse,
    FilterItemsResponse,
    EquipmentSummaryResponse,
    EncumbranceResponse,
    ACBonusResponse,
    SaveBonusesResponse,
    AttributeBonusesResponse,
    SkillBonusesResponse,
    InventorySummaryResponse
)

logger = logging.getLogger(__name__)

router = APIRouter()


@router.get("/characters/{character_id}/inventory/equipment/", response_model=EquipmentInfoResponse)
def get_equipment_info(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get information about all equipped items"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        equipment = inventory_manager.get_equipment_info()
        
        return EquipmentInfoResponse(equipment=equipment)
        
    except Exception as e:
        logger.error(f"Failed to get equipment info for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipment info: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/summary/", response_model=InventorySummaryResponse)
def get_inventory_summary(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get summary of character's inventory"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        summary = inventory_manager.get_inventory_summary()
        
        return InventorySummaryResponse(summary=summary)
        
    except Exception as e:
        logger.error(f"Failed to get inventory summary for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get inventory summary: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/encumbrance/", response_model=EncumbranceResponse)
def get_encumbrance(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Calculate character's encumbrance"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        encumbrance = inventory_manager.calculate_encumbrance()
        
        return EncumbranceResponse(encumbrance=encumbrance)
        
    except Exception as e:
        logger.error(f"Failed to calculate encumbrance for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to calculate encumbrance: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/", response_model=EquipmentBonusesResponse)
def get_equipment_bonuses(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get all equipment bonuses"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        bonuses = inventory_manager.get_equipment_bonuses()
        
        return EquipmentBonusesResponse(bonuses=bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get equipment bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipment bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/ac/", response_model=ACBonusResponse)
def get_ac_bonus(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get AC bonus from equipment"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        ac_bonus = inventory_manager.get_ac_bonus()
        
        return ACBonusResponse(ac_bonus=ac_bonus)
        
    except Exception as e:
        logger.error(f"Failed to get AC bonus for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get AC bonus: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/saves/", response_model=SaveBonusesResponse)
def get_save_bonuses(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get saving throw bonuses from equipment"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        save_bonuses = inventory_manager.get_save_bonuses()
        
        return SaveBonusesResponse(save_bonuses=save_bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get save bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/attributes/", response_model=AttributeBonusesResponse)
def get_attribute_bonuses(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get attribute bonuses from equipment"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        attribute_bonuses = inventory_manager.get_attribute_bonuses()
        
        return AttributeBonusesResponse(attribute_bonuses=attribute_bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get attribute bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get attribute bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/skills/", response_model=SkillBonusesResponse)
def get_skill_bonuses(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get skill bonuses from equipment"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        skill_bonuses = inventory_manager.get_skill_bonuses()
        
        return SkillBonusesResponse(skill_bonuses=skill_bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get skill bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get skill bonuses: {str(e)}"
        )


@router.post("/characters/{character_id}/inventory/equip/", response_model=EquipItemResponse)
def equip_item(
    character_id: int,
    request: EquipItemRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Equip an item in a slot"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        success, warnings = inventory_manager.equip_item(request.item_data, request.slot)
        
        message = f"Item equipped in {request.slot}" if success else "Failed to equip item"
        
        return EquipItemResponse(
            success=success,
            warnings=warnings,
            message=message,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to equip item for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to equip item: {str(e)}"
        )


@router.post("/characters/{character_id}/inventory/unequip/", response_model=UnequipItemResponse)
def unequip_item(
    character_id: int,
    request: UnequipItemRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Unequip an item from a slot"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        item_data = inventory_manager.unequip_item(request.slot)
        
        success = item_data is not None
        message = f"Item unequipped from {request.slot}" if success else f"No item in {request.slot}"
        
        return UnequipItemResponse(
            success=success,
            item_data=item_data,
            message=message,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to unequip item for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to unequip item: {str(e)}"
        )


@router.post("/characters/{character_id}/inventory/add/", response_model=AddToInventoryResponse)
def add_to_inventory(
    character_id: int,
    request: AddToInventoryRequest,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Add an item to inventory"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        success = inventory_manager.add_to_inventory(request.item_data)
        
        message = "Item added to inventory" if success else "Failed to add item to inventory"
        
        return AddToInventoryResponse(
            success=success,
            message=message,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to add item to inventory for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to add item to inventory: {str(e)}"
        )


@router.delete("/characters/{character_id}/inventory/{item_index}/", response_model=RemoveFromInventoryResponse)
def remove_from_inventory(
    character_id: int,
    item_index: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Remove an item from inventory by index"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        item_data = inventory_manager.remove_from_inventory(item_index)
        
        success = item_data is not None
        message = f"Item removed from inventory at index {item_index}" if success else f"No item at index {item_index}"
        
        return RemoveFromInventoryResponse(
            success=success,
            item_data=item_data,
            message=message,
            has_unsaved_changes=session.has_unsaved_changes()
        )
        
    except Exception as e:
        logger.error(f"Failed to remove item from inventory for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to remove item from inventory: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/weapons/", response_model=AllWeaponsResponse)
def get_all_weapons(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get all available weapons"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        weapons = inventory_manager.get_all_weapons()
        
        return AllWeaponsResponse(weapons=weapons)
        
    except Exception as e:
        logger.error(f"Failed to get all weapons for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all weapons: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/armor/", response_model=AllArmorResponse)
def get_all_armor(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get all available armor and shields"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        armor = inventory_manager.get_all_armor()
        
        return AllArmorResponse(armor=armor)
        
    except Exception as e:
        logger.error(f"Failed to get all armor for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all armor: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/custom/", response_model=CustomItemsResponse)
def get_custom_items(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get all custom/mod items in character's possession"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        custom_items = inventory_manager.get_custom_items()
        
        return CustomItemsResponse(custom_items=custom_items)
        
    except Exception as e:
        logger.error(f"Failed to get custom items for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get custom items: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/filter/{item_type}/", response_model=FilterItemsResponse)
def filter_items_by_type(
    character_id: int,
    item_type: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Filter base items by type"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        items = inventory_manager.filter_items_by_type(item_type)
        
        return FilterItemsResponse(items=items)
        
    except Exception as e:
        logger.error(f"Failed to filter items by type for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to filter items by type: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/equipment-summary/", response_model=EquipmentSummaryResponse)
def get_equipment_summary_by_slot(
    character_id: int,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get detailed summary of equipped items by slot"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        equipment_summary = inventory_manager.get_equipment_summary_by_slot()
        
        return EquipmentSummaryResponse(equipment_summary=equipment_summary)
        
    except Exception as e:
        logger.error(f"Failed to get equipment summary for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipment summary: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/equipped/{slot}/")
def get_equipped_item(
    character_id: int,
    slot: str,
    char_session: CharacterSessionDep = Depends(get_character_session_dep)
):
    """Get item equipped in a specific slot"""
    character, session = char_session
    manager = session.character_manager
    
    try:
        inventory_manager = manager.get_manager('inventory')
        item = inventory_manager.get_equipped_item(slot)
        
        return {"item": item}
        
    except Exception as e:
        logger.error(f"Failed to get equipped item for character {character_id}, slot {slot}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipped item: {str(e)}"
        )