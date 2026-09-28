import { useCallback } from 'react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import { inventoryAPI, EquipItemRequest, UnequipItemRequest, UpdateItemRequest } from '@/services/inventoryApi';

// 'appearance' carries the equipped armor/cloak/helm tints the 3D viewer
// reads; without it, equipping a cloak rebuilds the mesh with a stale tint.
const EQUIP_DEPENDENT_SUBSYSTEMS = ['abilityScores', 'combat', 'saves', 'skills', 'appearance'] as const;

export function useInventoryManagement() {
  const t = useTranslations();
  const { character, invalidateSubsystems } = useCharacterContext();
  const inventoryData = useSubsystem('inventory');

  // The backend mutation has already committed by the time this runs. If the
  // refresh itself throws (e.g. a transient IPC failure), just surface that
  // error to the caller - don't call undo() here. undo() reverts whatever the
  // most recently recorded history entry is, which isn't necessarily this
  // mutation if another one was recorded in the meantime (e.g. two inventory
  // actions fired close together), so it can silently revert an unrelated,
  // successful change instead of the one that actually failed to refresh.
  const refreshAfterMutation = useCallback(async (refresh: () => Promise<void>) => {
    try {
      await refresh();
    } catch (err) {
      console.error('Post-mutation refresh failed:', err);
      throw err;
    }
  }, []);

  const refreshInventory = useCallback(async () => {
    await inventoryData.load({ silent: true });
  }, [inventoryData]);

  const refreshInventoryAndEquipStats = useCallback(async () => {
    await inventoryData.load({ silent: true });
    await invalidateSubsystems([...EQUIP_DEPENDENT_SUBSYSTEMS]);
  }, [inventoryData, invalidateSubsystems]);

  const equipItem = useCallback(async (request: EquipItemRequest) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.equipItem(character.id, request);
    if (response.success) {
      await refreshAfterMutation(refreshInventoryAndEquipStats);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const unequipItem = useCallback(async (request: UnequipItemRequest) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.unequipItem(character.id, request);
    if (response.success) {
      await refreshAfterMutation(refreshInventoryAndEquipStats);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const deleteItem = useCallback(async (itemIndex: number) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.deleteItem(character.id, itemIndex);
    if (response.success) {
      await refreshAfterMutation(refreshInventory);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventory]);

  const addItemByBaseType = useCallback(async (baseItemId: number, iconTemplateResref?: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.addItemByBaseType(character.id, {
      base_item_id: baseItemId,
      icon_template_resref: iconTemplateResref,
    });
    if (response.success) {
      await refreshAfterMutation(refreshInventory);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventory]);

  const updateItem = useCallback(async (request: UpdateItemRequest) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.updateItem(character.id, request);
    if (response.success) {
      await refreshAfterMutation(refreshInventoryAndEquipStats);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const addItemFromTemplate = useCallback(async (resref: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.addItemFromTemplate(character.id, resref);
    if (response.success) {
      await refreshAfterMutation(refreshInventoryAndEquipStats);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const transformItem = useCallback(async (target: { itemIndex?: number; slot?: string }, targetResref: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.transformItem(character.id, {
      item_index: target.itemIndex ?? null,
      slot: target.slot ?? null,
      target_resref: targetResref,
    });
    if (response.success) {
      await refreshAfterMutation(refreshInventoryAndEquipStats);
    }
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

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
