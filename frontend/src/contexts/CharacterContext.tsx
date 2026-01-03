'use client';

import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react';
import { CharacterAPI, CharacterData } from '@/services/characterApi';
import DynamicAPI from '@/lib/utils/dynamicApi';

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

// Add metadata interfaces for classes
export interface ClassInfo {
  id: number;
  name: string;
  label: string;
  type: 'base' | 'prestige';
  focus: string;
  max_level: number;
  hit_die: number;
  skill_points: number;
  is_spellcaster: boolean;
  has_arcane: boolean;
  has_divine: boolean;
  primary_ability: string;
  bab_progression: string;
  alignment_restricted: boolean;
  description?: string;
  prerequisites?: Record<string, unknown>;
}

export interface FocusInfo {
  id: string;
  name: string;
  description: string;
  icon: string;
}

export interface CategorizedClassesResponse {
  categories: {
    base: Record<string, ClassInfo[]>;
    prestige: Record<string, ClassInfo[]>;
    npc: Record<string, ClassInfo[]>;
  };
  focus_info: Record<string, FocusInfo>;
  total_classes: number;
  character_context?: {
    current_classes: unknown;
    prestige_requirements?: unknown[];
    can_multiclass: boolean;
    multiclass_slots_used: number;
  };
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
  inventory: { endpoint: 'inventory' },
  abilityScores: { endpoint: 'abilities' },
  combat: { endpoint: 'combat/state' },
  saves: { endpoint: 'saves/summary' }, // Updated to match backend
  classes: { endpoint: 'classes/state' },
};

