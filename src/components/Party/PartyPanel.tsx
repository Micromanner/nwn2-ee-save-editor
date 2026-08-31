import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Card, Elevation } from '@blueprintjs/core';
import { T } from '../theme';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useIcon } from '@/hooks/useIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { CharacterStateAPI, type RosterEntryInfo } from '@/lib/api/character-state';
import { gameStateAPI } from '@/services/gameStateApi';
import { display } from '@/utils/dataHelpers';

// Two campaign companions whose roster `RosName` doesn't match their
// companion-influence id (see get_companion_definitions() in
// src-tauri/src/parsers/xml/parser.rs, which is the source of truth).
const EXPLICIT_ROS_NAME_ALIASES: Record<string, string> = {
  kaelyn: 'dove',
  one_of_many: 'oneofmany',
};

const CARD_GRID: React.CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fill, minmax(140px, 170px))',
  gap: 10,
  justifyContent: 'center',
};

function PaneHeader({ title, right }: { title: string; right?: ReactNode }) {
  return (
    <div
      style={{
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        height: 36, padding: '0 16px', borderBottom: `1px solid ${T.borderLight}`, flexShrink: 0,
      }}
    >
      <span className="t-section">{title}</span>
      {right}
    </div>
  );
}

function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0]!.toUpperCase())
    .join('');
}

// Placeholder until portraits are rendered by our own pipeline.
function FallbackPortrait({ name, icon }: { name: string; icon: string | null }) {
  const iconUrl = useIcon(icon);
  return (
    <div
      style={{
        width: '100%', height: '100%',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: T.surfaceAlt,
      }}
    >
      {iconUrl ? (
        <img src={iconUrl} alt="" style={{ width: '50%', opacity: 0.9 }} />
      ) : (
        <span className="t-3xl t-semibold" style={{ color: T.textMuted }}>
          {initials(name)}
        </span>
      )}
    </div>
  );
}

