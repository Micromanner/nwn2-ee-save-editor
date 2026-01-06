import { useState, useEffect, useCallback } from 'react';
import { CharacterAPI } from '@/services/characterApi';
import { useCharacterContext } from '@/contexts/CharacterContext';

interface Skill {
  id: number;
  name: string;
  rank: number;
  max_rank: number;
  total_bonus: number;
  ability_modifier: number;
  misc_modifier: number;
  is_class_skill: boolean;
}


interface UseSkillsReturn {
  skills: Skill[];
  availableSkillPoints: number;
  totalSpentPoints: number;
  totalSkillPoints: number;
  isLoading: boolean;
  isUpdating: boolean;
  error: string | null;
  updateSkillRank: (skillId: number, newRank: number) => Promise<void>;
  resetAllSkills: () => Promise<void>;
  refreshSkills: () => Promise<void>;
}

export function useSkills(): UseSkillsReturn {
  const { character } = useCharacterContext();
  const [skills, setSkills] = useState<Skill[]>([]);
  const [availableSkillPoints, setAvailableSkillPoints] = useState(0);
  const [totalSpentPoints, setTotalSpentPoints] = useState(0);
  const [totalSkillPoints, setTotalSkillPoints] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadSkillsData = useCallback(async () => {
    if (!character?.id) return;
    
    setIsLoading(true);
    setError(null);
    
    try {
      const data = await CharacterAPI.getSkillsState(character.id);
      
      // Filter out skills with DEL_ prefix
      const validSkills = (data.skills || []).filter(skill => 
        !skill.name.startsWith('DEL_')
      );
      
      setSkills(validSkills);
      setAvailableSkillPoints(data.skill_points?.available || 0);
      setTotalSpentPoints(data.skill_points?.spent || 0);
      setTotalSkillPoints((data.skill_points?.available || 0) + (data.skill_points?.spent || 0));
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load skills');
    } finally {
      setIsLoading(false);
    }
  }, [character?.id]);

  useEffect(() => {
    loadSkillsData();
  }, [loadSkillsData]);

  const updateSkillRank = useCallback(async (skillId: number, newRank: number) => {
    if (!character?.id) return;
    
    const skill = skills.find(s => s.id === skillId);
    if (!skill) return;

    const oldRank = skill.rank;
    const rankDifference = newRank - oldRank;
    
    // Calculate cost based on whether it's a class skill
    // Class skills cost 1 point per rank, cross-class skills cost 2 points per rank
    const costPerRank = skill.is_class_skill ? 1 : 2;
    const totalCost = rankDifference * costPerRank;
    
    // Check if we have enough points (when increasing)
    if (rankDifference > 0 && totalCost > availableSkillPoints) {
      setError(`Not enough skill points. Need ${totalCost} points but only have ${availableSkillPoints}`);
      return;
    }
    
    // Optimistically update UI
    setSkills(prevSkills => 
      prevSkills.map(s => 
        s.id === skillId 
          ? { 
              ...s, 
              rank: newRank,
              // Recalculate total bonus: ranks + ability modifier + misc bonuses
              total_bonus: newRank + s.ability_modifier + (s.total_bonus - s.rank - s.ability_modifier)
            }
          : s
      )
    );
    
    // Update points based on actual cost
    setAvailableSkillPoints(prev => prev - totalCost);
    setTotalSpentPoints(prev => prev + totalCost);
    
    setIsUpdating(true);
    setError(null);
    
    try {
      const updates = { [skillId]: newRank };
      await CharacterAPI.updateSkills(character.id, updates);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update skill');
      // Revert on error
      await loadSkillsData();
    } finally {
      setIsUpdating(false);
    }
  }, [character?.id, skills, availableSkillPoints, loadSkillsData]);

  const resetAllSkills = useCallback(async () => {
    if (!character?.id) return;
    
    setIsUpdating(true);
    setError(null);
    
    try {
      await CharacterAPI.resetSkills(character.id);
      await loadSkillsData();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to reset skills');
    } finally {
      setIsUpdating(false);
    }
  }, [character?.id, loadSkillsData]);

  return {
    skills,
    availableSkillPoints,
    totalSpentPoints,
    totalSkillPoints,
    isLoading,
    isUpdating,
    error,
    updateSkillRank,
    resetAllSkills,
    refreshSkills: loadSkillsData,
  };
}