// Subsystem dependency map - defines which subsystems need refresh when another updates
// Used by hooks to trigger silent refresh of dependent subsystems after updates
// Example: When inventory changes (equip/unequip), refresh abilityScores, combat, saves, skills
// eslint-disable-next-line @typescript-eslint/no-unused-vars
const SUBSYSTEM_DEPENDENCIES: Record<SubsystemType, SubsystemType[]> = {
  abilityScores: ['abilityScores', 'combat', 'saves', 'skills'], // Self + dependents for AC/stats refresh
  inventory: ['abilityScores', 'combat', 'saves', 'skills'], // Equipment affects stats, AC, saves, skills
  classes: ['abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells'], // Class/level affects everything
  combat: [],
  saves: [],
  skills: [],
  feats: ['combat'], // Feats affect BAB, AC, saves
  spells: [],
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
  
  // Metadata store
  categorizedClasses: CategorizedClassesResponse | null;
  isMetadataLoading: boolean;
  
  // Persistent counts (prevent flickering/reset)
  totalFeats: number;
  totalSpells: number;
  setTotalFeats: (count: number) => void;
  setTotalSpells: (count: number) => void;
  
  // Actions
  loadCharacter: (characterId: number) => Promise<void>;
  importCharacter: (savePath: string) => Promise<void>;
  loadSubsystem: (subsystem: SubsystemType, options?: { force?: boolean; silent?: boolean }) => Promise<unknown>;
  updateSubsystem: (subsystem: SubsystemType, data: unknown) => Promise<void>;
  updateSubsystemData: (subsystem: SubsystemType, data: unknown) => void;
  invalidateSubsystems: (subsystems: SubsystemType[]) => Promise<void>;
  clearCharacter: () => void;

  refreshAll: () => Promise<void>;
  loadMetadata: () => Promise<void>;
  updateCharacterPartial: (data: Partial<CharacterData>) => void;
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
  const [categorizedClasses, setCategorizedClasses] = useState<CategorizedClassesResponse | null>(null);
  const [isMetadataLoading, setIsMetadataLoading] = useState(false);
  const [totalFeats, setTotalFeats] = useState<number>(0);
  const [totalSpells, setTotalSpells] = useState<number>(0);

  // Generic subsystem loader - always fetch fresh, no caching
  const loadSubsystem = useCallback(async (
    subsystem: SubsystemType,
    options: { force?: boolean; silent?: boolean } = {}
  ): Promise<unknown> => {
    if (!characterId) {
      console.warn(`Cannot load ${subsystem}: No character loaded`);
      return null;
    }

    const config = SUBSYSTEM_CONFIG[subsystem];
    const { silent = false } = options;

    // Always fetch fresh data - no caching
    console.log(`Loading fresh ${subsystem} data${silent ? ' (silent)' : ''}`);

    // Update loading state (skip if silent)
    if (!silent) {
      setSubsystems(prev => ({
        ...prev,
        [subsystem]: { ...prev[subsystem], isLoading: true, error: null }
      }));
    }

    try {
      const response = await DynamicAPI.fetch(`/characters/${characterId}/${config.endpoint}`, {
        cache: options.force ? 'reload' : 'default'
      });

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
      const response = await DynamicAPI.fetch(`/characters/${characterId}/${SUBSYSTEM_CONFIG[subsystem].endpoint}`, {
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

  // Invalidate and silently refresh multiple subsystems
  const invalidateSubsystems = useCallback(async (subsystems: SubsystemType[]) => {
    if (!characterId) return;

    const refreshPromises = subsystems.map(subsystem =>
      loadSubsystem(subsystem, { silent: true }).catch(err =>
        console.error(`Silent refresh failed for ${subsystem}:`, err)
      )
    );

    await Promise.all(refreshPromises);
  }, [characterId, loadSubsystem]);

  // Clear character data and close backend session
  const clearCharacter = useCallback(async () => {
    // Close backend session if one exists
    if (characterId) {
      try {
        await DynamicAPI.fetch(`/session/characters/${characterId}/session/stop`, {
          method: 'DELETE'
        });
        console.log(`Backend session closed for character ${characterId}`);
      } catch (err) {
        // Log but don't throw - we still want to clear frontend state
        console.warn('Failed to close backend session:', err);
      }
    }
    
    setCharacterId(null);
    setCharacter(null);
    setError(null);
    setSubsystems(initializeSubsystems());
    setCategorizedClasses(null);
  }, [characterId]);

  const loadMetadataInternal = useCallback(async (id: number) => {
    setIsMetadataLoading(true);
    try {
      const response = await DynamicAPI.fetch(`/characters/${id}/classes/categorized?include_unplayable=true`);
      if (response.ok) {
        const data = await response.json();
        setCategorizedClasses(data);
      }
    } catch (err) {
      console.error('Failed to load class metadata:', err);
    } finally {
      setIsMetadataLoading(false);
    }
  }, []);

  const loadMetadata = useCallback(async () => {
    if (characterId) {
      await loadMetadataInternal(characterId);
    }
  }, [characterId, loadMetadataInternal]);

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
      
      // Load metadata
      await loadMetadataInternal(id);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load character';
      setError(errorMessage);
      console.error('CharacterContext - Failed to load character:', err);
    } finally {
      setIsLoading(false);
    }
  }, [loadMetadataInternal]);

  // Import character from save
  const importCharacter = useCallback(async (savePath: string) => {
    setIsLoading(true);
    setError(null);
    
    try {
      // Close any existing backend session first
      if (characterId) {
        try {
          await DynamicAPI.fetch(`/session/characters/${characterId}/session/stop`, {
            method: 'DELETE'
          });
          console.log(`Closed previous session for character ${characterId} before importing new save`);
        } catch (err) {
          console.warn('Failed to close previous backend session:', err);
        }
      }
      
      // Step 1: Import the save game (creates backend session)
      const importResponse = await CharacterAPI.importCharacter(savePath);
      const newCharacterId = importResponse.id;
      
      if (!newCharacterId) {
        throw new Error('Import successful but no character ID returned');
      }
      
      // Step 2: Fetch complete character state from backend session
      const characterData = await CharacterAPI.getCharacterState(newCharacterId);
      
      // Step 3: Populate frontend context with complete data
      setCharacter(characterData);
      setCharacterId(newCharacterId);
      
      // Reset subsystems
      setSubsystems(initializeSubsystems());
      
      // Load metadata immediately after import
      await loadMetadataInternal(newCharacterId);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to import character';
      setError(errorMessage);
      console.error('Failed to import character:', err);
    } finally {
      setIsLoading(false);
    }

  }, [characterId, loadMetadataInternal]);



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

  // partial update without reload
  const updateCharacterPartial = useCallback((data: Partial<CharacterData>) => {
      setCharacter(prev => prev ? { ...prev, ...data } : null);
  }, []);

  const value: CharacterContextState = {
    character,
    characterId,
    isLoading,
    error,
    subsystems,
    categorizedClasses,
    isMetadataLoading,
    loadCharacter,
    importCharacter,
    loadSubsystem,
    updateSubsystem,
    updateSubsystemData,
    invalidateSubsystems,
    clearCharacter,
    refreshAll,
    loadMetadata,
    totalFeats,
    totalSpells,
    setTotalFeats,
    setTotalSpells,
    updateCharacterPartial,
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
  load: (options?: { force?: boolean; silent?: boolean }) => Promise<unknown>;
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
    load: (options?: { force?: boolean; silent?: boolean }) => loadSubsystem(subsystem, options),
    updateData,
  };
}