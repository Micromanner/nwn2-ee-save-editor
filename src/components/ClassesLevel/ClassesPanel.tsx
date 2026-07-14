import { useState, useEffect, useMemo } from 'react';
import { Button, Card, Elevation, HTMLTable, NonIdealState, ProgressBar, Spinner, Tag } from '@blueprintjs/core';
import { GiBrokenShield, GiVisoredHelm, GiHazardSign } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { T } from '../theme';
import { StepInput } from '../shared';
import { ClassSelectorDialog } from './ClassSelectorDialog';
import { useSubsystem, useCharacterContext } from '@/contexts/CharacterContext';
import { useClassesLevel, type ClassesData, type ClassInfo } from '@/hooks/useClassesLevel';
import { useTranslations } from '@/hooks/useTranslations';
import { formatModifier, formatNumber } from '@/utils/dataHelpers';
import { capXP, aggregateClassStats, hasLevelMismatch, VAULT_MAX_VISIBLE_XP } from '@/utils/classUtils';
import { useErrorHandler } from '@/hooks/useErrorHandler';

function SectionLabel({ children, right }: { children: string; right?: React.ReactNode }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 8 }}>
      <div className="t-section" style={{ color: T.accent }}>
        {children}
      </div>
      {right}
    </div>
  );
}

