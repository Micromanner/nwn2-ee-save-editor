import { useState, useCallback, useMemo } from 'react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { apiClient } from '@/lib/api/client';

export interface FeatSlotInfo {
  general: number;
  bonus: number;
}

export interface FeatInfo {
  id: number;
  name: string;
  description?: string;
  icon?: string;
  can_take?: boolean;
  prerequisites_met?: boolean;
}

export interface SpellInfo {
  id: number;
  name: string;
  icon?: string;
  school_id?: number;
  school_name?: string;
  description?: string;
}

export interface LevelUpRequirements {
  can_level_up: boolean;
  current_level: number;
  new_level: number;
  class_id: number;
  class_name: string;
  new_class_level: number;
  hp_gain: number;
  skill_points: number;
  feat_slots: FeatSlotInfo;
  has_ability_increase: boolean;
  spell_slots_gained: Record<number, number>;
  available_feats: FeatInfo[];
  available_spells: Record<number, SpellInfo[]>;
  class_skills: number[];
  cross_class_skills: number[];
  is_spellcaster: boolean;
  current_abilities: Record<string, number>;
}

export interface SpellSelection {
  spell_level: number;
  spell_id: number;
}

export interface LevelUpSelections {
  class_id: number;
  ability_increase?: string;
  feats: number[];
  skills: Record<number, number>;
  spells: SpellSelection[];
}

export type LevelUpStep = 'summary' | 'ability' | 'feats' | 'skills' | 'spells' | 'confirm';

export interface LevelUpState {
  step: LevelUpStep;
  requirements: LevelUpRequirements | null;
  selections: LevelUpSelections;
  isLoading: boolean;
  error: string | null;
}

export function useLevelUp(classId: number) {
  const { characterId, invalidateSubsystems } = useCharacterContext();

  const [state, setState] = useState<LevelUpState>({
    step: 'summary',
    requirements: null,
    selections: {
      class_id: classId,
      feats: [],
      skills: {},
      spells: []
    },
    isLoading: false,
    error: null
  });

  const fetchRequirements = useCallback(async () => {
    if (!characterId) return;

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      const requirements = await apiClient.get<LevelUpRequirements>(
        `/characters/${characterId}/levelup/check/${classId}`
      );
      setState(prev => ({
        ...prev,
        requirements,
        selections: { ...prev.selections, class_id: classId },
        isLoading: false
      }));
    } catch (err) {
      setState(prev => ({
        ...prev,
        error: err instanceof Error ? err.message : 'Failed to load requirements',
        isLoading: false
      }));
    }
  }, [characterId, classId]);

  const reset = useCallback(() => {
    setState({
      step: 'summary',
      requirements: null,
      selections: {
        class_id: classId,
        feats: [],
        skills: {},
        spells: []
      },
      isLoading: false,
      error: null
    });
  }, [classId]);

  const setAbilityIncrease = useCallback((ability: string) => {
    setState(prev => ({
      ...prev,
      selections: { ...prev.selections, ability_increase: ability }
    }));
  }, []);

  const addFeat = useCallback((featId: number) => {
    setState(prev => ({
      ...prev,
      selections: {
        ...prev.selections,
        feats: [...prev.selections.feats, featId]
      }
    }));
  }, []);

  const removeFeat = useCallback((featId: number) => {
    setState(prev => ({
      ...prev,
      selections: {
        ...prev.selections,
        feats: prev.selections.feats.filter(id => id !== featId)
      }
    }));
  }, []);

  const setSkillPoints = useCallback((skillId: number, points: number) => {
    setState(prev => {
      const newSkills = { ...prev.selections.skills };
      if (points <= 0) {
        delete newSkills[skillId];
      } else {
        newSkills[skillId] = points;
      }
      return {
        ...prev,
        selections: {
          ...prev.selections,
          skills: newSkills
        }
      };
    });
  }, []);

  const addSpell = useCallback((spellLevel: number, spellId: number) => {
    setState(prev => ({
      ...prev,
      selections: {
        ...prev.selections,
        spells: [...prev.selections.spells, { spell_level: spellLevel, spell_id: spellId }]
      }
    }));
  }, []);

  const removeSpell = useCallback((spellLevel: number, spellId: number) => {
    setState(prev => ({
      ...prev,
      selections: {
        ...prev.selections,
        spells: prev.selections.spells.filter(
          s => !(s.spell_level === spellLevel && s.spell_id === spellId)
        )
      }
    }));
  }, []);

  const goToStep = useCallback((step: LevelUpStep) => {
    setState(prev => ({ ...prev, step }));
  }, []);

  const applyLevelUp = useCallback(async () => {
    if (!characterId) return false;

    setState(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      await apiClient.post(`/characters/${characterId}/levelup/apply`, state.selections);
      await invalidateSubsystems(['classes', 'abilityScores', 'combat', 'saves', 'skills', 'feats', 'spells']);
      return true;
    } catch (err) {
      setState(prev => ({
        ...prev,
        error: err instanceof Error ? err.message : 'Failed to apply level up',
        isLoading: false
      }));
      return false;
    }
  }, [characterId, state.selections, invalidateSubsystems]);

  const totalSkillPointsSpent = useMemo(() => {
    return Object.values(state.selections.skills).reduce((sum, pts) => sum + pts, 0);
  }, [state.selections.skills]);

  const remainingSkillPoints = useMemo(() => {
    return (state.requirements?.skill_points || 0) - totalSkillPointsSpent;
  }, [state.requirements?.skill_points, totalSkillPointsSpent]);

  const totalFeatSlots = useMemo(() => {
    return (state.requirements?.feat_slots.general || 0) + (state.requirements?.feat_slots.bonus || 0);
  }, [state.requirements?.feat_slots]);

  const remainingFeatSlots = useMemo(() => {
    return totalFeatSlots - state.selections.feats.length;
  }, [totalFeatSlots, state.selections.feats.length]);

  const canProceed = useMemo(() => {
    if (!state.requirements) return false;

    switch (state.step) {
      case 'summary':
        return true;
      case 'ability':
        return !state.requirements.has_ability_increase || !!state.selections.ability_increase;
      case 'feats':
        return true; // Allow proceeding with fewer feats selected
      case 'skills':
        return true; // Allow proceeding with unspent points (per user preference)
      case 'spells':
        return true; // Allow proceeding
      case 'confirm':
        return true;
      default:
        return true;
    }
  }, [state.step, state.requirements, state.selections]);

  return {
    ...state,
    fetchRequirements,
    reset,
    setAbilityIncrease,
    addFeat,
    removeFeat,
    setSkillPoints,
    addSpell,
    removeSpell,
    goToStep,
    applyLevelUp,
    totalSkillPointsSpent,
    remainingSkillPoints,
    totalFeatSlots,
    remainingFeatSlots,
    canProceed
  };
}
