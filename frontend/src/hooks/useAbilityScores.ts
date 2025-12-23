import { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';

export interface AbilityScore {
  name: string;
  shortName: string;
  value: number;
  modifier: number;
  baseValue?: number;
  breakdown?: {
    levelUp: number;
    racial: number;
    equipment: number;
    enhancement: number;
    temporary: number;
  };
}

export interface CharacterStats {
  hitPoints: number;
  maxHitPoints: number;
  experience: number;
  level: number;
  
  // Combat stats with base (editable) and total (calculated)
  armorClass: {
    base: number;
    total: number;
    dexMod?: number;
    equipment?: number;
  };
  initiative: {
    base: number;
    total: number;
    dexMod?: number;
    feats?: number;
  };
  
  // Saving throws with base (editable) and total (calculated)
  fortitude: {
    base: number;
    total: number;
    abilityMod?: number;
    classMod?: number;
    racial?: number;
    feat?: number;
  };
  reflex: {
    base: number;
    total: number;
    abilityMod?: number;
    classMod?: number;
    racial?: number;
    feat?: number;
  };
  will: {
    base: number;
    total: number;
    abilityMod?: number;
    classMod?: number;
    racial?: number;
    feat?: number;
  };
}

export interface AbilityScoreState {
  base_attributes: Record<string, number>;
  attribute_modifiers: Record<string, number>;
  point_buy_cost: number;
  detailed_modifiers: {
    racial_modifiers: Record<string, number>;
    item_modifiers: Record<string, number>;
    level_up_modifiers: Record<string, number>;
    total_modifiers: Record<string, number>;
    base_modifiers: Record<string, number>;
    enhancement_modifiers: Record<string, number>;
    temporary_modifiers: Record<string, number>;
  };
  effective_attributes: Record<string, number>;
  total_modifiers: Record<string, number>;
  derived_stats: {
    hit_points: {
      current: number;
      maximum: number;
    };
  };
  combat_stats?: {
    armor_class: number;
    initiative: number;
  };
  saving_throws?: {
    fortitude: number;
    reflex: number;
    will: number;
  };
}

export interface Alignment {
  lawChaos: number;
  goodEvil: number;
}

export function useAbilityScores(abilityScoreData?: AbilityScoreState | null) {
  const t = useTranslations();
  const { characterId, invalidateSubsystems } = useCharacterContext();

  // Local state for optimistic updates
  const [localAbilityScoreOverrides, setLocalAbilityScoreOverrides] = useState<Record<string, number>>({});
  const [localStatsOverrides, setLocalStatsOverrides] = useState<Partial<CharacterStats>>({});

  // Sticky data pattern: Keep the last valid data to prevent flickering to 0 during refreshes
  const lastValidDataRef = useRef<AbilityScoreState | null>(null);
  
  // Helper to check if data is "complete enough" to replace our sticky data
  // This prevents replacing good data with an empty/partial object causing flicker
  const isDataValid = (data: AbilityScoreState | null | undefined): data is AbilityScoreState => {
    if (!data) return false;
    // Must have saving throws structure
    if (!data.saving_throws || Object.keys(data.saving_throws).length === 0) return false;
    // Must have combat stats
    if (!data.combat_stats) return false;
    // Must have detailed modifiers for breakdown
    if (!data.detailed_modifiers || Object.keys(data.detailed_modifiers).length === 0) return false;
    
    return true;
  };

  if (isDataValid(abilityScoreData)) {
    lastValidDataRef.current = abilityScoreData;
  }
  
  // Use valid new data, or fallback to last valid sticky data
  const dataToUse = isDataValid(abilityScoreData) ? abilityScoreData : lastValidDataRef.current;

  // Reset local overrides when REAL (VALID) data arrives and changes
  useEffect(() => {
    // Only clear overrides if we have actual new valid data from the backend
    if (isDataValid(abilityScoreData)) {
      setLocalAbilityScoreOverrides({});
      setLocalStatsOverrides({});
    }
  }, [abilityScoreData]);

  // Utility function to calculate ability modifier
  const calculateModifier = useCallback((value: number): number => {
    return Math.floor((value - 10) / 2);
  }, []);

  // Transform attributeData into frontend format with local overrides
  const abilityScores = useMemo((): AbilityScore[] => {
    if (!dataToUse) return [];

    // For editing: use base attributes + local overrides (base only, no modifiers)
    // For display: use backend's pre-calculated effective attributes
    const getEditValue = (attrKey: string) => {
      // Prioritize local override, then base from data, then default
      return localAbilityScoreOverrides[attrKey] ?? dataToUse.base_attributes[attrKey] ?? 10;
    };

    const getDisplayValue = (attrKey: string) => {
      // If we have a local override, we should ideally calculate the effective value
      // But for now, we'll try to be smart: 
      // If there's an override, use it as the base for display too, 
      // adding the difference between effective and base from the backend data
      const override = localAbilityScoreOverrides[attrKey];
      if (override !== undefined) {
          const originalBase = dataToUse.base_attributes[attrKey] ?? 10;
          const originalEffective = dataToUse.effective_attributes?.[attrKey] ?? originalBase;
          const bonus = originalEffective - originalBase;
          return override + bonus;
      }
      return dataToUse.effective_attributes?.[attrKey] ?? dataToUse.base_attributes[attrKey] ?? 10;
    };
    
    // Helper to safely get modifiers from detailed_modifiers
    const getDetailedMod = (category: keyof typeof dataToUse.detailed_modifiers, attr: string) => {
      return dataToUse.detailed_modifiers?.[category]?.[attr] ?? 0;
    };

    return [
      {
        name: t('abilityScores.strength'),
        shortName: 'STR',
        value: getDisplayValue('Str'),
        modifier: calculateModifier(getDisplayValue('Str')),
        baseValue: getEditValue('Str'),
        breakdown: {
          levelUp: getDetailedMod('level_up_modifiers', 'Str'),
          racial: getDetailedMod('racial_modifiers', 'Str'),
          equipment: getDetailedMod('item_modifiers', 'Str'),
          enhancement: getDetailedMod('enhancement_modifiers', 'Str'),
          temporary: getDetailedMod('temporary_modifiers', 'Str')
        }
      },
      {
        name: t('abilityScores.dexterity'),
        shortName: 'DEX',
        value: getDisplayValue('Dex'),
        modifier: calculateModifier(getDisplayValue('Dex')),
        baseValue: getEditValue('Dex'),
        breakdown: {
          levelUp: getDetailedMod('level_up_modifiers', 'Dex'),
          racial: getDetailedMod('racial_modifiers', 'Dex'),
          equipment: getDetailedMod('item_modifiers', 'Dex'),
          enhancement: getDetailedMod('enhancement_modifiers', 'Dex'),
          temporary: getDetailedMod('temporary_modifiers', 'Dex')
        }
      },
      {
        name: t('abilityScores.constitution'),
        shortName: 'CON',
        value: getDisplayValue('Con'),
        modifier: calculateModifier(getDisplayValue('Con')),
        baseValue: getEditValue('Con'),
        breakdown: {
          levelUp: getDetailedMod('level_up_modifiers', 'Con'),
          racial: getDetailedMod('racial_modifiers', 'Con'),
          equipment: getDetailedMod('item_modifiers', 'Con'),
          enhancement: getDetailedMod('enhancement_modifiers', 'Con'),
          temporary: getDetailedMod('temporary_modifiers', 'Con')
        }
      },
      {
        name: t('abilityScores.intelligence'),
        shortName: 'INT',
        value: getDisplayValue('Int'),
        modifier: calculateModifier(getDisplayValue('Int')),
        baseValue: getEditValue('Int'),
        breakdown: {
          levelUp: getDetailedMod('level_up_modifiers', 'Int'),
          racial: getDetailedMod('racial_modifiers', 'Int'),
          equipment: getDetailedMod('item_modifiers', 'Int'),
          enhancement: getDetailedMod('enhancement_modifiers', 'Int'),
          temporary: getDetailedMod('temporary_modifiers', 'Int')
        }
      },
      {
        name: t('abilityScores.wisdom'),
        shortName: 'WIS',
        value: getDisplayValue('Wis'),
        modifier: calculateModifier(getDisplayValue('Wis')),
        baseValue: getEditValue('Wis'),
        breakdown: {
          levelUp: getDetailedMod('level_up_modifiers', 'Wis'),
          racial: getDetailedMod('racial_modifiers', 'Wis'),
          equipment: getDetailedMod('item_modifiers', 'Wis'),
          enhancement: getDetailedMod('enhancement_modifiers', 'Wis'),
          temporary: getDetailedMod('temporary_modifiers', 'Wis')
        }
      },
      {
        name: t('abilityScores.charisma'),
        shortName: 'CHA',
        value: getDisplayValue('Cha'),
        modifier: calculateModifier(getDisplayValue('Cha')),
        baseValue: getEditValue('Cha'),
        breakdown: {
          levelUp: getDetailedMod('level_up_modifiers', 'Cha'),
          racial: getDetailedMod('racial_modifiers', 'Cha'),
          equipment: getDetailedMod('item_modifiers', 'Cha'),
          enhancement: getDetailedMod('enhancement_modifiers', 'Cha'),
          temporary: getDetailedMod('temporary_modifiers', 'Cha')
        }
      },
    ];
  }, [dataToUse, localAbilityScoreOverrides, t, calculateModifier]);

  // Transform stats from attributeData with local overrides
  const stats = useMemo((): CharacterStats => {
    if (!dataToUse) {
      return {
        hitPoints: 0,
        maxHitPoints: 0,
        experience: 0,
        level: 1,
        armorClass: { base: 10, total: 10 },
        fortitude: { base: 0, total: 0 },
        reflex: { base: 0, total: 0 },
        will: { base: 0, total: 0 },
        initiative: { base: 0, total: 0 },
      };
    }
    
    // Extract base and total values from backend objects, checking local overrides first
    const extractBaseTotal = (
      obj: unknown, 
      statType: 'ac' | 'initiative' | 'fortitude' | 'reflex' | 'will',
      fallbackObj?: unknown
    ) => {
      // Check for local overrides first (for persistence across tab switches)
      const overrideKey = statType === 'ac' ? 'armorClass' : statType;
      const localOverride = localStatsOverrides[overrideKey as keyof CharacterStats];
      
      // If primary obj is missing but we have a fallback, use it entirely
      if ((obj === null || obj === undefined) && fallbackObj) {
         return extractBaseTotal(fallbackObj, statType);
      }

      const fallbackData = (fallbackObj && typeof fallbackObj === 'object') ? fallbackObj as Record<string, unknown> : {};

      if (typeof obj === 'number') {
        const baseValue = localOverride && typeof localOverride === 'object' && 'base' in localOverride 
          ? (localOverride as { base: number }).base 
          : 0;
        return { base: baseValue, total: obj };
      }
      
      if (typeof obj === 'object' && obj !== null) {
        const objData = obj as Record<string, unknown>;
        
        let base = 0;
        let total = 0;
        const result: { base: number; total: number; [key: string]: number } = { base, total };
        
        // Get total value
        total = (typeof objData.total === 'number' ? objData.total : 
                typeof objData.value === 'number' ? objData.value : 
                typeof fallbackData.total === 'number' ? fallbackData.total as number : 0);
        
        // Check for local override base value first
        // We calculate defaults/derived first, then apply override to base
        
        // Get base value from backend based on stat type
        switch (statType) {
          case 'ac':
            const components = objData.components as Record<string, unknown> | undefined;
            const fallbackComponents = fallbackData.components as Record<string, unknown> | undefined;
            
            base = (typeof components?.natural === 'number' ? components.natural : 
                   typeof fallbackComponents?.natural === 'number' ? fallbackComponents.natural as number : 0);
                   
            result.dexMod = (typeof components?.dex === 'number' ? components.dex : 
                            typeof fallbackComponents?.dex === 'number' ? fallbackComponents.dex as number : 0);
                            
            result.equipment = ((typeof components?.armor === 'number' ? components.armor : 
                               typeof fallbackComponents?.armor === 'number' ? fallbackComponents.armor as number : 0) + 
                              (typeof components?.shield === 'number' ? components.shield : 
                               typeof fallbackComponents?.shield === 'number' ? fallbackComponents.shield as number : 0));
            break;
          case 'initiative':
            base = (typeof objData.misc_bonus === 'number' ? objData.misc_bonus : 
                   typeof fallbackData.misc_bonus === 'number' ? fallbackData.misc_bonus as number : 0);
                   
            result.dexMod = (typeof objData.dex_modifier === 'number' ? objData.dex_modifier : 
                            typeof fallbackData.dex_modifier === 'number' ? fallbackData.dex_modifier as number : 0);
                            
            result.feats = (typeof objData.improved_initiative === 'number' ? objData.improved_initiative : 
                           typeof fallbackData.improved_initiative === 'number' ? fallbackData.improved_initiative as number : 0);
            break;
          case 'fortitude':
          case 'reflex':
          case 'will':
            // Base (editable) now maps to the misc bonus from backend
            // Class contribution (objData.base) maps to classMod
            base = (typeof objData.misc === 'number' ? objData.misc : 
                   typeof fallbackData.misc === 'number' ? fallbackData.misc as number : 0);
            
            result.abilityMod = (typeof objData.ability === 'number' ? objData.ability : 
                                typeof fallbackData.ability === 'number' ? fallbackData.ability as number : 0);
                                
            result.classMod = (typeof objData.base === 'number' ? objData.base : 
                              typeof fallbackData.base === 'number' ? fallbackData.base as number : 0);
                              
            result.racial = (typeof objData.racial === 'number' ? objData.racial : 
                            typeof fallbackData.racial === 'number' ? fallbackData.racial as number : 0);
                            
            result.feat = (typeof objData.feat === 'number' ? objData.feat : 
                          typeof fallbackData.feat === 'number' ? fallbackData.feat as number : 0);
            break;
        }

        // Apply override if exists
        if (localOverride && typeof localOverride === 'object' && 'base' in localOverride) {
          base = (localOverride as { base: number }).base;
        }
        
        result.base = base;
        result.total = total;
        return result;
      }
      return { base: 0, total: 0 };
    };
    
    // Build stats object
    // We pass fallback objects from the Ref if needed, using granular field fallbacks
    const fallbackStats = lastValidDataRef.current;
    
    const stats = {
      hitPoints: localStatsOverrides.hitPoints ?? dataToUse.derived_stats.hit_points.current,
      maxHitPoints: localStatsOverrides.maxHitPoints ?? dataToUse.derived_stats.hit_points.maximum,
      experience: localStatsOverrides.experience ?? 0,
      level: localStatsOverrides.level ?? 1,
      armorClass: extractBaseTotal(dataToUse.combat_stats?.armor_class, 'ac', fallbackStats?.combat_stats?.armor_class),
      fortitude: extractBaseTotal(dataToUse.saving_throws?.fortitude, 'fortitude', fallbackStats?.saving_throws?.fortitude),
      reflex: extractBaseTotal(dataToUse.saving_throws?.reflex, 'reflex', fallbackStats?.saving_throws?.reflex),
      will: extractBaseTotal(dataToUse.saving_throws?.will, 'will', fallbackStats?.saving_throws?.will),
      initiative: extractBaseTotal(dataToUse.combat_stats?.initiative, 'initiative', fallbackStats?.combat_stats?.initiative),
    };

    return stats;
  }, [dataToUse, localStatsOverrides]);

  // Alignment state - fetched from backend
  const [alignment, setAlignment] = useState<Alignment>({
    lawChaos: 50,
    goodEvil: 50,
  });
  
  // Fetch alignment from backend when character loads
  useEffect(() => {
    const fetchAlignment = async () => {
      if (!characterId) return;
      
      try {
        const alignmentData = await CharacterAPI.getAlignment(characterId);
        setAlignment({
          lawChaos: alignmentData.lawChaos,
          goodEvil: alignmentData.goodEvil,
        });
      } catch (error) {
        console.error('Failed to fetch alignment:', error);
      }
    };
    
    fetchAlignment();
  }, [characterId]);

  // Real-time ability score updates with optimistic UI updates
  const updateAbilityScore = useCallback(async (index: number, newValue: number) => {
    if (!characterId || !abilityScores[index]) return;
    
    // Note: newValue is the new BASE attribute value (before racial/item bonuses)
    const clampedValue = Math.max(3, Math.min(50, newValue));
    const attr = abilityScores[index];
    const attributeMapping = {
      'STR': 'Str',
      'DEX': 'Dex', 
      'CON': 'Con',
      'INT': 'Int',
      'WIS': 'Wis',
      'CHA': 'Cha'
    };
    
    const backendAttrName = attributeMapping[attr.shortName as keyof typeof attributeMapping];
    if (!backendAttrName) return;
    
    // Optimistic update - immediately update UI
    setLocalAbilityScoreOverrides(prev => ({
      ...prev,
      [backendAttrName]: clampedValue
    }));
    
    try {
      // Send update to backend - backend cache will persist changes
      const result = await CharacterAPI.updateAttributes(characterId, {
        [backendAttrName]: clampedValue
      });

      console.log('Ability score update result:', result);

      // Backend confirmed the change - keep local override for now
      // It will be cleared when new data is loaded from backend

      // Silently refresh dependent subsystems (abilityScores for AC/stats, combat, saves, skills)
      await invalidateSubsystems(['abilityScores', 'combat', 'saves', 'skills']);

    } catch (err) {
      console.error('Failed to update ability score:', err);
      // Revert optimistic update on error
      setLocalAbilityScoreOverrides(prev => {
        const updated = { ...prev };
        delete updated[backendAttrName];
        return updated;
      });
      throw err;
    }
  }, [characterId, abilityScores, invalidateSubsystems]);

  const updateAbilityScoreByShortName = useCallback(async (shortName: string, newValue: number) => {
    const index = abilityScores.findIndex(attr => attr.shortName === shortName);
    if (index !== -1) {
      await updateAbilityScore(index, newValue);
    }
  }, [abilityScores, updateAbilityScore]);

  // Stats management - calls backend APIs for specific stats
  const updateStats = useCallback(async (updates: Partial<CharacterStats>) => {
    if (!characterId) return;
    
    console.log('Stats update requested:', updates);
    
    // Optimistic update - immediately update UI
    setLocalStatsOverrides(prev => ({ ...prev, ...updates }));
    
    try {
      // Handle Natural Armor (AC base) updates
      if (updates.armorClass?.base !== undefined) {
        const result = await CharacterAPI.updateArmorClass(characterId, updates.armorClass.base);
        console.log('Natural armor update result:', result);
      }
      
      
      // Handle Initiative misc bonus updates
      if (updates.initiative?.base !== undefined) {
        const result = await CharacterAPI.updateInitiativeBonus(characterId, updates.initiative.base);
        console.log('Initiative bonus update result:', result);
      }
      
      // Handle Saving Throw misc bonus updates
      const saveUpdates: Record<string, number> = {};
      if (updates.fortitude?.base !== undefined) saveUpdates.fortitude = updates.fortitude.base;
      if (updates.reflex?.base !== undefined) saveUpdates.reflex = updates.reflex.base;
      if (updates.will?.base !== undefined) saveUpdates.will = updates.will.base;
      
      if (Object.keys(saveUpdates).length > 0) {
        const result = await CharacterAPI.updateSavingThrows(characterId, saveUpdates);
        console.log('Saving throws update result:', result);
      }
      
      // Other stats like HP are handled differently
      if (updates.hitPoints !== undefined || updates.maxHitPoints !== undefined) {
        console.warn('Hit points updates not yet implemented - need separate endpoint');
      }

      // Silently refresh dependent subsystems after AC/Initiative/Saves updates
      await invalidateSubsystems(['abilityScores', 'combat', 'saves']);

    } catch (err) {
      console.error('Failed to update stats:', err);
      // Revert optimistic update on error
      setLocalStatsOverrides(prev => {
        const reverted = { ...prev };
        Object.keys(updates).forEach(key => delete reverted[key as keyof CharacterStats]);
        return reverted;
      });
      throw err;
    }
  }, [characterId, invalidateSubsystems]);

  // Alignment management - now using backend endpoint
  const updateAlignment = useCallback(async (updates: Partial<Alignment>) => {
    if (!characterId) return;
    
    const newAlignment = { ...alignment, ...updates };
    
    // Optimistic update - immediately update UI
    setAlignment(newAlignment);
    
    try {
      // Send update to backend
      const result = await CharacterAPI.updateAlignment(characterId, newAlignment);
      console.log('Alignment update result:', result);
      
      // Update with server response to ensure consistency
      setAlignment({
        lawChaos: result.lawChaos,
        goodEvil: result.goodEvil,
      });
      
    } catch (err) {
      console.error('Failed to update alignment:', err);
      // Revert optimistic update on error
      setAlignment(alignment);
      throw err;
    }
  }, [characterId, alignment]);

  // Get specific attribute by short name
  const getAbilityScore = useCallback((shortName: string): AbilityScore | undefined => {
    return abilityScores.find(attr => attr.shortName === shortName);
  }, [abilityScores]);

  // Get specific ability score modifier
  const getAbilityScoreModifier = useCallback((shortName: string): number => {
    const attr = getAbilityScore(shortName);
    return attr ? attr.modifier : 0;
  }, [getAbilityScore]);

  return {
    // State
    abilityScores,
    stats,
    alignment,

    // Ability Score functions
    updateAbilityScore,
    updateAbilityScoreByShortName,
    getAbilityScore,
    getAbilityScoreModifier,
    calculateModifier,

    // Stats functions (read-only, updated by backend)
    updateStats,

    // Alignment functions
    updateAlignment,
  };
}