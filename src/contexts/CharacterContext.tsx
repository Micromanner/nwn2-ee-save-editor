import React, { createContext, useContext, useState, useCallback, useEffect, useMemo, useRef, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CharacterAPI, CharacterData, LegitimateFeatsResponse, LegitimateSpellsResponse } from '@/services/characterApi';
import { inventoryAPI, type ItemEditorMetadataResponse } from '@/services/inventoryApi';
import { CharacterStateAPI } from '@/lib/api/character-state';
import type { RosterEntryInfo } from '@/lib/api/character-state';
import { usePathsChangedEvent } from '@/hooks/usePathsChangedEvent';
import { useTranslations } from '@/hooks/useTranslations';
import type {
  AbilitiesState,
  AppearanceState,
  CombatSummary,
  SkillsState,
  FeatsState,
  SaveSummary,
  ClassesState,
  SpellsState,
  FullInventorySummary,
  HistoryState,
} from '@/lib/bindings';

// Re-export the Rust types as our subsystem data types
export type AbilitiesData = AbilitiesState;
export type CombatData = CombatSummary;
export type SkillsData = SkillsState;
export type FeatsData = FeatsState;
export type SavesData = SaveSummary;
export type ClassesData = ClassesState;
export type SpellsData = SpellsState;
export type InventoryData = FullInventorySummary;
export type AppearanceData = AppearanceState;

export type ActiveSource = { kind: 'player' } | { kind: 'companion'; rosName: string };

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
  prerequisites?: {
    base_attack_bonus: number | null;
    skills: [string, number][];
    feats: string[];
    alignment: string | null;
  } | null;
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
export type SubsystemType = 'feats' | 'spells' | 'skills' | 'inventory' | 'abilityScores' | 'combat' | 'saves' | 'classes' | 'appearance';

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
  appearance: { endpoint: 'appearance/state' },
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
  appearance: AppearanceData;
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
    appearance: SubsystemData<AppearanceData>;
  };
  
  // Metadata store
  categorizedClasses: CategorizedClassesResponse | null;
  isMetadataLoading: boolean;
  
  // Preloaded game data caches (loaded on character import)
  allFeatsCache: LegitimateFeatsResponse | null;
  allSpellsCache: LegitimateSpellsResponse | null;
  itemEditorMetadata: ItemEditorMetadataResponse | null;

  // Persistent counts (prevent flickering/reset)
  totalFeats: number;
  totalSpells: number;
  setTotalFeats: (count: number) => void;
  setTotalSpells: (count: number) => void;
  
  // History
  historyState: HistoryState | null;
  undo: () => Promise<void>;
  redo: () => Promise<void>;

  // Roster / active source (player vs. companion)
  roster: RosterEntryInfo[];
  activeSource: ActiveSource;
  sessionKind: 'save' | 'vault';
  playerName: string | null;
  playerClassId: number | null;
  isPreloading: boolean;
  refreshRoster: () => Promise<void>;
  switchToCompanion: (rosName: string) => Promise<void>;
  switchToPlayer: () => Promise<void>;

  // Actions
  loadCharacter: (characterId: number) => Promise<void>;
  importCharacter: (savePath: string, playerIndex?: number) => Promise<void>;
  adoptReloadedSave: () => Promise<void>;
  loadSubsystem: (subsystem: SubsystemType, options?: { force?: boolean; silent?: boolean }) => Promise<unknown>;
  updateSubsystem: (subsystem: SubsystemType, data: unknown) => Promise<void>;
  updateSubsystemData: (subsystem: SubsystemType, data: unknown) => void;
  invalidateSubsystems: (subsystems: SubsystemType[]) => Promise<void>;
  clearCharacter: () => void;

  refreshAll: () => Promise<void>;
  refreshCharacter: () => Promise<void>;
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
    appearance: {
      data: null,
      isLoading: false,
      error: null,
      lastFetched: null,
    },
  };
};

