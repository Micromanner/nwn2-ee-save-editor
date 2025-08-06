import { useState, useEffect, useCallback, useMemo } from 'react';
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

export interface ClassesData {
  classes: Array<{
    id: number;
    level: number;
    name: string;
  }>;
  total_level: number;
  multiclass: boolean;
  can_multiclass: boolean;
  combat_stats: CombatStats;
}

export function useClassesLevel(classesData?: ClassesData | null) {
  const { characterId } = useCharacterContext();
  const [categorizedClasses, setCategorizedClasses] = useState<any>(null);
  const [isUpdating, setIsUpdating] = useState(false);

  // Load categorized classes for additional class info
  useEffect(() => {
    const loadCategorizedClasses = async () => {
      if (!characterId) return;
      
      try {
        const categorized = await apiClient.get(`/characters/${characterId}/classes/categorized/?include_unplayable=true`);
        setCategorizedClasses(categorized);
      } catch (err) {
        console.error('Failed to load categorized classes:', err);
      }
    };

    loadCategorizedClasses();
  }, [characterId]);

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
      
      return {
        id: cls.id,
        name: cls.name,
        level: cls.level,
        hitDie: classInfo?.hit_die || 8,
        baseAttackBonus: Math.floor(classesData.combat_stats.base_attack_bonus * (cls.level / classesData.total_level)),
        fortitudeSave: Math.floor(classesData.combat_stats.base_fortitude * (cls.level / classesData.total_level)),
        reflexSave: Math.floor(classesData.combat_stats.base_reflex * (cls.level / classesData.total_level)),
        willSave: Math.floor(classesData.combat_stats.base_will * (cls.level / classesData.total_level)),
        skillPoints: (classInfo?.skill_points || 2) * cls.level,
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
    
    setIsUpdating(true);
    
    try {
      await apiClient.post(`/characters/${characterId}/classes/level-up/`, {
        class_id: classId,
        level_change: delta
      });
    } catch (err) {
      console.error('Error adjusting class level:', err);
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes]);

  const changeClass = useCallback(async (classId: number, newClassInfo: ClassInfo) => {
    if (!characterId) return;
    
    // Check if new class is already taken
    if (classes.some(c => c.id !== classId && c.id === newClassInfo.id)) {
      throw new Error('This class is already assigned to the character');
    }
    
    setIsUpdating(true);
    
    try {
      await apiClient.post(`/characters/${characterId}/classes/change/`, {
        old_class_id: classId,
        new_class_id: newClassInfo.id,
        preserve_level: true
      });
    } catch (err) {
      console.error('Error changing class:', err);
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes]);

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
      await apiClient.post(`/characters/${characterId}/classes/add/`, {
        class_id: classInfo.id
      });
    } catch (err) {
      console.error('Error adding class:', err);
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes, classesData]);

  const removeClass = useCallback(async (classId: number) => {
    if (!characterId) return;
    
    if (classes.length <= 1) {
      throw new Error('Character must have at least one class');
    }
    
    setIsUpdating(true);
    
    try {
      await apiClient.post(`/characters/${characterId}/classes/remove/`, {
        class_id: classId
      });
    } catch (err) {
      console.error('Error removing class:', err);
      throw err;
    } finally {
      setIsUpdating(false);
    }
  }, [characterId, classes]);

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
    
    // Additional data
    categorizedClasses,
    findClassInfoById,
    
    // State
    isUpdating,
    
    // Actions
    adjustClassLevel,
    changeClass,
    addClass,
    removeClass,
  };
}