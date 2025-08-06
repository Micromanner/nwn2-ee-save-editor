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
}



const INVENTORY_COLS = 8;
const INVENTORY_ROWS = 8;

export default function InventoryEditor() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const inventoryData = useSubsystem('inventory');
  
  // Load inventory data only if character exists and data hasn't been loaded
  useEffect(() => {
    if (character && !inventoryData.data && !inventoryData.isLoading) {
      inventoryData.load();
    }
  }, [character, inventoryData.data, inventoryData.isLoading]);
  
  // Mock items for testing
  const mockItems: Item[] = [
    { id: '1', name: 'Longsword +1', type: 'weapon', rarity: 'uncommon' },
    { id: '2', name: 'Healing Potion', type: 'consumable', stackSize: 5, maxStack: 20, rarity: 'common' },
    { id: '3', name: 'Chainmail', type: 'armor', rarity: 'common' },
    { id: '4', name: 'Ring of Protection', type: 'accessory', rarity: 'rare' },
    { id: '5', name: 'Flaming Greatsword', type: 'weapon', rarity: 'epic' },
    { id: '6', name: 'Amulet of Natural Armor', type: 'accessory', rarity: 'uncommon' },
    { id: '7', name: 'Boots of Speed', type: 'armor', rarity: 'rare' },
    { id: '8', name: 'Scroll of Fireball', type: 'consumable', stackSize: 3, maxStack: 10, rarity: 'uncommon' },
  ];

  // Initialize inventory with some items
  const initInventory = () => {
    const inv = Array(INVENTORY_COLS * INVENTORY_ROWS).fill(null);
    inv[0] = mockItems[0];
    inv[1] = mockItems[1];
    inv[8] = mockItems[2];
    inv[9] = mockItems[3];
    inv[16] = mockItems[4];
    inv[17] = mockItems[5];
    inv[24] = mockItems[6];
    inv[25] = mockItems[7];
    return inv;
  };

  const [inventory, setInventory] = useState<(Item | null)[]>(initInventory());
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterType, setFilterType] = useState<string>('all');

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
              
              {/* Character Model Placeholder */}
              <div className="relative mx-auto w-48 h-64 mb-6">
                <div className="absolute inset-0 bg-gradient-to-b from-[rgb(var(--color-surface-1))] to-[rgb(var(--color-surface-2))] rounded-lg border border-[rgb(var(--color-surface-border)/0.6)] flex items-center justify-center">
                  <svg className="w-24 h-24 text-[rgb(var(--color-text-muted))]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                  </svg>
                </div>
                
                {/* Equipment Slots */}
                <div className="absolute -left-16 top-0 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Head" />
                <div className="absolute -right-16 top-0 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Neck" />
                <div className="absolute -left-16 top-16 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Cloak" />
                <div className="absolute -right-16 top-16 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Chest" />
                <div className="absolute -left-20 top-32 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Left Hand" />
                <div className="absolute -right-20 top-32 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Right Hand" />
                <div className="absolute -left-16 bottom-16 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Gloves" />
                <div className="absolute -right-16 bottom-16 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Belt" />
                <div className="absolute -left-16 bottom-0 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Ring Left" />
                <div className="absolute -right-16 bottom-0 w-12 h-12 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)]" title="Ring Right" />
              </div>
              
              {/* Quick Slots */}
              <div className="space-y-2">
                <h4 className="text-sm font-medium text-[rgb(var(--color-text-secondary))]">{t('inventory.quickSlots')}</h4>
                <div className="grid grid-cols-3 gap-2">
                  {['arrow', 'bullet', 'bolt'].map((slotId) => (
                    <div key={slotId} className="bg-[rgb(var(--color-surface-1))] rounded p-2 border border-[rgb(var(--color-surface-border)/0.6)]">
                      <div className="w-full h-12 bg-[rgb(var(--color-surface-2))] rounded border border-[rgb(var(--color-surface-border)/0.4)] flex items-center justify-center">
                        <span className="text-xs text-[rgb(var(--color-text-muted))] capitalize">{slotId}s</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </CardContent>
          </Card>
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
                    {t('inventory.weight')}: <span className="text-[rgb(var(--color-text-secondary))]">45.2 / 150 lbs</span>
                  </span>
                  <span className="text-[rgb(var(--color-text-muted))]">
                    {t('inventory.gold')}: <span className="text-[rgb(var(--color-warning))]">1,234</span>
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

      {/* Item Details Panel */}
      {selectedItem && (
        <Card>
          <CardContent className="p-6">
            <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))] mb-4">{t('inventory.itemDetails')}</h3>
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
              <div className="space-y-2">
                <div className="w-24 h-24 bg-[rgb(var(--color-surface-1))] rounded border border-[rgb(var(--color-surface-border)/0.6)] mx-auto" />
                <h4 className={`text-center font-medium ${getRarityTextColor(selectedItem.rarity)}`}>
                  {selectedItem.name}
                </h4>
              </div>
              <div className="md:col-span-2 space-y-2">
                <p className="text-sm text-[rgb(var(--color-text-muted))]">{t('inventory.type')}: {selectedItem.type}</p>
                <p className="text-sm text-[rgb(var(--color-text-muted))]">{t('inventory.rarity')}: {selectedItem.rarity || 'common'}</p>
                {selectedItem.stackSize && (
                  <p className="text-sm text-[rgb(var(--color-text-muted))]">
                    {t('inventory.stack')}: {selectedItem.stackSize} / {selectedItem.maxStack}
                  </p>
                )}
                <div className="pt-4">
                  <Button className="mr-2">
                    {t('actions.equip')}
                  </Button>
                  <Button variant="danger">
                    {t('actions.destroy')}
                  </Button>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}