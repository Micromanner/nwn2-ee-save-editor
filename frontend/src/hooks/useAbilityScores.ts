import { useState, useCallback, useMemo, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI, type RaceDataResponse } from '@/services/characterApi';

export interface AbilityScore {
  name: string;
  shortName: string;
  value: number;
  modifier: number;
  baseValue?: number;
  breakdown?: {
    racial: number;
    equipment: number;
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
  const { characterId } = useCharacterContext();

  // Local state for optimistic updates
  const [localAbilityScoreOverrides, setLocalAbilityScoreOverrides] = useState<Record<string, number>>({});
  const [localStatsOverrides, setLocalStatsOverrides] = useState<Partial<CharacterStats>>({});
  
  // Race data state for combined racial modifiers
  const [raceData, setRaceData] = useState<RaceDataResponse | null>(null);

  // Reset local overrides when abilityScoreData changes (new character loaded)
  useEffect(() => {
    setLocalAbilityScoreOverrides({});
    setLocalStatsOverrides({});
  }, [abilityScoreData]);

  // Fetch race data when character changes to get combined racial modifiers
  useEffect(() => {
    const fetchRaceData = async () => {
      if (!characterId) {
        setRaceData(null);
        return;
      }
      
      try {
        const raceResponse = await CharacterAPI.getRaceData(characterId);
        setRaceData(raceResponse);
      } catch (error) {
        console.error('Failed to fetch race data:', error);
        setRaceData(null);
      }
    };
    
    fetchRaceData();
  }, [characterId]);

  // Utility function to calculate ability modifier
  const calculateModifier = useCallback((value: number): number => {
    return Math.floor((value - 10) / 2);
  }, []);

  // Transform attributeData into frontend format with local overrides
  const abilityScores = useMemo((): AbilityScore[] => {
    if (!abilityScoreData) return [];
    
    // For editing: use base attributes + local overrides
    // For display: show effective attributes (which include all modifiers)
    const getDisplayValue = (attrKey: string) => {
      // If we have local overrides (user is editing), calculate effective value
      if (localAbilityScoreOverrides[attrKey] !== undefined) {
        const baseValue = localAbilityScoreOverrides[attrKey];
        // Use combined racial modifiers from race manager instead of separate racial_modifiers
        const racial = raceData?.ability_modifiers?.[attrKey] ?? 0;
        const item = abilityScoreData.detailed_modifiers?.item_modifiers?.[attrKey] ?? 0;
        const levelup = abilityScoreData.detailed_modifiers?.level_up_modifiers?.[attrKey] ?? 0;
        return baseValue + racial + item + levelup;
      }
      // Otherwise use the backend's calculated effective attributes
      return abilityScoreData.effective_attributes?.[attrKey] ?? abilityScoreData.base_attributes[attrKey] ?? 10;
    };

    const getEditValue = (attrKey: string) => {
      return localAbilityScoreOverrides[attrKey] ?? abilityScoreData.base_attributes[attrKey] ?? 10;
    };
    
    return [
      { 
        name: t('abilityScores.strength'), 
        shortName: 'STR', 
        value: getDisplayValue('Str'), 
        modifier: abilityScoreData.total_modifiers?.Str ?? calculateModifier(getDisplayValue('Str')), 
        baseValue: getEditValue('Str'),
        breakdown: {
          racial: raceData?.ability_modifiers?.Str ?? 0,
          equipment: abilityScoreData.detailed_modifiers?.item_modifiers?.Str ?? 0
        }
      },
      { 
        name: t('abilityScores.dexterity'), 
        shortName: 'DEX', 
        value: getDisplayValue('Dex'), 
        modifier: abilityScoreData.total_modifiers?.Dex ?? calculateModifier(getDisplayValue('Dex')), 
        baseValue: getEditValue('Dex'),
        breakdown: {
          racial: raceData?.ability_modifiers?.Dex ?? 0,
          equipment: abilityScoreData.detailed_modifiers?.item_modifiers?.Dex ?? 0
        }
      },
      { 
        name: t('abilityScores.constitution'), 
        shortName: 'CON', 
        value: getDisplayValue('Con'), 
        modifier: abilityScoreData.total_modifiers?.Con ?? calculateModifier(getDisplayValue('Con')), 
        baseValue: getEditValue('Con'),
        breakdown: {
          racial: raceData?.ability_modifiers?.Con ?? 0,
          equipment: abilityScoreData.detailed_modifiers?.item_modifiers?.Con ?? 0
        }
      },
      { 
        name: t('abilityScores.intelligence'), 
        shortName: 'INT', 
        value: getDisplayValue('Int'), 
        modifier: abilityScoreData.total_modifiers?.Int ?? calculateModifier(getDisplayValue('Int')), 
        baseValue: getEditValue('Int'),
        breakdown: {
          racial: raceData?.ability_modifiers?.Int ?? 0,
          equipment: abilityScoreData.detailed_modifiers?.item_modifiers?.Int ?? 0
        }
      },
      { 
        name: t('abilityScores.wisdom'), 
        shortName: 'WIS', 
        value: getDisplayValue('Wis'), 
        modifier: abilityScoreData.total_modifiers?.Wis ?? calculateModifier(getDisplayValue('Wis')), 
        baseValue: getEditValue('Wis'),
        breakdown: {
          racial: raceData?.ability_modifiers?.Wis ?? 0,
          equipment: abilityScoreData.detailed_modifiers?.item_modifiers?.Wis ?? 0
        }
      },
      { 
        name: t('abilityScores.charisma'), 
        shortName: 'CHA', 
        value: getDisplayValue('Cha'), 
        modifier: abilityScoreData.total_modifiers?.Cha ?? calculateModifier(getDisplayValue('Cha')), 
        baseValue: getEditValue('Cha'),
        breakdown: {
          racial: raceData?.ability_modifiers?.Cha ?? 0,
          equipment: abilityScoreData.detailed_modifiers?.item_modifiers?.Cha ?? 0
        }
      },
    ];
  }, [abilityScoreData, localAbilityScoreOverrides, raceData, t, calculateModifier]);

  // Transform stats from attributeData with local overrides
  const stats = useMemo((): CharacterStats => {
    if (!abilityScoreData) {
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
    const extractBaseTotal = (obj: unknown, statType: 'ac' | 'initiative' | 'fortitude' | 'reflex' | 'will') => {
      // Check for local overrides first (for persistence across tab switches)
      const overrideKey = statType === 'ac' ? 'armorClass' : statType;
      const localOverride = localStatsOverrides[overrideKey as keyof CharacterStats];
      
      if (typeof obj === 'number') {
        // Simple number - assume it's total, no editable base
        const baseValue = localOverride && typeof localOverride === 'object' && 'base' in localOverride 
          ? (localOverride as { base: number }).base 
          : 0;
        return { base: baseValue, total: obj };
      }
      if (typeof obj === 'object' && obj !== null) {
        const objData = obj as Record<string, unknown>;
        
        let base = 0;
        let total = 0;
        let result: any = { base, total };
        
        // Get total value
        total = (typeof objData.total === 'number' ? objData.total : 
                typeof objData.value === 'number' ? objData.value : 0);
        
        // Check for local override base value first
        if (localOverride && typeof localOverride === 'object' && 'base' in localOverride) {
          base = (localOverride as { base: number }).base;
        } else {
          // Get base value from backend based on stat type
          switch (statType) {
            case 'ac':
              // Natural Armor comes from components.natural (from NaturalAC GFF field)
              const components = objData.components as Record<string, unknown> | undefined;
              base = (typeof components?.natural === 'number' ? components.natural : 0);
              // Add detailed AC breakdown
              result.dexMod = (typeof components?.dex === 'number' ? components.dex : 0);
              result.equipment = ((typeof components?.armor === 'number' ? components.armor : 0) + 
                                (typeof components?.shield === 'number' ? components.shield : 0));
              break;
            case 'initiative':
              // Initiative base is misc_bonus (editable miscellaneous bonus)
              base = (typeof objData.misc_bonus === 'number' ? objData.misc_bonus : 0);
              // Add detailed initiative breakdown
              result.dexMod = (typeof objData.dex_modifier === 'number' ? objData.dex_modifier : 0);
              result.feats = (typeof objData.improved_initiative === 'number' ? objData.improved_initiative : 0);
              break;
            case 'fortitude':
            case 'reflex':
            case 'will':
              // Saving throws base comes from the 'base' field in the save object
              base = (typeof objData.base === 'number' ? objData.base : 0);
              // Add detailed save breakdown
              result.abilityMod = (typeof objData.ability === 'number' ? objData.ability : 0);
              result.classMod = (typeof objData.base === 'number' ? objData.base : 0); // Class contribution is the base
              result.racial = (typeof objData.racial === 'number' ? objData.racial : 0);
              result.feat = (typeof objData.feat === 'number' ? objData.feat : 0);
              break;
          }
        }
        
        result.base = base;
        result.total = total;
        return result;
      }
      return { base: 0, total: 0 };
    };
    
    // Build stats object with local overrides already integrated by extractBaseTotal
    const stats = {
      hitPoints: localStatsOverrides.hitPoints ?? abilityScoreData.derived_stats.hit_points.current,
      maxHitPoints: localStatsOverrides.maxHitPoints ?? abilityScoreData.derived_stats.hit_points.maximum,
      experience: localStatsOverrides.experience ?? 0, // TODO: Add experience field to backend data
      level: localStatsOverrides.level ?? 1, // TODO: Add level field to backend data
      armorClass: extractBaseTotal(abilityScoreData.combat_stats?.armor_class, 'ac'),
      fortitude: extractBaseTotal(abilityScoreData.saving_throws?.fortitude, 'fortitude'),
      reflex: extractBaseTotal(abilityScoreData.saving_throws?.reflex, 'reflex'),
      will: extractBaseTotal(abilityScoreData.saving_throws?.will, 'will'),
      initiative: extractBaseTotal(abilityScoreData.combat_stats?.initiative, 'initiative'),
    };

    return stats;
  }, [abilityScoreData, localStatsOverrides]);

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
  }, [characterId, abilityScores]);

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
  }, [characterId]);

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