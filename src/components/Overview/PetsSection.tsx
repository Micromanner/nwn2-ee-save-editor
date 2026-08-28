import { useCallback, useEffect, useState } from 'react';
import { Button, Callout, InputGroup } from '@blueprintjs/core';
import { GiQuillInk } from 'react-icons/gi';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { display } from '@/utils/dataHelpers';
import type { PetSlot, PetsState } from '@/lib/bindings';
import { GameIcon } from '../shared/GameIcon';
import { DropdownSelect } from '../shared';
import { T } from '../theme';

const TYPE_NONE = -1;

type SlotKey = 'animal_companion' | 'familiar';

const SLOT_COMMANDS: Record<SlotKey, string> = {
  animal_companion: 'set_animal_companion',
  familiar: 'set_familiar',
};

// A slot is only worth showing when the game could act on it: the character
// has the summoning feat, or a previous editor/level-up already stored data.
function isSlotRelevant(slot: PetSlot): boolean {
  return slot.has_feat || slot.type_id !== TYPE_NONE || slot.name !== '';
}

function PetSlotRow({
  slot,
  labelKey,
  featHintKey,
  onChange,
}: {
  slot: PetSlot;
  labelKey: string;
  featHintKey: string;
  onChange: (update: { name?: string; typeId?: number }) => void;
}) {
  const t = useTranslations();
  const [isEditingName, setIsEditingName] = useState(false);
  const [name, setName] = useState(slot.name);

  useEffect(() => {
    if (!isEditingName) setName(slot.name);
  }, [slot.name, isEditingName]);

  const handleNameSave = useCallback(() => {
    if (name !== slot.name) {
      onChange({ name });
    }
    setIsEditingName(false);
  }, [name, slot.name, onChange]);

  const handleNameCancel = useCallback(() => {
    setName(slot.name);
    setIsEditingName(false);
  }, [slot.name]);

  return (
    <div style={{ padding: '5px 0' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span className="t-md" style={{ color: T.textMuted, flex: 1 }}>{t(labelKey)}</span>
        <DropdownSelect
          small
          width={180}
          value={slot.type_id === TYPE_NONE ? t('overview.pets.none') : (slot.options.find(o => o.id === slot.type_id)?.name ?? String(slot.type_id))}
          items={[
            { id: TYPE_NONE, label: t('overview.pets.none') },
            ...slot.options.map(o => ({ id: o.id, label: o.name })),
          ]}
          onChange={typeId => onChange({ typeId })}
        />
        {/* Fixed width so the select does not shift when the name toggles to an input */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 4, width: 208 }}>
          {isEditingName ? (
            <>
              <InputGroup
                small
                autoFocus
                value={name}
                placeholder={t('overview.pets.namePlaceholder')}
                onChange={e => setName(e.target.value)}
                onKeyDown={e => {
                  if (e.key === 'Enter') handleNameSave();
                  if (e.key === 'Escape') handleNameCancel();
                }}
                style={{ width: 140 }}
              />
              <Button icon="tick" intent="primary" small onClick={handleNameSave} />
              <Button icon="cross" minimal small onClick={handleNameCancel} />
            </>
          ) : (
            <span
              className="editable-row t-md t-semibold"
              style={{ display: 'inline-flex', alignItems: 'center', gap: 4, cursor: 'pointer', color: T.text }}
              onClick={() => setIsEditingName(true)}
            >
              {display(slot.name)}
              <GameIcon icon={GiQuillInk} size={12} style={{ color: T.textMuted }} />
            </span>
          )}
        </div>
      </div>
      {!slot.has_feat && (
        <div className="t-sm" style={{ color: T.textMuted, marginTop: 2 }}>
          {t(featHintKey)}
        </div>
      )}
    </div>
  );
}

export function PetsSection() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { characterId, activeSource } = useCharacterContext();
  const [pets, setPets] = useState<PetsState | null>(null);
  const [edited, setEdited] = useState(false);

  useEffect(() => {
    setEdited(false);
    if (!characterId) {
      setPets(null);
      return;
    }
    let cancelled = false;
    invoke<PetsState>('get_pets')
      .then(state => { if (!cancelled) setPets(state); })
      .catch(err => { if (!cancelled) handleError(err); });
    return () => { cancelled = true; };
  // Keyed on activeSource too: switching player/companion keeps the same id
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [characterId, activeSource]);

  const updateSlot = useCallback(async (key: SlotKey, update: { name?: string; typeId?: number }) => {
    try {
      // Explicit nulls: an omitted key is not a valid Option arg for Tauri.
      const state = await invoke<PetsState>(SLOT_COMMANDS[key], {
        name: update.name ?? null,
        typeId: update.typeId ?? null,
      });
      setPets(state);
      setEdited(true);
    } catch (err) {
      handleError(err);
    }
  }, [handleError]);

  if (!pets) return null;

  const showCompanion = isSlotRelevant(pets.animal_companion);
  const showFamiliar = isSlotRelevant(pets.familiar);
  if (!showCompanion && !showFamiliar) return null;

  // Remount the rows on a player/companion switch so an open name editor can
  // never keep the previous character's text.
  const sourceKey = activeSource.kind === 'companion' ? activeSource.rosName : 'player';

  return (
    <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
      <div className="t-section" style={{ marginBottom: 8 }}>{t('overview.pets.title')}</div>
      {showCompanion && (
        <PetSlotRow
          key={`companion-${sourceKey}`}
          slot={pets.animal_companion}
          labelKey="overview.pets.animalCompanion"
          featHintKey="overview.pets.noAnimalCompanionFeat"
          onChange={update => updateSlot('animal_companion', update)}
        />
      )}
      {showFamiliar && (
        <PetSlotRow
          key={`familiar-${sourceKey}`}
          slot={pets.familiar}
          labelKey="overview.pets.familiar"
          featHintKey="overview.pets.noFamiliarFeat"
          onChange={update => updateSlot('familiar', update)}
        />
      )}
      {edited && (
        <Callout intent="warning" compact style={{ marginTop: 8 }}>
          {t('overview.pets.resummonHint')}
        </Callout>
      )}
    </div>
  );
}
