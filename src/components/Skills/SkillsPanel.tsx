import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { Alert, Button, Card, Elevation, HTMLTable, InputGroup, NonIdealState, Tag, Icon, Tooltip } from '@blueprintjs/core';
import { GiBrokenShield, GiVisoredHelm, GiAnticlockwiseRotation, GiMagnifyingGlass } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { T } from '../theme';
import { LoadingState, ModCell, mod, StepInput } from '../shared';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useSkillManagement } from '@/hooks/useSkillManagement';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { applySkillOverrides, categorizeSkills, calculateSkillBudget, filterAndSortSkills, skillMisc, type SkillSortColumn } from '@/utils/skillUtils';
import { useTranslations } from '@/hooks/useTranslations';
import { display } from '@/utils/dataHelpers';

type SortCol = SkillSortColumn | null;
type SortDir = 'asc' | 'desc';

function SortIcon({ active, dir }: { active: boolean; dir: SortDir }) {
  if (!active) return null;
  return (
    <Icon
      icon={dir === 'asc' ? 'caret-up' : 'caret-down'}
      size={10}
      style={{ marginLeft: 4 }}
    />
  );
}

export function SkillsPanel() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const skillsSubsystem = useSubsystem('skills');
  const { updateSkills, resetSkills } = useSkillManagement();
  const { handleError } = useErrorHandler();

  const [filter, setFilter] = useState('');
  const [sortCol, setSortCol] = useState<SortCol>('name');
  const [sortDir, setSortDir] = useState<SortDir>('asc');
  const [localOverrides, setLocalOverrides] = useState<Record<number, number>>({});
  const [isResetting, setIsResetting] = useState(false);
  const pendingRef = useRef<Record<number, number>>({});
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ownReloadsRef = useRef(0);
  const [fixedTotalBudget, setFixedTotalBudget] = useState<number | null>(null);
  const [showResetConfirm, setShowResetConfirm] = useState(false);

  const { data: skillsData, isLoading, error } = skillsSubsystem;

  useEffect(() => {
    if (character?.id && !skillsData && !isLoading && !error) {
      skillsSubsystem.load();
    }
  }, [character?.id, skillsData, isLoading, error, skillsSubsystem]);

  useEffect(() => {
    setLocalOverrides({});
  }, [skillsData]);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  // The backend clamps an overdrawn pool at 0, so the budget stays fixed across
  // this panel's own rank edits to keep the overdraw visible. Any other reload
  // (undo, redo, level changes) re-reads it.
  useEffect(() => {
    if (!skillsData) return;
    if (ownReloadsRef.current > 0) {
      ownReloadsRef.current -= 1;
      return;
    }
    setFixedTotalBudget(skillsData.total_available);
  }, [skillsData]);

  const { allSkills, budget } = useMemo(() => {
    const skills = applySkillOverrides(
      categorizeSkills(skillsData?.class_skills, skillsData?.cross_class_skills),
      localOverrides
    );
    const totalAvailable = fixedTotalBudget ?? skillsData?.total_available ?? 0;
    return {
      allSkills: skills,
      budget: calculateSkillBudget(totalAvailable, skillsData?.spent_points || 0),
    };
  }, [skillsData, localOverrides, fixedTotalBudget]);

  const sorted = useMemo(
    () => filterAndSortSkills(allSkills, filter, sortCol, sortDir),
    [allSkills, filter, sortCol, sortDir]
  );

  const handleSort = (col: SortCol) => {
    if (sortCol === col) {
      setSortDir(d => d === 'asc' ? 'desc' : 'asc');
    } else {
      setSortCol(col);
      setSortDir('asc');
    }
  };

  const flushSkillUpdates = useCallback(async () => {
    const batch = { ...pendingRef.current };
    pendingRef.current = {};
    if (Object.keys(batch).length === 0) return;

    ownReloadsRef.current += 1;
    try {
      await updateSkills(batch);
    } catch (err) {
      ownReloadsRef.current = Math.max(0, ownReloadsRef.current - 1);
      handleError(err);
      setLocalOverrides(prev => {
        const updated = { ...prev };
        for (const id of Object.keys(batch)) delete updated[Number(id)];
        return updated;
      });
    }
  }, [updateSkills, handleError]);

  const handleRankChange = useCallback((skillId: number, newRank: number) => {
    if (!character?.id || newRank < 0) return;

    setLocalOverrides(prev => ({ ...prev, [skillId]: newRank }));
    pendingRef.current[skillId] = newRank;

    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(flushSkillUpdates, 50);
  }, [character?.id, flushSkillUpdates]);

  const handleReset = async () => {
    if (!character?.id) return;
    setIsResetting(true);
    try {
      await resetSkills();
    } catch (err) {
      handleError(err);
    } finally {
      setIsResetting(false);
    }
  };

  const thSortable = (col: SortCol) => ({
    style: { textAlign: 'center' as const, cursor: 'pointer', userSelect: 'none' as const },
    onClick: () => handleSort(col),
  });

  if (isLoading && !skillsData) {
    return (
      <LoadingState text={t('common.loadingSection', { section: t('navigation.skills').toLowerCase() })} />
    );
  }

  if (error) {
    return (
      <div style={{ padding: 16 }}>
        <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} title={t('common.failedToLoad', { section: t('navigation.skills').toLowerCase() })} description={error} />
      </div>
    );
  }

  if (!character || !skillsData) {
    return (
      <div style={{ padding: 16 }}>
        <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('common.noCharacterLoaded')} description={t('common.loadSaveToView', { section: t('navigation.skills').toLowerCase() })} />
      </div>
    );
  }

  const levellingClass = skillsData.levelling_class_name ?? display(null);

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>

        <div style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '10px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
          <span style={{ color: T.textMuted }}>{t('skills.spent')}: <strong style={{ color: T.text }}>{budget.displayedSpent}</strong></span>
          <span style={{ color: T.textMuted }}>{t('skills.available')}: <strong style={{ color: T.accent }}>{budget.available}</strong></span>
          <span style={{ color: budget.overdrawn > 0 ? T.negative : T.textMuted }}>{t('skills.overdrawn')}: <strong>{budget.overdrawn}</strong></span>
          <Tooltip content={t('skills.levellingAsHint')} placement="bottom">
            <span style={{ color: T.textMuted, display: 'inline-flex', alignItems: 'center', gap: 4 }}>
              {t('skills.levellingAs')}: <strong style={{ color: T.text }}>{levellingClass}</strong>
              <Icon icon="info-sign" size={12} />
            </span>
          </Tooltip>
          <div style={{ flex: 1 }} />
          <InputGroup
            leftIcon="search" placeholder={t('skills.searchSkills')} value={filter}
            onChange={e => setFilter(e.target.value)}
            rightElement={filter ? <Button icon="cross" minimal small onClick={() => setFilter('')} /> : undefined}
            style={{ maxWidth: 240 }}
          />
          <Button icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} text={t('skills.reset')} minimal style={{ color: T.textMuted }}
            onClick={() => setShowResetConfirm(true)} disabled={isResetting} />
        </div>

        <div style={{ padding: '12px 16px 16px' }}>
          <HTMLTable compact striped bordered interactive style={{ width: '100%', tableLayout: 'fixed' }}>
            <colgroup>
              <col style={{ width: 240 }} />
              <col style={{ width: 120 }} />
              <col style={{ width: 64 }} />
              <col style={{ width: 64 }} />
              <col style={{ width: 64 }} />
              <col style={{ width: 80 }} />
            </colgroup>
            <thead>
              <tr>
                <th {...thSortable('name')} style={{ textAlign: 'left', cursor: 'pointer', userSelect: 'none' }}>
                  {t('skills.skillName')}<SortIcon active={sortCol === 'name'} dir={sortDir} />
                </th>
                <th {...thSortable('ranks')}>
                  {t('skills.ranks')}<SortIcon active={sortCol === 'ranks'} dir={sortDir} />
                </th>
                <th {...thSortable('modifier')}>
                  {t('skills.ability')}<SortIcon active={sortCol === 'modifier'} dir={sortDir} />
                </th>
                <th {...thSortable('misc')}>
                  {t('skills.misc')}<SortIcon active={sortCol === 'misc'} dir={sortDir} />
                </th>
                <th {...thSortable('total')}>
                  {t('skills.total')}<SortIcon active={sortCol === 'total'} dir={sortDir} />
                </th>
                <th {...thSortable('class')}>
                  {t('skills.class')}<SortIcon active={sortCol === 'class'} dir={sortDir} />
                </th>
              </tr>
            </thead>
            <tbody>
              {sorted.map(s => {
                const misc = skillMisc(s);
                return (
                  <tr key={s.skill_id}>
                    <td>
                      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                        <strong style={{ color: T.text }}>{s.name}</strong>
                        <span style={{ color: T.textMuted, display: 'inline-block', width: 28 }}>{t(`abilityScores.short.${s.ability.slice(0, 3).toLowerCase()}`)}</span>
                        {s.armor_check_penalty && (
                          <Tag minimal className="t-xs" style={{ padding: '0 4px', lineHeight: '16px', background: T.sectionBg, color: T.accent }}>{t('skills.acp')}</Tag>
                        )}
                        {!s.is_class_skill && (
                          <Tag minimal className="t-xs" style={{ padding: '0 4px', lineHeight: '16px', background: T.sectionBg, color: T.textMuted }}>{t('skills.twoPoints')}</Tag>
                        )}
                      </span>
                    </td>
                    <td style={{ textAlign: 'center' }}>
                      <StepInput
                        value={s.ranks}
                        onValueChange={(v) => handleRankChange(s.skill_id, v)}
                        min={0} max={99} width={88}
                      />
                    </td>
                    <td style={{ textAlign: 'center' }}><ModCell value={s.modifier} /></td>
                    <td style={{ textAlign: 'center', color: T.textMuted }}>{misc ? mod(misc) : '-'}</td>
                    <td className="t-bold" style={{ textAlign: 'center', color: T.text }}>{mod(s.total)}</td>
                    <td style={{ textAlign: 'center' }}>
                      {s.is_class_skill
                        ? <span className="t-medium" style={{ color: T.positive }}>{t('skills.class')}</span>
                        : <span style={{ color: T.textMuted }}>{t('skills.cross')}</span>}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </HTMLTable>
          <div style={{ marginTop: 8, display: 'flex', gap: 16, color: T.textMuted }}>
            <span><Tag minimal className="t-xs" style={{ padding: '0 4px', lineHeight: '16px', background: T.sectionBg, color: T.accent }}>{t('skills.acp')}</Tag> {t('skills.armorCheck')}</span>
            <span><Tag minimal className="t-xs" style={{ padding: '0 4px', lineHeight: '16px', background: T.sectionBg, color: T.textMuted }}>{t('skills.twoPoints')}</Tag> {t('skills.crossClassLegend', { class: levellingClass })}</span>
          </div>
        </div>
      </Card>

      <Alert
        isOpen={showResetConfirm}
        onCancel={() => setShowResetConfirm(false)}
        onConfirm={() => { setShowResetConfirm(false); handleReset(); }}
        cancelButtonText={t('skills.cancel')}
        confirmButtonText={t('skills.reset')}
        intent="danger"
        icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />}
      >
        <p>{t('skills.resetConfirmMessage')}</p>
      </Alert>
    </div>
  );
}
