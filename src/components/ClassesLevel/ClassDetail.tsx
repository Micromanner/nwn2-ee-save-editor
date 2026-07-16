import { useState, useEffect } from 'react';
import { Spinner } from '@blueprintjs/core';
import { GiCancel, GiCheckMark } from 'react-icons/gi';
import { T } from '../theme';
import { GameIcon } from '../shared/GameIcon';
import { display } from '@/utils/dataHelpers';
import type { ClassInfo } from '@/hooks/useClassesLevel';
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

function formatDescription(raw: string, className: string): string {
  let html = raw;
  // Remove the first line that just repeats the class name (e.g. <color=Gold><b>Bard</b></color>)
  html = html.replace(/^<color=\w+><b>[^<]*<\/b><\/color>\s*/i, '');
  // Remove prestige class preamble
  html = html.replace(/\(PRESTIGE CLASS:.*?\)\s*/i, '');
  // Convert NWN2 color tags — use our accent color instead of Gold
  html = html.replace(/<color=Gold>/gi, `<span style="color:${T.accent}">`);
  html = html.replace(/<color=(\w+)>/gi, '<span style="color:$1">');
  html = html.replace(/<\/color>/gi, '</span>');
  // Convert newlines to <br>
  html = html.replace(/\n/g, '<br/>');
  return html;
}

