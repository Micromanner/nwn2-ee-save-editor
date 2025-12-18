'use client';

import React, { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { inventoryAPI } from '@/services/inventoryApi';
import { useToast } from '@/contexts/ToastContext';

interface InventorySidebarFooterProps {
  encumbrance?: {
    total_weight: number | string;
    light_load: number | string;
    medium_load: number | string;
    heavy_load: number | string;
    encumbrance_level: string;
  };
}

export default function InventorySidebarFooter({ encumbrance }: InventorySidebarFooterProps) {
  const t = useTranslations();
  const { character, refreshAll } = useCharacterContext();
  const { showToast } = useToast();
  
  // Gold State
  const [goldValue, setGoldValue] = useState<string>('');
  const [isUpdatingGold, setIsUpdatingGold] = useState(false);

  // Sync gold value when character changes
  useEffect(() => {
    if (character?.gold !== undefined) {
      setGoldValue(character.gold.toString());
    }
  }, [character?.gold]);

  const handleUpdateGold = async () => {
    if (!character?.id || isUpdatingGold) return;

    const cleanValue = goldValue.replace(/,/g, '');
    const numericValue = parseInt(cleanValue, 10);

    if (isNaN(numericValue) || numericValue < 0 || numericValue > 2147483647) {
      showToast(t('inventory.invalidGold'), 'error');
      setGoldValue(character?.gold?.toString() || '0');
      return;
    }

    if (numericValue === character?.gold) return;

    setIsUpdatingGold(true);
    try {
      const response = await inventoryAPI.updateGold(character.id, numericValue);
      if (response.success) {
        showToast(t('inventory.goldUpdated'), 'success');
        // Refresh all character data to keep context in sync
        if (refreshAll) await refreshAll();
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

  // Safe number parsing
  const safeToNumber = (value: unknown, defaultValue: number = 0): number => {
    if (typeof value === 'number') return value;
    if (typeof value === 'string') {
      const parsed = parseFloat(value);
      return isNaN(parsed) ? defaultValue : parsed;
    }
    return defaultValue;
  };

  const currentWeight = safeToNumber(encumbrance?.total_weight);
  const maxWeight = safeToNumber(encumbrance?.heavy_load, 150);
  const weightPercentage = Math.min(100, (currentWeight / maxWeight) * 100);

  // Determine progress bar color
  let progressBarColor = 'bg-[rgb(var(--color-success))]';
  if (weightPercentage > 66) progressBarColor = 'bg-[rgb(var(--color-error))]';
  else if (weightPercentage > 33) progressBarColor = 'bg-[rgb(var(--color-warning))]';

  if (!character) return null;

  const hasGoldChanged = goldValue !== (character?.gold?.toString() || '0');

  return (
    <div className="w-full pt-4 border-t border-[rgb(var(--color-surface-border)/0.4)] space-y-4">
        {/* Carry Weight Section */}
        <div className="space-y-2">
             <div className="flex justify-between items-end">
                <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider">
                    {t('inventory.weight')}
                </h4>
                <span className="text-sm font-medium text-[rgb(var(--color-text-primary))]">
                    {currentWeight.toFixed(1)} <span className="text-[rgb(var(--color-text-muted))]">/ {maxWeight.toFixed(0)} lbs</span>
                </span>
            </div>

            <div className="h-2 w-full bg-[rgb(var(--color-surface-2))] rounded-full overflow-hidden border border-[rgb(var(--color-surface-border))]">
                <div
                    className={`h-full ${progressBarColor} transition-all duration-300`}
                    style={{ width: `${weightPercentage}%` }}
                />
            </div>
            

        </div>

        {/* Gold Section */}
        <div className="space-y-2">
            <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider">
                {t('inventory.gold')}
            </h4>
            <div className="flex items-center gap-2">
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
                    className="flex-1 text-lg font-bold text-[rgb(var(--color-text-primary))] bg-[rgb(var(--color-surface-2))] h-9"
                    disabled={isUpdatingGold}
                />
                <div className="flex gap-1">
                     <Button
                        size="sm"
                        onClick={handleUpdateGold}
                        disabled={isUpdatingGold || !hasGoldChanged}
                        className={`h-9 w-9 p-0 ${!hasGoldChanged ? 'opacity-50' : ''}`}
                        title={t('actions.save')}
                        variant="ghost"
                      >
                        <span className={hasGoldChanged ? "text-[rgb(var(--color-success))]" : "text-[rgb(var(--color-text-muted))]"} >✓</span>
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        onClick={() => setGoldValue(character?.gold?.toString() || '0')}
                        disabled={isUpdatingGold || !hasGoldChanged}
                        className={`h-9 w-9 p-0 ${!hasGoldChanged ? 'opacity-30' : 'opacity-100'}`}
                         title={t('actions.cancel')}
                      >
                        <span className="text-[rgb(var(--color-text-muted))]">✕</span>
                      </Button>
                </div>
            </div>
        </div>
    </div>
  );
}
