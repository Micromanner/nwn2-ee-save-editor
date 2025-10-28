"""
Inventory router - Complete inventory management endpoints
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
# from fastapi_models.inventory_models import (...) - moved to lazy loading

router = APIRouter()


@router.get("/characters/{character_id}/inventory")
def get_inventory(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get complete inventory data including items and equipment"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import InventorySummaryResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        summary = inventory_manager.get_inventory_summary()
        
        return InventorySummaryResponse(summary=summary)
        
    except Exception as e:
        logger.error(f"Failed to get inventory for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get inventory: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/equipment")
def get_equipment_info(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get information about all equipped items"""
    from fastapi_models.inventory_models import EquipmentInfoResponse
    session = char_session
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



@router.get("/characters/{character_id}/inventory/encumbrance")
def get_encumbrance(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Calculate character's encumbrance"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import EncumbranceResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        encumbrance = inventory_manager.calculate_encumbrance()
        
        return EncumbranceResponse(encumbrance=encumbrance)
        
    except Exception as e:
        logger.error(f"Failed to calculate encumbrance for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to calculate encumbrance: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses")
def get_equipment_bonuses(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get all equipment bonuses"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import EquipmentBonusesResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        bonuses = inventory_manager.get_equipment_bonuses()
        
        return EquipmentBonusesResponse(bonuses=bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get equipment bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipment bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/ac")
def get_ac_bonus(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get AC bonus from equipment"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import ACBonusResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        ac_bonus = inventory_manager.get_ac_bonus()
        
        return ACBonusResponse(ac_bonus=ac_bonus)
        
    except Exception as e:
        logger.error(f"Failed to get AC bonus for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get AC bonus: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/saves")
def get_save_bonuses(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get saving throw bonuses from equipment"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import SaveBonusesResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        save_bonuses = inventory_manager.get_save_bonuses()
        
        return SaveBonusesResponse(save_bonuses=save_bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get save bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get save bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/attributes")
def get_attribute_bonuses(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get attribute bonuses from equipment"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import AttributeBonusesResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        attribute_bonuses = inventory_manager.get_attribute_bonuses()
        
        return AttributeBonusesResponse(attribute_bonuses=attribute_bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get attribute bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get attribute bonuses: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/bonuses/skills")
def get_skill_bonuses(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get skill bonuses from equipment"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import SkillBonusesResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        skill_bonuses = inventory_manager.get_skill_bonuses()
        
        return SkillBonusesResponse(skill_bonuses=skill_bonuses)
        
    except Exception as e:
        logger.error(f"Failed to get skill bonuses for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get skill bonuses: {str(e)}"
        )


@router.post("/characters/{character_id}/inventory/equip")
def equip_item(
    character_id: int,
    request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """Equip an item in a slot"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import EquipItemRequest, EquipItemResponse
        
        session = char_session
        manager = session.character_manager
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


@router.post("/characters/{character_id}/inventory/unequip")
def unequip_item(
    character_id: int,
    request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """Unequip an item from a slot"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import UnequipItemRequest, UnequipItemResponse
        
        session = char_session
        manager = session.character_manager
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


@router.post("/characters/{character_id}/inventory/add")
def add_to_inventory(
    character_id: int,
    request,  # Type removed for lazy loading
    char_session: CharacterSessionDep
):
    """Add an item to inventory"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import AddToInventoryRequest, AddToInventoryResponse
        
        session = char_session
        manager = session.character_manager
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


@router.delete("/characters/{character_id}/inventory/{item_index}")
def remove_from_inventory(
    character_id: int,
    item_index: int,
    char_session: CharacterSessionDep
):
    """Remove an item from inventory by index"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import RemoveFromInventoryResponse
        
        session = char_session
        manager = session.character_manager
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


@router.get("/characters/{character_id}/inventory/weapons")
def get_all_weapons(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get all available weapons"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import AllWeaponsResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        weapons = inventory_manager.get_all_weapons()
        
        return AllWeaponsResponse(weapons=weapons)
        
    except Exception as e:
        logger.error(f"Failed to get all weapons for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all weapons: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/armor")
def get_all_armor(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get all available armor and shields"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import AllArmorResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        armor = inventory_manager.get_all_armor()
        
        return AllArmorResponse(armor=armor)
        
    except Exception as e:
        logger.error(f"Failed to get all armor for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get all armor: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/custom")
def get_custom_items(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get all custom/mod items in character's possession"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import CustomItemsResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        custom_items = inventory_manager.get_custom_items()
        
        return CustomItemsResponse(custom_items=custom_items)
        
    except Exception as e:
        logger.error(f"Failed to get custom items for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get custom items: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/filter/{item_type}")
def filter_items_by_type(
    character_id: int,
    item_type: int,
    char_session: CharacterSessionDep
):
    """Filter base items by type"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import FilterItemsResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        items = inventory_manager.filter_items_by_type(item_type)
        
        return FilterItemsResponse(items=items)
        
    except Exception as e:
        logger.error(f"Failed to filter items by type for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to filter items by type: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/equipment-summary")
def get_equipment_summary_by_slot(
    character_id: int,
    char_session: CharacterSessionDep
):
    """Get detailed summary of equipped items by slot"""
    try:
        # Lazy imports for performance
        from fastapi_models.inventory_models import EquipmentSummaryResponse
        
        session = char_session
        manager = session.character_manager
        inventory_manager = manager.get_manager('inventory')
        equipment_summary = inventory_manager.get_equipment_summary_by_slot()
        
        return EquipmentSummaryResponse(equipment_summary=equipment_summary)
        
    except Exception as e:
        logger.error(f"Failed to get equipment summary for character {character_id}: {e}")
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Failed to get equipment summary: {str(e)}"
        )


@router.get("/characters/{character_id}/inventory/equipped/{slot}")
def get_equipped_item(
    character_id: int,
    slot: str,
    char_session: CharacterSessionDep
):
    """Get item equipped in a specific slot"""
    session = char_session
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