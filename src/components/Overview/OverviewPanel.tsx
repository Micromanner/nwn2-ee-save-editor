import { useState, useEffect, useCallback } from 'react';
import { Button, ButtonGroup, Card, Elevation, H4, InputGroup, NonIdealState, ProgressBar, Spinner, TextArea } from '@blueprintjs/core';
import { GiQuillInk, GiVisoredHelm } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { GameIcon } from '../shared/GameIcon';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { T } from '../theme';
import { KVRow, mod, StepInput } from '../shared';
import { DeityDialog } from './DeityDialog';
import { RaceDialog } from './RaceDialog';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { display, formatModifier, formatNumber } from '@/utils/dataHelpers';
import { invoke } from '@tauri-apps/api/core';

const ALIGNMENT_GRID = [
  { key: 'lawfulGood', lc: 85, ge: 85, color: '#FFD700' },
  { nameKey: 'neutralGood', key: 'neutralGood', lc: 50, ge: 85, color: '#87CEEB' },
  { nameKey: 'chaoticGood', key: 'chaoticGood', lc: 15, ge: 85, color: '#228B22' },
  { nameKey: 'lawfulNeutral', key: 'lawfulNeutral', lc: 85, ge: 50, color: '#71797E' },
  { nameKey: 'trueNeutral', key: 'trueNeutral', lc: 50, ge: 50, color: '#A0522D' },
  { nameKey: 'chaoticNeutral', key: 'chaoticNeutral', lc: 15, ge: 50, color: '#FF4500' },
  { nameKey: 'lawfulEvil', key: 'lawfulEvil', lc: 85, ge: 15, color: '#8B0000' },
  { nameKey: 'neutralEvil', key: 'neutralEvil', lc: 50, ge: 15, color: '#556B2F' },
  { nameKey: 'chaoticEvil', key: 'chaoticEvil', lc: 15, ge: 15, color: '#483D8B' },
].map(a => ({ ...a, nameKey: a.key, labelKey: `alignment.names.${a.key}` }));

function getAlignmentIndex(lc: number, ge: number): number {
  const col = lc >= 70 ? 0 : lc <= 30 ? 2 : 1;
  const row = ge >= 70 ? 0 : ge <= 30 ? 2 : 1;
  return row * 3 + col;
}

const ABILITY_DEFS = [
  { key: 'Str' as const, fallbackKey: 'strength' as const, labelKey: 'abilityScores.names.strength' },
  { key: 'Dex' as const, fallbackKey: 'dexterity' as const, labelKey: 'abilityScores.names.dexterity' },
  { key: 'Con' as const, fallbackKey: 'constitution' as const, labelKey: 'abilityScores.names.constitution' },
  { key: 'Int' as const, fallbackKey: 'intelligence' as const, labelKey: 'abilityScores.names.intelligence' },
  { key: 'Wis' as const, fallbackKey: 'wisdom' as const, labelKey: 'abilityScores.names.wisdom' },
  { key: 'Cha' as const, fallbackKey: 'charisma' as const, labelKey: 'abilityScores.names.charisma' },
];

