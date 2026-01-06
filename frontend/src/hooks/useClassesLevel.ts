import { useState, useCallback, useMemo } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { apiClient } from '@/lib/api/client';

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
  description?: string;
  prerequisites?: Record<string, unknown>;
}

export interface CombatStats {
  base_attack_bonus: number;
  melee_attack_bonus: number;
  ranged_attack_bonus: number;
  multiple_attacks: number[];
  fortitude_save: number;
  reflex_save: number;
  will_save: number;
  base_fortitude: number;
  base_reflex: number;
  base_will: number;
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

export interface ClassesData {
  classes: Array<{
    id: number;
    level: number;
    name: string;
    skill_points?: number;
    bab?: number;
    fort_save?: number;
    ref_save?: number;
    will_save?: number;
    hit_die?: number;
  }>;
  total_level: number;
  multiclass: boolean;
  can_multiclass: boolean;
  combat_stats: CombatStats;
  xp_progress?: XPProgress;
}

export interface XPProgress {
  current_xp: number;
  current_xp_level: number;
  total_class_level: number;
  next_level_xp: number | null;
  xp_to_next: number | null;
  current_level_min_xp: number;
  next_level_min_xp: number;
  progress_percent: number;
}

export function useClassesLevel(classesData?: ClassesData | null) {
  const { characterId, invalidateSubsystems, categorizedClasses, isMetadataLoading } = useCharacterContext();
  const [isUpdating, setIsUpdating] = useState(false);
  const [isLoadingXP, _setIsLoadingXP] = useState(false);

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
  const classes = useMemo((): ClassLevel[] => {
    if (!classesData || !categorizedClasses) return [];
    
    return classesData.classes.map(cls => {
      const classInfo = findClassInfoById(cls.id);
      
      // Use backend calculated skill points directly (now typically available)
      const skillPoints = cls.skill_points || 0;
      
      return {
        id: cls.id,
        name: cls.name,
        level: cls.level,
        hitDie: cls.hit_die ?? classInfo?.hit_die ?? 8,
        baseAttackBonus: cls.bab ?? 0,
        fortitudeSave: cls.fort_save ?? 0,
        reflexSave: cls.ref_save ?? 0,
        willSave: cls.will_save ?? 0,
        skillPoints: skillPoints,
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

  const adjustClassLevel = useCallback(async (classId: number, delta: number) => {
    if (!characterId) return;
    
    const cls = classes.find(c => c.id === classId);
    if (!cls) return;

    const newLevel = Math.max(1, cls.level + delta);
    if (newLevel === cls.level) return;
    
    // Check prestige class level limits
    if (delta > 0 && cls.max_level && cls.max_level > 0) {
      if (newLevel > cls.max_level) {
        throw new Error(`Cannot add level to ${cls.name}: maximum level is ${cls.max_level}, character already has ${cls.level} levels`);
      }
    }
    
    setIsUpdating(true);

    try {
      const response = await apiClient.post(`/characters/${characterId}/classes/level-up`, {
        class_id: classId,
        level_change: delta,
      });

      // Silently refresh all dependent subsystems
      await invalidateSubsystems(['classes', 'abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells']);
      
      return response;
    } catch (err) {
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes, invalidateSubsystems]);

  const changeClass = useCallback(async (classId: number, newClassInfo: ClassInfo) => {
    if (!characterId) return;
    
    // Check if new class is already taken
    if (classes.some(c => c.id !== classId && c.id === newClassInfo.id)) {
      throw new Error('This class is already assigned to the character');
    }
    
    setIsUpdating(true);

    try {
      await apiClient.post(`/characters/${characterId}/classes/change`, {
        old_class_id: classId,
        class_id: newClassInfo.id,
        preserve_level: true,
      });

      // Silently refresh all dependent subsystems
      await invalidateSubsystems(['classes', 'abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells']);
    } catch (err) {
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes, invalidateSubsystems]);

  const addClass = useCallback(async (classInfo: ClassInfo) => {
    if (!characterId || !classesData) return;
    
    // Check if class is already assigned
    if (classes.some(c => c.id === classInfo.id)) {
      throw new Error('This class is already assigned to the character');
    }
    
    // Check class/level limits
    if (classes.length >= 4) {
      throw new Error('Maximum of 4 classes allowed');
    }
    
    if (classesData.total_level >= 60) {
      throw new Error('Maximum level of 60 reached');
    }
    
    setIsUpdating(true);

    try {
      const response = await apiClient.post(`/characters/${characterId}/classes/add`, {
        class_id: classInfo.id,
      });

      // Silently refresh all dependent subsystems
      await invalidateSubsystems(['classes', 'abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells']);
      
      return response;
    } catch (err) {
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes, classesData, invalidateSubsystems]);

  const removeClass = useCallback(async (classId: number) => {
    if (!characterId) return;
    
    if (classes.length <= 1) {
      throw new Error('Character must have at least one class');
    }
    
    setIsUpdating(true);

    try {
      await apiClient.post(`/characters/${characterId}/classes/remove/${classId}`, {});

      // Silently refresh all dependent subsystems
      await invalidateSubsystems(['classes', 'abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells']);
    } catch (err) {
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes, invalidateSubsystems]);

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
      await apiClient.post(`/characters/${characterId}/classes/experience`, { xp });
      await fetchXPProgress();
    } catch (err) {
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, fetchXPProgress]);

  return {
    // Data from subsystem
    classes,
    totalLevel: classesData?.total_level || 0,
    multiclass: classesData?.multiclass || false,
    canMulticlass: classesData?.can_multiclass || false,
    combatStats: classesData?.combat_stats || {
      base_attack_bonus: 0,
      melee_attack_bonus: 0,
      ranged_attack_bonus: 0,
      multiple_attacks: [],
      fortitude_save: 0,
      reflex_save: 0,
      will_save: 0,
      base_fortitude: 0,
      base_reflex: 0,
      base_will: 0,
    },

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