'use client';

import { useState, useEffect, useMemo } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { inventoryAPI } from '@/services/inventoryApi';
import { useToast } from '@/contexts/ToastContext';
import ItemDetailsPanel from './ItemDetailsPanel';
import { InventoryFilters, ItemTypeFilter, ItemSortOption, StatusFilter } from './InventoryFilters';
import { useInventorySearch } from '@/hooks/useInventorySearch';

interface Item {
  id: string;
  name: string;
  icon?: string;
  stackSize?: number;
  maxStack?: number;
  type: 'weapon' | 'armor' | 'accessory' | 'consumable' | 'misc';
  equipped?: boolean;
  slot?: string;
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



const INVENTORY_COLS = 7;
const INVENTORY_ROWS = 8;

const SLOT_MAPPING: Record<string, string> = {
  'helmet': 'head', 'chest': 'chest', 'belt': 'belt', 'boots': 'boots',
  'neck': 'neck', 'cloak': 'cloak', 'gloves': 'gloves',
  'l ring': 'left_ring', 'r ring': 'right_ring',
  'l hand': 'left_hand', 'r hand': 'right_hand',
  'arrows': 'arrows', 'bullets': 'bullets', 'bolts': 'bolts'
};

// Utility function to safely convert values to numbers
const safeToNumber = (value: unknown, defaultValue: number = 0): number => {
  if (typeof value === 'number') return value;
  if (typeof value === 'string') {
    const parsed = parseFloat(value);
    return isNaN(parsed) ? defaultValue : parsed;
  }
  return defaultValue;
};

export default function InventoryEditor() {
  const t = useTranslations();
  const { character, invalidateSubsystems } = useCharacterContext();
  const inventoryData = useSubsystem('inventory');
  const { showToast } = useToast();
  const [isEquipping, setIsEquipping] = useState(false);

  // Load inventory data only if character exists and data hasn't been loaded
  useEffect(() => {
    if (character && !inventoryData.data && !inventoryData.isLoading) {
      inventoryData.load();
    }
  }, [character, inventoryData]);
  

  // Parse inventory data from backend
  const parseInventoryData = (inventoryData: LocalInventoryData | null): (Item | null)[] => {
    const inv = Array(INVENTORY_COLS * INVENTORY_ROWS).fill(null);

    if (!inventoryData?.summary) {
      return inv;
    }

    const { inventory_items, equipped_items } = inventoryData.summary;

    // Create a set of equipped base items for quick lookup
    const equippedBaseItems = new Set<number>();
    Object.values(equipped_items || {}).forEach((equipData: EquippedItem) => {
      if (equipData?.base_item) {
        equippedBaseItems.add(equipData.base_item);
      }
    });

    // Convert inventory items to display format
    inventory_items?.forEach((itemInfo: InventoryItem, index: number) => {
      if (index < INVENTORY_COLS * INVENTORY_ROWS && itemInfo) {
        const baseItem = itemInfo.base_item || 0;
        const isCustom = itemInfo.is_custom || false;
        const itemName = itemInfo.name || `Item ${baseItem}`;

        // Check if this item type is equipped
        const isEquipped = equippedBaseItems.has(baseItem);

        inv[index] = {
          id: `inventory_${itemInfo.index}`,
          name: itemName,
          type: itemInfo.category || 'misc',
          rarity: isCustom ? 'legendary' : 'common',
          equipped: isEquipped,
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
  const [goldValue, setGoldValue] = useState<string>('');
  const [isUpdatingGold, setIsUpdatingGold] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [itemToDelete, setItemToDelete] = useState<{index: number; name: string; isPlot: boolean} | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  // Build searchable items list with indices for filtering
  const inventoryItemsWithIndices = useMemo(() => {
    return inventory
      .map((item, index) => ({ item, originalIndex: index }))
      .filter((entry): entry is { item: Item; originalIndex: number } => entry.item !== null);
  }, [inventory]);

  // Apply fuzzy search
  const { searchResults } = useInventorySearch(
    inventoryItemsWithIndices.map(entry => entry.item),
    searchQuery
  );

  // Memoize the summary for stable reference
  const inventorySummary = useMemo(() => {
    return (inventoryData.data as unknown as LocalInventoryData)?.summary;
  }, [inventoryData.data]);

  // Apply type, status filters and sorting
  const filteredAndSortedItems = useMemo(() => {
    const getItemDetails = (originalIndex: number) => {
      return inventorySummary?.inventory_items?.[originalIndex];
    };
    let result = inventoryItemsWithIndices;

    // Filter by search results if searching
    if (searchQuery.trim().length >= 2) {
      const searchResultNames = new Set(searchResults.map(item => item.id));
      result = result.filter(entry => searchResultNames.has(entry.item.id));
    }

    // Filter by type
    if (typeFilter !== 'all') {
      result = result.filter(entry => entry.item.type === typeFilter);
    }

    // Filter by status
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

    // Sort items
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

  // Build display grid - shows sorted/filtered items compactly
  const displayItems = useMemo((): { item: Item | null; originalIndex: number }[] => {
    const hasFilters = searchQuery.trim().length >= 2 || typeFilter !== 'all' || statusFilters.size > 0;
    const isSorting = sortBy !== 'name';

    if (!hasFilters && !isSorting) {
      // No filters or sorting - return original inventory with indices
      return inventory.map((item, index) => ({ item, originalIndex: index }));
    }

    // Return filtered/sorted items, padded with empty slots to fill grid
    const result: { item: Item | null; originalIndex: number }[] = filteredAndSortedItems.map(entry => ({
      item: entry.item,
      originalIndex: entry.originalIndex
    }));

    // Pad with empty slots to maintain grid structure
    const totalSlots = INVENTORY_COLS * INVENTORY_ROWS;
    while (result.length < totalSlots) {
      result.push({ item: null, originalIndex: -1 });
    }

    return result;
  }, [inventory, filteredAndSortedItems, searchQuery, typeFilter, statusFilters, sortBy]);

  // Handler to equip an item from inventory
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

  // Handler to unequip an item
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
        showToast(response.message, 'success');
        await inventoryData.load();
        await invalidateSubsystems(['abilityScores', 'combat', 'saves', 'skills']);
        setSelectedItem(null);
        setSelectedItemRawData(null);
        setSelectedItemInventoryIndex(null);
      } else {
        showToast(response.message, 'error');
      }
    } catch (error) {
      showToast(`Failed to unequip item: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
    } finally {
      setIsEquipping(false);
    }
  };

  // Sync gold value when character changes
  useEffect(() => {
    if (character?.gold !== undefined) {
      setGoldValue(character.gold.toString());
    }
  }, [character?.gold]);

  // Handler to update gold
  const handleUpdateGold = async () => {
    if (!character?.id || isUpdatingGold) return;

    const cleanValue = goldValue.replace(/,/g, '');
    const numericValue = parseInt(cleanValue, 10);

    if (isNaN(numericValue) || numericValue < 0 || numericValue > 2147483647) {
      showToast(t('inventory.invalidGold'), 'error');
      setGoldValue(character?.gold?.toString() || '0');
      return;
    }

    // Only update if value changed
    if (numericValue === character?.gold) {
      return;
    }

    setIsUpdatingGold(true);
    try {
      const response = await inventoryAPI.updateGold(character.id, numericValue);

      if (response.success) {
        showToast(t('inventory.goldUpdated'), 'success');
      } else {
        showToast(response.message, 'error');
        setGoldValue(character?.gold?.toString() || '0');
      }
    } catch (error) {
      showToast(`Failed to update gold: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
      setGoldValue(character?.gold?.toString() || '0');
    } finally {
      setIsUpdatingGold(false);
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

  // Get default equip slot for selected inventory item from backend data
  const getSelectedItemDefaultSlot = (): string | null => {
    if (selectedItemInventoryIndex === null) return null;
    const inventoryItem = inventorySummary?.inventory_items?.[selectedItemInventoryIndex];
    return inventoryItem?.default_slot || null;
  };

  // Check if selected inventory item can be equipped
  const canEquipSelectedItem = (): boolean => {
    if (selectedItemInventoryIndex === null || !selectedItem || selectedItem.equipped) return false;
    const inventoryItem = inventorySummary?.inventory_items?.[selectedItemInventoryIndex];
    return !!(inventoryItem?.default_slot && selectedItemRawData);
  };

  // Helper function to get equipped item for a slot
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

  // Equipment slot component
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

    return (
      <div className="flex flex-col items-center">
        <div
          className={`w-12 h-12 rounded border-2 flex items-center justify-center relative transition-colors ${
            equippedItem
              ? 'bg-[rgb(var(--color-primary)/0.1)] border-[rgb(var(--color-primary))] cursor-pointer hover:bg-[rgb(var(--color-primary)/0.2)]'
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
        <span className="text-xs text-[rgb(var(--color-text-muted))] mt-1 uppercase">{displayLabel}</span>
      </div>
    );
  };

  // Update inventory when backend data changes
  useEffect(() => {
    if (inventoryData.data) {
      setInventory(parseInventoryData(inventoryData.data as unknown as LocalInventoryData));
    }
  }, [inventoryData.data]);

  const getRarityColor = (rarity?: string) => {
    switch (rarity) {
      case 'uncommon': return 'border-[rgb(var(--color-success))]';
      case 'rare': return 'border-[rgb(var(--color-primary))]';
      case 'epic': return 'border-[rgb(var(--color-secondary))]';
      case 'legendary': return 'border-[rgb(var(--color-warning))]';
      default: return 'border-[rgb(var(--color-surface-border)/0.6)]';
    }
  };

  const handleDragStart = (e: React.DragEvent, item: Item, index: number) => {
    e.dataTransfer.setData('item', JSON.stringify(item));
    e.dataTransfer.setData('fromIndex', index.toString());
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
  };

  const handleDrop = (e: React.DragEvent, toIndex: number) => {
    e.preventDefault();
    const item = JSON.parse(e.dataTransfer.getData('item'));
    const fromIndex = parseInt(e.dataTransfer.getData('fromIndex'));
    
    const newInventory = [...inventory];
    newInventory[fromIndex] = null;
    newInventory[toIndex] = item;
    setInventory(newInventory);
  };

  // Early return for loading/error states
  if (inventoryData.isLoading) {
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
    <div className="space-y-6">
      <div className="flex flex-col lg:flex-row gap-6">
        {/* Combined Equipment & Inventory */}
        <div>
          <Card>
            <CardContent className="p-6">
              <div className="flex gap-6">
                {/* Character Equipment */}
                <div className="flex-shrink-0" style={{ width: '240px' }}>
                  <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mt-1.5 mb-4">{t('inventory.equipment')}</h3>

                  {/* Row 1: Helmet, Neck */}
                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <div></div>
                    <EquipmentSlot slotName="Helmet" slotLabel="H" />
                    <EquipmentSlot slotName="Neck" slotLabel="N" />
                    <div></div>
                  </div>

                  {/* Row 2: L Hand, Chest, Cloak, R Hand */}
                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <EquipmentSlot slotName="L Hand" slotLabel="L" />
                    <EquipmentSlot slotName="Chest" slotLabel="A" />
                    <EquipmentSlot slotName="Cloak" slotLabel="C" />
                    <EquipmentSlot slotName="R Hand" slotLabel="R" />
                  </div>

                  {/* Row 3: L Ring, Belt, Gloves, R Ring */}
                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <EquipmentSlot slotName="L Ring" slotLabel="L" />
                    <EquipmentSlot slotName="Belt" slotLabel="B" />
                    <EquipmentSlot slotName="Gloves" slotLabel="G" />
                    <EquipmentSlot slotName="R Ring" slotLabel="R" />
                  </div>

                  {/* Row 4: Boots */}
                  <div className="grid grid-cols-4 gap-2 mb-2">
                    <div></div>
                    <EquipmentSlot slotName="Boots" slotLabel="F" />
                    <div></div>
                    <div></div>
                  </div>

                  {/* Ammo Row */}
                  <div className="grid grid-cols-4 gap-2 pt-2 border-t border-[rgb(var(--color-surface-border)/0.3)]">
                    <div></div>
                    <EquipmentSlot slotName="Arrows" slotLabel="Arr" />
                    <EquipmentSlot slotName="Bullets" slotLabel="Bul" />
                    <EquipmentSlot slotName="Bolts" slotLabel="Bol" />
                  </div>
                </div>

                {/* Inventory Grid */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">{t('inventory.inventory')}</h3>
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

                  {/* Inventory Grid */}
                  {filteredAndSortedItems.length === 0 && inventoryItemsWithIndices.length > 0 ? (
                    <div className="p-8 bg-[rgb(var(--color-surface-1))] rounded text-center text-[rgb(var(--color-text-muted))]">
                      {t('inventory.filters.noResults')}
                    </div>
                  ) : (
                    <div className="grid gap-1.5 p-2 bg-[rgb(var(--color-surface-1))] rounded w-fit" style={{ gridTemplateColumns: 'repeat(7, 3rem)' }}>
                      {displayItems.map((entry, displayIndex) => {
                        const { item, originalIndex } = entry;
                        const inventoryItem = originalIndex >= 0 ? inventorySummary?.inventory_items?.[originalIndex] : null;
                        const rawItemData = inventoryItem?.item;

                        return (
                        <div
                          key={displayIndex}
                          className={`w-12 h-12 bg-[rgb(var(--color-surface-2))] border-2 ${
                            item ? (item.equipped ? 'border-[rgb(var(--color-primary))] bg-[rgb(var(--color-primary)/0.1)]' : getRarityColor(item.rarity)) : 'border-[rgb(var(--color-surface-border)/0.4)]'
                          } rounded hover:border-[rgb(var(--color-surface-border))] transition-colors cursor-pointer relative group`}
                          onDragOver={handleDragOver}
                          onDrop={(e) => handleDrop(e, originalIndex >= 0 ? originalIndex : displayIndex)}
                          onClick={() => {
                            setSelectedItem(item);
                            setSelectedItemRawData(rawItemData as Record<string, unknown> | null);
                            setSelectedItemInventoryIndex(originalIndex >= 0 ? originalIndex : null);
                          }}
                        >
                          {item && (
                            <div
                              draggable
                              onDragStart={(e) => handleDragStart(e, item, originalIndex)}
                              className="w-full h-full p-1 flex items-center justify-center"
                            >
                              <div className="w-8 h-8 bg-[rgb(var(--color-surface-3))] rounded flex items-center justify-center text-xs font-bold">
                                {item.name.charAt(0)}
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
                          )}
                        </div>
                        );
                      })}
                    </div>
                  )}

                  {/* Inventory Info */}
                  <div className="mt-4 flex items-center gap-4 text-sm w-fit">
                    <span className="text-[rgb(var(--color-text-muted))]">
                      {t('inventory.weight')}: <span className="text-[rgb(var(--color-text-secondary))]">
                        {safeToNumber((inventoryData.data as unknown as LocalInventoryData)?.summary?.encumbrance?.total_weight).toFixed(1)} / {safeToNumber((inventoryData.data as unknown as LocalInventoryData)?.summary?.encumbrance?.heavy_load, 150).toFixed(0)} lbs
                      </span>
                    </span>
                    <span className="text-[rgb(var(--color-text-muted))] flex items-center gap-2">
                      <span>{t('inventory.gold')}:</span>
                      <Input
                        type="text"
                        value={goldValue}
                        onChange={(e) => {
                          const value = e.target.value;
                          if (value === '' || /^\d+$/.test(value)) {
                            setGoldValue(value);
                          }
                        }}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleUpdateGold();
                          if (e.key === 'Escape') setGoldValue(character?.gold?.toString() || '0');
                        }}
                        className="!w-32 h-6 text-sm"
                        disabled={isUpdatingGold}
                      />
                      <Button
                        size="sm"
                        onClick={handleUpdateGold}
                        disabled={isUpdatingGold || goldValue === (character?.gold?.toString() || '0')}
                        className="h-6 px-2 text-xs"
                        title={t('actions.save')}
                      >
                        ✓
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => setGoldValue(character?.gold?.toString() || '0')}
                        disabled={isUpdatingGold}
                        className="h-6 px-2 text-xs"
                        title={t('actions.cancel')}
                      >
                        ✕
                      </Button>
                    </span>
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Item Details Panel */}
        <div>
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
                const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                const equippedItem = summary?.equipped_items?.[mappedSlot];
                return equippedItem?.decoded_properties;
              }

              return undefined;
            })()}
            description={(() => {
              if (!selectedItem) return undefined;

              if (selectedItemInventoryIndex !== null) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const inventoryItem = summary?.inventory_items?.[selectedItemInventoryIndex];
                return inventoryItem?.description;
              }

              if (selectedItem.equipped && selectedItem.slot) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                const equippedItem = summary?.equipped_items?.[mappedSlot];
                return equippedItem?.description;
              }

              return undefined;
            })()}
            weight={(() => {
              if (!selectedItem) return undefined;

              if (selectedItemInventoryIndex !== null) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const inventoryItem = summary?.inventory_items?.[selectedItemInventoryIndex];
                return inventoryItem?.weight;
              }

              if (selectedItem.equipped && selectedItem.slot) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                const equippedItem = summary?.equipped_items?.[mappedSlot];
                return equippedItem?.weight;
              }

              return undefined;
            })()}
            value={(() => {
              if (!selectedItem) return undefined;

              if (selectedItemInventoryIndex !== null) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const inventoryItem = summary?.inventory_items?.[selectedItemInventoryIndex];
                return inventoryItem?.value;
              }

              if (selectedItem.equipped && selectedItem.slot) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                const equippedItem = summary?.equipped_items?.[mappedSlot];
                return equippedItem?.value;
              }

              return undefined;
            })()}
            baseAc={(() => {
              if (!selectedItem) return undefined;

              if (selectedItemInventoryIndex !== null) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const inventoryItem = summary?.inventory_items?.[selectedItemInventoryIndex];
                return inventoryItem?.base_ac;
              }

              if (selectedItem.equipped && selectedItem.slot) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                const equippedItem = summary?.equipped_items?.[mappedSlot];
                return equippedItem?.base_ac;
              }

              return undefined;
            })()}
            rawData={(() => {
              if (!selectedItem) return undefined;

              if (selectedItemInventoryIndex !== null) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const inventoryItem = summary?.inventory_items?.[selectedItemInventoryIndex];
                return inventoryItem?.item as Record<string, unknown>;
              }

              if (selectedItem.equipped && selectedItem.slot) {
                const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                const mappedSlot = SLOT_MAPPING[selectedItem.slot.toLowerCase()];
                const equippedItem = summary?.equipped_items?.[mappedSlot];
                return equippedItem?.item_data;
              }

              return undefined;
            })()}
            onEquip={selectedItem && !selectedItem.equipped && selectedItemRawData ? () => {
              const targetSlot = getSelectedItemDefaultSlot();
              if (targetSlot) {
                handleEquipItem(selectedItemRawData, targetSlot, selectedItemInventoryIndex);
              }
            } : undefined}
            onUnequip={selectedItem?.equipped && selectedItem.slot ? () => handleUnequipItem(selectedItem.slot!) : undefined}
            isEquipping={isEquipping}
            canEquip={canEquipSelectedItem()}
            canUnequip={!!(selectedItem?.equipped && selectedItem.slot && selectedItemRawData)}
            onDestroy={selectedItem && selectedItemInventoryIndex !== null ? handleDeleteItem : undefined}
          />
        </div>
      </div>

      {showDeleteConfirm && itemToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
          <Card className="max-w-md w-full mx-4">
            <CardContent className="p-6">
              <h3 className="text-lg font-semibold mb-4">
                {t('inventory.confirmDeleteTitle')}
              </h3>
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
  );
}