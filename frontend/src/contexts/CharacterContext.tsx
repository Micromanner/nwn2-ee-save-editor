'use client';

import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react';
import { CharacterAPI, CharacterData } from '@/services/characterApi';

// Subsystem data interfaces
export interface AbilitiesData {
  abilities?: Record<string, number>;
  effective_attributes?: Record<string, number>;
  derived_stats?: {
    hit_points?: {
      current?: number;
      maximum?: number;
    };
    [key: string]: unknown;
  };
}

export interface CombatData {
  base_attack_bonus?: number | {
    total_bab?: number;
    [key: string]: unknown;
  };
  armor_class?: {
    total?: number;
    [key: string]: unknown;
  };
  attack_bonuses?: {
    melee?: number;
    ranged?: number;
    melee_attack_bonus?: number;
    ranged_attack_bonus?: number;
  };
  initiative?: number | {
    total?: number;
    [key: string]: unknown;
  };
  fortitude?: number | {
    total?: number;
    [key: string]: unknown;
  };
  reflex?: number | {
    total?: number;
    [key: string]: unknown;
  };
  will?: number | {
    total?: number;
    [key: string]: unknown;
  };
  summary?: {
    spent_points?: number;
    total_feats?: number;
    base_attack_bonus?: number;
  };
}

export interface SkillsData {
  skill_points_available?: number;
  spent_points?: number;
  available_points?: number;
  total_available?: number;
  overspent?: number;
  total_ranks?: number;
  skills_with_ranks?: number;
  class_skills?: Array<{
    id: number;
    name: string;
    key_ability: string;
    current_ranks: number;
    max_ranks: number;
    total_modifier: number;
    is_class_skill: boolean;
    armor_check: boolean;
  }>;
  cross_class_skills?: Array<{
    id: number;
    name: string;
    key_ability: string;
    current_ranks: number;
    max_ranks: number;
    total_modifier: number;
    is_class_skill: boolean;
    armor_check: boolean;
  }>;
  error: string | null;
}

export interface FeatsData {
  summary?: {
    total?: number;
    spent_points?: number;
  };
  error?: string | null;
  [key: string]: unknown;
}

export interface SavesData {
  fortitude?: number | { total?: number; [key: string]: unknown };
  reflex?: number | { total?: number; [key: string]: unknown };
  will?: number | { total?: number; [key: string]: unknown };
  [key: string]: unknown;
}

export interface ClassesData {
  classes?: Array<{
    id: number;
    level: number;
    name: string;
  }>;
  total_level?: number;
  multiclass?: boolean;
  can_multiclass?: boolean;
  [key: string]: unknown;
}

export interface SpellsData {
  [key: string]: unknown;
}

export interface InventoryData {
  [key: string]: unknown;
}

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
  inventory: { endpoint: 'inventory/summary' }, // Updated to get inventory summary
  abilityScores: { endpoint: 'abilities' },
  combat: { endpoint: 'combat/state' },
  saves: { endpoint: 'saves/summary' }, // Updated to match backend
  classes: { endpoint: 'classes/state' },
};

// Subsystem type mappings
interface SubsystemTypeMap {
  feats: FeatsData;
  spells: SpellsData;
  skills: SkillsData;
  inventory: InventoryData;
  abilityScores: AbilitiesData;
  combat: CombatData;
  saves: SavesData;
  classes: ClassesData;
}

// Context state interface
interface CharacterContextState {
  // Core character data
  character: CharacterData | null;
  characterId: number | null;
  isLoading: boolean;
  error: string | null;
  
  // Typed subsystem data store
  subsystems: {
    feats: SubsystemData<FeatsData>;
    spells: SubsystemData<SpellsData>;
    skills: SubsystemData<SkillsData>;
    inventory: SubsystemData<InventoryData>;
    abilityScores: SubsystemData<AbilitiesData>;
    combat: SubsystemData<CombatData>;
    saves: SubsystemData<SavesData>;
    classes: SubsystemData<ClassesData>;
  };
  
