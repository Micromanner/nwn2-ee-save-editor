import { useState, useEffect, useCallback } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import DynamicAPI from '@/lib/utils/dynamicApi';

export interface SavesData {
  fortitude: number;
  reflex: number; 
  will: number;
  breakdown?: {
    fortitude: {
      base: number;
      ability: number;
      misc: number;
      total: number;
    };
    reflex: {
      base: number;
      ability: number;
      misc: number;
      total: number;
    };
    will: {
      base: number;
      ability: number;
      misc: number;
      total: number;
    };
  };
}

export function useSaves() {
  const { character } = useCharacterContext();
  const [savesData, setSavesData] = useState<SavesData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadSaves = useCallback(async () => {
    if (!character?.id) return;

    setIsLoading(true);
    setError(null);

    try {
      // Try to fetch saves data from backend
      const response = await DynamicAPI.fetch(`/characters/${character.id}/saves/summary`);
      
      if (response.ok) {
        const data = await response.json();
        setSavesData(data);
      } else {
        // Fallback to character data if saves endpoint doesn't exist
        setSavesData({
          fortitude: character.saves?.fortitude || 0,
          reflex: character.saves?.reflex || 0,
          will: character.saves?.will || 0
        });
      }
    } catch (err) {
      // Fallback to character data
      setSavesData({
        fortitude: character.saves?.fortitude || 0,
        reflex: character.saves?.reflex || 0,
        will: character.saves?.will || 0
      });
      setError(err instanceof Error ? err.message : 'Failed to load saves');
    } finally {
      setIsLoading(false);
    }
  }, [character]);

  useEffect(() => {
    if (character?.id) {
      loadSaves();
    }
  }, [character?.id, loadSaves]);

  // Update saving throw misc bonuses
  const updateSavingThrowBonus = useCallback(async (saveType: 'fortitude' | 'reflex' | 'will', bonus: number) => {
    if (!character?.id) return;
    
    try {
      await CharacterAPI.updateSavingThrows(character.id, { [saveType]: bonus });
      
      // Reload saves data to get updated values
      await loadSaves();
      
    } catch (err) {
      throw err;
    }
  }, [character?.id, loadSaves]);

  return {
    savesData,
    isLoading,
    error,
    reload: loadSaves,
    updateSavingThrowBonus
  };
}