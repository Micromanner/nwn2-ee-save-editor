import { useState, useEffect, useCallback } from 'react';
import { apiClient } from '@/lib/api/client';

export interface ClassFeature {
  name: string;
  type: 'proficiency' | 'feat' | 'spell' | 'combat' | 'skill' | 'ability';
  description: string;
  icon?: string;
  details?: Record<string, unknown>;
}

export interface LevelProgression {
  level: number;
  hit_points: number;
  skill_points: number;
  base_attack_bonus: number;
  saves: {
    fortitude: number;
    reflex: number;
    will: number;
  };
  features: ClassFeature[];
  feats: ClassFeature[];
  spells?: {
    level_0: number;
    level_1: number;
    level_2: number;
    level_3: number;
    level_4: number;
    level_5: number;
    level_6: number;
    level_7: number;
    level_8: number;
    level_9: number;
  };
}

export interface ClassProgression {
  class_id: number;
  class_name: string;
  basic_info: {
    hit_die: number;
    skill_points_per_level: number;
    bab_progression: string;
    save_progression: string;
    is_spellcaster: boolean;
    spell_type: 'arcane' | 'divine' | 'none';
  };
  level_progression: LevelProgression[];
  max_level_shown: number;
  proficiencies?: {
    weapons: string[];
    armor: string[];
    shields: boolean;
  };
  description?: {
    summary: string;
    features: string[];
    abilities: string[];
    restrictions: string[];
  };
}

export interface UseClassProgressionOptions {
  maxLevel?: number;
  includeSpells?: boolean;
  includeProficiencies?: boolean;
  autoFetch?: boolean;
}

export interface UseClassProgressionReturn {
  // Data
  progression: ClassProgression | null;
  
  // State
  isLoading: boolean;
  error: string | null;
  
  // Actions
  fetchProgression: (classId: number, options?: UseClassProgressionOptions) => Promise<void>;
  clearProgression: () => void;
  
  // Computed data for UI
  currentLevelFeatures: (level: number) => ClassFeature[];
  getLevelRange: (startLevel: number, endLevel: number) => LevelProgression[];
  getProgressionSummary: () => {
    totalFeatures: number;
    spellLevels: number;
    combatProgression: string;
    skillProgression: string;
  } | null;
}

