import { useState, useEffect, useCallback } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useToast } from '@/contexts/ToastContext';
import { inventoryAPI } from '@/services/inventoryApi';

const MAX_GOLD = 2147483647;

export interface UseGoldInputOptions {
  onSuccess?: () => Promise<void>;
}

export interface UseGoldInputResult {
  goldValue: string;
  setGoldValue: (value: string) => void;
  isUpdatingGold: boolean;
  hasGoldChanged: boolean;
  handleUpdateGold: () => Promise<void>;
  handleGoldInputChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  handleGoldKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  resetGold: () => void;
}

export function useGoldInput(options?: UseGoldInputOptions): UseGoldInputResult {
  const t = useTranslations();
  const { character, refreshAll } = useCharacterContext();
  const { showToast } = useToast();

  const [goldValue, setGoldValue] = useState<string>('');
  const [isUpdatingGold, setIsUpdatingGold] = useState(false);

  useEffect(() => {
    if (character?.gold !== undefined) {
      setGoldValue(character.gold.toString());
    }
  }, [character?.gold]);

  const hasGoldChanged = goldValue !== (character?.gold?.toString() || '0');

  const resetGold = useCallback(() => {
    setGoldValue(character?.gold?.toString() || '0');
  }, [character?.gold]);

  const handleUpdateGold = useCallback(async () => {
    if (!character?.id || isUpdatingGold) return;

    const cleanValue = goldValue.replace(/,/g, '');
    const numericValue = parseInt(cleanValue, 10);

    if (isNaN(numericValue) || numericValue < 0 || numericValue > MAX_GOLD) {
      showToast(t('inventory.invalidGold'), 'error');
      resetGold();
      return;
    }

    if (numericValue === character?.gold) return;

    setIsUpdatingGold(true);
    try {
      const response = await inventoryAPI.updateGold(character.id, numericValue);
      if (response.success) {
        showToast(t('inventory.goldUpdated'), 'success');
        if (options?.onSuccess) {
          await options.onSuccess();
        } else if (refreshAll) {
          await refreshAll();
        }
      } else {
        showToast(response.message, 'error');
        resetGold();
      }
    } catch (error) {
      showToast(`Failed to update gold: ${error instanceof Error ? error.message : 'Unknown error'}`, 'error');
      resetGold();
    } finally {
      setIsUpdatingGold(false);
    }
  }, [character?.id, character?.gold, goldValue, isUpdatingGold, showToast, t, resetGold, options, refreshAll]);

  const handleGoldInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (value === '' || /^\d+$/.test(value)) {
      setGoldValue(value);
    }
  }, []);

  const handleGoldKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') handleUpdateGold();
    if (e.key === 'Escape') resetGold();
  }, [handleUpdateGold, resetGold]);

  return {
    goldValue,
    setGoldValue,
    isUpdatingGold,
    hasGoldChanged,
    handleUpdateGold,
    handleGoldInputChange,
    handleGoldKeyDown,
    resetGold,
  };
}
