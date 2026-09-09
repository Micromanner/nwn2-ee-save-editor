import { useState } from 'react';
import { Button, Icon, Popover, Tag } from '@blueprintjs/core';
import type { IconType } from 'react-icons';
import { GiSkills, GiStarMedal, GiMuscleUp, GiSpellBook, GiUpgrade } from 'react-icons/gi';
import { T } from '../theme';
import { GameIcon } from './GameIcon';
import { useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';

interface PendingGain {
  label: string;
  count: number;
  icon: IconType;
  color: string;
  tagIntent: 'success' | 'primary' | 'warning' | 'none';
  tabId: string;
}

interface SpellClass {
  name: string;
  casterType: string;
  byLevel: Record<number, number>;
  total: number;
}

interface PendingSpellLearning {
  class_id: number;
  class_name: string;
  caster_type: string;
  by_level: Record<number, number>;
  total: number;
}

const SPELL_COLOR = '#5c7cfa';

interface LevelHelperProps {
  onNavigate?: (tabId: string) => void;
}

export function LevelHelper({ onNavigate }: LevelHelperProps) {
  const t = useTranslations();
  const [isOpen, setIsOpen] = useState(false);
  const [spellsExpanded, setSpellsExpanded] = useState(false);

  const skillsSub = useSubsystem('skills');
  const featsSub = useSubsystem('feats');
  const abilitiesSub = useSubsystem('abilityScores');
  const spellsSub = useSubsystem('spells');

  const remainingSkillPoints = skillsSub.data
    ? skillsSub.data.total_available - skillsSub.data.spent_points
    : 0;

  const openFeatSlots = featsSub.data?.feat_slots?.open_slots ?? 0;

  const pendingAbilityIncreases = abilitiesSub.data?.point_summary?.available ?? 0;

  const gains: PendingGain[] = [
    remainingSkillPoints > 0 && {
      label: t('levelHelper.skillPoints'),
      count: remainingSkillPoints,
      icon: GiSkills,
      color: T.positive,
      tagIntent: 'success' as const,
      tabId: 'skills',
    },
    openFeatSlots > 0 && {
      label: t('levelHelper.featSlots'),
      count: openFeatSlots,
      icon: GiStarMedal,
      color: T.accent,
      tagIntent: 'primary' as const,
      tabId: 'feats',
    },
    pendingAbilityIncreases > 0 && {
      label: t('levelHelper.abilityScoreIncrease'),
      count: pendingAbilityIncreases,
      icon: GiMuscleUp,
      color: T.gold,
      tagIntent: 'warning' as const,
      tabId: 'abilities',
    },
  ].filter(Boolean) as PendingGain[];

  const pending: PendingSpellLearning[] = spellsSub.data?.pending_spell_learning ?? [];

  const spellClasses: SpellClass[] = pending
    .filter(cls => cls.total > 0)
    .map(cls => ({
      name: cls.class_name,
      casterType: cls.caster_type,
      byLevel: cls.by_level,
      total: cls.total,
    }));

  const spellTotal = spellClasses.reduce((s, c) => s + c.total, 0);
  const hasSpells = spellTotal > 0;
  const totalPending = gains.reduce((s, g) => s + g.count, 0) + spellTotal;

  if (totalPending === 0) return null;

  const handleRowClick = (tabId: string) => {
    onNavigate?.(tabId);
  };

  const rowStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    padding: '8px 10px',
    borderRadius: 4,
    cursor: 'pointer',
    background: T.surfaceAlt,
    transition: 'background 0.15s',
  };

  const hoverHandlers = {
    onMouseEnter: (e: React.MouseEvent<HTMLDivElement>) => { e.currentTarget.style.background = T.sectionBg; },
    onMouseLeave: (e: React.MouseEvent<HTMLDivElement>) => { e.currentTarget.style.background = T.surfaceAlt; },
  };

  const content = (
    <div style={{ width: 280, background: T.surface, borderRadius: 4, overflow: 'hidden' }}>
      <div style={{
        padding: '10px 14px',
        background: T.sectionBg,
        borderBottom: `1px solid ${T.sectionBorder}`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
      }}>
        <span className="t-section" style={{ letterSpacing: '0.04em' }}>
          {t('levelHelper.pendingAllocations')}
        </span>
        <Button minimal small icon="cross" onClick={() => setIsOpen(false)} />
      </div>

      <div style={{ padding: '8px 10px' }}>
        <p className="t-sm" style={{ color: T.textMuted, margin: '0 0 8px 0' }}>
          {t('levelHelper.pendingGainsMessage')}
        </p>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {gains.map(gain => (
            <div
              key={gain.tabId}
              onClick={() => handleRowClick(gain.tabId)}
              style={rowStyle}
              {...hoverHandlers}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={gain.icon} size={14} style={{ color: gain.color }} />
                <span className="t-medium" style={{ color: T.text }}>{gain.label}</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                <Tag minimal round intent={gain.tagIntent} className="t-sm t-center" style={{ minWidth: 20 }}>
                  {gain.count}
                </Tag>
                <Icon icon="chevron-right" size={12} style={{ color: T.textMuted }} />
              </div>
            </div>
          ))}

          {hasSpells && (
            <div>
              <div style={rowStyle} {...hoverHandlers}>
                <div
                  style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1 }}
                  onClick={() => handleRowClick('spells')}
                >
                  <GameIcon icon={GiSpellBook} size={14} style={{ color: SPELL_COLOR }} />
                  <span className="t-medium" style={{ color: T.text }}>{t('levelHelper.spellsToLearn')}</span>
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <Tag minimal round className="t-sm t-center" style={{ minWidth: 20, color: SPELL_COLOR }}>
                    {spellTotal}
                  </Tag>
                  <Button
                    minimal
                    small
                    icon={spellsExpanded ? 'chevron-up' : 'chevron-down'}
                    onClick={(e) => { e.stopPropagation(); setSpellsExpanded(!spellsExpanded); }}
                    style={{ minHeight: 16, minWidth: 16, padding: 0 }}
                  />
                </div>
              </div>

              {spellsExpanded && (
                <div style={{
                  marginLeft: 16,
                  paddingLeft: 12,
                  borderLeft: `2px solid ${T.border}`,
                  marginTop: 4,
                }}>
                  {spellClasses.map((cls, i) => (
                    <div key={i} style={{ marginBottom: i < spellClasses.length - 1 ? 8 : 0 }}>
                      <div className="t-semibold" style={{ color: T.text, marginBottom: 2 }}>
                        {cls.name}
                        {cls.casterType === 'spellbook' && (
                          <span style={{ color: T.textMuted, marginLeft: 4 }}>({t('levelHelper.spellbook')})</span>
                        )}
                      </div>
                      {cls.casterType === 'spellbook' ? (
                        <div className="t-sm" style={{ display: 'flex', justifyContent: 'space-between', color: T.textMuted, paddingLeft: 8 }}>
                          <span>{t('levelHelper.freeSpells')}</span>
                          <span className="t-semibold" style={{ color: SPELL_COLOR }}>{cls.total}</span>
                        </div>
                      ) : (
                        Object.entries(cls.byLevel)
                          .sort(([a], [b]) => Number(a) - Number(b))
                          .map(([level, count]) => (
                            <div key={level} className="t-sm" style={{ display: 'flex', justifyContent: 'space-between', color: T.textMuted, paddingLeft: 8 }}>
                              <span>{Number(level) === 0 ? t('levelHelper.cantrips') : t('levelHelper.spellLevel', { level })}</span>
                              <span className="t-semibold" style={{ color: SPELL_COLOR }}>{count}</span>
                            </div>
                          ))
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );

  return (
    <div style={{ position: 'fixed', bottom: 24, right: 24, zIndex: 50 }}>
      <Popover
        content={content}
        isOpen={isOpen}
        placement="top-end"
        minimal
        popoverClassName="level-helper-popover"
      >
        <Button
          intent="primary"
          icon={<GameIcon icon={GiUpgrade} size={20} />}
          onClick={() => setIsOpen(!isOpen)}
          style={{
            width: 48,
            height: 48,
            borderRadius: '50%',
            boxShadow: '0 4px 12px rgba(0,0,0,0.25)',
            position: 'relative',
          }}
        />
      </Popover>

      <div className="t-sm t-bold" style={{
        position: 'absolute',
        top: -4,
        right: -4,
        background: T.negative,
        color: '#fff',
        width: 22,
        height: 22,
        borderRadius: '50%',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        border: `2px solid ${T.surface}`,
        pointerEvents: 'none',
      }}>
        {totalPending}
      </div>
    </div>
  );
}
