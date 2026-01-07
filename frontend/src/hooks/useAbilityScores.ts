import { useState, useCallback, useMemo, useEffect } from 'react';
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

export interface PointSummary {
  total_available: number;
  total_spent: number;
  available: number;
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
  point_summary?: PointSummary;
}

export interface Alignment {
  lawChaos: number;
  goodEvil: number;
}

export function useAbilityScores(abilityScoreData?: AbilityScoreState | null) {
  const t = useTranslations();
  const { characterId, invalidateSubsystems } = useCharacterContext();

  const [localAbilityScoreOverrides, setLocalAbilityScoreOverrides] = useState<Record<string, number>>({});
  const [localStatsOverrides, setLocalStatsOverrides] = useState<Partial<CharacterStats>>({});

  useEffect(() => {
    if (abilityScoreData) {
      setLocalAbilityScoreOverrides({});
      setLocalStatsOverrides({});
    }
  }, [abilityScoreData]);

  const calculateModifier = useCallback((value: number): number => {
    return Math.floor((value - 10) / 2);
  }, []);

  const abilityScores = useMemo((): AbilityScore[] => {
    if (!abilityScoreData) return [];

    const getEditValue = (attrKey: string) => {
      return localAbilityScoreOverrides[attrKey] ?? abilityScoreData.base_attributes[attrKey] ?? 10;
    };

    const getDisplayValue = (attrKey: string) => {
      const override = localAbilityScoreOverrides[attrKey];
      if (override !== undefined) {
          const originalBase = abilityScoreData.base_attributes[attrKey] ?? 10;
          const originalEffective = abilityScoreData.effective_attributes?.[attrKey] ?? originalBase;
          const bonus = originalEffective - originalBase;
          return override + bonus;
      }
      return abilityScoreData.effective_attributes?.[attrKey] ?? abilityScoreData.base_attributes[attrKey] ?? 10;
    };
    
    const getDetailedMod = (category: keyof typeof abilityScoreData.detailed_modifiers, attr: string) => {
      return abilityScoreData.detailed_modifiers?.[category]?.[attr] ?? 0;
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
  }, [abilityScoreData, localAbilityScoreOverrides, t, calculateModifier]);

  const stats = useMemo((): CharacterStats => {
    const defaultStats: CharacterStats = {
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

    if (!abilityScoreData) {
      return defaultStats;
    }
    
    const extractStat = (
      obj: unknown, 
      statType: 'ac' | 'initiative' | 'fortitude' | 'reflex' | 'will'
    ) => {
      const overrideKey = statType === 'ac' ? 'armorClass' : statType;
      const localOverride = localStatsOverrides[overrideKey as keyof CharacterStats];
      
      if (obj === null || obj === undefined) {
        return { base: 0, total: 0 };
      }

      if (typeof obj === 'number') {
        const baseValue = localOverride && typeof localOverride === 'object' && 'base' in localOverride 
          ? (localOverride as { base: number }).base 
          : 0;
        return { base: baseValue, total: obj };
      }
      
      if (typeof obj === 'object') {
        const objData = obj as Record<string, unknown>;
        
        let base = 0;
        const total = (typeof objData.total === 'number' ? objData.total : 
                typeof objData.value === 'number' ? objData.value : 0);
        const result: { base: number; total: number; [key: string]: number } = { base, total };
        
        switch (statType) {
          case 'ac':
            const components = objData.components as Record<string, unknown> | undefined;
            base = typeof components?.natural === 'number' ? components.natural : 0;
            result.dexMod = typeof components?.dex === 'number' ? components.dex : 0;
            result.equipment = (typeof components?.armor === 'number' ? components.armor : 0) + 
                              (typeof components?.shield === 'number' ? components.shield : 0);
            break;
          case 'initiative':
            base = typeof objData.misc_bonus === 'number' ? objData.misc_bonus : 0;
            result.dexMod = typeof objData.dex_modifier === 'number' ? objData.dex_modifier : 0;
            result.feats = typeof objData.improved_initiative === 'number' ? objData.improved_initiative : 0;
            break;
          case 'fortitude':
          case 'reflex':
          case 'will':
            base = typeof objData.misc === 'number' ? objData.misc : 0;
            result.abilityMod = typeof objData.ability === 'number' ? objData.ability : 0;
            result.classMod = typeof objData.base === 'number' ? objData.base : 0;
            result.racial = typeof objData.racial === 'number' ? objData.racial : 0;
            result.feat = typeof objData.feat === 'number' ? objData.feat : 0;
            break;
        }

        if (localOverride && typeof localOverride === 'object' && 'base' in localOverride) {
          base = (localOverride as { base: number }).base;
        }
        
        result.base = base;
        return result;
      }
      return { base: 0, total: 0 };
    };
    
    return {
      hitPoints: localStatsOverrides.hitPoints ?? abilityScoreData.derived_stats.hit_points.current,
      maxHitPoints: localStatsOverrides.maxHitPoints ?? abilityScoreData.derived_stats.hit_points.maximum,
      experience: localStatsOverrides.experience ?? 0,
      level: localStatsOverrides.level ?? 1,
      armorClass: extractStat(abilityScoreData.combat_stats?.armor_class, 'ac'),
      fortitude: extractStat(abilityScoreData.saving_throws?.fortitude, 'fortitude'),
      reflex: extractStat(abilityScoreData.saving_throws?.reflex, 'reflex'),
      will: extractStat(abilityScoreData.saving_throws?.will, 'will'),
      initiative: extractStat(abilityScoreData.combat_stats?.initiative, 'initiative'),
    };
  }, [abilityScoreData, localStatsOverrides]);

  const [alignment, setAlignment] = useState<Alignment>({
    lawChaos: 50,
    goodEvil: 50,
  });

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

  const updateAbilityScore = useCallback(async (index: number, newValue: number) => {
    if (!characterId || !abilityScores[index]) return;

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

    setLocalAbilityScoreOverrides(prev => ({
      ...prev,
      [backendAttrName]: clampedValue
    }));
    
    try {
      await CharacterAPI.updateAttributes(characterId, {
        [backendAttrName]: clampedValue
      });
      await invalidateSubsystems(['abilityScores', 'combat', 'skills']);

    } catch (err) {
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

  const updateStats = useCallback(async (updates: Partial<CharacterStats>) => {
    if (!characterId) return;

    setLocalStatsOverrides(prev => ({ ...prev, ...updates }));
    
    try {
      if (updates.armorClass?.base !== undefined) {
        await CharacterAPI.updateArmorClass(characterId, updates.armorClass.base);
      }
      
      if (updates.initiative?.base !== undefined) {
        await CharacterAPI.updateInitiativeBonus(characterId, updates.initiative.base);
      }

      const saveUpdates: Record<string, number> = {};
      if (updates.fortitude?.base !== undefined) saveUpdates.fortitude = updates.fortitude.base;
      if (updates.reflex?.base !== undefined) saveUpdates.reflex = updates.reflex.base;
      if (updates.will?.base !== undefined) saveUpdates.will = updates.will.base;
      
      if (Object.keys(saveUpdates).length > 0) {
        await CharacterAPI.updateSavingThrows(characterId, saveUpdates);
      }

      await invalidateSubsystems(['abilityScores', 'combat']);

    } catch (err) {
      setLocalStatsOverrides(prev => {
        const reverted = { ...prev };
        Object.keys(updates).forEach(key => delete reverted[key as keyof CharacterStats]);
        return reverted;
      });
      throw err;
    }
  }, [characterId, invalidateSubsystems]);

  const updateAlignment = useCallback(async (updates: Partial<Alignment>) => {
    if (!characterId) return;
    
    const newAlignment = { ...alignment, ...updates };

    setAlignment(newAlignment);
    
    try {
      const result = await CharacterAPI.updateAlignment(characterId, newAlignment);
      
      setAlignment({
        lawChaos: result.lawChaos,
        goodEvil: result.goodEvil,
      });
    } catch (err) {
      setAlignment(alignment);
      throw err;
    }
  }, [characterId, alignment]);

  const getAbilityScore = useCallback((shortName: string): AbilityScore | undefined => {
    return abilityScores.find(attr => attr.shortName === shortName);
  }, [abilityScores]);

  const getAbilityScoreModifier = useCallback((shortName: string): number => {
    const attr = getAbilityScore(shortName);
    return attr ? attr.modifier : 0;
  }, [getAbilityScore]);

  return {
    abilityScores,
    stats,
    alignment,

    updateAbilityScore,
    updateAbilityScoreByShortName,
    getAbilityScore,
    getAbilityScoreModifier,
    calculateModifier,
    updateStats,

    updateAlignment,
    
    pointSummary: abilityScoreData?.point_summary,
  };
}