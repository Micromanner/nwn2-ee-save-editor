'use client';

import React, { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { display, formatNumber, formatModifier } from '@/utils/dataHelpers'; // Assuming these exist or will be imported
import { inventoryAPI } from '@/services/inventoryApi';
import { useToast } from '@/contexts/ToastContext';

interface InventoryCharacterSummaryProps {
  encumbrance?: {
    total_weight: number | string;
    light_load: number | string;
    medium_load: number | string;
    heavy_load: number | string;
    encumbrance_level: string;
  };
  combatStats?: {
    ac: number;
    bab: number;
  };
}

export default function InventoryCharacterSummary({ encumbrance, combatStats }: InventoryCharacterSummaryProps) {
  const t = useTranslations();
  const { character } = useCharacterContext();
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

  return (
    <Card className="min-w-[320px] h-full">
      <CardContent className="p-6 space-y-8">
        {/* Header: Character Info */}
        <div className="text-center space-y-2">
           <h3 className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
            {display(character.name)}
          </h3>
          <div className="flex flex-col items-center gap-1 text-sm text-[rgb(var(--color-text-secondary))]">
            {character.classes?.map((cls, idx) => (
                <span key={idx} className="px-2 py-0.5 bg-[rgb(var(--color-surface-2))] rounded">
                  {cls.name} {cls.level}
                </span>
            ))}
             <span className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                {character.subrace ? `${character.subrace} ${character.race}` : display(character.race)}
            </span>
          </div>
        </div>

        <div className="border-t border-[rgb(var(--color-surface-border)/0.4)] my-4" />

        {/* Quick Stats Section */}
        <div className="space-y-4">
             <h4 className="text-xs font-semibold uppercase text-[rgb(var(--color-text-muted))] tracking-wider border-b border-[rgb(var(--color-surface-border)/0.4)] pb-2">
                Quick Stats
            </h4>
            <div className="grid grid-cols-2 gap-4">
                 <div className="bg-[rgb(var(--color-surface-2))] p-3 rounded-lg flex flex-col items-center justify-center">
                    <span className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">
                        {combatStats?.ac || '-'}
                    </span>
                    <span className="text-xs text-[rgb(var(--color-text-muted))] uppercase">AC</span>
                 </div>
                 <div className="bg-[rgb(var(--color-surface-2))] p-3 rounded-lg flex flex-col items-center justify-center">
                    <span className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">
                        {combatStats?.bab !== undefined ? formatModifier(combatStats.bab) : '-'}
                    </span>
                    <span className="text-xs text-[rgb(var(--color-text-muted))] uppercase">BAB</span>
                 </div>
            </div>
        </div>

      </CardContent>
    </Card>
  );
}
