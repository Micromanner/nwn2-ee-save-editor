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

  // Load character by ID
  const loadCharacter = useCallback(async (characterId: number) => {
    setIsLoading(true);
    setError(null);
    
    try {
      // Try to get comprehensive character state first
      const characterData = await CharacterAPI.getCharacterState(characterId);
      setCharacter(characterData);
      setCurrentCharacterId(characterId);
    } catch (err) {
      // Fallback to basic character details if state endpoint fails
      try {
        const characterData = await CharacterAPI.getCharacterDetails(characterId);
        setCharacter(characterData);
        setCurrentCharacterId(characterId);
      } catch (fallbackErr) {
        const errorMessage = fallbackErr instanceof Error ? fallbackErr.message : 'Failed to load character';
        setError(errorMessage);
        console.error('Failed to load character:', fallbackErr);
      }
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Import character from save file
  const importCharacter = useCallback(async (savePath: string) => {
    setIsLoading(true);
    setError(null);
    
    try {
      const importResult = await CharacterAPI.importCharacter(savePath);
      // After import, load the full character data
      await loadCharacter(importResult.id);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to import character';
      setError(errorMessage);
      console.error('Failed to import character:', err);
    } finally {
      setIsLoading(false);
    }
  }, [loadCharacter]);

  // Refresh current character
  const refreshCharacter = useCallback(async () => {
    if (currentCharacterId !== null) {
      await loadCharacter(currentCharacterId);
    }
  }, [currentCharacterId, loadCharacter]);

  // Auto-load first character on mount (for development)
  useEffect(() => {
    // In production, this would be triggered by user action
    // For now, try to load the first available character
    const loadFirstCharacter = async () => {
      try {
        const characters = await CharacterAPI.listCharacters();
        if (characters.length > 0) {
          await loadCharacter(characters[0].id!);
        }
      } catch (err) {
        console.log('No characters available to load');
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