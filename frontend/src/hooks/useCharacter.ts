import { useState, useEffect, useCallback } from 'react';
import { CharacterAPI, CharacterData } from '@/services/characterApi';

interface UseCharacterResult {
  character: CharacterData | null;
  isLoading: boolean;
  error: string | null;
  loadCharacter: (characterId: number) => Promise<void>;
  importCharacter: (savePath: string) => Promise<void>;
  refreshCharacter: () => Promise<void>;
}

export function useCharacter(): UseCharacterResult {
  const [character, setCharacter] = useState<CharacterData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [currentCharacterId, setCurrentCharacterId] = useState<number | null>(null);

  const loadCharacter = useCallback(async (characterId: number) => {
    setIsLoading(true);
    setError(null);
    
    try {
      const characterData = await CharacterAPI.getCharacterState(characterId);
      setCharacter(characterData);
      setCurrentCharacterId(characterId);
    } catch (_err) { // eslint-disable-line @typescript-eslint/no-unused-vars
      try {
        const characterData = await CharacterAPI.getCharacterDetails(characterId);
        setCharacter(characterData);
        setCurrentCharacterId(characterId);
      } catch (fallbackErr) {
        const errorMessage = fallbackErr instanceof Error ? fallbackErr.message : 'Failed to load character';
        setError(errorMessage);
      }
    } finally {
      setIsLoading(false);
    }
  }, []);

  const importCharacter = useCallback(async (savePath: string) => {
    setIsLoading(true);
    setError(null);
    
    try {
      const importResult = await CharacterAPI.importCharacter(savePath);
      await loadCharacter(importResult.id);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to import character';
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  }, [loadCharacter]);

  const refreshCharacter = useCallback(async () => {
    if (currentCharacterId !== null) {
      await loadCharacter(currentCharacterId);
    }
  }, [currentCharacterId, loadCharacter]);

  useEffect(() => {
    const loadFirstCharacter = async () => {
      try {
        const characters = await CharacterAPI.listCharacters();
        if (characters.length > 0) {
          await loadCharacter(characters[0].id!);
        }
      } catch (_err) { // eslint-disable-line @typescript-eslint/no-unused-vars
        // No characters available
      }
    };

    // Only auto-load in development
    if (process.env.NODE_ENV === 'development') {
      loadFirstCharacter();
    }
  }, [loadCharacter]);

  return {
    character,
    isLoading,
    error,
    loadCharacter,
    importCharacter,
    refreshCharacter
  };
}