export function ClassesPanel() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { sessionKind } = useCharacterContext();

  const classesSubsystem = useSubsystem('classes');
  const {
    classes,
    totalLevel,
    xpProgress,
    categorizedClasses,
    isUpdating,
    adjustClassLevel,
    changeClass,
    addClass,
    removeClass,
    isAtMaxLevel,
    getRemainingLevels,
    setExperience,
    fetchXPProgress,
  } = useClassesLevel(classesSubsystem.data as ClassesData | null);

  const [xpInput, setXpInput] = useState('');
  const [selectorOpen, setSelectorOpen] = useState(false);
  const [changingIndex, setChangingIndex] = useState<number | null>(null);

  const maxLevel = 60;
  const maxClasses = 4;

  // Sync XP input when xpProgress loads or changes
  useEffect(() => {
    if (xpProgress) {
      setXpInput(xpProgress.current_xp.toString());
    }
  }, [xpProgress]);

  // Load subsystem on mount if not already loaded
  useEffect(() => {
    if (!classesSubsystem.data && !classesSubsystem.isLoading) {
      classesSubsystem.load().catch(() => {});
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [classesSubsystem.data, classesSubsystem.isLoading]);

  // Fetch XP progress when data is loaded but xpProgress missing
  useEffect(() => {
    if (classesSubsystem.data && !xpProgress) {
      fetchXPProgress().catch(() => {});
    }
  }, [classesSubsystem.data, xpProgress, fetchXPProgress]);

  const levelHistoryReversed = useMemo(
    () => [...(classesSubsystem.data?.level_history ?? [])].reverse(),
    [classesSubsystem.data?.level_history]
  );

  const currentXP = xpProgress?.current_xp ?? 0;
  const xpDirty = xpInput !== currentXP.toString();
  const levelMismatch = hasLevelMismatch(xpProgress, totalLevel);
  const inputXp = parseInt(xpInput, 10);
  const showVaultXpWarning =
    sessionKind === 'vault' && (isNaN(inputXp) ? currentXP : inputXp) > VAULT_MAX_VISIBLE_XP;
  const { totalBAB, totalFort, totalRef, totalWill } = useMemo(
    () => aggregateClassStats(
      classes.map(c => ({
        baseAttackBonus: c.baseAttackBonus,
        fortitudeSave: c.fortitudeSave,
        reflexSave: c.reflexSave,
        willSave: c.willSave,
      }))
    ),
    [classes]
  );

  const handleXpSubmit = async () => {
    let val = parseInt(xpInput, 10);
    if (isNaN(val) || val < 0) return;
    val = capXP(val);
    if (val.toString() !== xpInput) setXpInput(val.toString());
    try {
      await setExperience(val);
    } catch (err) {
      handleError(err);
    }
  };

  const handleXpReset = () => {
    setXpInput(currentXP.toString());
  };

  const handleAdjustLevel = async (index: number, delta: number) => {
    const cls = classes[index];
    if (!cls) return;
    try {
      await adjustClassLevel(cls.id, delta);
    } catch (err) {
      handleError(err);
    }
  };

  const handleRemoveClass = async (index: number) => {
    const cls = classes[index];
    if (!cls) return;
    try {
      await removeClass(cls.id);
    } catch (err) {
      handleError(err);
    }
  };

  const handleOpenChangeClass = (index: number) => {
    setChangingIndex(index);
    setSelectorOpen(true);
  };

  const handleOpenAddClass = () => {
    setChangingIndex(null);
    setSelectorOpen(true);
  };

  const handleSelectClass = async (classInfo: ClassInfo) => {
    try {
      if (changingIndex !== null) {
        const cls = classes[changingIndex];
        if (cls) {
          await changeClass(cls.id, classInfo);
        }
      } else {
        await addClass(classInfo);
      }
    } catch (err) {
      handleError(err);
    } finally {
      setSelectorOpen(false);
      setChangingIndex(null);
    }
  };

  // Loading state
  if (classesSubsystem.isLoading && !classesSubsystem.data) {
    return (
      <div style={{ padding: 32, display: 'flex', justifyContent: 'center' }}>
        <Spinner size={32} />
      </div>
    );
  }

  // Error state
  if (classesSubsystem.error && !classesSubsystem.data) {
    return (
      <div style={{ padding: 16 }}>
        <NonIdealState
          icon={<GameIcon icon={GiBrokenShield} size={40} />}
          title={t('common.failedToLoad', { section: t('navigation.classes').toLowerCase() })}
          description={classesSubsystem.error}
          action={
            <Button intent="primary" onClick={() => classesSubsystem.load()}>
              {t('common.retry')}
            </Button>
          }
        />
      </div>
    );
  }

  // Empty / no character state
  if (!classesSubsystem.data) {
    return (
      <div style={{ padding: 16 }}>
        <NonIdealState
          icon={<GameIcon icon={GiVisoredHelm} size={40} />}
          title={t('common.noCharacterLoaded')}
          description={t('common.loadSaveToView', { section: t('navigation.classes').toLowerCase() })}
        />
      </div>
    );
  }

  // Build currentClasses in the shape ClassSelectorDialog expects
  const selectorCurrentClasses = classes.map(c => ({
    id: c.id,
    name: c.name,
  }));

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>

      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>

        {/* XP Bar */}
        <div style={{ padding: '10px 16px', display: 'flex', alignItems: 'center', gap: 12, borderBottom: `1px solid ${T.borderLight}` }}>
          <span className="t-semibold" style={{ color: T.textMuted }}>{t('classes.experience')}</span>
          <input
            type="text"
            value={xpInput}
            onChange={(e) => {
              const v = e.target.value;
              if (v === '' || /^\d+$/.test(v)) setXpInput(v);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleXpSubmit();
              if (e.key === 'Escape') handleXpReset();
            }}
            className="bp6-input"
            style={{ width: 110, textAlign: 'center', padding: '2px 8px', height: 26 }}
            disabled={isUpdating}
          />
          <Button minimal icon="tick" intent="success" onClick={handleXpSubmit} disabled={!xpDirty || isUpdating} style={{ opacity: xpDirty ? 1 : 0.3 }} />
          <Button minimal icon="cross" onClick={handleXpReset} disabled={!xpDirty || isUpdating} style={{ opacity: xpDirty ? 1 : 0.3 }} />
          <div style={{ flex: 1 }}>
            <ProgressBar
              value={xpProgress ? xpProgress.progress_percent / 100 : 0}
              intent="primary"
              stripes={false}
              animate={false}
              style={{ height: 4 }}
            />
          </div>
          <span style={{ color: T.textMuted, whiteSpace: 'nowrap' }}>
            {t('classes.lvl')} {xpProgress?.current_level ?? '-'} | {formatNumber(xpProgress?.xp_remaining)} {t('classes.xpToNextLevel')}
          </span>
          {levelMismatch && xpProgress && (
            <Tag minimal round intent="warning" icon={<GameIcon icon={GiHazardSign} size={12} />}>
              {t('classes.xpLevelMismatchWarning', {
                xpLevel: String(xpProgress.current_level),
                classLevel: String(totalLevel),
              })}
            </Tag>
          )}
        </div>

        {showVaultXpWarning && (
          <div style={{ padding: '6px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
            <Tag minimal round intent="warning" icon={<GameIcon icon={GiHazardSign} size={12} />}>
              {t('classes.vaultExpWarning')}
            </Tag>
          </div>
        )}

        {/* Classes Table */}
        <div style={{ padding: '12px 16px 16px' }}>
          <SectionLabel>{t('classes.currentClasses')}</SectionLabel>
          <HTMLTable compact striped bordered style={{ width: '100%', tableLayout: 'fixed' }}>
            <colgroup>
              <col style={{ width: 140 }} />
              <col style={{ width: 110 }} />
              <col style={{ width: 60 }} />
              <col style={{ width: 60 }} />
              <col style={{ width: 60 }} />
              <col style={{ width: 60 }} />
              <col style={{ width: 50 }} />
              <col style={{ width: 50 }} />
              <col style={{ width: 72 }} />
            </colgroup>
            <thead>
              <tr>
                <th>{t('classes.class')}</th>
                <th style={{ textAlign: 'center' }}>{t('classes.level')}</th>
                <th style={{ textAlign: 'center' }}>{t('classes.bab')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.fortitude')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.reflex')}</th>
                <th style={{ textAlign: 'center' }}>{t('abilityScores.will')}</th>
                <th style={{ textAlign: 'center' }}>{t('classes.hitDie')}</th>
                <th style={{ textAlign: 'center' }}>{t('classes.skillPoints')}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {classes.map((cls, i) => {
                const atMax = isAtMaxLevel(cls.id);
                const remaining = getRemainingLevels(cls.id);
                const classLevelCap = (cls.max_level && cls.max_level > 0) ? cls.max_level : maxLevel;
                const maxForClass = Math.min(classLevelCap, maxLevel - totalLevel + cls.level);
                return (
                  <tr key={`${cls.id}-${i}`}>
                    <td>
                      <Button small minimal rightIcon="edit" onClick={() => handleOpenChangeClass(i)}
                        className="t-medium" style={{ color: T.accent }} disabled={isUpdating}>
                        {cls.name}
                      </Button>
                    </td>
                    <td style={{ textAlign: 'center' }}>
                      <StepInput
                        value={cls.level}
                        onValueChange={(v) => handleAdjustLevel(i, v - cls.level)}
                        min={1}
                        max={maxForClass}
                        width={88}
                        disabled={isUpdating}
                      />
                      {cls.max_level != null && cls.max_level > 0 && cls.max_level < 60 && (
                        <div className="t-xs" style={{ color: atMax ? T.negative : T.textMuted, marginTop: 1 }}>
                          {atMax ? 'Max Level' : `${remaining} left`}
                        </div>
                      )}
                    </td>
                    <td className="t-medium" style={{ textAlign: 'center' }}>{formatModifier(cls.baseAttackBonus)}</td>
                    <td className="t-medium" style={{ textAlign: 'center' }}>{formatModifier(cls.fortitudeSave)}</td>
                    <td className="t-medium" style={{ textAlign: 'center' }}>{formatModifier(cls.reflexSave)}</td>
                    <td className="t-medium" style={{ textAlign: 'center' }}>{formatModifier(cls.willSave)}</td>
                    <td style={{ textAlign: 'center' }}>d{cls.hitDie}</td>
                    <td style={{ textAlign: 'center' }}>{cls.skillPoints}</td>
                    <td style={{ textAlign: 'center' }}>
                      {classes.length > 1 && (
                        <Button small minimal icon="trash" intent="danger" disabled={isUpdating} onClick={() => handleRemoveClass(i)}>
                          {t('classes.remove')}
                        </Button>
                      )}
                    </td>
                  </tr>
                );
              })}
              <tr className="t-bold">
                <td style={{ color: T.textMuted }}>{t('classes.stats')}</td>
                <td style={{ textAlign: 'center' }}>{totalLevel}<span className="t-medium" style={{ color: T.textMuted }}>/60</span></td>
                <td style={{ textAlign: 'center' }}>{formatModifier(totalBAB)}</td>
                <td style={{ textAlign: 'center' }}>{formatModifier(totalFort)}</td>
                <td style={{ textAlign: 'center' }}>{formatModifier(totalRef)}</td>
                <td style={{ textAlign: 'center' }}>{formatModifier(totalWill)}</td>
                <td />
                <td />
                <td />
              </tr>
            </tbody>
          </HTMLTable>
          {classes.length < maxClasses && totalLevel < maxLevel && (
            <button
              onClick={handleOpenAddClass}
              disabled={isUpdating}
              className="t-md t-medium"
              style={{
                width: '100%', marginTop: 8, padding: '8px 0',
                border: `2px dashed ${T.border}`, borderRadius: 4,
                background: 'transparent', color: T.accent,
                cursor: isUpdating ? 'not-allowed' : 'pointer',
                transition: 'background 0.15s, border-color 0.15s',
                opacity: isUpdating ? 0.5 : 1,
              }}
              onMouseEnter={e => { if (!isUpdating) { e.currentTarget.style.background = `${T.accent}10`; e.currentTarget.style.borderColor = T.accent; } }}
              onMouseLeave={e => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.borderColor = T.border; }}
            >
              + {t('classes.addClass')}
            </button>
          )}
        </div>
      </Card>

      {/* Level History */}
      <Card elevation={Elevation.ONE} style={{ padding: '12px 16px 16px', background: T.surface }}>
        <SectionLabel>{t('classes.levelHistory')}</SectionLabel>
        {levelHistoryReversed.length === 0 ? (
          <div className="t-base" style={{ color: T.textMuted, padding: '8px 0' }}>{t('classes.noLevelHistory')}</div>
        ) : (
          <HTMLTable compact striped bordered style={{ width: '100%', tableLayout: 'fixed' }}>
            <colgroup>
              <col style={{ width: 64 }} />
              <col style={{ width: 120 }} />
              <col style={{ width: 72 }} />
              <col style={{ width: 48 }} />
              <col style={{ width: 72 }} />
              <col style={{ width: '25%' }} />
              <col />
            </colgroup>
            <thead>
              <tr>
                <th style={{ textAlign: 'center' }}>{t('classes.level')}</th>
                <th>{t('classes.class')}</th>
                <th style={{ textAlign: 'center' }}>{t('classes.hpGained')}</th>
                <th style={{ textAlign: 'center' }}>SP Left</th>
                <th style={{ textAlign: 'center' }}>{t('classes.abilityIncrease')}</th>
                <th>{t('classes.skills')}</th>
                <th>{t('classes.feats')}</th>
              </tr>
            </thead>
            <tbody>
              {levelHistoryReversed.map(lv => (
                <tr key={lv.character_level}>
                  <td className="t-semibold" style={{ textAlign: 'center' }}>{lv.character_level}</td>
                  <td className="t-medium" style={{ color: T.accent }}>{lv.class_name}</td>
                  <td className="t-medium" style={{ textAlign: 'center', color: T.positive }}>+{lv.hp_gained}</td>
                  <td style={{ textAlign: 'center' }}>{lv.skill_points_remaining}</td>
                  <td className={lv.ability_increase ? 't-semibold' : undefined} style={{ textAlign: 'center', color: lv.ability_increase ? T.accent : T.textMuted }}>
                    {lv.ability_increase ?? '-'}
                  </td>
                  <td>
                    {lv.skills_gained && lv.skills_gained.length > 0
                      ? lv.skills_gained.map(s => `${s.name} +${s.ranks}`).join(', ')
                      : <span style={{ color: T.textMuted }}>-</span>
                    }
                  </td>
                  <td>
                    {lv.feats_gained && lv.feats_gained.length > 0
                      ? lv.feats_gained.map(f => f.name.replace(/<[^>]*>/g, '')).join(', ')
                      : <span style={{ color: T.textMuted }}>-</span>
                    }
                  </td>
                </tr>
              ))}
            </tbody>
          </HTMLTable>
        )}
      </Card>

      <ClassSelectorDialog
        isOpen={selectorOpen}
        onClose={() => { setSelectorOpen(false); setChangingIndex(null); }}
        onSelect={handleSelectClass}
        currentClassIds={selectorCurrentClasses.map(c => c.id)}
        categorizedClasses={categorizedClasses}
        isChanging={changingIndex !== null}
        totalLevel={totalLevel}
        maxLevel={maxLevel}
        maxClasses={maxClasses}
        currentClassCount={classes.length}
      />
    </div>
  );
}
