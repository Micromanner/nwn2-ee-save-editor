import { useCallback } from 'react';
import { CharacterAPI } from '@/services/characterApi';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';

const SPELL_DEPENDENT_SUBSYSTEMS = ['combat'] as const;

export function useSpellManagement() {
  const t = useTranslations();
  const { character, invalidateSubsystems } = useCharacterContext();
  const spells = useSubsystem('spells');

  const addSpell = useCallback(async (spellId: number, classIndex: number, spellLevel: number) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await CharacterAPI.manageSpell(character.id, 'add', spellId, classIndex, spellLevel);
    await spells.load({ force: true });
    await invalidateSubsystems([...SPELL_DEPENDENT_SUBSYSTEMS]);
    return response;
  }, [t, character?.id, spells, invalidateSubsystems]);

  const removeSpell = useCallback(async (spellId: number, classIndex: number, spellLevel: number) => {
    if (!character?.id) throw new Error(t('common.noCharacterLoaded'));

    const response = await CharacterAPI.manageSpell(character.id, 'remove', spellId, classIndex, spellLevel);
    await spells.load({ force: true });
    await invalidateSubsystems([...SPELL_DEPENDENT_SUBSYSTEMS]);
    return response;
  }, [t, character?.id, spells, invalidateSubsystems]);

  return { addSpell, removeSpell };
}
