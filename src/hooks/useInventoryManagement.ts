import { useCallback } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import { inventoryAPI, EquipItemRequest, UnequipItemRequest, UpdateItemRequest } from '@/services/inventoryApi';

// 'appearance' carries the equipped armor/cloak/helm tints the 3D viewer
// reads; without it, equipping a cloak rebuilds the mesh with a stale tint.
const EQUIP_DEPENDENT_SUBSYSTEMS = ['abilityScores', 'combat', 'saves', 'skills', 'appearance'] as const;

/** An equipped item by slot, or a backpack item by index (inside the bag at containerIndex). */
export interface ItemLocation {
  itemIndex?: number;
  containerIndex?: number | null;
  slot?: string;
}

export function useInventoryManagement() {
  const t = useTranslations();
  const { character, invalidateSubsystems } = useCharacterContext();

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

  // invalidateSubsystems also refreshes the undo/redo state for the new history entry.
  const refreshInventory = useCallback(async () => {
    await invalidateSubsystems(['inventory']);
  }, [invalidateSubsystems]);

  const refreshInventoryAndEquipStats = useCallback(async () => {
    await invalidateSubsystems(['inventory', ...EQUIP_DEPENDENT_SUBSYSTEMS]);
  }, [invalidateSubsystems]);

  const equipItem = useCallback(async (request: EquipItemRequest) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await inventoryAPI.equipItem(request);
    await refreshAfterMutation(refreshInventoryAndEquipStats);
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const unequipItem = useCallback(async (request: UnequipItemRequest) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await inventoryAPI.unequipItem(request);
    await refreshAfterMutation(refreshInventoryAndEquipStats);
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const destroyEquippedItem = useCallback(async (slot: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await inventoryAPI.destroyEquippedItem(slot);
    await refreshAfterMutation(refreshInventoryAndEquipStats);
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const deleteItem = useCallback(async (itemIndex: number, containerIndex: number | null) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await inventoryAPI.deleteItem(itemIndex, containerIndex);
    await refreshAfterMutation(refreshInventory);
  }, [t, character?.id, refreshAfterMutation, refreshInventory]);

  const addItemByBaseType = useCallback(async (baseItemId: number, iconTemplateResref?: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.addItemByBaseType({
      base_item_id: baseItemId,
      icon_template_resref: iconTemplateResref,
    });
    await refreshAfterMutation(refreshInventory);
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventory]);

  const updateItem = useCallback(async (request: UpdateItemRequest) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await inventoryAPI.updateItem(request);
    await refreshAfterMutation(refreshInventoryAndEquipStats);
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const addItemFromTemplate = useCallback(async (resref: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await inventoryAPI.addItemFromTemplate(resref);
    await refreshAfterMutation(refreshInventoryAndEquipStats);
    return response;
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  const transformItem = useCallback(async (target: ItemLocation, targetResref: string) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await inventoryAPI.transformItem({
      item_index: target.itemIndex ?? null,
      container_index: target.containerIndex ?? null,
      slot: target.slot ?? null,
      target_resref: targetResref,
    });
    await refreshAfterMutation(refreshInventoryAndEquipStats);
  }, [t, character?.id, refreshAfterMutation, refreshInventoryAndEquipStats]);

  return {
    equipItem,
    unequipItem,
    deleteItem,
    destroyEquippedItem,
    addItemByBaseType,
    updateItem,
    addItemFromTemplate,
    transformItem,
  };
}