// --- COMMENTED OUT: structured data sections ---
// interface SpellSlots { level_0: number; level_1: number; level_2: number; level_3: number; level_4: number; level_5: number; level_6: number; level_7: number; level_8: number; level_9: number; }
// interface LevelProgressionEntry { level: number; base_attack_bonus: number; fortitude_save: number; reflex_save: number; will_save: number; features: { name: string; feature_type: string; description: string }[]; spell_slots: SpellSlots | null; }
// interface ClassProgression { class_id: number; class_name: string; basic_info: { hit_die: number; skill_points_per_level: number; bab_progression: string; save_progression: string; is_spellcaster: boolean; spell_type: string; }; level_progression: LevelProgressionEntry[]; max_level_shown: number; }
//
// interface ParsedDescription { flavor: string | null; abilities: string | null; }
//
// function parseDescription(raw: string): ParsedDescription {
//   let text = raw.replace(/<[^>]*>/g, '').replace(/&nbsp;/g, ' ');
//   text = text.replace(/\(PRESTIGE CLASS:.*?\)\s*/i, '');
//   let flavor: string | null = text;
//   const flavorCut = [/\bRequirements:\s*/i, /\bClass Features:\s*/i, /\bSpecial:\s*/i];
//   for (const re of flavorCut) { const idx = flavor.search(re); if (idx > 0) flavor = flavor.substring(0, idx); }
//   flavor = flavor.replace(/\s+/g, ' ').trim();
//   if (flavor.length <= 10) flavor = null;
//   let abilities: string | null = null;
//   const abilitiesMatch = text.match(/\bClass Abilities:\s*([\s\S]*)/i);
//   if (abilitiesMatch) { abilities = abilitiesMatch[1].trim(); }
//   else { const levelMatch = text.match(/(Level \d+:[\s\S]*)/i); if (levelMatch) { abilities = levelMatch[1].trim(); } }
//   if (abilities) { abilities = abilities.replace(/\s*(Level \d+:)/g, '\n$1').replace(/\s*- ([A-Z])/g, '\n- $1').replace(/^\n/, '').replace(/[ \t]+/g, ' ').trim(); if (abilities.length <= 5) abilities = null; }
//   return { flavor, abilities };
// }
//
// function formatBabProgression(table: string): string {
//   const lower = table.toLowerCase();
//   if (lower.includes('atk_1') || lower.includes('atk1')) return 'Full (+1/level)';
//   if (lower.includes('atk_2') || lower.includes('atk2')) return '3/4 (+3/4 level)';
//   if (lower.includes('atk_3') || lower.includes('atk3')) return 'Half (+1/2 level)';
//   return table;
// }
//
// const statRow: React.CSSProperties = { display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '3px 0' };
// --- END COMMENTED OUT ---

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
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
        <Spinner size={24} />
      </div>
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
          dangerouslySetInnerHTML={{ __html: formatDescription(detail.description, detail.name) }}
        />
      )}

      {/* --- COMMENTED OUT: structured data sections ---
      {/* Class Stats *}
      <div>
        <div style={sectionTitle}>{t('classes.classStats')}</div>
        <div style={{ background: T.surfaceAlt, borderRadius: 4, padding: '6px 10px' }}>
          <div style={statRow}>
            <span style={{ color: T.textMuted }}>{t('classes.hitDie')}</span>
            <span className="t-medium" style={{ color: T.text }}>d{detail?.hit_die ?? cls.hit_die}</span>
          </div>
          <div style={statRow}>
            <span style={{ color: T.textMuted }}>{t('classes.skillPoints')}</span>
            <span className="t-medium" style={{ color: T.text }}>{detail?.skill_points ?? cls.skill_points}{t('classes.perLevel')}</span>
          </div>
          <div style={statRow}>
            <span style={{ color: T.textMuted }}>{t('classes.bab')}</span>
            <span className="t-medium" style={{ color: T.text }}>{formatBabProgression(detail?.bab_progression ?? cls.bab_progression)}</span>
          </div>
          {detail?.save_progression && (
            <div style={statRow}>
              <span style={{ color: T.textMuted }}>{t('classes.savingThrows')}</span>
              <span className="t-medium" style={{ color: T.text }}>
                {t('classes.fortitude')}: {detail.save_progression.fortitude}, {t('classes.reflex')}: {detail.save_progression.reflex}, {t('classes.will')}: {detail.save_progression.will}
              </span>
            </div>
          )}
          <div style={statRow}>
            <span style={{ color: T.textMuted }}>{t('classes.spellcasting')}</span>
            <span className="t-medium" style={{ color: T.text }}>
              {detail?.is_spellcaster
                ? (detail.spell_type === 'arcane' ? t('classes.arcane') : t('classes.divine'))
                : t('classes.none')}
            </span>
          </div>
          {(detail?.max_level ?? 0) > 0 && (
            <div style={statRow}>
              <span style={{ color: T.textMuted }}>{t('classes.level')}</span>
              <span className="t-medium" style={{ color: T.text }}>Max {detail!.max_level}</span>
            </div>
          )}
        </div>
      </div>

      {/* Class Skills *}
      {detail?.class_skills && detail.class_skills.length > 0 && (
        <div>
          <div style={sectionTitle}>{t('classes.classSkills')}</div>
          <div className="t-body" style={{ color: T.text }}>
            {detail.class_skills.join(', ')}
          </div>
        </div>
      )}

      {/* Level Progression Table *}
      {detail?.progression && detail.progression.level_progression.length > 0 && (
        <div>
          <div style={sectionTitle}>{t('classes.levelProgression')}</div>
          <HTMLTable compact bordered striped style={{ width: '100%' }}>
            <thead>
              <tr>
                <th>{t('classes.lvl')}</th>
                <th>{t('classes.bab')}</th>
                <th>{t('classes.fortitude')}</th>
                <th>{t('classes.reflex')}</th>
                <th>{t('classes.will')}</th>
                <th>{t('classes.feats')}</th>
              </tr>
            </thead>
            <tbody>
              {detail.progression.level_progression.map((entry) => (
                <tr key={entry.level}>
                  <td>{entry.level}</td>
                  <td>+{entry.base_attack_bonus}</td>
                  <td>+{entry.fortitude_save}</td>
                  <td>+{entry.reflex_save}</td>
                  <td>+{entry.will_save}</td>
                  <td>{entry.features.map(f => f.name).join(', ') || '-'}</td>
                </tr>
              ))}
            </tbody>
          </HTMLTable>
        </div>
      )}

      {/* Class Abilities from description *}
      {parsed?.abilities && (
        <div>
          <div style={sectionTitle}>{t('classes.classAbilities')}</div>
          <div className="t-body" style={{ color: T.text, whiteSpace: 'pre-line' }}>
            {parsed.abilities}
          </div>
        </div>
      )}
      --- END COMMENTED OUT --- */}
    </div>
  );
}
