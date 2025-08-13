import { useState, useCallback, useMemo, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';

export interface Attribute {
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

  
  // Combat stats with base (editable) and total (calculated)
  armorClass: {
    base: number;
    total: number;
  };
  initiative: {
    base: number;
    total: number;
  };
  
  // Saving throws with base (editable) and total (calculated)
  fortitude: {
    base: number;
    total: number;
  };
  reflex: {
    base: number;
    total: number;
  };
  will: {
    base: number;
    total: number;
  };
}

export interface AttributeState {
  base_attributes: Record<string, number>;
  attribute_modifiers: Record<string, number>;
  point_buy_cost: number;
  racial_modifiers: Record<string, number>;
  item_modifiers: Record<string, number>;
  level_up_modifiers: Record<string, number>;
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

export function useAttributes(attributeData?: AttributeState | null) {
  const t = useTranslations();
  const { characterId } = useCharacterContext();

  // Local state for optimistic updates
  const [localAttributeOverrides, setLocalAttributeOverrides] = useState<Record<string, number>>({});

  // Reset local overrides when attributeData changes (new character loaded)
  useEffect(() => {
    setLocalAttributeOverrides({});
  }, [attributeData]);

  // Utility function to calculate ability modifier
  const calculateModifier = useCallback((value: number): number => {
    return Math.floor((value - 10) / 2);
  }, []);

  // Transform attributeData into frontend format with local overrides
  const attributes = useMemo((): Attribute[] => {
    if (!attributeData) return [];
    
    // For editing: use base attributes + local overrides
    // For display: show effective attributes (which include all modifiers)
    const getDisplayValue = (attrKey: string) => {
      // If we have local overrides (user is editing), calculate effective value
      if (localAttributeOverrides[attrKey] !== undefined) {
        const baseValue = localAttributeOverrides[attrKey];
        const racial = attributeData.racial_modifiers?.[attrKey] ?? 0;
        const item = attributeData.item_modifiers?.[attrKey] ?? 0;
        const levelup = attributeData.level_up_modifiers?.[attrKey] ?? 0;
        return baseValue + racial + item + levelup;
      }
      // Otherwise use the backend's calculated effective attributes
      return attributeData.effective_attributes?.[attrKey] ?? attributeData.base_attributes[attrKey] ?? 10;
    };

    const getEditValue = (attrKey: string) => {
      return localAttributeOverrides[attrKey] ?? attributeData.base_attributes[attrKey] ?? 10;
    };
    
    return [
      { 
        name: t('abilities.strength'), 
        shortName: 'STR', 
        value: getDisplayValue('Str'), 
        modifier: attributeData.total_modifiers?.Str ?? calculateModifier(getDisplayValue('Str')), 
        baseValue: getEditValue('Str'),
        breakdown: {
          racial: attributeData.racial_modifiers?.Str ?? 0,
          equipment: attributeData.item_modifiers?.Str ?? 0
        }
      },
      { 
        name: t('abilities.dexterity'), 
        shortName: 'DEX', 
        value: getDisplayValue('Dex'), 
        modifier: attributeData.total_modifiers?.Dex ?? calculateModifier(getDisplayValue('Dex')), 
        baseValue: getEditValue('Dex'),
        breakdown: {
          racial: attributeData.racial_modifiers?.Dex ?? 0,
          equipment: attributeData.item_modifiers?.Dex ?? 0
        }
      },
      { 
        name: t('abilities.constitution'), 
        shortName: 'CON', 
        value: getDisplayValue('Con'), 
        modifier: attributeData.total_modifiers?.Con ?? calculateModifier(getDisplayValue('Con')), 
        baseValue: getEditValue('Con'),
        breakdown: {
          racial: attributeData.racial_modifiers?.Con ?? 0,
          equipment: attributeData.item_modifiers?.Con ?? 0
        }
      },
      { 
        name: t('abilities.intelligence'), 
        shortName: 'INT', 
        value: getDisplayValue('Int'), 
        modifier: attributeData.total_modifiers?.Int ?? calculateModifier(getDisplayValue('Int')), 
        baseValue: getEditValue('Int'),
        breakdown: {
          racial: attributeData.racial_modifiers?.Int ?? 0,
          equipment: attributeData.item_modifiers?.Int ?? 0
        }
      },
      { 
        name: t('abilities.wisdom'), 
        shortName: 'WIS', 
        value: getDisplayValue('Wis'), 
        modifier: attributeData.total_modifiers?.Wis ?? calculateModifier(getDisplayValue('Wis')), 
        baseValue: getEditValue('Wis'),
        breakdown: {
          racial: attributeData.racial_modifiers?.Wis ?? 0,
          equipment: attributeData.item_modifiers?.Wis ?? 0
        }
      },
      { 
        name: t('abilities.charisma'), 
        shortName: 'CHA', 
        value: getDisplayValue('Cha'), 
        modifier: attributeData.total_modifiers?.Cha ?? calculateModifier(getDisplayValue('Cha')), 
        baseValue: getEditValue('Cha'),
        breakdown: {
          racial: attributeData.racial_modifiers?.Cha ?? 0,
          equipment: attributeData.item_modifiers?.Cha ?? 0
        }
      },
    ];
  }, [attributeData, localAttributeOverrides, t, calculateModifier]);

  // Transform stats from attributeData
  const stats = useMemo((): CharacterStats => {
    if (!attributeData) {
      return {
        hitPoints: 0,
        maxHitPoints: 0,
        armorClass: { base: 10, total: 10 },
        fortitude: { base: 0, total: 0 },
        reflex: { base: 0, total: 0 },
        will: { base: 0, total: 0 },
        initiative: { base: 0, total: 0 },
      };
    }
    
    // Extract base and total values from backend objects
    const extractBaseTotal = (obj: unknown, statType: 'ac' | 'initiative' | 'fortitude' | 'reflex' | 'will') => {
      if (typeof obj === 'number') {
        // Simple number - assume it's total, no editable base
        return { base: 0, total: obj };
      }
      if (typeof obj === 'object' && obj !== null) {
        const objData = obj as Record<string, unknown>;
        
        let base = 0;
        let total = 0;
        
        // Get total value
        total = (typeof objData.total === 'number' ? objData.total : 
                typeof objData.value === 'number' ? objData.value : 0);
        
        // Get base value based on stat type
        switch (statType) {
          case 'ac':
            // Natural Armor comes from components.natural (from NaturalAC GFF field)
            const components = objData.components as Record<string, unknown> | undefined;
            base = (typeof components?.natural === 'number' ? components.natural : 0);
            break;
          case 'initiative':
            // Initiative base is misc_bonus (editable miscellaneous bonus)
            base = (typeof objData.misc_bonus === 'number' ? objData.misc_bonus : 0);
            break;
          case 'fortitude':
          case 'reflex':
          case 'will':
            // Saving throws base comes from the 'base' field in the save object
            base = (typeof objData.base === 'number' ? objData.base : 0);
            break;
        }
        
        return { base, total };
      }
      return { base: 0, total: 0 };
    };
    
    return {
      hitPoints: attributeData.derived_stats.hit_points.current,
      maxHitPoints: attributeData.derived_stats.hit_points.maximum,
      armorClass: extractBaseTotal(attributeData.combat_stats?.armor_class, 'ac'),
      fortitude: extractBaseTotal(attributeData.saving_throws?.fortitude, 'fortitude'),
      reflex: extractBaseTotal(attributeData.saving_throws?.reflex, 'reflex'),
      will: extractBaseTotal(attributeData.saving_throws?.will, 'will'),
      initiative: extractBaseTotal(attributeData.combat_stats?.initiative, 'initiative'),
    };
  }, [attributeData]);

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

  // Real-time attribute updates with optimistic UI updates
  const updateAttribute = useCallback(async (index: number, newValue: number) => {
    if (!characterId || !attributes[index]) return;
    
    // Note: newValue is the new BASE attribute value (before racial/item bonuses)
    const clampedValue = Math.max(3, Math.min(50, newValue));
    const attr = attributes[index];
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
    setLocalAttributeOverrides(prev => ({
      ...prev,
      [backendAttrName]: clampedValue
    }));
    
    try {
      // Send update to backend - backend cache will persist changes
      const result = await CharacterAPI.updateAttributes(characterId, {
        [backendAttrName]: clampedValue
      });
      
      console.log('Attribute update result:', result);
      
      // Backend confirmed the change - keep local override for now
      // It will be cleared when new data is loaded from backend
      
    } catch (err) {
      console.error('Failed to update attribute:', err);
      // Revert optimistic update on error
      setLocalAttributeOverrides(prev => {
        const updated = { ...prev };
        delete updated[backendAttrName];
        return updated;
      });
      throw err;
    }
  }, [characterId, attributes]);

  const updateAttributeByShortName = useCallback(async (shortName: string, newValue: number) => {
    const index = attributes.findIndex(attr => attr.shortName === shortName);
    if (index !== -1) {
      await updateAttribute(index, newValue);
    }
  }, [attributes, updateAttribute]);

  // Stats management - calls backend APIs for specific stats
  const updateStats = useCallback(async (updates: Partial<CharacterStats>) => {
    if (!characterId) return;
    
    console.log('Stats update requested:', updates);
    
    try {
      // Handle Natural Armor (AC base) updates
      if (updates.armorClass?.base !== undefined) {
        const result = await CharacterAPI.updateArmorClass(characterId, updates.armorClass.base);
        console.log('Natural armor update result:', result);
      }
      
      // Handle saving throw bonus updates
      const saveUpdates: Record<string, number> = {};
      if (updates.fortitude?.base !== undefined) saveUpdates.fortitude = updates.fortitude.base;
      if (updates.reflex?.base !== undefined) saveUpdates.reflex = updates.reflex.base;
      if (updates.will?.base !== undefined) saveUpdates.will = updates.will.base;
      
      if (Object.keys(saveUpdates).length > 0) {
        const result = await CharacterAPI.updateSavingThrows(characterId, saveUpdates);
        console.log('Saving throws update result:', result);
      }
      
      // Initiative doesn't have a direct GFF field in NWN2 - it's calculated from DEX + feats
      if (updates.initiative?.base !== undefined) {
        console.warn('Initiative bonus updates not yet implemented - NWN2 calculates from DEX + feats');
      }
      
      // Other stats like HP are handled differently
      if (updates.hitPoints !== undefined || updates.maxHitPoints !== undefined) {
        console.warn('Hit points updates not yet implemented - need separate endpoint');
      }
      
    } catch (err) {
      console.error('Failed to update stats:', err);
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
  const getAttribute = useCallback((shortName: string): Attribute | undefined => {
    return attributes.find(attr => attr.shortName === shortName);
  }, [attributes]);

  // Get specific attribute modifier
  const getAttributeModifier = useCallback((shortName: string): number => {
    const attr = getAttribute(shortName);
    return attr ? attr.modifier : 0;
  }, [getAttribute]);

  return {
    // State
    attributes,
    stats,
    alignment,

    // Attribute functions
    updateAttribute,
    updateAttributeByShortName,
    getAttribute,
    getAttributeModifier,
    calculateModifier,

    // Stats functions (read-only, updated by backend)
    updateStats,

    // Alignment functions
    updateAlignment,
  };
}