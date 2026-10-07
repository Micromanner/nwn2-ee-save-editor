import { useCallback } from 'react';
import { CharacterAPI } from '@/services/characterApi';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';

const SKILL_DEPENDENT_SUBSYSTEMS = ['combat', 'classes'] as const;

export function useSkillManagement() {
  const t = useTranslations();
  const { character, invalidateSubsystems } = useCharacterContext();
  const skills = useSubsystem('skills');

  const updateSkills = useCallback(async (updates: Record<number, number>) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await CharacterAPI.updateSkills(updates);
    await skills.load({ silent: true });
    await invalidateSubsystems([...SKILL_DEPENDENT_SUBSYSTEMS]);
  }, [t, character?.id, skills, invalidateSubsystems]);

  const resetSkills = useCallback(async () => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    await CharacterAPI.resetSkills();
    await skills.load({ silent: true });
    await invalidateSubsystems([...SKILL_DEPENDENT_SUBSYSTEMS]);
  }, [t, character?.id, skills, invalidateSubsystems]);

  return { updateSkills, resetSkills };
}