  // Actions
  loadCharacter: (characterId: number) => Promise<void>;
  importCharacter: (savePath: string) => Promise<void>;
  loadSubsystem: (subsystem: SubsystemType, force?: boolean) => Promise<unknown>;
  updateSubsystem: (subsystem: SubsystemType, data: unknown) => Promise<void>;
  updateSubsystemData: (subsystem: SubsystemType, data: unknown) => void;
  clearCharacter: () => void;
  refreshAll: () => Promise<void>;
}

// Create context
const CharacterContext = createContext<CharacterContextState | undefined>(undefined);

// Initialize subsystems state
const initializeSubsystems = (): CharacterContextState['subsystems'] => {
  return {
    feats: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    spells: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    skills: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    inventory: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    abilityScores: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    combat: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    saves: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
    classes: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
  };
};

// Provider component
export function CharacterProvider({ children }: { children: ReactNode }) {
  const [characterId, setCharacterId] = useState<number | null>(null);
  const [character, setCharacter] = useState<CharacterData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [subsystems, setSubsystems] = useState<CharacterContextState['subsystems']>(initializeSubsystems());

  // Generic subsystem loader - always fetch fresh, no caching
  const loadSubsystem = useCallback(async (subsystem: SubsystemType, _force = false): Promise<unknown> => { // eslint-disable-line @typescript-eslint/no-unused-vars
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
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL!}/characters/${characterId}/${config.endpoint}`);
      
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
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL!}/characters/${characterId}/${SUBSYSTEM_CONFIG[subsystem].endpoint}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
      });

      if (!response.ok) {
        // Revert on failure and reload
        await loadSubsystem(subsystem);
        throw new Error('Failed to update');
      }
    } catch (_err) {
      console.error(`Failed to update ${subsystem}:`, _err);
      throw _err;
    }
  }, [characterId, loadSubsystem]);

  // Update subsystem data directly without HTTP request (for using API response data)
  const updateSubsystemData = useCallback((subsystem: SubsystemType, data: unknown) => {
    setSubsystems(prev => ({
      ...prev,
      [subsystem]: {
        ...prev[subsystem],
        data,
        lastFetched: new Date()
      }
    }));
  }, []);

  // Load character
  const loadCharacter = useCallback(async (id: number) => {
    setIsLoading(true);
    setError(null);
    console.log('CharacterContext - Loading character with ID:', id);
    
    try {
      const data = await CharacterAPI.getCharacterState(id);
      console.log('CharacterContext - Received character data:', data);
      setCharacter(data);
      setCharacterId(id);
      
      // Reset subsystems when loading new character
      setSubsystems(initializeSubsystems());
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load character';
      setError(errorMessage);
      console.error('CharacterContext - Failed to load character:', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Import character from save
  const importCharacter = useCallback(async (savePath: string) => {
    setIsLoading(true);
    setError(null);
    
    try {
      // Step 1: Import the save game (creates backend session)
      const importResponse = await CharacterAPI.importCharacter(savePath);
      const characterId = importResponse.id;
      
      if (!characterId) {
        throw new Error('Import successful but no character ID returned');
      }
      
      // Step 2: Fetch complete character state from backend session
      const characterData = await CharacterAPI.getCharacterState(characterId);
      
      // Step 3: Populate frontend context with complete data
      setCharacter(characterData);
      setCharacterId(characterId);
      
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
    updateSubsystemData,
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

// Typed hook for specific subsystems with proper type inference
export function useSubsystem<K extends SubsystemType>(subsystem: K): {
  data: SubsystemTypeMap[K] | null;
  isLoading: boolean;
  error: string | null;
  lastFetched: Date | null;
  load: (force?: boolean) => Promise<unknown>;
  updateData: (newData: SubsystemTypeMap[K]) => void;
} {
  const { subsystems, loadSubsystem, updateSubsystemData } = useCharacterContext();
  
  const subsystemData = subsystems[subsystem];
  
  // Update data directly without HTTP request (for using API response data)
  const updateData = useCallback((newData: SubsystemTypeMap[K]) => {
    updateSubsystemData(subsystem, newData);
  }, [subsystem, updateSubsystemData]);
  
  return {
    data: subsystemData.data as SubsystemTypeMap[K] | null,
    isLoading: subsystemData.isLoading,
    error: subsystemData.error,
    lastFetched: subsystemData.lastFetched,
    load: (force?: boolean) => loadSubsystem(subsystem, force),
    updateData,
  };
}