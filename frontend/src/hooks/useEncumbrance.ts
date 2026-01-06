import { useMemo } from 'react';
import { safeToNumber } from '@/utils/dataHelpers';

export interface EncumbranceData {
  total_weight: number | string;
  light_load: number | string;
  medium_load: number | string;
  heavy_load: number | string;
  encumbrance_level: string;
}

export interface EncumbranceResult {
  currentWeight: number;
  maxWeight: number;
  weightPercentage: number;
  progressBarColor: string;
}

export function useEncumbrance(encumbrance?: EncumbranceData): EncumbranceResult {
  return useMemo(() => {
    const currentWeight = safeToNumber(encumbrance?.total_weight);
    const maxWeight = safeToNumber(encumbrance?.heavy_load, 150);
    const weightPercentage = Math.min(100, (currentWeight / maxWeight) * 100);

    let progressBarColor = 'bg-[rgb(var(--color-success))]';
    if (weightPercentage > 66) {
      progressBarColor = 'bg-[rgb(var(--color-error))]';
    } else if (weightPercentage > 33) {
      progressBarColor = 'bg-[rgb(var(--color-warning))]';
    }

    return { currentWeight, maxWeight, weightPercentage, progressBarColor };
  }, [encumbrance?.total_weight, encumbrance?.heavy_load]);
}