// Provider component
export function CharacterProvider({ children }: { children: ReactNode }) {
  const t = useTranslations();
  const [characterId, setCharacterId] = useState<number | null>(null);
  const [character, setCharacter] = useState<CharacterData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [subsystems, setSubsystems] = useState<CharacterContextState['subsystems']>(initializeSubsystems());
  const [categorizedClasses, setCategorizedClasses] = useState<CategorizedClassesResponse | null>(null);
  const [isMetadataLoading, setIsMetadataLoading] = useState(false);
  const [totalFeats, setTotalFeats] = useState<number>(0);
  const [totalSpells, setTotalSpells] = useState<number>(0);
  const [allFeatsCache, setAllFeatsCache] = useState<LegitimateFeatsResponse | null>(null);
  const [allSpellsCache, setAllSpellsCache] = useState<LegitimateSpellsResponse | null>(null);
  const [itemEditorMetadata, setItemEditorMetadata] = useState<ItemEditorMetadataResponse | null>(null);
  const [historyState, setHistoryState] = useState<HistoryState | null>(null);
  const [roster, setRoster] = useState<RosterEntryInfo[]>([]);
  const [activeSource, setActiveSource] = useState<ActiveSource>({ kind: 'player' });
  const [sessionKind, setSessionKind] = useState<'save' | 'vault'>('save');
  const [playerName, setPlayerName] = useState<string | null>(null);
  const [playerClassId, setPlayerClassId] = useState<number | null>(null);
  const [isPreloading, setIsPreloading] = useState(false);
  const preloadToken = useRef(0);

  // Keep the player's identity snapshot in sync while the player is the
  // active character, so the sidebar can show it while a companion is loaded.
  useEffect(() => {
    if (activeSource.kind === 'player' && character) {
      setPlayerName(character.name ?? null);
      setPlayerClassId(character.classes?.[0]?.class_id ?? null);
    }
  }, [character, activeSource]);

  // Generic subsystem loader - always fetch fresh, no caching
  const loadSubsystem = useCallback(async (
    subsystem: SubsystemType,
    options: { force?: boolean; silent?: boolean } = {}
  ): Promise<unknown> => {
    if (!characterId) {
      return null;
    }

    const { silent = false } = options;

    // Update loading state (skip if silent)
    if (!silent) {
      setSubsystems(prev => ({
        ...prev,
        [subsystem]: { ...prev[subsystem], isLoading: true, error: null }
      }));
    }

    try {
      let data;
      // Use new aggregated state API for each subsystem
      switch (subsystem) {
        case 'feats':
            data = await CharacterStateAPI.getFeats();
            break;
        case 'spells':
            data = await CharacterStateAPI.getSpells();
            break;
        case 'skills':
            data = await CharacterStateAPI.getSkills();
            break;
        case 'inventory':
            data = await CharacterStateAPI.getInventory();
            break;
        case 'abilityScores':
            data = await CharacterStateAPI.getAbilities();
            break;
        case 'combat':
            data = await CharacterStateAPI.getCombat();
            break;
        case 'saves':
            data = await CharacterStateAPI.getSaves();
            break;
        case 'classes':
            data = await CharacterStateAPI.getClasses();
            break;
        case 'appearance':
            data = await CharacterStateAPI.getAppearance();
            break;
        default:
            throw new Error(`Unknown subsystem: ${subsystem}`);
      }

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
      const errorMessage = err instanceof Error ? err.message : t('errors.loadingFailed');

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
  }, [t, characterId]);

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

    // Updates are now handled via specific mutations in components, 
    // Generic PATCH is not supported in Tauri backend.
    // This method kept for optimistic interface compatibility.
  }, [characterId]);

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

  const refreshHistoryState = useCallback(async () => {
    try {
      const state = await CharacterStateAPI.getHistoryState();
      setHistoryState(state);
    } catch {
      // No character loaded or history not available — ignore silently
    }
  }, []);

  // Invalidate and silently refresh multiple subsystems
  const invalidateSubsystems = useCallback(async (subsystems: SubsystemType[]) => {
    if (!characterId) return;

    // Feat prerequisites compare against abilities, classes, skills and saves,
    // so any edit makes the preloaded list stale; the Feats tab refetches.
    setAllFeatsCache(null);

    const refreshPromises = subsystems.map(subsystem =>
      loadSubsystem(subsystem, { silent: true }).catch(err =>
        console.error(`Silent refresh failed for ${subsystem}:`, err)
      )
    );

    await Promise.all(refreshPromises);
    void refreshHistoryState();
  }, [characterId, loadSubsystem, refreshHistoryState]);

  // Clear character data and close backend session
  const clearCharacter = useCallback(async () => {
    // Session management in Rust is implicit/in-memory
    setCharacterId(null);
    setCharacter(null);
    setError(null);
    setSubsystems(initializeSubsystems());
    setCategorizedClasses(null);
    setAllFeatsCache(null);
    setAllSpellsCache(null);
    setItemEditorMetadata(null);
    setHistoryState(null);
    setRoster([]);
    setActiveSource({ kind: 'player' });
    setSessionKind('save');
    setPlayerName(null);
    setPlayerClassId(null);
  }, []);

  // The backend closes its session when the game paths or the language change.
  usePathsChangedEvent(() => {
    void clearCharacter();
  });

  const allSubsystems: SubsystemType[] = ['feats', 'spells', 'skills', 'inventory', 'abilityScores', 'combat', 'saves', 'classes', 'appearance'];

  // Undo and redo can unstage or restage a quest edit.
  const undo = useCallback(async () => {
    try {
      const result = await CharacterStateAPI.undo();
      if (result.applied) {
        window.dispatchEvent(new Event('quest-edits-changed'));
      }
    } catch (err) {
      console.error('Undo failed:', err);
    }
  }, []);

  const redo = useCallback(async () => {
    try {
      const result = await CharacterStateAPI.redo();
      if (result.applied) {
        window.dispatchEvent(new Event('quest-edits-changed'));
      }
    } catch (err) {
      console.error('Redo failed:', err);
    }
  }, []);

  const loadMetadataInternal = useCallback(async (_id: number) => {
    setIsMetadataLoading(true);
    try {
      const categories = await invoke<CategorizedClassesResponse>('get_all_categorized_classes');
      setCategorizedClasses(categories);
    } catch (err) {
      console.error('Failed to load class metadata:', err);
      setCategorizedClasses(null);
    } finally {
      setIsMetadataLoading(false);
    }
  }, []);

  const loadMetadata = useCallback(async () => {
    if (characterId) {
      await loadMetadataInternal(characterId);
    }
  }, [characterId, loadMetadataInternal]);

  const preloadGameData = useCallback((id: number) => {
    // Track completion so character switching stays locked until every
    // background fetch has landed (a switch mid-preload would let stale
    // subsystem data from the previous character overwrite the new one).
    const token = ++preloadToken.current;
    setIsPreloading(true);

    const tasks: Promise<unknown>[] = [
      CharacterAPI.getLegitimateFeats(id, { page: 1, limit: 10000 })
        .then(setAllFeatsCache)
        .catch(err => console.error('Background preload failed for all feats:', err)),
      CharacterAPI.getLegitimateSpells(id, { page: 1, limit: 10000 })
        .then(setAllSpellsCache)
        .catch(err => console.error('Background preload failed for all spells:', err)),
      inventoryAPI.getEditorMetadata(id)
        .then(setItemEditorMetadata)
        .catch(err => console.error('Background preload failed for item editor metadata:', err)),
    ];

    // Preload all subsystems so tabs open instantly
    const subsystemPreloads: [SubsystemType, () => Promise<unknown>][] = [
      ['feats', CharacterStateAPI.getFeats],
      ['spells', CharacterStateAPI.getSpells],
      ['skills', CharacterStateAPI.getSkills],
      ['inventory', CharacterStateAPI.getInventory],
      ['abilityScores', CharacterStateAPI.getAbilities],
      ['combat', CharacterStateAPI.getCombat],
      ['saves', CharacterStateAPI.getSaves],
      ['classes', CharacterStateAPI.getClasses],
    ];
    for (const [key, fetcher] of subsystemPreloads) {
      tasks.push(
        fetcher()
          .then(data => setSubsystems(prev => ({
            ...prev,
            [key]: { data, isLoading: false, error: null, lastFetched: new Date() }
          })))
          .catch(err => console.error(`Background preload failed for ${key}:`, err))
      );
    }

    Promise.all(tasks).finally(() => {
      if (preloadToken.current === token) setIsPreloading(false);
    });
  }, []);

  const loadCharacter = useCallback(async (id: number) => {
    setIsLoading(true);
    setError(null);

    try {
      const data = await CharacterAPI.getCharacterState(id);
      setCharacter(data);
      setCharacterId(id);

      // Reset subsystems when loading new character
      setSubsystems(initializeSubsystems());
      setHistoryState(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : t('errors.loadingFailed');
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }

    // Load metadata and preload game data in background (non-blocking)
    loadMetadataInternal(id);
    preloadGameData(id);
  }, [t, loadMetadataInternal, preloadGameData]);

  const refreshRoster = useCallback(async () => {
    try {
      setRoster(await CharacterStateAPI.listRoster());
    } catch (err) {
      console.error('Failed to load roster:', err);
      setRoster([]);
    }
  }, []);

  const switchToCompanion = useCallback(async (rosName: string) => {
    await CharacterStateAPI.loadCompanion(rosName);
    setActiveSource({ kind: 'companion', rosName });
    if (characterId) await loadCharacter(characterId);
    await refreshHistoryState();
  }, [characterId, loadCharacter, refreshHistoryState]);

  const switchToPlayer = useCallback(async () => {
    await CharacterStateAPI.loadPlayer();
    setActiveSource({ kind: 'player' });
    if (characterId) await loadCharacter(characterId);
    await refreshHistoryState();
  }, [characterId, loadCharacter, refreshHistoryState]);

  // Take over the session the backend holds. A fresh id makes id-keyed panels refetch.
  const adoptSession = useCallback(async (newCharacterId: number, isVault: boolean) => {
    const characterData = await CharacterAPI.getCharacterState(newCharacterId);

    setCharacter(characterData);
    setCharacterId(newCharacterId);
    setSessionKind(isVault ? 'vault' : 'save');

    setSubsystems(initializeSubsystems());
    setHistoryState(null);
    setActiveSource({ kind: 'player' });
    if (isVault) {
      setRoster([]);
    } else {
      refreshRoster();
    }

    // Load metadata and preload game data in background (non-blocking)
    loadMetadataInternal(newCharacterId);
    preloadGameData(newCharacterId);
  }, [loadMetadataInternal, preloadGameData, refreshRoster]);

  const runSessionLoad = useCallback(async (load: () => Promise<void>) => {
    setIsLoading(true);
    setError(null);
    try {
      await load();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to import character';
      setError(errorMessage);
      throw err;
    } finally {
      setIsLoading(false);
    }
  }, []);

  // Import character from save
  const importCharacter = useCallback((savePath: string, playerIndex?: number) => runSessionLoad(async () => {
    const importResponse = await CharacterAPI.importCharacter(savePath, playerIndex);
    await adoptSession(importResponse.id, savePath.toLowerCase().endsWith('.bic'));
  }), [runSessionLoad, adoptSession]);

  // The backend reloaded the loaded save itself (after a backup restore).
  const adoptReloadedSave = useCallback(
    () => runSessionLoad(() => adoptSession(Date.now(), false)),
    [runSessionLoad, adoptSession],
  );

  const refreshAll = useCallback(async () => {
    if (!characterId) return;

    await loadCharacter(characterId);

    const loadPromises = Object.keys(SUBSYSTEM_CONFIG).map(subsystem =>
      loadSubsystem(subsystem as SubsystemType).catch(() => {})
    );

    await Promise.all(loadPromises);
  }, [characterId, loadCharacter, loadSubsystem]);

  const refreshCharacter = useCallback(async () => {
    if (!characterId) return;
    try {
      const data = await CharacterAPI.getCharacterState(characterId);
      setCharacter(data);
    } catch (err) {
      console.error('Failed to refresh character:', err);
    }
  }, [characterId]);

  // Staged quest edits change the loaded player and the roster in memory.
  useEffect(() => {
    const onQuestEditsChanged = () => {
      void refreshCharacter();
      void invalidateSubsystems(allSubsystems);
      void refreshRoster();
    };
    window.addEventListener('quest-edits-changed', onQuestEditsChanged);
    return () => window.removeEventListener('quest-edits-changed', onQuestEditsChanged);
  }, [refreshCharacter, invalidateSubsystems, refreshRoster]);

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
    allFeatsCache,
    allSpellsCache,
    itemEditorMetadata,
    historyState,
    undo,
    redo,
    roster,
    activeSource,
    sessionKind,
    playerName,
    playerClassId,
    isPreloading,
    refreshRoster,
    switchToCompanion,
    switchToPlayer,
    loadCharacter,
    importCharacter,
    adoptReloadedSave,
    loadSubsystem,
    updateSubsystem,
    updateSubsystemData,
    invalidateSubsystems,
    clearCharacter,
    refreshAll,
    refreshCharacter,
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

  // Memoize load function to prevent infinite re-renders
  const load = useCallback((options?: { force?: boolean; silent?: boolean }) => {
    return loadSubsystem(subsystem, options);
  }, [subsystem, loadSubsystem]);

  // Memoize the returned object so its identity stays stable between renders
  // unless the underlying subsystem state actually changed. Without this,
  // every render produced a fresh object, which retriggered any effect that
  // depended on it (e.g. the load-on-mount effects in the panels).
  return useMemo(() => ({
    data: subsystemData.data as SubsystemTypeMap[K] | null,
    isLoading: subsystemData.isLoading,
    error: subsystemData.error,
    lastFetched: subsystemData.lastFetched,
    load,
    updateData,
  }), [subsystemData.data, subsystemData.isLoading, subsystemData.error, subsystemData.lastFetched, load, updateData]);
}
