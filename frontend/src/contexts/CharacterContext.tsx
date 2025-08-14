'use client';

import React, { createContext, useContext, useState, useCallback, useRef, ReactNode } from 'react';
import { CharacterAPI, CharacterData } from '@/services/characterApi';

// Generic subsystem data structure
interface SubsystemData<T = unknown> {
  data: T | null;
  isLoading: boolean;
  error: string | null;
  lastFetched: Date | null;
}

// Define available subsystems
export type SubsystemType = 'feats' | 'spells' | 'skills' | 'inventory' | 'abilityScores' | 'combat' | 'saves' | 'classes';

// Subsystem configuration - no caching, always fetch fresh
const SUBSYSTEM_CONFIG: Record<SubsystemType, { endpoint: string }> = {
  feats: { endpoint: 'feats/state' },
  spells: { endpoint: 'spells/state' },
  skills: { endpoint: 'skills/state' },
  inventory: { endpoint: 'inventory/state' },
  abilityScores: { endpoint: 'attributes/state' },
  combat: { endpoint: 'combat/state' },
  saves: { endpoint: 'saves/state' },
  classes: { endpoint: 'classes/state' },
};

// Context state interface
interface CharacterContextState {
  // Core character data
  character: CharacterData | null;
  characterId: number | null;
  isLoading: boolean;
  error: string | null;
  
  // Generic subsystem data store
  subsystems: Record<SubsystemType, SubsystemData>;
  
  // Actions
  loadCharacter: (characterId: number) => Promise<void>;
  importCharacter: (savePath: string) => Promise<void>;
  loadSubsystem: (subsystem: SubsystemType, force?: boolean) => Promise<unknown>;
  updateSubsystem: (subsystem: SubsystemType, data: unknown) => Promise<void>;
  clearCharacter: () => void;
  refreshAll: () => Promise<void>;
}

// Create context
const CharacterContext = createContext<CharacterContextState | undefined>(undefined);

// Initialize subsystems state
const initializeSubsystems = (): Record<SubsystemType, SubsystemData> => {
  const subsystems = {} as Record<SubsystemType, SubsystemData>;
  
  Object.keys(SUBSYSTEM_CONFIG).forEach((key) => {
    subsystems[key as SubsystemType] = {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    };
  });
  
  return subsystems;
};

// Provider component
export function CharacterProvider({ children }: { children: ReactNode }) {
  const [characterId, setCharacterId] = useState<number | null>(null);
  const [character, setCharacter] = useState<CharacterData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [subsystems, setSubsystems] = useState<Record<SubsystemType, SubsystemData>>(initializeSubsystems());

  // Generic subsystem loader - always fetch fresh, no caching
  const loadSubsystem = useCallback(async (subsystem: SubsystemType, force = false): Promise<unknown> => {
    if (!characterId) {
      console.warn(`Cannot load ${subsystem}: No character loaded`);
      return null;
    }

    const config = SUBSYSTEM_CONFIG[subsystem];

    // Always fetch fresh data - no caching
    console.log(`Loading fresh ${subsystem} data`);

    // Update loading state
    setSubsystems(prev => ({
      ...prev,
      [subsystem]: { ...prev[subsystem], isLoading: true, error: null }
    }));

    try {
      const response = await fetch(`http://localhost:8000/api/characters/${characterId}/${config.endpoint}/`);
      
      if (!response.ok) {
        throw new Error(`Failed to load ${subsystem}: ${response.statusText}`);
      }
      
      const data = await response.json();
      
      // Update subsystem state
      setSubsystems(prev => ({
        ...prev,
        [subsystem]: {
          data,
          isLoading: false,
          error: null,
          lastFetched: new Date()
        }
      }));
      
      return data;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : `Failed to load ${subsystem}`;
      
      setSubsystems(prev => ({
        ...prev,
        [subsystem]: {
          ...prev[subsystem],
          isLoading: false,
          error: errorMessage
        }
      }));
      
      console.error(`Failed to load ${subsystem}:`, err);
      throw err;
    }
  }, [characterId]);

  // Update subsystem data (for optimistic updates)
  const updateSubsystem = useCallback(async (subsystem: SubsystemType, data: unknown) => {
    if (!characterId) return;

    // Optimistically update local state
    setSubsystems(prev => ({
      ...prev,
      [subsystem]: {
        ...prev[subsystem],
        data,
        lastFetched: new Date()
      }
    }));

    // Here you would also send the update to the backend
    try {
      const response = await fetch(`http://localhost:8000/api/characters/${characterId}/${SUBSYSTEM_CONFIG[subsystem].endpoint}/`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
      });

      if (!response.ok) {
        // Revert on failure and reload
        await loadSubsystem(subsystem);
        throw new Error('Failed to update');
      }
    } catch (err) {
      console.error(`Failed to update ${subsystem}:`, err);
      throw err;
    }
  }, [characterId, loadSubsystem]);

  // Load character
  const loadCharacter = useCallback(async (id: number) => {
    setIsLoading(true);
    setError(null);
    
    try {
      const data = await CharacterAPI.getCharacterState(id);
      setCharacter(data);
      setCharacterId(id);
      
      // Reset subsystems when loading new character
      setSubsystems(initializeSubsystems());
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load character';
      setError(errorMessage);
      console.error('Failed to load character:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Import character from save
  const importCharacter = useCallback(async (savePath: string) => {
    setIsLoading(true);
    setError(null);
    
    try {
      const data = await CharacterAPI.importCharacter(savePath);
      setCharacter(data);
      setCharacterId(data.id || null);
      
      // Reset subsystems
      setSubsystems(initializeSubsystems());
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to import character';
      setError(errorMessage);
      console.error('Failed to import character:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Refresh all data
  const refreshAll = useCallback(async () => {
    if (!characterId) return;
    
    // Reload character
    await loadCharacter(characterId);
    
    // Reload all subsystems
    const loadPromises = Object.keys(SUBSYSTEM_CONFIG).map(subsystem =>
      loadSubsystem(subsystem as SubsystemType).catch(err => 
        console.error(`Failed to refresh ${subsystem}:`, err)
      )
    );
    
    await Promise.all(loadPromises);
  }, [characterId, loadCharacter, loadSubsystem]);

  // Clear character data
  const clearCharacter = useCallback(() => {
    setCharacterId(null);
    setCharacter(null);
    setError(null);
    setSubsystems(initializeSubsystems());
  }, []);

  const value: CharacterContextState = {
    character,
    characterId,
    isLoading,
    error,
    subsystems,
    loadCharacter,
    importCharacter,
    loadSubsystem,
    updateSubsystem,
    clearCharacter,
    refreshAll,
  };

  return (
    <CharacterContext.Provider value={value}>
      {children}
    </CharacterContext.Provider>
  );
}

// Hook to use character context
export function useCharacterContext() {
  const context = useContext(CharacterContext);
  if (!context) {
    throw new Error('useCharacterContext must be used within a CharacterProvider');
  }
  return context;
}

// Typed hook for specific subsystems
export function useSubsystem<T = unknown>(subsystem: SubsystemType) {
  const { subsystems, loadSubsystem } = useCharacterContext();
  
  const subsystemData = subsystems[subsystem] as SubsystemData<T>;
  
  return {
    ...subsystemData,
    load: (force?: boolean) => loadSubsystem(subsystem, force),
  };
}