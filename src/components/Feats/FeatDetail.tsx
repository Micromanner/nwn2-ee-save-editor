import { useState } from 'react';
import { Button } from '@blueprintjs/core';
import { GiCancel, GiCheckMark } from 'react-icons/gi';
import { T, FEAT_TYPE_COLORS } from '../theme';
import { GameIcon } from '../shared/GameIcon';
import type { FeatInfo, BackendFeatPrerequisites } from '@/components/Feats/types';
import { FEAT_TYPE_LABELS, getFeatTypeLabel } from '@/utils/featUtils';
import { DetailSection, FormattedDescription } from '../shared';
import { display } from '@/utils/dataHelpers';
import { useTranslations } from '@/hooks/useTranslations';
import { useIcon } from '@/hooks/useIcon';
import { parseFeatDescription } from '@/utils/descriptionParser';

interface FeatDetailProps {
  feat: FeatInfo | null;
  isOwned: boolean;
  onAdd?: (featId: number) => Promise<void>;
  onRemove?: (featId: number) => Promise<void>;
}


function isBackendPrerequisites(prereqs: unknown): prereqs is BackendFeatPrerequisites {
  if (!prereqs || typeof prereqs !== 'object') return false;
  const p = prereqs as Record<string, unknown>;
  return 'feats' in p || 'abilities' in p || 'bab' in p || 'level' in p || 'skills' in p;
}

export function FeatDetail({ feat, isOwned, onAdd, onRemove }: FeatDetailProps) {
  const t = useTranslations();
  const [busy, setBusy] = useState(false);
  const iconUrl = useIcon(feat?.icon);

  if (!feat) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: T.textMuted }}>
        {t('feats.selectFeatDetail')}
      </div>
    );
  }

  const handleAdd = async () => {
    if (!onAdd) return;
    setBusy(true);
    try { await onAdd(feat.id); } finally { setBusy(false); }
  };

  const handleRemove = async () => {
    if (!onRemove) return;
    setBusy(true);
    try { await onRemove(feat.id); } finally { setBusy(false); }
  };

  const labelKey = getFeatTypeLabel(feat.type);
  const typeColor = FEAT_TYPE_COLORS[labelKey] || T.textMuted;
  const prereqs = isBackendPrerequisites(feat.prerequisites) ? feat.prerequisites : null;

  const hasPrereqs = prereqs && (
    (prereqs.feats?.length ?? 0) > 0 ||
    (prereqs.abilities?.length ?? 0) > 0 ||
    prereqs.bab !== null ||
    prereqs.level !== null ||
    (prereqs.skills?.length ?? 0) > 0
  );

  return (
    <div style={{ padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 12 }}>

      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          {iconUrl && (
            <img
              src={iconUrl}
              alt=""
              width={32}
              height={32}
              style={{ borderRadius: 4, flexShrink: 0 }}
            />
          )}
          <div>
            <span className="t-bold" style={{ color: T.text }}>{display(feat.name)}</span>
            <span style={{ color: T.textMuted }}> — </span>
            <span className="t-medium" style={{ color: typeColor }}>{t(labelKey)}</span>
            {feat.protected && (
              <span className="t-sm t-italic" style={{ marginLeft: 8, color: T.textMuted }}>{t('feats.protected')}</span>
            )}
          </div>
        </div>
        <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>
          {isOwned && onRemove && (
            <Button
              small
              intent="danger"
              icon="trash"
              text={t('placeholders.removeFeat')}
              loading={busy}
              onClick={handleRemove}
            />
          )}
          {!isOwned && onAdd && (
            <Button
              small
              intent="success"
              icon="plus"
              text={t('placeholders.addFeat')}
              loading={busy}
              onClick={handleAdd}
            />
          )}
        </div>
      </div>

      {feat.description && (() => {
        const parsed = parseFeatDescription(feat.description);
        return (
          <DetailSection title={t('feats.description')}>
            <FormattedDescription sections={parsed.sections} />
          </DetailSection>
        );
      })()}

      {feat.missing_requirements && feat.missing_requirements.length > 0 && (
        <DetailSection title={t('feats.missingRequirements')}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {feat.missing_requirements.map((req, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={GiCancel} size={14} style={{ color: T.negative }} />
                <span style={{ color: T.text }}>{req}</span>
              </div>
            ))}
          </div>
        </DetailSection>
      )}

      {hasPrereqs && prereqs && (
        <DetailSection title={t('feats.prerequisites')}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {prereqs.feats?.map(p => (
              <div key={p.id} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={p.met ? GiCheckMark : GiCancel} size={14} style={{ color: p.met ? T.positive : T.negative }} />
                <span style={{ color: T.text }}>{p.name}</span>
              </div>
            ))}
            {prereqs.abilities?.map((p, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={p.met ? GiCheckMark : GiCancel} size={14} style={{ color: p.met ? T.positive : T.negative }} />
                <span style={{ color: T.text }}>{t(`abilityScores.short.${p.ability.slice(0, 3).toLowerCase()}`)} {p.required}</span>
                <span style={{ color: T.textMuted }}>({p.current}/{p.required})</span>
              </div>
            ))}
            {prereqs.bab && (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={prereqs.bab.met ? GiCheckMark : GiCancel} size={14} style={{ color: prereqs.bab.met ? T.positive : T.negative }} />
                <span style={{ color: T.text }}>{t('classes.bab')} +{prereqs.bab.required}</span>
                <span style={{ color: T.textMuted }}>({prereqs.bab.current}/{prereqs.bab.required})</span>
              </div>
            )}
            {prereqs.level && (
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={prereqs.level.met ? GiCheckMark : GiCancel} size={14} style={{ color: prereqs.level.met ? T.positive : T.negative }} />
                <span style={{ color: T.text }}>{t('classes.level')} {prereqs.level.required}</span>
                <span style={{ color: T.textMuted }}>({prereqs.level.current}/{prereqs.level.required})</span>
              </div>
            )}
            {prereqs.skills?.map((p, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <GameIcon icon={p.met ? GiCheckMark : GiCancel} size={14} style={{ color: p.met ? T.positive : T.negative }} />
                <span style={{ color: T.text }}>{p.skill} {p.required} {t('feats.ranks')}</span>
                <span style={{ color: T.textMuted }}>({p.current}/{p.required})</span>
              </div>
            ))}
            {!isOwned && onAdd && (
              prereqs.feats?.some(p => !p.met) ||
              prereqs.abilities?.some(p => !p.met) ||
              (prereqs.bab && !prereqs.bab.met) ||
              (prereqs.level && !prereqs.level.met) ||
              prereqs.skills?.some(p => !p.met)
            ) && (
              <div className="t-sm" style={{ color: T.textMuted, fontStyle: 'italic', marginTop: 4 }}>
                {t('feats.autoAddHint')}
              </div>
            )}
          </div>
        </DetailSection>
      )}
    </div>
  );
}