function CompanionCard({
  entry, classLine, metaLine, influence, fallbackIcon, onToggle,
}: {
  entry: RosterEntryInfo;
  classLine: string;
  metaLine: string;
  influence: number | null;
  fallbackIcon: string | null;
  onToggle: () => void;
}) {
  const t = useTranslations();
  const inParty = entry.staged_in_party ?? entry.in_party;
  const actionLabel = inParty ? t('roster.removeFromParty') : t('roster.addToParty');
  return (
    <button
      className="party-card"
      onClick={onToggle}
      title={actionLabel}
      style={{
        background: T.surface,
        border: `1px solid ${T.borderLight}`,
      }}
    >
      <div
        style={{
          margin: '8px 8px 0', padding: 2, borderRadius: 4,
          background: T.sidebar,
        }}
      >
        <div
          style={{
            position: 'relative', aspectRatio: '4 / 5', overflow: 'hidden', borderRadius: 3,
            background: T.surfaceAlt, boxShadow: 'inset 0 0 0 1px rgba(45,45,48,0.15)',
          }}
        >
          <FallbackPortrait name={entry.char_name} icon={fallbackIcon} />
          <div
            className="party-card-action t-base"
            style={{
              position: 'absolute', left: 0, right: 0, bottom: 0,
              padding: '12px 6px 5px', textAlign: 'center', color: '#f5f0e8',
              background: 'linear-gradient(transparent, rgba(30,30,34,0.85))',
            }}
          >
            {actionLabel}
          </div>
        </div>
      </div>
      <div style={{ padding: '6px 10px 8px', display: 'flex', flexDirection: 'column', gap: 1, minWidth: 0 }}>
        <div className="t-md t-semibold" style={{ color: T.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {entry.char_name}
        </div>
        <div className="t-base" style={{ color: T.textMuted, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {classLine}
        </div>
        {metaLine && (
          <div className="t-base" style={{ color: T.textMuted }}>{metaLine}</div>
        )}
        {influence != null && (
          <div className="t-base" style={{ color: T.textMuted }}>
            {t('roster.influenceLine', { value: influence })}
          </div>
        )}
      </div>
    </button>
  );
}

function EmptySlot() {
  const t = useTranslations();
  return (
    <div
      style={{
        border: `2px dashed ${T.border}`, borderRadius: 6, minHeight: 200,
        display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 4,
        color: T.textMuted, background: 'rgba(45,45,48,0.02)',
      }}
    >
      <span className="t-3xl" style={{ opacity: 0.5 }}>+</span>
      <span className="t-md">{t('roster.emptySlot')}</span>
    </div>
  );
}

/// Save-level party composition: the active party as a framed slot row
/// (empty slots up to the party limit), the remaining campaign cast as
/// reserves, other roster entries separate. Clicking a card toggles staged
/// membership; the save button writes it to roster.rst (+ .ros FactionID
/// mirror).
export function PartyPanel() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { characterId, roster, refreshRoster } = useCharacterContext();
  const [classNames, setClassNames] = useState<Record<number, string | null>>({});
  const [classIcons, setClassIcons] = useState<Record<number, string | null>>({});
  const [partyLimit, setPartyLimit] = useState<number | null>(null);
  const [recognizedRosNames, setRecognizedRosNames] = useState<Set<string> | null>(null);
  const [influenceByRos, setInfluenceByRos] = useState<Record<string, number>>({});
  const attemptedClassIds = useRef<Set<number>>(new Set());

  useEffect(() => {
    CharacterStateAPI.getPartyLimit()
      .then(setPartyLimit)
      .catch(() => setPartyLimit(null));
  }, []);

  useEffect(() => {
    const ids = [...new Set(roster.flatMap(r => r.classes.map(c => c.class_id)))];
    const missing = ids.filter(id => !attemptedClassIds.current.has(id));
    if (missing.length === 0) return;
    missing.forEach(id => attemptedClassIds.current.add(id));

    Promise.allSettled(
      missing.map(async id => {
        const [name, icon] = await Promise.all([
          invoke<string>('get_class_name', { classId: id }).catch(() => null),
          invoke<string | null>('get_class_icon', { classId: id }).catch(() => null),
        ]);
        return [id, name, icon] as const;
      }),
    ).then(results => {
      const names: Record<number, string | null> = {};
      const icons: Record<number, string | null> = {};
      for (const r of results) {
        if (r.status !== 'fulfilled') continue;
        const [id, name, icon] = r.value;
        names[id] = name;
        icons[id] = icon;
      }
      setClassNames(prev => ({ ...prev, ...names }));
      setClassIcons(prev => ({ ...prev, ...icons }));
    });
  }, [roster]);

  // Recognized campaign companion cast - same source the Influence page uses:
  // get_companion_influence marks the detected campaign's official companions
  // as "explicit". Anything else (e.g. a story NPC that also has a .ros file)
  // is left out. No campaign detected -> treat every roster entry as recognized.
  // The same response carries current influence values for the cards.
  useEffect(() => {
    if (!characterId) {
      setRecognizedRosNames(null);
      setInfluenceByRos({});
      return;
    }
    let cancelled = false;
    gameStateAPI.getCompanionInfluence(characterId)
      .then(response => {
        if (cancelled) return;
        const names = new Set<string>();
        const influences: Record<string, number> = {};
        for (const [compId, status] of Object.entries(response.companions)) {
          const rosName = (EXPLICIT_ROS_NAME_ALIASES[compId] ?? compId).toLowerCase();
          if (status.source === 'explicit') names.add(rosName);
          if (status.influence != null) influences[rosName] = status.influence;
        }
        setRecognizedRosNames(names.size > 0 ? names : null);
        setInfluenceByRos(influences);
      })
      .catch(() => {
        setRecognizedRosNames(null);
        setInfluenceByRos({});
      });
    return () => { cancelled = true; };
  }, [characterId]);

  const toggle = async (entry: RosterEntryInfo) => {
    try {
      const current = entry.staged_in_party ?? entry.in_party;
      await CharacterStateAPI.stagePartyMembership([
        { ros_name: entry.ros_name, in_party: !current },
      ]);
      await refreshRoster();
    } catch (error) {
      handleError(error);
    }
  };

  const { activeParty, reserves, otherRoster } = useMemo(() => {
    const recognized: RosterEntryInfo[] = [];
    const other: RosterEntryInfo[] = [];
    for (const entry of roster) {
      (!recognizedRosNames || recognizedRosNames.has(entry.ros_name.toLowerCase()) ? recognized : other).push(entry);
    }
    const inParty = (r: RosterEntryInfo) => r.staged_in_party ?? r.in_party;
    return {
      activeParty: recognized.filter(inParty),
      reserves: recognized.filter(r => !inParty(r)),
      otherRoster: other,
    };
  }, [roster, recognizedRosNames]);

  const countLabel = partyLimit != null
    ? t('roster.partyCount', { count: activeParty.length, limit: partyLimit })
    : t('roster.partyCountNoLimit', { count: activeParty.length });
  const emptySlots = partyLimit != null ? Math.max(0, partyLimit - activeParty.length) : 0;

  const renderCard = (entry: RosterEntryInfo) => {
    const genderLabel = entry.gender === 0 ? t('roster.male') : entry.gender === 1 ? t('roster.female') : null;
    const totalLevel = entry.classes.reduce((sum, c) => sum + c.level, 0);
    return (
      <CompanionCard
        key={entry.ros_name}
        entry={entry}
        fallbackIcon={entry.classes[0] ? (classIcons[entry.classes[0].class_id] ?? null) : null}
        classLine={entry.classes
          .map(c => `${display(classNames[c.class_id])} ${c.level}`)
          .join(' / ')}
        metaLine={[
          totalLevel > 0 ? t('roster.levelLine', { level: totalLevel }) : null,
          entry.subrace_name,
          genderLabel,
        ].filter(Boolean).join(' · ')}
        influence={influenceByRos[entry.ros_name.toLowerCase()] ?? null}
        onToggle={() => void toggle(entry)}
      />
    );
  };

  return (
    <div style={{ padding: 16, height: '100%' }}>
      <Card
        elevation={Elevation.ZERO}
        style={{
          padding: 0, background: T.surface, overflow: 'hidden',
          display: 'flex', height: '100%', border: `1px solid ${T.border}`, boxShadow: 'none',
        }}
      >
        <div style={{ width: '50%', borderRight: `1px solid ${T.borderLight}`, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <PaneHeader
            title={t('roster.activeParty')}
            right={(
              <span
                className="t-base t-semibold"
                style={{ color: partyLimit != null && activeParty.length > partyLimit ? T.accent : T.textMuted }}
              >
                {countLabel}
              </span>
            )}
          />
          <div style={{ flex: 1, overflow: 'auto', padding: 12 }}>
            <div style={CARD_GRID}>
              {activeParty.map(renderCard)}
              {Array.from({ length: emptySlots }, (_, i) => <EmptySlot key={`empty-${i}`} />)}
            </div>
          </div>
        </div>

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <PaneHeader title={t('roster.reserves')} />
          <div style={{ flex: 1, overflow: 'auto', padding: 12 }}>
            {reserves.length > 0 ? (
              <div style={CARD_GRID}>{reserves.map(renderCard)}</div>
            ) : (
              <div className="t-md" style={{ color: T.textMuted }}>{t('roster.reservesEmpty')}</div>
            )}
            {otherRoster.length > 0 && (
              <>
                <div className="t-section" style={{ margin: '32px 0 12px' }}>
                  {t('roster.otherRosterEntries')}
                </div>
                <div style={CARD_GRID}>{otherRoster.map(renderCard)}</div>
              </>
            )}
          </div>
        </div>
      </Card>
    </div>
  );
}
