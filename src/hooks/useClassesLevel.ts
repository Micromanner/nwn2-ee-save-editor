import { useState, useCallback, useMemo } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { invoke } from '@tauri-apps/api/core';
import type { XpProgress } from '@/lib/bindings';

export interface ClassLevel {
  id: number;
  name: string;
  level: number;
  hitDie: number;
  baseAttackBonus: number;
  fortitudeSave: number;
  reflexSave: number;
  willSave: number;
  skillPoints: number;
  spellcaster: boolean;
  spellType?: 'arcane' | 'divine';
  primaryAbility: string;
  max_level?: number;
  bab_progression?: string;
  alignment_restricted?: boolean;
  focus?: string;
}

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
  icon?: string;
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

// Re-export ClassesState from bindings as ClassesData for compatibility
import type { ClassesState } from '@/lib/bindings';
export type ClassesData = ClassesState;

export type { XpProgress } from '@/lib/bindings';

const CLASS_SUBSYSTEMS = ['classes', 'abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells'] as const;

export function useClassesLevel(classesData?: ClassesData | null) {
  const { characterId, invalidateSubsystems, categorizedClasses, isMetadataLoading } = useCharacterContext();
  const [isUpdating, setIsUpdating] = useState(false);
  const [isLoadingXP] = useState(false);

  // Helper function to find class info from categorized data
  const findClassInfoById = useCallback((classId: number): ClassInfo | undefined => {
    if (!categorizedClasses) return undefined;
    
    for (const classType of ['base', 'prestige', 'npc'] as const) {
      for (const focusClasses of Object.values(categorizedClasses.categories[classType])) {
        const found = (focusClasses as ClassInfo[]).find(cls => cls.id === classId);
        if (found) return found;
      }
    }
    return undefined;
  }, [categorizedClasses]);

  // Transform classesData into frontend format
  // Note: categorizedClasses is optional - basic display works without it
  const classes = useMemo((): ClassLevel[] => {
    if (!classesData) return [];

    // classesData is ClassesState from backend, which has 'entries' field
    const entries = classesData.entries || [];

    return entries.map(entry => {
      // Try to get enriched metadata from categorizedClasses if available
      const classInfo = categorizedClasses ? findClassInfoById(entry.class_id) : undefined;

      return {
        id: entry.class_id,
        name: entry.name,
        level: entry.level,
        hitDie: entry.hit_die || classInfo?.hit_die || 8,
        baseAttackBonus: entry.base_attack_bonus ?? 0,
        fortitudeSave: entry.fortitude_save ?? 0,
        reflexSave: entry.reflex_save ?? 0,
        willSave: entry.will_save ?? 0,
        skillPoints: entry.skill_points_per_level ?? 0,
        spellcaster: classInfo?.is_spellcaster || false,
        spellType: classInfo?.has_arcane ? 'arcane' : classInfo?.has_divine ? 'divine' : undefined,
        primaryAbility: classInfo?.primary_ability || 'STR',
        max_level: classInfo?.max_level,
        bab_progression: classInfo?.bab_progression,
        alignment_restricted: classInfo?.alignment_restricted,
        focus: classInfo?.focus
      };
    });
  }, [classesData, categorizedClasses, findClassInfoById]);

  // Always refresh class-related subsystems after a mutation (success OR failure)
  // so a partial backend failure can't leave the UI showing stale pre-failure state.
  const runWithRefresh = useCallback(async (op: () => Promise<void>) => {
    setIsUpdating(true);
    try {
      await op();
    } finally {
      await invalidateSubsystems([...CLASS_SUBSYSTEMS])
        .catch(err => console.warn('Class subsystem refresh failed', err));
      setIsUpdating(false);
    }
  }, [invalidateSubsystems]);

  const adjustClassLevel = useCallback(async (classId: number, delta: number) => {
    if (!characterId) return;

    const cls = classes.find(c => c.id === classId);
    if (!cls) return;

    const newLevel = Math.max(1, cls.level + delta);
    if (newLevel === cls.level) return;

    if (delta > 0 && cls.max_level && cls.max_level > 0 && newLevel > cls.max_level) {
      throw new Error(`Cannot add level to ${cls.name}: maximum level is ${cls.max_level}, character already has ${cls.level} levels`);
    }

    await runWithRefresh(async () => {
      if (delta > 0) {
        await invoke('add_class_level', { classId, count: delta });
      } else {
        await invoke('remove_class_levels', { classId, count: Math.abs(delta) });
      }
    });
  }, [characterId, classes, runWithRefresh]);

  const changeClass = useCallback(async (classId: number, newClassInfo: ClassInfo) => {
    if (!characterId) return;

    if (classes.some(c => c.id !== classId && c.id === newClassInfo.id)) {
      throw new Error('This class is already assigned to the character');
    }

    await runWithRefresh(async () => {
      await invoke('change_class', {
        oldClassId: classId,
        newClassId: newClassInfo.id,
      });
    });
  }, [characterId, classes, runWithRefresh]);

  const addClass = useCallback(async (classInfo: ClassInfo) => {
    if (!characterId || !classesData) return;

    if (classes.some(c => c.id === classInfo.id)) {
      throw new Error('This class is already assigned to the character');
    }
    if (classes.length >= 4) {
      throw new Error('Maximum of 4 classes allowed');
    }
    if (classesData.total_level >= 60) {
      throw new Error('Maximum level of 60 reached');
    }

    await runWithRefresh(async () => {
      await invoke('add_class_level', { classId: classInfo.id });
    });
  }, [characterId, classes, classesData, runWithRefresh]);

  const removeClass = useCallback(async (classId: number) => {
    if (!characterId) return;

    if (classes.length <= 1) {
      throw new Error('Character must have at least one class');
    }

    await runWithRefresh(async () => {
      await invoke('remove_class_entry', { classId });
    });
  }, [characterId, classes, runWithRefresh]);

  // Helper function to check if a class can level up
  const canLevelUp = useCallback((classId: number): boolean => {
    const cls = classes.find(c => c.id === classId);
    if (!cls) return false;
    
    // Check prestige class level limits
    if (cls.max_level && cls.max_level > 0) {
      return cls.level < cls.max_level;
    }
    
    // Base classes can level up until character level cap
    return (classesData?.total_level || 0) < 60;
  }, [classes, classesData?.total_level]);

  // Helper function to get remaining levels for prestige classes
  const getRemainingLevels = useCallback((classId: number): number | null => {
    const cls = classes.find(c => c.id === classId);
    if (!cls || !cls.max_level || cls.max_level <= 0) return null;
    
    return Math.max(0, cls.max_level - cls.level);
  }, [classes]);

  // Helper function to check if a class is at max level
  const isAtMaxLevel = useCallback((classId: number): boolean => {
    const cls = classes.find(c => c.id === classId);
    if (!cls || !cls.max_level || cls.max_level <= 0) return false;

    return cls.level >= cls.max_level;
  }, [classes]);

  // Fetch XP progress
  const xpProgress = useMemo(() => {
     if (classesData?.xp_progress) {
       return classesData.xp_progress;
     }
     return null;
  }, [classesData]);

  const fetchXPProgress = useCallback(async () => {
    if (!characterId) return;
    try {
      await invalidateSubsystems(['classes']);
    } catch {
      // XP progress fetch failed silently
    }
  }, [characterId, invalidateSubsystems]); 



  // Set experience points
  const setExperience = useCallback(async (xp: number) => {
    if (!characterId) return;

    setIsUpdating(true);
    try {
      await invoke('set_experience', { xp });
    } finally {
      await fetchXPProgress();
      setIsUpdating(false);
    }
  }, [characterId, fetchXPProgress]);

  // Derive multiclass status from entries
  const isMulticlass = (classesData?.entries?.length || 0) > 1;
  const canAddMoreClasses = (classesData?.entries?.length || 0) < 4;

  return {
    // Data from subsystem
    classes,
    totalLevel: classesData?.total_level || 0,
    multiclass: isMulticlass,
    canMulticlass: canAddMoreClasses,

    // XP data
    xpProgress,
    isLoadingXP,

    // Additional data
    categorizedClasses,
    findClassInfoById,

    // State
    isUpdating,
    isMetadataLoading,

    // Actions
    adjustClassLevel,
    changeClass,
    addClass,
    removeClass,
    fetchXPProgress,
    setExperience,

    // Helper functions for prestige class limits
    canLevelUp,
    getRemainingLevels,
    isAtMaxLevel,
  };
}