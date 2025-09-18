'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';

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

    const { inventory_items } = inventoryData.summary;
    
    // Convert inventory items to display format
    inventory_items?.forEach((itemInfo: InventoryItem, index: number) => {
      if (index < INVENTORY_COLS * INVENTORY_ROWS && itemInfo) {
        const baseItem = itemInfo.base_item || 0;
        const isCustom = itemInfo.is_custom || false;
        const itemName = itemInfo.name || `Item ${baseItem}`;
        
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
          equipped: false,
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
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<string>('all');

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
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Helmet</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Chest</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Belt</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Boots</span>
                  </div>
                </div>

                {/* Column 2: Neck, Cloak, Gloves */}
                <div className="space-y-2">
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Neck</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Cloak</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">Gloves</span>
                  </div>
                </div>

                {/* Column 3: Left Ring, Right Ring */}
                <div className="space-y-2">
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">L Ring</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">R Ring</span>
                  </div>
                </div>

                {/* Column 4: Left Hand, Right Hand, Ammo */}
                <div className="space-y-2">
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">L Hand</span>
                  </div>
                  <div className="text-center">
                    <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                    <span className="text-xs text-[rgb(var(--color-text-muted))]">R Hand</span>
                  </div>
                </div>
              </div>

              {/* Ammo Row */}
              <div className="grid grid-cols-3 gap-2 mb-6">
                <div className="text-center">
                  <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                  <span className="text-xs text-[rgb(var(--color-text-muted))]">Arrows</span>
                </div>
                <div className="text-center">
                  <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                  <span className="text-xs text-[rgb(var(--color-text-muted))]">Bullets</span>
                </div>
                <div className="text-center">
                  <div className="w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto mb-1" />
                  <span className="text-xs text-[rgb(var(--color-text-muted))]">Bolts</span>
                </div>
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
                      <Button className="w-full" size="sm" disabled={selectedItem.equipped}>
                        {selectedItem.equipped ? 'Equipped' : t('actions.equip')}
                      </Button>
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
                {inventory.map((item, index) => (
                  <div
                    key={index}
                    className={`aspect-square bg-[rgb(var(--color-surface-2))] border-2 ${
                      item ? getRarityColor(item.rarity) : 'border-[rgb(var(--color-surface-border)/0.4)]'
                    } rounded hover:border-[rgb(var(--color-surface-border))] transition-colors cursor-pointer relative group`}
                    onDragOver={handleDragOver}
                    onDrop={(e) => handleDrop(e, index)}
                    onClick={() => setSelectedItem(item)}
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
                      </div>
                    )}
                  </div>
                ))}
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