export function useClassProgression(
  characterId?: string,
  options: UseClassProgressionOptions = {}
): UseClassProgressionReturn {
  const [progression, setProgression] = useState<ClassProgression | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const {
    maxLevel = 20,
    includeSpells = true,
    includeProficiencies = true,
    autoFetch = false
  } = options;

  const fetchProgression = useCallback(async (
    classId: number,
    fetchOptions?: UseClassProgressionOptions
  ) => {
    const opts = { ...options, ...fetchOptions };
    
    setIsLoading(true);
    setError(null);
    
    try {
      // Build URL with character context if available
      const baseUrl = characterId 
        ? `/characters/${characterId}/classes/features/${classId}`
        : `/classes/features/${classId}`;
      
      // Build query parameters
      const params = new URLSearchParams({
        max_level: (opts.maxLevel || maxLevel).toString(),
        include_spells: (opts.includeSpells ?? includeSpells).toString(),
        include_proficiencies: (opts.includeProficiencies ?? includeProficiencies).toString()
      });
      
      const url = `${baseUrl}?${params.toString()}`;
      const data = await apiClient.get<ClassProgression>(url);
      
      setProgression(data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to fetch class progression';
      setError(errorMessage);
      console.error('Error fetching class progression:', err);
    } finally {
      setIsLoading(false);
    }
  }, [characterId, maxLevel, includeSpells, includeProficiencies, options]);

  const clearProgression = useCallback(() => {
    setProgression(null);
    setError(null);
  }, []);

  // Get features available at a specific level
  const currentLevelFeatures = useCallback((level: number): ClassFeature[] => {
    if (!progression || level < 1 || level > progression.level_progression.length) {
      return [];
    }
    
    const levelData = progression.level_progression[level - 1];
    return [...(levelData.features || []), ...(levelData.feats || [])];
  }, [progression]);

  // Get progression data for a range of levels
  const getLevelRange = useCallback((startLevel: number, endLevel: number): LevelProgression[] => {
    if (!progression) return [];
    
    const start = Math.max(1, startLevel) - 1;
    const end = Math.min(progression.level_progression.length, endLevel);
    
    return progression.level_progression.slice(start, end);
  }, [progression]);

  // Get summary statistics for UI display
  const getProgressionSummary = useCallback(() => {
    if (!progression) return null;
    
    const totalFeatures = progression.level_progression.reduce(
      (total, level) => total + (level.features?.length || 0) + (level.feats?.length || 0),
      0
    );
    
    const maxSpellLevel = progression.basic_info.is_spellcaster
      ? Math.min(9, Math.floor(progression.max_level_shown / 2))
      : 0;
    
    const finalLevel = progression.level_progression[progression.level_progression.length - 1];
    const combatProgression = finalLevel 
      ? `BAB: +${finalLevel.base_attack_bonus}, HD: d${progression.basic_info.hit_die}`
      : 'Unknown';
    
    const skillProgression = `${progression.basic_info.skill_points_per_level} points/level`;
    
    return {
      totalFeatures,
      spellLevels: maxSpellLevel,
      combatProgression,
      skillProgression
    };
  }, [progression]);

  // Auto-fetch effect
  useEffect(() => {
    if (autoFetch && characterId) {
      // This would require a default class ID - skip auto-fetch for now
      // Could be enhanced to fetch for current character's primary class
    }
  }, [autoFetch, characterId]);

  return {
    // Data
    progression,
    
    // State
    isLoading,
    error,
    
    // Actions
    fetchProgression,
    clearProgression,
    
    // Computed data
    currentLevelFeatures,
    getLevelRange,
    getProgressionSummary
  };
}

// Utility hook for getting progression data for multiple classes
export function useMultiClassProgression(
  characterId?: string,
  classIds: number[] = [],
  options: UseClassProgressionOptions = {}
) {
  const [progressions, setProgressions] = useState<Record<number, ClassProgression>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [errors, setErrors] = useState<Record<number, string>>({});

  const fetchAllProgressions = useCallback(async () => {
    if (classIds.length === 0) return;
    
    setIsLoading(true);
    setErrors({});
    
    const newProgressions: Record<number, ClassProgression> = {};
    const newErrors: Record<number, string> = {};
    
    // Fetch all progressions in parallel
    const promises = classIds.map(async (classId) => {
      try {
        const baseUrl = characterId 
          ? `/characters/${characterId}/classes/features/${classId}`
          : `/classes/features/${classId}`;
        
        const params = new URLSearchParams({
          max_level: (options.maxLevel || 20).toString(),
          include_spells: (options.includeSpells ?? true).toString(),
          include_proficiencies: (options.includeProficiencies ?? true).toString()
        });
        
        const url = `${baseUrl}?${params.toString()}`;
        const data = await apiClient.get<ClassProgression>(url);
        
        newProgressions[classId] = data;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : 'Failed to fetch class progression';
        newErrors[classId] = errorMessage;
        console.error(`Error fetching progression for class ${classId}:`, err);
      }
    });
    
    await Promise.all(promises);
    
    setProgressions(newProgressions);
    setErrors(newErrors);
    setIsLoading(false);
  }, [characterId, classIds, options]);

  const getProgression = useCallback((classId: number) => {
    return progressions[classId] || null;
  }, [progressions]);

  const getError = useCallback((classId: number) => {
    return errors[classId] || null;
  }, [errors]);

  return {
    progressions,
    isLoading,
    errors,
    fetchAllProgressions,
    getProgression,
    getError,
    hasAnyData: Object.keys(progressions).length > 0
  };
}