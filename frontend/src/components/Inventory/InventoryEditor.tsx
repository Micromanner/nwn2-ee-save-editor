'use client';

import { useState, useEffect, useMemo } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Plus, X } from 'lucide-react';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { inventoryAPI } from '@/services/inventoryApi';
import { useToast } from '@/contexts/ToastContext';
import ItemDetailsPanel from './ItemDetailsPanel';
import InventoryCharacterSummary from './InventoryCharacterSummary';
import InventorySidebarFooter from './InventorySidebarFooter';
import ItemPropertyEditor from './ItemPropertyEditor';
import { InventoryFilters, ItemTypeFilter, ItemSortOption, StatusFilter } from './InventoryFilters';
import { useInventorySearch } from '@/hooks/useInventorySearch';
import { DndContext, DragEndEvent, useSensor, useSensors, PointerSensor, DragStartEvent, DragOverlay, useDraggable, useDroppable } from '@dnd-kit/core';
import AddItemModal from './AddItemModal';
import { BaseItem, ItemTemplate } from '@/services/inventoryApi';
import { safeToNumber } from '@/utils/dataHelpers';
import { getRarityBorderColor } from '@/utils/itemHelpers';


interface Item {
  id: string;
  name: string;
  icon?: string;
  stackSize?: number;
  maxStack?: number;
  type: 'weapon' | 'armor' | 'accessory' | 'consumable' | 'misc';
  equipped?: boolean;
  slot?: string;
  defaultSlot?: string;
  rarity?: 'common' | 'uncommon' | 'rare' | 'epic' | 'legendary';
  enhancement_bonus?: number;
  charges?: number;
  is_custom?: boolean;
  is_identified?: boolean;
  is_plot?: boolean;
  is_cursed?: boolean;
  is_stolen?: boolean;
}

interface InventoryItem {
  index: number;
  item: Record<string, unknown>;
  base_item: number;
  base_item_name?: string;
  name: string;
  description?: string;
  weight: number;
  value: number;
  is_custom: boolean;
  stack_size: number;
  enhancement: number;
  charges?: number;
  identified: boolean;
  plot: boolean;
  cursed: boolean;
  stolen: boolean;
  base_ac?: number | null;
  category: 'weapon' | 'armor' | 'accessory' | 'consumable' | 'misc';
  equippable_slots: string[];
  default_slot: string | null;
  decoded_properties?: Array<{
    property_id: number;
    label: string;
    description: string;
    bonus_type: string;
    bonus_value?: number;
    [key: string]: unknown;
  }>;
}

interface InventoryEncumbrance {
  total_weight: number | string;
  light_load: number | string;
  medium_load: number | string;
  heavy_load: number | string;
  encumbrance_level: string;
}

interface EquippedItem {
  base_item: number;
  base_item_name?: string;
  custom: boolean;
  name: string;
  description?: string;
  weight: number;
  value: number;
  item_data: Record<string, unknown>;
  base_ac?: number | null;
  decoded_properties?: Array<{
    property_id: number;
    label: string;
    description: string;
    bonus_type: string;
    bonus_value?: number;
    [key: string]: unknown;
  }>;
}

interface InventorySummary {
  total_items: number;
  inventory_items: InventoryItem[];
  equipped_items: Record<string, EquippedItem>;
  custom_items: Record<string, unknown>[];
  encumbrance: InventoryEncumbrance;
}

interface LocalInventoryData {
  summary: InventorySummary;
}



const INVENTORY_COLS = 8;
const INVENTORY_ROWS = 8;

const SLOT_MAPPING: Record<string, string> = {
  'helmet': 'head', 'head': 'head',
  'chest': 'chest',
  'belt': 'belt',
  'boots': 'boots',
  'neck': 'neck',
  'cloak': 'cloak',
  'gloves': 'gloves',
  'l ring': 'left_ring', 'left_ring': 'left_ring',
  'r ring': 'right_ring', 'right_ring': 'right_ring',
  'l hand': 'left_hand', 'left_hand': 'left_hand',
  'r hand': 'right_hand', 'right_hand': 'right_hand',
  'arrows': 'arrows', 'bullets': 'bullets', 'bolts': 'bolts'
};