export function OverviewPanel() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { character, characterId, isLoading, updateCharacterPartial, refreshAll, refreshCharacter } = useCharacterContext();

  useEffect(() => {
    if (characterId) {
      refreshCharacter();
    }
    // Refresh on mount so HP/abilities/saves reflect changes made in other tabs
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [characterId]);

  const [isEditingName, setIsEditingName] = useState(false);
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');

  const [isEditingBio, setIsEditingBio] = useState(false);
  const [biography, setBiography] = useState('');
  const [savedBiography, setSavedBiography] = useState('');

  const [isDeityOpen, setIsDeityOpen] = useState(false);
  const [isRaceOpen, setIsRaceOpen] = useState(false);

  const [hp, setHp] = useState(0);
  const [maxHp, setMaxHp] = useState(1);
  const [hpSaving, setHpSaving] = useState(false);

  const [age, setAge] = useState(0);
  const [ageSaving, setAgeSaving] = useState(false);

  const [lawChaos, setLawChaos] = useState(50);
  const [goodEvil, setGoodEvil] = useState(50);
  const [alignmentSaving, setAlignmentSaving] = useState(false);


  useEffect(() => {
    if (!character) return;
    setFirstName(character.first_name ?? '');
    setLastName(character.last_name ?? '');
    const bio = character.biography ?? '';
    setBiography(bio);
    setSavedBiography(bio);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id]);

  useEffect(() => {
    if (!character) return;
    setHp(character.hitPoints ?? 0);
    setMaxHp(character.maxHitPoints ?? 1);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.hitPoints, character?.maxHitPoints]);

  useEffect(() => {
    if (!character) return;
    setAge(character.age ?? 0);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id]);

  useEffect(() => {
    if (!character?.alignmentValues) return;
    setLawChaos(character.alignmentValues.law_chaos);
    setGoodEvil(character.alignmentValues.good_evil);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id]);


  const handleNameSave = useCallback(async () => {
    if (!characterId) return;
    try {
      await CharacterAPI.updateCharacter(characterId, { first_name: firstName, last_name: lastName });
      updateCharacterPartial({ first_name: firstName, last_name: lastName, name: `${firstName} ${lastName}`.trim() });
      setIsEditingName(false);
    } catch (err) {
      handleError(err);
      setIsEditingName(false);
    }
  }, [characterId, firstName, lastName, updateCharacterPartial, handleError]);

  const handleNameCancel = useCallback(() => {
    setFirstName(character?.first_name ?? '');
    setLastName(character?.last_name ?? '');
    setIsEditingName(false);
  }, [character]);

  const handleBioSave = useCallback(async () => {
    if (!characterId) return;
    try {
      await CharacterAPI.setBiography(characterId, biography);
      updateCharacterPartial({ biography });
      setSavedBiography(biography);
    } catch (err) {
      handleError(err);
    }
    setIsEditingBio(false);
  }, [characterId, biography, updateCharacterPartial, handleError]);

  const handleBioCancel = useCallback(() => {
    setBiography(savedBiography);
    setIsEditingBio(false);
  }, [savedBiography]);

  const handleDeitySelect = useCallback(async (deityName: string) => {
    if (!characterId) return;
    try {
      await CharacterAPI.setDeity(characterId, deityName);
      updateCharacterPartial({ deity: deityName });
    } catch (err) {
      handleError(err);
    }
  }, [characterId, updateCharacterPartial, handleError]);

  const handleRaceChange = useCallback(async (raceId: number, subrace: string | null) => {
    if (!characterId) return;
    try {
      await invoke('change_race', { raceId, subrace });
      await refreshAll();
      setIsRaceOpen(false);
    } catch (err) {
      handleError(err);
    }
  }, [characterId, refreshAll, handleError]);

  const handleGenderChange = useCallback(async (genderId: number) => {
    if (!characterId) return;
    try {
      await CharacterAPI.updateCharacter(characterId, { gender: genderId });
      const genderStr = genderId === 0 ? 'Male' : 'Female';
      updateCharacterPartial({ gender: genderStr, gender_id: genderId });
    } catch (err) {
      handleError(err);
    }
  }, [characterId, updateCharacterPartial, handleError]);

  const handleAgeChange = useCallback(async (newAge: number) => {
    if (!characterId || ageSaving) return;
    setAge(newAge);
    setAgeSaving(true);
    try {
      await CharacterAPI.updateCharacter(characterId, { age: newAge });
      updateCharacterPartial({ age: newAge });
    } catch (err) {
      handleError(err);
    }
    setAgeSaving(false);
  }, [characterId, ageSaving, updateCharacterPartial, handleError]);

  const handleHpChange = useCallback(async (newCurrent: number, newMax: number) => {
    if (!characterId || hpSaving) return;
    setHpSaving(true);
    try {
      await CharacterAPI.updateHitPoints(characterId, newCurrent, newMax);
      updateCharacterPartial({ hitPoints: newCurrent, maxHitPoints: newMax });
    } catch (err) {
      handleError(err);
    }
    setHpSaving(false);
  }, [characterId, hpSaving, updateCharacterPartial, handleError]);

  const handleAlignmentSelect = useCallback(async (lc: number, ge: number) => {
    if (!characterId || alignmentSaving) return;
    setLawChaos(lc);
    setGoodEvil(ge);
    setAlignmentSaving(true);
    try {
      const res = await CharacterAPI.updateAlignment(characterId, { law_chaos: lc, good_evil: ge });
      updateCharacterPartial({ alignment: res.alignment_string });
    } catch (err) {
      handleError(err);
    }
    setAlignmentSaving(false);
  }, [characterId, alignmentSaving, updateCharacterPartial, handleError]);

  const handleAlignmentStep = useCallback(async (field: 'lc' | 'ge', value: number) => {
    const lc = field === 'lc' ? value : lawChaos;
    const ge = field === 'ge' ? value : goodEvil;
    await handleAlignmentSelect(lc, ge);
  }, [lawChaos, goodEvil, handleAlignmentSelect]);

  if (isLoading && !character) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 400 }}>
        <Spinner size={40} />
      </div>
    );
  }

  if (!character) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: 400 }}>
        <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('common.noCharacterLoaded')} description={t('common.loadSaveToView', { section: t('navigation.overview').toLowerCase() })} />
      </div>
    );
  }

  const hpPct = maxHp > 0 ? hp / maxHp : 0;

  const ac = character.armorClass ?? 0;
  const bab = character.baseAttackBonus ?? 0;
  const melee = character.meleeAttackBonus ?? 0;
  const ranged = character.rangedAttackBonus ?? 0;
  const initTotal = character.initiative ?? 0;

  const fort = character.saves?.fortitude ?? 0;
  const reflex = character.saves?.reflex ?? 0;
  const will = character.saves?.will ?? 0;

  const classes = character.classes ?? [];
  const domains = character.domains ?? [];

  const totalSkillPoints = character.totalSkillPoints ?? character.skill_points_available ?? null;
  const totalFeats = character.totalFeats ?? null;
  const knownSpells = character.knownSpells ?? null;

  const campaignName = character.campaignName ?? null;
  const moduleName = character.moduleName ?? null;
  const locationName = character.location ?? null;
  const gameAct = character.gameAct != null ? String(character.gameAct) : null;
  const difficultyLabel = character.difficultyLabel ?? null;
  const gt = character.gameTime;
  const gameTimeStr = gt?.year != null
    ? t('overview.gameTimeValue', { year: gt.year, month: gt.month, day: gt.day, hour: gt.hour })
    : null;
  const lastSavedStr = character.lastSaved
    ? (() => { try { return new Date(character.lastSaved!).toLocaleDateString(); } catch { return character.lastSaved; } })()
    : character.lastSavedTimestamp != null ? new Date(character.lastSavedTimestamp * 1000).toLocaleDateString()
    : null;

  const questDetails = character.questDetails;
  const completedQuests = questDetails?.summary.completed_quests ?? character.completedQuests ?? 0;
  const activeQuests = questDetails?.summary.active_quests ?? character.currentQuests ?? 0;
  const completionRate = questDetails?.progress_stats.total_completion_rate;

  return (
    <div style={{ padding: 16, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12 }}>

      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>

        <div style={{ padding: '14px 16px 12px' }}>
          {isEditingName ? (
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
              <InputGroup small value={firstName} onChange={e => setFirstName(e.target.value)} placeholder={t('overview.firstName')} style={{ width: 160 }} />
              <InputGroup small value={lastName} onChange={e => setLastName(e.target.value)} placeholder={t('overview.lastName')} style={{ width: 160 }} />
              <Button icon="tick" intent="primary" small onClick={handleNameSave} />
              <Button icon="cross" minimal small onClick={handleNameCancel} />
            </div>
          ) : (
            <div style={{ marginBottom: 4 }}>
              <span className="editable-row" style={{ display: 'inline-flex', alignItems: 'baseline', gap: 8, cursor: 'pointer' }} onClick={() => setIsEditingName(true)}>
                <H4 style={{ margin: 0, color: T.text }}>{display(firstName)} {display(lastName)}</H4>
                <GameIcon icon={GiQuillInk} size={12} style={{ color: T.textMuted }} />
              </span>
            </div>
          )}
          <div style={{ color: T.textMuted, marginBottom: 10 }}>
            {classes.map(c => `${c.name} ${c.level}`).join(' / ')}
            {classes.length > 0 && <span style={{ color: T.textMuted }}>&mdash; {t('character.level')} {character.level}</span>}
          </div>

          <div className="t-md">
            <KVRow label={t('character.race')} value={
              <span className="editable-row" style={{ display: 'inline-flex', alignItems: 'center', gap: 4, cursor: 'pointer' }} onClick={() => setIsRaceOpen(true)}>
                {display(character.race)}
                <GameIcon icon={GiQuillInk} size={12} style={{ color: T.textMuted }} />
              </span>
            } />
            <KVRow label={t('character.genderLabel')} value={
              <ButtonGroup minimal>
                <Button small active={character.gender_id === 0} intent={character.gender_id === 0 ? 'primary' : 'none'} onClick={() => handleGenderChange(0)}>{t('character.gender.male')}</Button>
                <Button small active={character.gender_id === 1} intent={character.gender_id === 1 ? 'primary' : 'none'} onClick={() => handleGenderChange(1)}>{t('character.gender.female')}</Button>
              </ButtonGroup>
            } />
            <KVRow label={t('character.age')} value={
              <StepInput value={age} onValueChange={handleAgeChange} min={0} max={9999} width={88} zeroDisplay={display(null)} />
            } />
            <KVRow label={t('character.alignment')} value={t(ALIGNMENT_GRID[getAlignmentIndex(lawChaos, goodEvil)]?.labelKey) || display(character.alignment)} />
            <KVRow label={t('overview.deity')} value={
              <span className="editable-row" style={{ display: 'inline-flex', alignItems: 'center', gap: 4, cursor: 'pointer' }} onClick={() => setIsDeityOpen(true)}>
                {display(character.deity, 'None')}
                <GameIcon icon={GiQuillInk} size={12} style={{ color: T.textMuted }} />
              </span>
            } />
            <KVRow label={t('overview.background')} value={display(character.background?.name)} />
            <KVRow label={t('character.experience')} value={formatNumber(character.experience)} />
            <KVRow label={t('inventory.gold')} value={formatNumber(character.gold)} color={T.gold} />
          </div>

          {domains.length > 0 && (
            <div style={{ marginTop: 8 }}>
              <KVRow label={t('overview.domains')} value={domains.map(d => d.name).join(', ')} />
            </div>
          )}
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('overview.progression')}</div>
          <div className="t-md" style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
            <KVRow label={t('character.skillPoints')} value={display(totalSkillPoints)} />
            <KVRow label={t('character.totalFeats')} value={display(totalFeats)} />
            <KVRow label={t('character.knownSpells')} value={display(knownSpells)} />
          </div>
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '12px 16px' }}>
          <div style={{ marginBottom: 8 }}>
            <span className={isEditingBio ? undefined : 'editable-row'} style={{ display: 'inline-flex', alignItems: 'center', gap: 4, cursor: isEditingBio ? undefined : 'pointer' }} onClick={() => !isEditingBio && setIsEditingBio(true)}>
              <span className="t-section">{t('overview.biography')}</span>
              {!isEditingBio && (
                <GameIcon icon={GiQuillInk} size={12} style={{ color: T.textMuted }} />
              )}
            </span>
          </div>
          {isEditingBio ? (
            <div>
              <TextArea fill value={biography} onChange={e => setBiography(e.target.value)} rows={4}
                className="t-md"
              style={{ background: T.surface, borderColor: T.border, color: T.text, resize: 'vertical' }} />
              <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
                <Button small intent="primary" text={t('actions.save')} onClick={handleBioSave} />
                <Button small minimal text={t('actions.cancel')} onClick={handleBioCancel} />
              </div>
            </div>
          ) : (
            <p className="t-md t-body" style={{ margin: 0, color: T.textMuted }}>
              {biography || t('overview.noBiography')}
            </p>
          )}
        </div>

        <div className="t-md" style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('overview.campaign')}</div>
          <div style={{ marginBottom: 4 }}>
            <KVRow label={t('overview.campaignName')} value={display(campaignName)} />
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
            <KVRow label={t('overview.gameAct')} value={gameAct != null ? t('overview.actValue', { act: gameAct }) : '-'} />
            <KVRow label={t('overview.module')} value={display(moduleName)} />
            <KVRow label={t('character.location')} value={display(locationName)} />
            <KVRow label={t('overview.difficulty')} value={display(difficultyLabel)} />
            <KVRow label={t('character.lastSaved')} value={display(lastSavedStr)} />
            <KVRow label={t('overview.gameTime')} value={display(gameTimeStr)} />
          </div>
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('overview.questProgress')}</div>
          <div className="t-md" style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
            <KVRow label={t('character.questsCompleted')} value={display(completedQuests)} />
            <KVRow label={t('character.activeQuests')} value={display(activeQuests)} />
            {completionRate != null && (
              <KVRow label={t('overview.completion')} value={`${Math.round(completionRate)}%`} />
            )}
          </div>
        </div>
      </Card>

      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>

        <div style={{ padding: '12px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('abilityScores.health')}</div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 8 }}>
            <div style={{ flex: 1 }}>
              <ProgressBar value={hpPct} intent={hpPct >= 0.7 ? 'success' : hpPct >= 0.3 ? 'warning' : 'danger'} stripes={false} animate={false} style={{ height: 4 }} />
            </div>
            <span className="t-md t-bold" style={{ color: T.text, whiteSpace: 'nowrap' }}>
              {hp} / {maxHp}
              <span className="t-sm" style={{ color: T.textMuted, marginLeft: 4 }}>({Math.round(hpPct * 100)}%)</span>
            </span>
          </div>
          <div style={{ display: 'flex', gap: 10, justifyContent: 'center' }}>
            <div>
              <div className="t-xs t-semibold" style={{ color: T.textMuted, marginBottom: 3 }}>{t('common.current')}</div>
              <StepInput value={hp} onValueChange={v => { setHp(v); handleHpChange(v, maxHp); }} min={-10} max={maxHp} width={88} />
            </div>
            <div>
              <div className="t-xs t-semibold" style={{ color: T.textMuted, marginBottom: 3 }}>{t('common.max')}</div>
              <StepInput value={maxHp} onValueChange={v => { setMaxHp(v); handleHpChange(hp, v); }} min={1} max={9999} width={88} />
            </div>
          </div>
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('character.combatStats')}</div>
          <div className="t-md" style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
            <KVRow label={t('character.armorClass')} value={display(ac)} />
            <KVRow label={t('character.speed')} value={character.movementSpeed != null ? `${character.movementSpeed} ft` : '-'} />
            <KVRow label={t('character.baseAttackBonus')} value={formatModifier(bab)} />
            <KVRow label={t('character.initiative')} value={formatModifier(initTotal)} />
            <KVRow label={t('character.meleeAttack')} value={formatModifier(melee)} />
            <KVRow label={t('character.rangedAttack')} value={formatModifier(ranged)} />
            <KVRow label={t('character.size')} value={t(`character.size.${character.size?.toLowerCase()}`) || display(character.size)} />
          </div>
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('abilityScores.savingThrows')}</div>
          <div className="t-md" style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
            <KVRow label={t('abilityScores.fortitude')} value={formatModifier(fort)} />
            <KVRow label={t('abilityScores.reflex')} value={formatModifier(reflex)} />
            <KVRow label={t('abilityScores.will')} value={formatModifier(will)} />
          </div>
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('abilityScores.abilityScores')}</div>
          <div className="t-md" style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
            {ABILITY_DEFS.map(a => {
              const score = character.abilities?.[a.fallbackKey];
              const modVal = character.abilityModifiers?.[a.key] ?? (score != null ? Math.floor((score - 10) / 2) : null);
              return (
                <KVRow key={a.key} label={t(a.labelKey)} value={
                  <>
                    {display(score)}
                    {modVal != null && (
                      <span style={{ marginLeft: 4, color: modVal > 0 ? T.positive : modVal < 0 ? T.negative : T.textMuted }}>
                        {mod(modVal)}
                      </span>
                    )}
                  </>
                } />
              );
            })}
          </div>
        </div>

        <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
          <div className="t-section" style={{ marginBottom: 8 }}>{t('character.alignment')}</div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 3, marginBottom: 10 }}>
            {ALIGNMENT_GRID.map((a, i) => {
              const active = getAlignmentIndex(lawChaos, goodEvil) === i;
              return (
                <button
                  key={a.labelKey}
                  onClick={() => handleAlignmentSelect(a.lc, a.ge)}
                  disabled={alignmentSaving}
                  className={`t-sm t-center ${active ? 't-bold' : 't-medium'}`}
                  style={{
                    padding: '6px 4px', lineHeight: 1.2,
                    border: `2px solid ${active ? a.color : T.borderLight}`,
                    borderRadius: 4,
                    background: active ? `${a.color}20` : T.surface,
                    color: active ? a.color : T.textMuted,
                    cursor: alignmentSaving ? 'not-allowed' : 'pointer', transition: 'all 0.15s',
                  }}
                >
                  {t(a.labelKey)}
                </button>
              );
            })}
          </div>
          <div style={{ display: 'flex', gap: 10, justifyContent: 'center' }}>
            <div>
              <div className="t-xs t-semibold" style={{ color: T.textMuted, marginBottom: 3 }}>{t('alignment.lawChaosLabel')}</div>
              <StepInput value={lawChaos} onValueChange={v => handleAlignmentStep('lc', v)} min={0} max={100} width={88} />
            </div>
            <div>
              <div className="t-xs t-semibold" style={{ color: T.textMuted, marginBottom: 3 }}>{t('alignment.goodEvilLabel')}</div>
              <StepInput value={goodEvil} onValueChange={v => handleAlignmentStep('ge', v)} min={0} max={100} width={88} />
            </div>
          </div>
        </div>

      </Card>

      <DeityDialog
        isOpen={isDeityOpen}
        currentDeity={character.deity ?? ''}
        onClose={() => setIsDeityOpen(false)}
        onSelect={handleDeitySelect}
      />
      <RaceDialog
        isOpen={isRaceOpen}
        currentRaceId={character.race_id ?? 0}
        currentSubrace={character.subrace ?? null}
        onClose={() => setIsRaceOpen(false)}
        onSelect={handleRaceChange}
      />
    </div>
  );
}
