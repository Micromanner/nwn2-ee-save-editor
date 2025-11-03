'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { inventoryAPI } from '@/services/inventoryApi';
import { useToast } from '@/contexts/ToastContext';

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
  is_custom: boolean;
  stack_size: number;
  enhancement: number;
  charges?: number;
  identified: boolean;
  plot: boolean;
  cursed: boolean;
  stolen: boolean;
}

interface InventoryEncumbrance {
  total_weight: number | string;
  light_load: number | string;
  medium_load: number | string;
  heavy_load: number | string;
  encumbrance_level: string;
}

interface InventorySummary {
  total_items: number;
  inventory_items: InventoryItem[];
  equipped_items: Record<string, Record<string, unknown>>;
  custom_items: Record<string, unknown>[];
  encumbrance: InventoryEncumbrance;
}

interface LocalInventoryData {
  summary: InventorySummary;
}



const INVENTORY_COLS = 8;
const INVENTORY_ROWS = 8;

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
  const { character } = useCharacterContext();
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
    Object.values(equipped_items || {}).forEach((equipData: Record<string, unknown>) => {
      if (equipData?.base_item) {
        equippedBaseItems.add(equipData.base_item as number);
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
        
        // Determine item type based on base item type
        const getItemType = (baseItem: number): string => {
          // This is a simplified mapping - could be enhanced with actual base item data
          if (baseItem >= 0 && baseItem <= 40) return 'weapon';  // Rough weapon range
          if (baseItem >= 41 && baseItem <= 80) return 'armor';   // Rough armor range
          if (baseItem >= 81 && baseItem <= 120) return 'accessory'; // Rough accessory range
          return 'misc';
        };
        
        inv[index] = {
          id: `inventory_${itemInfo.index}`,
          name: itemName,
          type: getItemType(baseItem),
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
  const [filterType, setFilterType] = useState<string>('all');

  // Handler to equip an item from inventory
  const handleEquipItem = async (itemData: Record<string, unknown>, slot: string, inventoryIndex?: number | null) => {
    if (!character?.id) return;

    setIsEquipping(true);
    try {
      const response = await inventoryAPI.equipItem(character.id, {
        item_data: itemData,
        slot: slot.toLowerCase().replace(' ', '_'),
        inventory_index: inventoryIndex ?? undefined,
      });

      if (response.success) {
        showToast(response.message, 'success');
        await inventoryData.load();
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

    setIsEquipping(true);
    try {
      const response = await inventoryAPI.unequipItem(character.id, {
        slot: slot.toLowerCase().replace(' ', '_'),
      });

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
      showToast(`Failed to unequip item: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
    } finally {
      setIsEquipping(false);
    }
  };

  // Map BaseItem ID to equipment slot
  const getSlotForBaseItem = (baseItemId: number): string | null => {
    // NWN2 BaseItem to slot mapping
    const baseItemToSlot: Record<number, string> = {
      // Armor
      16: 'chest',
      // Shields
      14: 'left_hand',  // Light Shield
      56: 'left_hand',  // Heavy Shield
      57: 'left_hand',  // Tower Shield
      // Helmets
      85: 'head',
      // Boots
      26: 'boots',
      // Gloves/Gauntlets
      36: 'gloves',
      // Cloaks
      30: 'cloak',
      // Belts
      21: 'belt',
      // Amulets
      1: 'neck',
      // Rings
      52: 'left_ring',  // Can go in either ring slot, default to left
      // Ammunition (BaseItem IDs, not slot indexes!)
      20: 'arrows',
      27: 'bullets',
      25: 'bolts',
    };

    // Check direct mapping first
    if (baseItemToSlot[baseItemId]) {
      return baseItemToSlot[baseItemId];
    }

    // Weapons go to right_hand by default (BaseItem 0-60 range, excluding shields/armor)
    if (baseItemId >= 0 && baseItemId < 60 && ![14, 16, 21, 26, 30, 36, 56, 57].includes(baseItemId)) {
      return 'right_hand';
    }

    return null;
  };

  // Helper function to get equipped item for a slot
  const getEquippedItemForSlot = (slotName: string) => {
    if (!inventoryData.data) return null;
    const summary = (inventoryData.data as unknown as LocalInventoryData).summary;
    const equippedItems = summary?.equipped_items || {};
    
    // Map display slot names to backend slot names (using backend's EQUIPMENT_SLOTS mapping)
    const slotMapping: Record<string, string> = {
      'helmet': 'head',
      'chest': 'chest', 
      'belt': 'belt',
      'boots': 'boots',
      'neck': 'neck',
      'cloak': 'cloak',
      'gloves': 'gloves',
      'l ring': 'left_ring',
      'r ring': 'right_ring', 
      'l hand': 'left_hand',
      'r hand': 'right_hand',
      'arrows': 'arrows',
      'bullets': 'bullets',
      'bolts': 'bolts'
    };
    
    const mappedSlot = slotMapping[slotName.toLowerCase()];
    if (!mappedSlot || !equippedItems[mappedSlot]) return null;
    
    const equipData = equippedItems[mappedSlot] as Record<string, unknown>;
    return {
      name: (equipData.name as string) || `Item ${equipData.base_item}`, // Use backend-provided name
      base_item: equipData.base_item as number,
      is_custom: equipData.custom as boolean
    };
  };

  // Equipment slot component
  const EquipmentSlot = ({ slotName }: { slotName: string }) => {
    const equippedItem = getEquippedItemForSlot(slotName);

    const handleSlotClick = () => {
      if (equippedItem) {
        const summary = (inventoryData.data as unknown as LocalInventoryData).summary;
        const slotMapping: Record<string, string> = {
          'helmet': 'head',
          'chest': 'chest',
          'belt': 'belt',
          'boots': 'boots',
          'neck': 'neck',
          'cloak': 'cloak',
          'gloves': 'gloves',
          'l ring': 'left_ring',
          'r ring': 'right_ring',
          'l hand': 'left_hand',
          'r hand': 'right_hand',
          'arrows': 'arrows',
          'bullets': 'bullets',
          'bolts': 'bolts'
        };

        const mappedSlot = slotMapping[slotName.toLowerCase()];
        const rawItemData = mappedSlot ? (summary?.equipped_items[mappedSlot] as Record<string, unknown>)?.item_data : null;

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
        setSelectedItemInventoryIndex(null); // Equipped items are not in inventory
      }
    };
    
    return (
      <div className="text-center">
        <div 
          className={`w-12 h-12 rounded border-2 mx-auto mb-1 flex items-center justify-center relative transition-colors ${
            equippedItem 
              ? 'bg-[rgb(var(--color-primary)/0.1)] border-[rgb(var(--color-primary))] cursor-pointer hover:bg-[rgb(var(--color-primary)/0.2)]' 
              : 'bg-[rgb(var(--color-surface-1))] border-[rgb(var(--color-surface-border)/0.6)]'
          }`}
          onClick={handleSlotClick}
          role={equippedItem ? "button" : undefined}
          tabIndex={equippedItem ? 0 : undefined}
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
        <span className="text-xs text-[rgb(var(--color-text-muted))]">{slotName}</span>
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

  const getRarityTextColor = (rarity?: string) => {
    switch (rarity) {
      case 'uncommon': return 'text-[rgb(var(--color-success))]';
      case 'rare': return 'text-[rgb(var(--color-primary))]';
      case 'epic': return 'text-[rgb(var(--color-secondary))]';
      case 'legendary': return 'text-[rgb(var(--color-warning))]';
      default: return 'text-[rgb(var(--color-text-primary))]';
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
    <div className="space-y-6 max-w-7xl mx-auto">
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Character Equipment */}
        <div className="lg:col-span-1">
          <Card>
            <CardContent className="p-6">
              <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mb-4">{t('inventory.equipment')}</h3>
              
              {/* Equipment Slots Grid */}
              <div className="grid grid-cols-4 gap-2 mb-6">
                {/* Column 1: Helmet, Chest, Belt, Boots */}
                <div className="space-y-2">
                  <EquipmentSlot slotName="Helmet" />
                  <EquipmentSlot slotName="Chest" />
                  <EquipmentSlot slotName="Belt" />
                  <EquipmentSlot slotName="Boots" />
                </div>

                {/* Column 2: Neck, Cloak, Gloves */}
                <div className="space-y-2">
                  <EquipmentSlot slotName="Neck" />
                  <EquipmentSlot slotName="Cloak" />
                  <EquipmentSlot slotName="Gloves" />
                </div>

                {/* Column 3: Left Ring, Right Ring */}
                <div className="space-y-2">
                  <EquipmentSlot slotName="L Ring" />
                  <EquipmentSlot slotName="R Ring" />
                </div>

                {/* Column 4: Left Hand, Right Hand */}
                <div className="space-y-2">
                  <EquipmentSlot slotName="L Hand" />
                  <EquipmentSlot slotName="R Hand" />
                </div>
              </div>

              {/* Ammo Row */}
              <div className="grid grid-cols-3 gap-2 mb-6">
                <EquipmentSlot slotName="Arrows" />
                <EquipmentSlot slotName="Bullets" />
                <EquipmentSlot slotName="Bolts" />
              </div>
            </CardContent>
          </Card>

          {/* Item Details Panel - Moved here */}
          {selectedItem && (
            <Card className="mt-6">
              <CardContent className="p-6">
                <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mb-4">{t('inventory.itemDetails')}</h3>
                <div className="space-y-4">
                  <div className="text-center">
                    <div className="w-16 h-16 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-2" />
                    <h4 className={`font-medium ${getRarityTextColor(selectedItem.rarity)}`}>
                      {selectedItem.name}
                    </h4>
                  </div>
                  <div className="space-y-2">
                    <p className="text-sm text-[rgb(var(--color-text-muted))]">{t('inventory.type')}: {selectedItem.type}</p>
                    <p className="text-sm text-[rgb(var(--color-text-muted))]">{t('inventory.rarity')}: {selectedItem.rarity || 'common'}</p>
                    {selectedItem.slot && (
                      <p className="text-sm text-[rgb(var(--color-text-muted))]">Equipped in: {selectedItem.slot}</p>
                    )}
                    {selectedItem.enhancement_bonus && selectedItem.enhancement_bonus > 0 && (
                      <p className="text-sm text-[rgb(var(--color-text-muted))]">Enhancement: +{selectedItem.enhancement_bonus}</p>
                    )}
                    {selectedItem.charges && (
                      <p className="text-sm text-[rgb(var(--color-text-muted))]">Charges: {selectedItem.charges}</p>
                    )}
                    {selectedItem.stackSize && (
                      <p className="text-sm text-[rgb(var(--color-text-muted))]">
                        {t('inventory.stack')}: {selectedItem.stackSize} / {selectedItem.maxStack}
                      </p>
                    )}
                    {selectedItem.is_custom && (
                      <p className="text-sm text-[rgb(var(--color-warning))]">⚠️ Custom/Modded Item</p>
                    )}
                    {selectedItem.is_plot && (
                      <p className="text-sm text-[rgb(var(--color-primary))]">📜 Plot Item</p>
                    )}
                    {selectedItem.is_cursed && (
                      <p className="text-sm text-[rgb(var(--color-danger))]">💀 Cursed</p>
                    )}
                    {selectedItem.is_stolen && (
                      <p className="text-sm text-[rgb(var(--color-danger))]">🗡️ Stolen</p>
                    )}
                    <div className="pt-2 space-y-2">
                      {selectedItem.equipped && selectedItem.slot && selectedItemRawData && (
                        <Button
                          className="w-full"
                          size="sm"
                          onClick={() => handleUnequipItem(selectedItem.slot!)}
                          disabled={isEquipping}
                        >
                          {isEquipping ? t('actions.unequipping') : t('actions.unequip')}
                        </Button>
                      )}
                      {!selectedItem.equipped && selectedItemRawData && (() => {
                        const baseItemId = (selectedItemRawData as Record<string, unknown>).BaseItem as number;
                        const targetSlot = getSlotForBaseItem(baseItemId);

                        if (!targetSlot) {
                          return (
                            <p className="text-sm text-[rgb(var(--color-text-muted))] text-center">
                              Cannot equip this item type
                            </p>
                          );
                        }

                        // Get slot display name
                        const slotDisplayNames: Record<string, string> = {
                          'head': 'Head',
                          'chest': 'Chest',
                          'boots': 'Boots',
                          'gloves': 'Gloves',
                          'right_hand': 'Right Hand',
                          'left_hand': 'Left Hand',
                          'cloak': 'Cloak',
                          'left_ring': 'Left Ring',
                          'right_ring': 'Right Ring',
                          'neck': 'Neck',
                          'belt': 'Belt',
                          'arrows': 'Arrows',
                          'bullets': 'Bullets',
                          'bolts': 'Bolts',
                        };

                        return (
                          <Button
                            className="w-full"
                            size="sm"
                            onClick={() => handleEquipItem(selectedItemRawData, targetSlot, selectedItemInventoryIndex)}
                            disabled={isEquipping}
                            title={`Equip to ${slotDisplayNames[targetSlot]}`}
                          >
                            {isEquipping ? t('actions.equipping') : t('actions.equip')}
                          </Button>
                        );
                      })()}
                      <Button variant="danger" size="sm" className="w-full" disabled={selectedItem.is_plot}>
                        {selectedItem.is_plot ? 'Cannot Destroy' : t('actions.destroy')}
                      </Button>
                    </div>
                  </div>
                </div>
              </CardContent>
            </Card>
          )}
        </div>

        {/* Inventory Grid */}
        <div className="lg:col-span-2">
          <Card>
            <CardContent className="p-6">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">{t('inventory.inventory')}</h3>
                <div className="flex items-center space-x-2">
                  <Input
                    type="text"
                    placeholder={t('inventory.searchItems')}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="w-48"
                  />
                  <select
                    value={filterType}
                    onChange={(e) => setFilterType(e.target.value)}
                    className="px-3 py-2 bg-[rgb(var(--color-surface-1))] border-2 border-[rgb(var(--color-surface-border)/0.6)] rounded-md text-sm text-[rgb(var(--color-text-primary))] focus:border-[rgb(var(--color-primary))] focus:outline-none transition-colors"
                  >
                    <option value="all">{t('inventory.allItems')}</option>
                    <option value="weapon">{t('inventory.weapons')}</option>
                    <option value="armor">{t('inventory.armor')}</option>
                    <option value="accessory">{t('inventory.accessories')}</option>
                    <option value="consumable">{t('inventory.consumables')}</option>
                    <option value="misc">{t('inventory.miscellaneous')}</option>
                  </select>
                </div>
              </div>

              {/* Inventory Grid */}
              <div className="grid grid-cols-8 gap-1 p-2 bg-[rgb(var(--color-surface-1))] rounded">
                {inventory.map((item, index) => {
                  const summary = (inventoryData.data as unknown as LocalInventoryData)?.summary;
                  const inventoryItem = summary?.inventory_items?.[index];
                  const rawItemData = inventoryItem?.item;

                  return (
                  <div
                    key={index}
                    className={`aspect-square bg-[rgb(var(--color-surface-2))] border-2 ${
                      item ? (item.equipped ? 'border-[rgb(var(--color-primary))] bg-[rgb(var(--color-primary)/0.1)]' : getRarityColor(item.rarity)) : 'border-[rgb(var(--color-surface-border)/0.4)]'
                    } rounded hover:border-[rgb(var(--color-surface-border))] transition-colors cursor-pointer relative group`}
                    onDragOver={handleDragOver}
                    onDrop={(e) => handleDrop(e, index)}
                    onClick={() => {
                      setSelectedItem(item);
                      setSelectedItemRawData(rawItemData as Record<string, unknown> | null);
                      setSelectedItemInventoryIndex(index);
                    }}
                  >
                    {item && (
                      <div
                        draggable
                        onDragStart={(e) => handleDragStart(e, item, index)}
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

              {/* Inventory Info */}
              <div className="mt-4 flex items-center justify-between text-sm">
                <div className="flex items-center space-x-4">
                  <span className="text-[rgb(var(--color-text-muted))]">
                    {t('inventory.weight')}: <span className="text-[rgb(var(--color-text-secondary))]">
                      {safeToNumber((inventoryData.data as unknown as LocalInventoryData)?.summary?.encumbrance?.total_weight).toFixed(1)} / {safeToNumber((inventoryData.data as unknown as LocalInventoryData)?.summary?.encumbrance?.heavy_load, 150).toFixed(0)} lbs
                    </span>
                  </span>
                  <span className="text-[rgb(var(--color-text-muted))]">
                    {t('inventory.gold')}: <span className="text-[rgb(var(--color-warning))]">{character?.gold || 0}</span>
                  </span>
                </div>
                <div className="flex items-center space-x-2">
                  <Button variant="outline" size="sm">
                    {t('actions.sort')}
                  </Button>
                  <Button variant="danger" size="sm">
                    {t('actions.dropAll')}
                  </Button>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}