import { useState, useEffect } from 'react';
import { GiCancel, GiCheckMark } from 'react-icons/gi';
import { T } from '../theme';
import { GameIcon } from '../shared/GameIcon';
import { LoadingState } from '../shared';
import { display } from '@/utils/dataHelpers';
import { nwn2ToHtml } from '@/utils/nwn2Markup';
import type { ClassInfo } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import { useIcon } from '@/hooks/useIcon';
import { invoke } from '@tauri-apps/api/core';

interface PrerequisiteCheck {
  label: string;
  met: boolean;
  current_value: string | null;
}

interface ClassDetailResponse {
  id: number;
  name: string;
  description: string | null;
  hit_die: number;
  skill_points: number;
  bab_progression: string;
  is_spellcaster: boolean;
  spell_type: string | null;
  max_level: number;
  is_prestige: boolean;
  alignment_restriction: string | null;
  prerequisites: {
    base_attack_bonus: number | null;
    skills: [string, number][];
    feats: string[];
    alignment: string | null;
  };
  prerequisite_status: PrerequisiteCheck[];
  class_skills: string[];
  save_progression: { fortitude: string; reflex: string; will: string };
  progression: unknown;
}

interface ClassDetailProps {
  cls: ClassInfo | null;
  canSelect: boolean;
  selectReason?: string;
}

function formatDescription(raw: string): string {
  let text = raw;
  // Remove the first line that just repeats the class name (e.g. <color=Gold><b>Bard</b></color>)
  text = text.replace(/^<color=\w+><b>[^<]*<\/b><\/color>\s*/i, '');
  // Remove prestige class preamble
  text = text.replace(/\(PRESTIGE CLASS:.*?\)\s*/i, '');
  // Use our accent color instead of Gold, then escape/tokenize via nwn2ToHtml
  text = text.replace(/<color=Gold>/gi, `<color=${T.accent}>`);
  return nwn2ToHtml(text);
}

const sectionTitle: React.CSSProperties = {
  color: T.textMuted,
  marginBottom: 6,
};

export function ClassDetail({ cls, canSelect, selectReason }: ClassDetailProps) {
  const t = useTranslations();
  const [detail, setDetail] = useState<ClassDetailResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const iconUrl = useIcon(cls?.icon);

  useEffect(() => {
    if (!cls) {
      setDetail(null);
      return;
    }

    const ctrl = { cancelled: false };
    setLoading(true);

    invoke<ClassDetailResponse>('get_class_detail', { classId: cls.id })
      .then((res) => {
        if (!ctrl.cancelled) setDetail(res);
      })
      .catch((err) => {
        console.error('get_class_detail failed:', err);
        if (!ctrl.cancelled) setDetail(null);
      })
      .finally(() => {
        if (!ctrl.cancelled) setLoading(false);
      });

    return () => { ctrl.cancelled = true; };
  }, [cls?.id]);

  if (!cls) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', color: T.textMuted }}>
        {t('classes.selectClassToView')}
      </div>
    );
  }

  if (loading) {
    return (
      <LoadingState />
    );
  }

  const hasPrereqs = detail && detail.prerequisite_status.length > 0;

  return (
    <div style={{ padding: '12px 16px', display: 'flex', flexDirection: 'column', gap: 10, overflow: 'auto', height: '100%' }}>
      {/* Header */}
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
        <span className="t-bold" style={{ color: T.text }}>{display(cls.name)}</span>
        {!canSelect && selectReason && (
          <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: T.negative, marginTop: 4 }}>
            {selectReason}
          </div>
        )}
      </div>

      {/* Prerequisites (real backend validation) */}
      {hasPrereqs && (
        <div style={{ borderBottom: `1px solid ${T.borderLight}`, paddingBottom: 10 }}>
          <div className="t-semibold" style={sectionTitle}>
            {t('classes.prerequisites')}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            {detail?.prerequisite_status.map((check, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '2px 0' }}>
                <GameIcon
                  icon={check.met ? GiCheckMark : GiCancel}
                  size={14}
                  style={{ color: check.met ? T.positive : T.negative, flexShrink: 0 }}
                />
                <span style={{ color: check.met ? T.text : T.negative, flex: 1 }}>
                  {check.label}
                </span>
                {check.current_value && (
                  <span style={{ color: T.textMuted, flexShrink: 0 }}>
                    ({check.current_value})
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Full description (rendered from TLK with HTML formatting) */}
      {detail?.description && (
        <div
          style={{ color: T.text }}
          dangerouslySetInnerHTML={{ __html: formatDescription(detail.description) }}
        />
      )}
    </div>
  );
}