export default function InventoryEditor() {
  const t = useTranslations();
  const { character, invalidateSubsystems } = useCharacterContext();
  const inventoryData = useSubsystem('inventory');
  const combatSubsystem = useSubsystem('combat');
  const { showToast } = useToast();
  const [isEquipping, setIsEquipping] = useState(false);
  const [pendingEquipSlot, setPendingEquipSlot] = useState<string | null>(null);
  const [pendingUnequipItem, setPendingUnequipItem] = useState<{name: string; base_item: number} | null>(null);

  useEffect(() => {
    if (character) {
      if (!inventoryData.data && !inventoryData.isLoading) {
        inventoryData.load();
      }
      if (!combatSubsystem.data && !combatSubsystem.isLoading) {
        combatSubsystem.load();
      }
    }
  }, [character, inventoryData, combatSubsystem]);
  

  const parseInventoryData = (inventoryData: LocalInventoryData | null): (Item | null)[] => {
    const inv = Array(INVENTORY_COLS * INVENTORY_ROWS).fill(null);

    if (!inventoryData?.summary) {
      return inv;
    }

    const { inventory_items } = inventoryData.summary;

    inventory_items?.forEach((itemInfo: InventoryItem) => {
      const targetIndex = safeToNumber(itemInfo.index, -1);
      
      if (targetIndex >= 0 && targetIndex < INVENTORY_COLS * INVENTORY_ROWS && itemInfo) {
        const baseItem = itemInfo.base_item || 0;
        const isCustom = itemInfo.is_custom || false;
        const itemName = itemInfo.name || `Item ${baseItem}`;

        const isEquipped = false;

        inv[targetIndex] = {
          id: `inventory_${targetIndex}`,
          name: itemName,
          type: itemInfo.category || 'misc',
          rarity: isCustom ? 'legendary' : 'common',
          equipped: isEquipped,
          defaultSlot: itemInfo.default_slot || undefined,
          stackSize: itemInfo.stack_size > 1 ? itemInfo.stack_size : undefined,
          enhancement_bonus: itemInfo.enhancement || 0,
          charges: itemInfo.charges,
          is_custom: isCustom,
          is_identified: itemInfo.identified,
          is_plot: itemInfo.plot,
          is_cursed: itemInfo.cursed,
          is_stolen: itemInfo.stolen
        };
      }
    });

    return inv;
  };

  const [inventory, setInventory] = useState<(Item | null)[]>(() =>
    parseInventoryData(inventoryData.data as unknown as LocalInventoryData | null)
  );
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [selectedItemRawData, setSelectedItemRawData] = useState<Record<string, unknown> | null>(null);
  const [selectedItemInventoryIndex, setSelectedItemInventoryIndex] = useState<number | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [typeFilter, setTypeFilter] = useState<ItemTypeFilter>('all');
  const [statusFilters, setStatusFilters] = useState<Set<keyof StatusFilter>>(new Set());
  const [sortBy, setSortBy] = useState<ItemSortOption>('name');
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [itemToDelete, setItemToDelete] = useState<{index: number; name: string; isPlot: boolean} | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [showPropertyEditor, setShowPropertyEditor] = useState(false);

  // Add Item Modal State
  const [showAddItemModal, setShowAddItemModal] = useState(false);
  const [baseItems, setBaseItems] = useState<BaseItem[]>([]);
  const [templates, setTemplates] = useState<ItemTemplate[]>([]);
  const [isLoadingTemplates, setIsLoadingTemplates] = useState(false);
  const [hasLoadedAddData, setHasLoadedAddData] = useState(false);
  const [pendingNewItemIndex, setPendingNewItemIndex] = useState<number | null>(null);

  const inventoryItemsWithIndices = useMemo(() => {
    return inventory
      .map((item, index) => ({ item, originalIndex: index }))
      .filter((entry): entry is { item: Item; originalIndex: number } => entry.item !== null);
  }, [inventory]);

  const { searchResults } = useInventorySearch(
    inventoryItemsWithIndices.map(entry => entry.item),
    searchQuery
  );

  const inventorySummary = useMemo(() => {
    return (inventoryData.data as unknown as LocalInventoryData)?.summary;
  }, [inventoryData.data]);

  const filteredAndSortedItems = useMemo(() => {
    const getItemDetails = (originalIndex: number) => {
      return inventorySummary?.inventory_items?.[originalIndex];
    };
    let result = inventoryItemsWithIndices;

    if (searchQuery.trim().length >= 2) {
      const searchResultNames = new Set(searchResults.map(item => item.id));
      result = result.filter(entry => searchResultNames.has(entry.item.id));
    }

    if (typeFilter !== 'all') {
      result = result.filter(entry => entry.item.type === typeFilter);
    }

    if (statusFilters.size > 0) {
      result = result.filter(entry => {
        const item = entry.item;

        if (statusFilters.has('custom') && item.is_custom) return true;
        if (statusFilters.has('plot') && item.is_plot) return true;
        if (statusFilters.has('identified') && item.is_identified) return true;
        if (statusFilters.has('unidentified') && !item.is_identified) return true;
        if (statusFilters.has('enhanced') && (item.enhancement_bonus ?? 0) > 0) return true;

        return false;
      });
    }

    result = [...result].sort((a, b) => {
      const detailsA = getItemDetails(a.originalIndex);
      const detailsB = getItemDetails(b.originalIndex);

      switch (sortBy) {
        case 'name':
          return a.item.name.localeCompare(b.item.name);
        case 'value':
          return (detailsB?.value ?? 0) - (detailsA?.value ?? 0);
        case 'weight':
          return (detailsB?.weight ?? 0) - (detailsA?.weight ?? 0);
        case 'type':
          return a.item.type.localeCompare(b.item.type);
        default:
          return 0;
      }
    });

    return result;
  }, [inventoryItemsWithIndices, searchQuery, searchResults, typeFilter, statusFilters, sortBy, inventorySummary]);

  const displayItems = useMemo((): { item: Item | null; originalIndex: number }[] => {
    const hasFilters = searchQuery.trim().length >= 2 || typeFilter !== 'all' || statusFilters.size > 0;
    const isSorting = sortBy !== 'name';

    if (!hasFilters && !isSorting) {
      // No filters or sorting - return original inventory with indices
      return inventory.map((item, index) => ({ item, originalIndex: index }));
    }

    const result: { item: Item | null; originalIndex: number }[] = filteredAndSortedItems.map(entry => ({
      item: entry.item,
      originalIndex: entry.originalIndex
    }));

    const totalSlots = INVENTORY_COLS * INVENTORY_ROWS;
    while (result.length < totalSlots) {
      result.push({ item: null, originalIndex: -1 });
    }

    return result;
  }, [inventory, filteredAndSortedItems, searchQuery, typeFilter, statusFilters, sortBy]);

  const handleEquipItem = async (itemData: Record<string, unknown>, slot: string, inventoryIndex?: number | null) => {
    if (!character?.id) return;

    const mappedSlot = SLOT_MAPPING[slot.toLowerCase()];
    if (!mappedSlot) {
      showToast(`Invalid slot: ${slot}`, 'error');
      return;
    }

    setIsEquipping(true);
    try {
      const response = await inventoryAPI.equipItem(character.id, {
        item_data: itemData,
        slot: mappedSlot,
        inventory_index: inventoryIndex ?? undefined,
      });

      if (response.success) {
        setPendingEquipSlot(mappedSlot);
        showToast(response.message, 'success');
        await inventoryData.load();
        await invalidateSubsystems(['abilityScores', 'combat', 'saves', 'skills']);
      } else {
        showToast(response.message, 'error');
      }

      if (response.warnings.length > 0) {
        response.warnings.forEach(warning => showToast(warning, 'warning'));
      }
    } catch (error) {
      showToast(`Failed to equip item: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
    } finally {
      setIsEquipping(false);
    }
  };

  const handleUnequipItem = async (slot: string) => {
    if (!character?.id) return;

    const mappedSlot = SLOT_MAPPING[slot.toLowerCase()];
    if (!mappedSlot) {
      showToast(`Invalid slot: ${slot}`, 'error');
      return;
    }

    setIsEquipping(true);
    try {
      const response = await inventoryAPI.unequipItem(character.id, {
        slot: mappedSlot,
      });

      if (response.success) {
        if (selectedItem) {
          const baseItem = (selectedItemRawData?.base_item as number) || 0;
          setPendingUnequipItem({ name: selectedItem.name, base_item: baseItem });
        }

        showToast(response.message, 'success');
        await inventoryData.load();
        await invalidateSubsystems(['abilityScores', 'combat', 'saves', 'skills']);
      } else {
        showToast(response.message, 'error');
      }
    } catch (error) {
      showToast(`Failed to unequip item: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
    } finally {
      setIsEquipping(false);
    }
  };





  const handleDeleteItem = () => {
    if (selectedItemInventoryIndex === null || !selectedItem) return;

    setItemToDelete({
      index: selectedItemInventoryIndex,
      name: selectedItem.name,
      isPlot: selectedItem.is_plot || false
    });
    setShowDeleteConfirm(true);
  };

  const confirmDelete = async () => {
    if (!character?.id || !itemToDelete) return;

    setIsDeleting(true);
    try {
      const response = await inventoryAPI.deleteItem(character.id, itemToDelete.index);

      if (response.success) {
        showToast(response.message, 'success');
        await inventoryData.load();
        setSelectedItem(null);
        setSelectedItemRawData(null);
        setSelectedItemInventoryIndex(null);
      } else {
        showToast(response.message, 'error');
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      showToast(errorMessage, 'error');
    } finally {
      setIsDeleting(false);
      setShowDeleteConfirm(false);
      setItemToDelete(null);
    }
  };

  const cancelDelete = () => {
    setShowDeleteConfirm(false);
    setItemToDelete(null);
  };

  const handleAddByBaseType = async (baseItemId: number): Promise<number | null> => {
    if (!character?.id) return null;
    try {
      const response = await inventoryAPI.addItemByBaseType(character.id, { base_item_id: baseItemId });
      if (response.success) {
        showToast(response.message, 'success');
        const itemIndex = response.item_index ?? null;
        if (itemIndex !== null) {
          setPendingNewItemIndex(itemIndex);
        }
        await inventoryData.load();
        return itemIndex;
      } else {
        showToast(response.message, 'error');
        return null;
      }
    } catch (error) {
      showToast(`Failed to add item: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
      return null;
    }
  };

  const handleUpdateItem = async (updatedGffData: Record<string, unknown>) => {
    if (!character?.id) return;
    try {
      const response = await inventoryAPI.updateItem(character.id, {
        item_index: selectedItemInventoryIndex ?? undefined,
        slot: selectedItem?.equipped ? SLOT_MAPPING[selectedItem.slot?.toLowerCase() || ''] : undefined,
        item_data: updatedGffData
      });

      if (response.success) {
        showToast(response.message, 'success');
        await inventoryData.load();
        await invalidateSubsystems(['abilityScores', 'combat', 'saves', 'skills']);
      } else {
        showToast(response.message, 'error');
      }
    } catch (error) {
      showToast(`Failed to update item: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
    }
  };


  const handleOpenAddItem = async () => {
    setShowAddItemModal(true);
    if (!hasLoadedAddData && character?.id) {
        setIsLoadingTemplates(true);
        try {
            const [baseItemsRes, templatesRes] = await Promise.all([
                inventoryAPI.getAllBaseItems(character.id),
                inventoryAPI.getAvailableTemplates(character.id)
            ]);
            setBaseItems(baseItemsRes.base_items);
            setTemplates(templatesRes.templates);
            setHasLoadedAddData(true);
        } catch (error) {
             showToast(`Failed to load item data: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
        } finally {
            setIsLoadingTemplates(false);
        }
    }
  };

  const handleAddTemplate = async (templateResref: string) => {
      if (!character?.id) return;
      try {
          const response = await inventoryAPI.addItemFromTemplate(character.id, templateResref);
          if (response.success) {
              showToast(response.message, 'success');
              await inventoryData.load();
              await invalidateSubsystems(['abilityScores', 'combat', 'saves', 'skills']);
          } else {
              showToast(response.message, 'error');
          }
      } catch (error) {
          showToast(`Failed to add template: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
      }
  };

  const getSelectedItemDefaultSlot = (): string | null => {
    if (selectedItemInventoryIndex === null) return null;
    const inventoryItem = inventorySummary?.inventory_items?.[selectedItemInventoryIndex];
    if (!inventoryItem) return null;

    let targetSlot = inventoryItem.default_slot;

    if (inventoryItem.equippable_slots && inventoryItem.equippable_slots.length > 1) {
      const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
      if (summary && targetSlot) {
        const defaultOccupied = !!summary.equipped_items[targetSlot];

        if (defaultOccupied) {
          const emptySlot = inventoryItem.equippable_slots.find(slot => !summary.equipped_items[slot]);
          if (emptySlot) {
            targetSlot = emptySlot;
          }
        }
      }
    }

    return targetSlot || null;
  };

  const canEquipSelectedItem = (): boolean => {
    if (selectedItemInventoryIndex === null || !selectedItem || selectedItem.equipped) return false;
    return !!(selectedItem.defaultSlot && selectedItemRawData);
  };

  const getEquippedItemForSlot = (slotName: string) => {
    if (!inventoryData.data) return null;
    const summary = (inventoryData.data as unknown as LocalInventoryData).summary;
    const equippedItems = summary?.equipped_items || {};

    const mappedSlot = SLOT_MAPPING[slotName.toLowerCase()];
    if (!mappedSlot || !equippedItems[mappedSlot]) return null;

    const equipData = equippedItems[mappedSlot];
    return {
      name: equipData.name || `Item ${equipData.base_item}`,
      base_item: equipData.base_item,
      is_custom: equipData.custom
    };
  };

  const DraggableInventoryItem = ({ item, index, isSelected, onClick, children }: { item: Item; index: number; isSelected: boolean; onClick: () => void; children: React.ReactNode }) => {
    const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
      id: `inventory-item-${index}`,
      data: { type: 'inventory', item, index },
    });

    const style: React.CSSProperties | undefined = transform ? {
      opacity: isDragging ? 0.3 : 1,
    } : undefined;

    return (
      <div 
        ref={setNodeRef} 
        style={style} 
        {...attributes} 
        {...listeners}
        onClick={onClick}
        className={`aspect-square relative rounded border-2 
          ${isSelected
            ? 'bg-[rgb(var(--color-primary)/0.2)] border-[rgb(var(--color-primary))] shadow-[0_0_10px_rgb(var(--color-primary)/0.3)]'
            : `bg-[rgb(var(--color-surface-2))] ${getRarityBorderColor(item.rarity)} hover:border-[rgb(var(--color-primary)/0.5)]`
          }
           cursor-grab active:cursor-grabbing
        `}
      >
        {children}
      </div>
    );
  };

  const DroppableEquipmentSlot = ({ slotName, children }: { slotName: string; children: React.ReactNode }) => {
     const { setNodeRef, isOver } = useDroppable({
      id: `slot-${slotName}`,
      data: { type: 'slot', slot: slotName },
    });

    return (
      <div ref={setNodeRef} className={`rounded transition-colors ${isOver ? 'ring-2 ring-[rgb(var(--color-primary))] bg-[rgb(var(--color-primary)/0.1)]' : ''}`}>
        {children}
      </div>
    );
  };

  const DroppableInventoryArea = ({ children }: { children: React.ReactNode }) => {
      const { setNodeRef, isOver } = useDroppable({
          id: 'inventory-area',
          data: { type: 'inventory-area' }
      });
      return <div ref={setNodeRef} className={`h-full ${isOver ? 'bg-[rgb(var(--color-primary)/0.05)]' : ''}`}>{children}</div>;
  };

  const EquipmentSlot = ({ slotName, slotLabel }: { slotName: string; slotLabel?: string }) => {
    const equippedItem = getEquippedItemForSlot(slotName);
    const displayLabel = slotLabel || slotName.charAt(0);

    const handleSlotClick = () => {
      if (equippedItem) {
        const summary = (inventoryData.data as unknown as LocalInventoryData).summary;
        const mappedSlot = SLOT_MAPPING[slotName.toLowerCase()];
        const rawItemData = mappedSlot ? summary?.equipped_items[mappedSlot]?.item_data : null;

        const itemForDetails: Item = {
          id: `equipped_${slotName.toLowerCase().replace(' ', '_')}`,
          name: equippedItem.name,
          type: 'misc',
          rarity: equippedItem.is_custom ? 'legendary' : 'common',
          equipped: true,
          slot: slotName,
          is_custom: equippedItem.is_custom,
          is_identified: true,
          is_plot: false,
          is_cursed: false,
          is_stolen: false
        };
        setSelectedItem(itemForDetails);
        setSelectedItemRawData(rawItemData as Record<string, unknown> | null);
        setSelectedItemInventoryIndex(null);
      }
    };

    const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
        id: `equipped-item-${slotName}`,
        data: { type: 'equipped', item: equippedItem, slot: slotName },
        disabled: !equippedItem
    });

    const content = (
        <div
          ref={setNodeRef}
          {...(equippedItem ? { ...attributes, ...listeners } : {})}
          className={`w-12 h-12 rounded border-2 flex items-center justify-center relative transition-colors ${
            equippedItem
              ? `bg-[rgb(var(--color-primary)/0.1)] border-[rgb(var(--color-primary))] cursor-grab active:cursor-grabbing hover:bg-[rgb(var(--color-primary)/0.2)] ${isDragging ? 'opacity-50' : ''}`
              : 'bg-[rgb(var(--color-surface-1))] border-[rgb(var(--color-surface-border)/0.6)]'
          }`}
          onClick={handleSlotClick}
          role={equippedItem ? "button" : undefined}
          tabIndex={equippedItem ? 0 : undefined}
          title={slotName}
        >
          {equippedItem ? (
            <div className="w-8 h-8 bg-[rgb(var(--color-surface-3))] rounded flex items-center justify-center text-xs font-bold pointer-events-none">
              {equippedItem.name.charAt(0)}
            </div>
          ) : (
            <div className="text-xs text-[rgb(var(--color-text-muted))] font-bold pointer-events-none">
              -
            </div>
          )}
          {equippedItem?.is_custom && (
            <span className="absolute -top-1 -right-1 text-xs bg-[rgb(var(--color-warning))] text-white px-1 rounded-full pointer-events-none">
              !
            </span>
          )}
        </div>
    );

    return (
      <div className="flex flex-col items-center">
        <DroppableEquipmentSlot slotName={slotName}>
            {content}
        </DroppableEquipmentSlot>
        <span className="text-xs text-[rgb(var(--color-text-muted))] mt-1 uppercase">{displayLabel}</span>
      </div>
    );
  };

  useEffect(() => {
    if (inventoryData.data) {
      setInventory(parseInventoryData(inventoryData.data as unknown as LocalInventoryData));
    }
  }, [inventoryData.data]);

  useEffect(() => {
    if (pendingNewItemIndex !== null && inventoryData.data) {
      const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
      const inventoryItem = summary?.inventory_items?.find(i => safeToNumber(i.index, -1) === pendingNewItemIndex);

      if (inventoryItem) {
        const item: Item = {
          id: `inventory_${pendingNewItemIndex}`,
          name: inventoryItem.name || `Item ${inventoryItem.base_item}`,
          type: inventoryItem.category || 'misc',
          rarity: inventoryItem.is_custom ? 'legendary' : 'common',
          equipped: false,
          defaultSlot: inventoryItem.default_slot || undefined,
          stackSize: inventoryItem.stack_size > 1 ? inventoryItem.stack_size : undefined,
          enhancement_bonus: inventoryItem.enhancement || 0,
          charges: inventoryItem.charges,
          is_custom: inventoryItem.is_custom,
          is_identified: inventoryItem.identified,
          is_plot: inventoryItem.plot,
          is_cursed: inventoryItem.cursed,
          is_stolen: inventoryItem.stolen
        };

        setSelectedItem(item);
        setSelectedItemRawData(inventoryItem.item as Record<string, unknown> | null);
        setSelectedItemInventoryIndex(pendingNewItemIndex);
        setShowPropertyEditor(true);
        setPendingNewItemIndex(null);
        return;
      }
    }

    if (pendingEquipSlot && inventoryData.data) {
      const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
      const equipData = summary?.equipped_items?.[pendingEquipSlot];
      if (equipData) {
        setSelectedItem({
          id: `equipped_${pendingEquipSlot}`,
          name: equipData.name,
          type: 'misc',
          rarity: equipData.custom ? 'legendary' : 'common',
          equipped: true,
          slot: pendingEquipSlot,
          is_custom: equipData.custom,
          is_identified: true,
          is_plot: false,
          is_cursed: false,
          is_stolen: false
        });
        setSelectedItemRawData(equipData.item_data as Record<string, unknown> | null);
        setSelectedItemInventoryIndex(null);
        setPendingEquipSlot(null);
        return; 
      }
      
      if (!isEquipping) {
         setPendingEquipSlot(null);
      }
    }

    if (pendingUnequipItem && inventoryData.data) {
      const currentInventory = parseInventoryData(inventoryData.data as unknown as LocalInventoryData);
      const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
      
      const matchIndex = currentInventory.findIndex(item => 
        item && item.name === pendingUnequipItem.name
      );

      if (matchIndex !== -1 && currentInventory[matchIndex]) {
        const newItem = currentInventory[matchIndex]!;
        setSelectedItem(newItem);
        const rawItem = summary?.inventory_items?.find(i => safeToNumber(i.index, -1) === matchIndex)?.item;
        setSelectedItemRawData(rawItem as Record<string, unknown> | null);
        setSelectedItemInventoryIndex(matchIndex);
        setPendingUnequipItem(null);
        return;
      }
      
      if (!isEquipping) {
         setPendingUnequipItem(null);
      }
    }

    if (selectedItemInventoryIndex !== null) {
      const currentInventory = parseInventoryData(inventoryData.data as unknown as LocalInventoryData);
      const currentItem = currentInventory[selectedItemInventoryIndex];

      if (!currentItem) {
        setSelectedItem(null);
        setSelectedItemRawData(null);
        setSelectedItemInventoryIndex(null);
      } else {
        setSelectedItem(currentItem);
        const rawItem = (inventoryData.data as unknown as LocalInventoryData)?.summary?.inventory_items?.[selectedItemInventoryIndex]?.item;
        setSelectedItemRawData(rawItem as Record<string, unknown> | null);
      }
    } else if (selectedItem?.equipped && selectedItem.slot) {
      const currentEquipped = getEquippedItemForSlot(selectedItem.slot);
      if (!currentEquipped) {
        setSelectedItem(null);
        setSelectedItemRawData(null);
      }
    }
  // This effect intentionally only reacts to inventory data changes to sync selection state
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [inventoryData.data, selectedItemInventoryIndex]);

  const [activeDragItem, setActiveDragItem] = useState<{ item: Item; index: number } | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  );

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event;
    const itemData = active.data.current as { item: Item; index: number };
    if (itemData) {
      setActiveDragItem(itemData);
    }
  };

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveDragItem(null);

    const droppedOnInventory = !over || over.id === 'inventory-area';
    const activeData = active.data.current as { type: 'inventory' | 'equipped'; item: Item; slot?: string; index?: number };
    const overData = over?.data?.current as { type: 'slot' | 'inventory-area'; slot?: string } | undefined;

    if (!activeData) return;

    if (activeData.type === 'inventory' && overData?.type === 'slot' && overData.slot) {
      if (activeData.item && activeData.index !== undefined) {
          const inventoryItemInfo = inventorySummary?.inventory_items?.find(i => safeToNumber(i.index) === activeData.index);
          const rawItem = inventoryItemInfo?.item;

          if (inventoryItemInfo && rawItem) {
             const targetSlotName = overData.slot.toLowerCase();
             const mappedSlot = SLOT_MAPPING[targetSlotName];
             const allowedSlots = inventoryItemInfo.equippable_slots || [];

             if (!mappedSlot || !allowedSlots.includes(mappedSlot)) {
                 showToast(`Cannot equip ${activeData.item.name} in ${overData.slot}`, 'warning');
                 return;
             }

             await handleEquipItem(rawItem, overData.slot, activeData.index);
          }
      }
    }

    if (activeData.type === 'equipped' && activeData.slot && (droppedOnInventory || !overData?.type)) {
        const slotToUnequip = activeData.slot;
        if (!isEquipping) {
             await handleUnequipItem(slotToUnequip);
        }
    }
  };

  if (inventoryData.isLoading && !inventoryData.data) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
      </div>
    );
  }

  if (inventoryData.error) {
    return (
      <Card variant="error">
        <p className="text-error">{inventoryData.error}</p>
      </Card>
    );
  }

  if (!character || !inventoryData.data) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }

  return (
    <DndContext sensors={sensors} onDragStart={handleDragStart} onDragEnd={handleDragEnd}>
    <div className="space-y-6">
      <div className="flex flex-col lg:flex-row gap-2 items-stretch">
        <Card className="flex-shrink-0 min-h-[710px] flex flex-col">
          <CardContent className="p-6 flex-1 flex flex-col">
            <div className="flex-shrink-0 h-full flex flex-col" style={{ width: '240px' }}>
                  <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mt-1.5 mb-4">{t('inventory.equipment')}</h3>

                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <div></div>
                    <EquipmentSlot slotName="Helmet" slotLabel="H" />
                    <EquipmentSlot slotName="Neck" slotLabel="N" />
                    <div></div>
                  </div>

                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <EquipmentSlot slotName="L Hand" slotLabel="L" />
                    <EquipmentSlot slotName="Chest" slotLabel="A" />
                    <EquipmentSlot slotName="Cloak" slotLabel="C" />
                    <EquipmentSlot slotName="R Hand" slotLabel="R" />
                  </div>

                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <EquipmentSlot slotName="L Ring" slotLabel="L" />
                    <EquipmentSlot slotName="Belt" slotLabel="B" />
                    <EquipmentSlot slotName="Gloves" slotLabel="G" />
                    <EquipmentSlot slotName="R Ring" slotLabel="R" />
                  </div>

                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <div></div>
                    <EquipmentSlot slotName="Boots" slotLabel="F" />
                    <div></div>
                    <div></div>
                  </div>

                  <div className="grid grid-cols-4 gap-2 pt-2 border-t border-[rgb(var(--color-surface-border)/0.3)]">
                    <div></div>
                    <EquipmentSlot slotName="Arrows" slotLabel="Arr" />
                    <EquipmentSlot slotName="Bullets" slotLabel="Bul" />
                    <EquipmentSlot slotName="Bolts" slotLabel="Bol" />
                  </div>

                  <div className="mt-auto pt-6">
                    <InventorySidebarFooter
                      encumbrance={(inventoryData.data as unknown as LocalInventoryData)?.summary?.encumbrance}
                    />
                  </div>
            </div>
          </CardContent>
        </Card>

        <Card className="flex-1 min-w-0 min-h-[710px]">
          <CardContent className="p-6 h-full">
             <DroppableInventoryArea>
                <div className="min-w-0 h-full flex flex-col">
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">{t('inventory.inventory')}</h3>
                    <Button 
                      onClick={handleOpenAddItem}
                      variant="outline"
                      size="sm"
                      className="h-8 gap-1 border-[rgb(var(--color-primary)/0.5)] text-[rgb(var(--color-primary))] hover:bg-[rgb(var(--color-primary)/0.1)]"
                    >
                      <Plus className="w-4 h-4" />
                      Add Item
                    </Button>
                  </div>

                  <InventoryFilters
                    searchTerm={searchQuery}
                    onSearchChange={setSearchQuery}
                    typeFilter={typeFilter}
                    onTypeFilterChange={setTypeFilter}
                    statusFilters={statusFilters}
                    onStatusFiltersChange={setStatusFilters}
                    sortBy={sortBy}
                    onSortChange={setSortBy}
                    filteredCount={filteredAndSortedItems.length}
                    totalCount={inventoryItemsWithIndices.length}
                  />

                  {filteredAndSortedItems.length === 0 && inventoryItemsWithIndices.length > 0 ? (
                    <div className="p-8 bg-[rgb(var(--color-surface-1))] rounded text-center text-[rgb(var(--color-text-muted))]">
                      {t('inventory.filters.noResults')}
                    </div>
                  ) : (
                    <div className="grid gap-1.5 p-2 bg-[rgb(var(--color-surface-1))] rounded w-fit mx-auto" style={{ gridTemplateColumns: 'repeat(8, 3rem)' }}>
                      {displayItems.map((entry, displayIndex) => {
                        const { item, originalIndex } = entry;
                        const inventoryItem = originalIndex >= 0 ? inventorySummary?.inventory_items?.[originalIndex] : null;
                        const rawItemData = inventoryItem?.item;
                        const isSelected = selectedItem?.id === item?.id;

                        if (!item) {
                            return (
                                <div
                                key={displayIndex}
                                className="w-12 h-12 bg-[rgb(var(--color-surface-2))] border-2 border-[rgb(var(--color-surface-border)/0.4)] rounded"
                                />
                            );
                        }

                        return (
                            <DraggableInventoryItem
                                key={displayIndex}
                                item={item}
                                index={originalIndex}
                                isSelected={isSelected}
                                onClick={() => {
                                    setSelectedItem(item);
                                    setSelectedItemRawData(rawItemData as Record<string, unknown> | null);
                                    setSelectedItemInventoryIndex(originalIndex >= 0 ? originalIndex : null);
                                }}
                            >
                                <div className="w-full h-full p-1 flex items-center justify-center pointer-events-none">
                                    <div className="w-8 h-8 bg-[rgb(var(--color-surface-3))] rounded flex items-center justify-center text-xs font-bold">
                                         {item.icon ? (
                                            <img src={`/icons/${item.icon}.png`} alt={item.name} className="w-full h-full object-contain" />
                                         ) : (
                                            item.name.charAt(0)
                                         )}
                                    </div>
                                    {item.stackSize && (
                                        <span className="absolute bottom-0 right-0 text-xs bg-[rgb(var(--color-background)/0.9)] px-1 rounded">
                                        {item.stackSize}
                                        </span>
                                    )}
                                    {item.equipped && (
                                        <span className="absolute top-0 left-0 text-xs bg-[rgb(var(--color-primary))] text-white px-1 rounded-br font-bold">
                                        E
                                        </span>
                                    )}
                                </div>
                            </DraggableInventoryItem>
                        );
                      })}
                    </div>
                  )}
                </div>
              </DroppableInventoryArea>
          </CardContent>
        </Card>

        <div className="w-[340px] flex-shrink-0 flex flex-col gap-4 min-h-[710px]">
          <div className="flex-1 min-h-0">
            {selectedItem ? (
              <ItemDetailsPanel
                item={selectedItem}
                decodedProperties={(() => {
                  if (!selectedItem) return undefined;

                  if (selectedItemInventoryIndex !== null) {
                    const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                    const inventoryItem = summary?.inventory_items?.[selectedItemInventoryIndex];
                    return inventoryItem?.decoded_properties;
                  }

                  if (selectedItem.equipped && selectedItem.slot) {
                    const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                    const itemSlot = Object.entries(SLOT_MAPPING).find(([key]) => key.toLowerCase() === selectedItem.slot?.toLowerCase())?.[1];
                    if (itemSlot && summary?.equipped_items?.[itemSlot]) {
                      return summary.equipped_items[itemSlot].decoded_properties;
                    }
                  }
                  return undefined;
                })()}
                description={(() => {
                  if (selectedItemInventoryIndex !== null) {
                    return inventorySummary?.inventory_items?.[selectedItemInventoryIndex]?.description;
                  }
                  if (selectedItem.equipped && selectedItem.slot) {
                    const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                    const equipData = (inventoryData.data as unknown as LocalInventoryData)?.summary?.equipped_items?.[mappedSlot];
                    return equipData?.description;
                  }
                  return undefined;
                })()}
                baseItemName={(() => {
                  if (selectedItemInventoryIndex !== null) {
                    return inventorySummary?.inventory_items?.[selectedItemInventoryIndex]?.base_item_name;
                  }
                  if (selectedItem.equipped && selectedItem.slot) {
                    const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                    const equipData = (inventoryData.data as unknown as LocalInventoryData)?.summary?.equipped_items?.[mappedSlot];
                    return equipData?.base_item_name;
                  }
                  return undefined;
                })()}
                weight={(() => {
                  if (selectedItemInventoryIndex !== null) {
                    return inventorySummary?.inventory_items?.[selectedItemInventoryIndex]?.weight;
                  }
                  if (selectedItem.equipped && selectedItem.slot) {
                    const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                    const equipData = (inventoryData.data as unknown as LocalInventoryData)?.summary?.equipped_items?.[mappedSlot];
                    return equipData?.weight;
                  }
                  return undefined;
                })()}
                value={(() => {
                  if (selectedItemInventoryIndex !== null) {
                    return inventorySummary?.inventory_items?.[selectedItemInventoryIndex]?.value;
                  }
                  if (selectedItem.equipped && selectedItem.slot) {
                    const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                    const equipData = (inventoryData.data as unknown as LocalInventoryData)?.summary?.equipped_items?.[mappedSlot];
                    return equipData?.value;
                  }
                  return undefined;
                })()}
                baseAc={(() => {
                  if (selectedItemInventoryIndex !== null) {
                    return inventorySummary?.inventory_items?.[selectedItemInventoryIndex]?.base_ac;
                  }
                  if (selectedItem.equipped && selectedItem.slot) {
                    const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                    const equipData = (inventoryData.data as unknown as LocalInventoryData)?.summary?.equipped_items?.[mappedSlot];
                    return equipData?.base_ac;
                  }
                  return undefined;
                })()}
                rawData={selectedItemRawData || undefined}
                onEquip={() => canEquipSelectedItem() && getSelectedItemDefaultSlot() ? handleEquipItem(selectedItemRawData!, getSelectedItemDefaultSlot()!, selectedItemInventoryIndex) : undefined}
                onUnequip={() => selectedItem.equipped && selectedItem.slot && getEquippedItemForSlot(selectedItem.slot) ? handleUnequipItem(selectedItem.slot) : undefined}
                onEdit={() => setShowPropertyEditor(true)}
                onDestroy={selectedItemInventoryIndex !== null ? handleDeleteItem : undefined}
                isEquipping={isEquipping}
                canEquip={canEquipSelectedItem()}
                canUnequip={!!(selectedItem.equipped && selectedItem.slot)}
              />
            ) : (
              <InventoryCharacterSummary
                combatStats={{
                  ac: (combatSubsystem.data?.armor_class?.total || character.armorClass || 0),
                  bab: (typeof combatSubsystem.data?.base_attack_bonus === 'object' && combatSubsystem.data?.base_attack_bonus?.total_bab) || 
                      combatSubsystem.data?.summary?.base_attack_bonus || 
                      character.baseAttackBonus || 
                      0
                }}
              />
            )}
          </div>
          
          </div>

        <AddItemModal
            isOpen={showAddItemModal}
            onClose={() => setShowAddItemModal(false)}
            onAddBaseItem={handleAddByBaseType}
            onAddTemplate={handleAddTemplate}
            baseItems={baseItems}
            templates={templates}
            isLoadingTemplates={isLoadingTemplates}
        />


        {showPropertyEditor && selectedItemRawData && (
          <ItemPropertyEditor
            isOpen={showPropertyEditor}
            onClose={() => setShowPropertyEditor(false)}
            onSave={handleUpdateItem}
            itemData={selectedItemRawData}
            characterId={character?.id}
            itemIndex={selectedItemInventoryIndex}
            slot={selectedItem?.equipped ? selectedItem.slot : null}
          />
        )}
      </div>

      {showDeleteConfirm && itemToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
          <Card className="max-w-md w-full mx-4 outline-none">
            <CardContent className="p-6">
              <div className="flex justify-between items-start mb-4">
                <h3 className="text-lg font-semibold">
                  {t('inventory.confirmDeleteTitle')}
                </h3>
                <Button variant="ghost" size="sm" onClick={cancelDelete} className="p-1 h-auto">
                    <X className="w-5 h-5 text-[rgb(var(--color-text-muted))]" />
                </Button>
              </div>
              <p className="text-sm text-[rgb(var(--color-text-muted))] mb-2">
                {t('inventory.deleteItemName')}: <span className="font-semibold text-[rgb(var(--color-text))]">{itemToDelete.name}</span>
              </p>
              <p className="text-sm text-[rgb(var(--color-text-muted))] mb-6">
                {itemToDelete.isPlot ? t('inventory.deleteWarningPlot') : t('inventory.deleteWarningRegular')}
              </p>
              <div className="flex gap-3 justify-end">
                <Button
                  variant="ghost"
                  onClick={cancelDelete}
                  disabled={isDeleting}
                >
                  {t('actions.cancel')}
                </Button>
                <Button
                  variant="danger"
                  onClick={confirmDelete}
                  disabled={isDeleting}
                >
                  {isDeleting ? t('actions.deleting') : t('actions.delete')}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
     </div>

      <DragOverlay dropAnimation={null}>
        {activeDragItem ? (
          <div className="w-12 h-12 rounded border-2 border-[rgb(var(--color-primary))] bg-[rgb(var(--color-surface-3))] shadow-2xl flex items-center justify-center opacity-90 z-50 cursor-grabbing pointer-events-none">
             <div className="w-8 h-8 bg-[rgb(var(--color-surface-3))] rounded flex items-center justify-center text-xs font-bold">
                 {activeDragItem.item.icon ? (
                      <img src={`/icons/${activeDragItem.item.icon}.png`} alt={activeDragItem.item.name} className="w-full h-full object-contain" />
                 ) : (
                    activeDragItem.item.name.charAt(0)
                 )}
             </div>
          </div>
        ) : null}
      </DragOverlay>
    </DndContext>
  );
}
