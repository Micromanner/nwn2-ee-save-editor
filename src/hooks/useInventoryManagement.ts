import { useCallback } from 'react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { inventoryAPI, EquipItemRequest, UnequipItemRequest, UpdateItemRequest } from '@/services/inventoryApi';

const EQUIP_DEPENDENT_SUBSYSTEMS = ['abilityScores', 'combat', 'saves', 'skills'] as const;

export function useInventoryManagement() {
  const { character, invalidateSubsystems, undo } = useCharacterContext();
  const inventoryData = useSubsystem('inventory');

  // The backend mutation has already committed by the time the refresh runs.
  // If the refresh throws, the session and the UI would silently diverge (the
  // change kept but reported as failed), so roll back to the pre-mutation
  // history snapshot instead. Context undo() never throws and re-invalidates
  // every subsystem on success.
  const refreshOrRollback = useCallback(async (refresh: () => Promise<void>) => {
    try {
      await refresh();
    } catch (err) {
      console.error('Post-mutation refresh failed, rolling back:', err);
      await undo();
      throw err;
    }
  }, [undo]);

  const refreshInventory = useCallback(async () => {
    await inventoryData.load({ silent: true });
  }, [inventoryData]);

  const refreshInventoryAndEquipStats = useCallback(async () => {
    await inventoryData.load({ silent: true });
    await invalidateSubsystems([...EQUIP_DEPENDENT_SUBSYSTEMS]);
  }, [inventoryData, invalidateSubsystems]);

  const equipItem = useCallback(async (request: EquipItemRequest) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.equipItem(character.id, request);
    if (response.success) {
      await refreshOrRollback(refreshInventoryAndEquipStats);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventoryAndEquipStats]);

  const unequipItem = useCallback(async (request: UnequipItemRequest) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.unequipItem(character.id, request);
    if (response.success) {
      await refreshOrRollback(refreshInventoryAndEquipStats);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventoryAndEquipStats]);

  const deleteItem = useCallback(async (itemIndex: number) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.deleteItem(character.id, itemIndex);
    if (response.success) {
      await refreshOrRollback(refreshInventory);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventory]);

  const addItemByBaseType = useCallback(async (baseItemId: number, iconTemplateResref?: string) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.addItemByBaseType(character.id, {
      base_item_id: baseItemId,
      icon_template_resref: iconTemplateResref,
    });
    if (response.success) {
      await refreshOrRollback(refreshInventory);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventory]);

  const updateItem = useCallback(async (request: UpdateItemRequest) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.updateItem(character.id, request);
    if (response.success) {
      await refreshOrRollback(refreshInventoryAndEquipStats);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventoryAndEquipStats]);

  const addItemFromTemplate = useCallback(async (resref: string) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.addItemFromTemplate(character.id, resref);
    if (response.success) {
      await refreshOrRollback(refreshInventoryAndEquipStats);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventoryAndEquipStats]);

  const transformItem = useCallback(async (target: { itemIndex?: number; slot?: string }, targetResref: string) => {
    if (!character?.id) throw new Error('No character loaded');

    const response = await inventoryAPI.transformItem(character.id, {
      item_index: target.itemIndex,
      slot: target.slot,
      target_resref: targetResref,
    });
    if (response.success) {
      await refreshOrRollback(refreshInventoryAndEquipStats);
    }
    return response;
  }, [character?.id, refreshOrRollback, refreshInventoryAndEquipStats]);

  return {
    equipItem,
    unequipItem,
    deleteItem,
    addItemByBaseType,
    updateItem,
    addItemFromTemplate,
    transformItem,
  };
}
