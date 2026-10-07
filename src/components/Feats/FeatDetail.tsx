import { useState } from 'react';
import { Button } from '@blueprintjs/core';
import { GiCancel, GiCheckMark } from 'react-icons/gi';
import { T, FEAT_TYPE_COLORS } from '../theme';
import { GameIcon } from '../shared/GameIcon';
import type { FeatInfo, PrereqValue } from '@/lib/bindings';
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


function PrereqRow({ met, label, detail }: { met: boolean; label: string; detail?: string }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <GameIcon icon={met ? GiCheckMark : GiCancel} size={14} style={{ color: met ? T.positive : T.negative }} />
      <span style={{ color: T.text }}>{label}</span>
      {detail && <span style={{ color: T.textMuted }}>{detail}</span>}
    </div>
  );
}

const ratio = (v: PrereqValue) => `(${v.current}/${v.required})`;

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
  const prereqs = feat.prerequisites;

  const valuePrereqs: [PrereqValue | null, (v: PrereqValue) => string][] = [
    [prereqs.bab, v => `${t('classes.bab')} +${v.required}`],
    [prereqs.level, v => `${t('classes.level')} ${v.required}`],
    [prereqs.max_level, v => t('feats.maxLevel', { level: v.required }) as string],
    [prereqs.epic, () => t('feats.epicCharacter') as string],
    [prereqs.base_fortitude, v => `${t('feats.baseFortitude')} +${v.required}`],
    [prereqs.caster_level, v => `${t('feats.casterLevel')} ${v.required}`],
    [prereqs.spell_level, v => `${t('feats.spellLevel')} ${v.required}`],
  ];

  const hasAutoResolvable =
    prereqs.feats.some(p => !p.met) || prereqs.abilities.some(p => !p.met);

  const hasPrereqs = (
    prereqs.feats.length > 0 ||
    prereqs.any_feats.length > 0 ||
    prereqs.abilities.length > 0 ||
    valuePrereqs.some(([v]) => v !== null) ||
    prereqs.class_level !== null ||
    prereqs.skills.length > 0
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

      {hasPrereqs && (
        <DetailSection title={t('feats.prerequisites')}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {prereqs.feats.map(p => (
              <PrereqRow key={p.id} met={p.met} label={p.name} />
            ))}
            {prereqs.any_feats.length > 0 && (
              <PrereqRow
                met={prereqs.any_feats.some(p => p.met)}
                label={t('feats.oneOf', { feats: prereqs.any_feats.map(p => p.name).join(', ') }) as string}
              />
            )}
            {prereqs.abilities.map((p, i) => (
              <PrereqRow
                key={i}
                met={p.met}
                label={`${t(`abilityScores.short.${p.ability.slice(0, 3).toLowerCase()}`)} ${p.required}`}
                detail={ratio(p)}
              />
            ))}
            {prereqs.class_level && (
              <PrereqRow
                met={prereqs.class_level.met}
                label={`${prereqs.class_level.class_name} ${prereqs.class_level.required}`}
                detail={ratio(prereqs.class_level)}
              />
            )}
            {valuePrereqs.map(([v, label], i) => v && (
              <PrereqRow key={i} met={v.met} label={label(v)} detail={ratio(v)} />
            ))}
            {prereqs.skills.map((p, i) => (
              <PrereqRow key={i} met={p.met} label={`${p.skill} ${p.required} ${t('feats.ranks')}`} detail={ratio(p)} />
            ))}
            {!isOwned && onAdd && hasAutoResolvable && (
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
