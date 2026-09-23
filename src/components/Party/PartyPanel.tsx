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
import { useModelThumbnail } from '@/hooks/useModelThumbnail';
import type { TintColors } from '@/components/ModelViewer/materials';
import type { ModelData } from '@/components/ModelViewer/types';

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

// Shown while the rendered portrait is pending or failed. Only the class
// logo pulses while loading; the initials fallback stays static.
function FallbackPortrait({ name, icon, loading }: { name: string; icon: string | null; loading: boolean }) {
  const iconUrl = useIcon(icon);
  return (
    <div
      style={{
        width: '100%', height: '100%',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}
    >
      {iconUrl ? (
        <img src={iconUrl} alt="" className={loading ? 'portrait-loading' : undefined} style={{ width: '50%', opacity: 0.9 }} />
      ) : (
        <span className="t-3xl t-semibold" style={{ color: 'rgba(245,240,232,0.55)' }}>
          {initials(name)}
        </span>
      )}
    </div>
  );
}

// All portrait meshes arrive with baked override_tints; this identity tint is
// only the fallback for a mesh without one.
const PORTRAIT_TINTS: TintColors = {
  channel1: [1, 1, 1], channel2: [1, 1, 1], channel3: [1, 1, 1],
};

function CompanionPortrait({ cacheKey, rosName, name, icon }: {
  cacheKey: string;
  rosName: string;
  name: string;
  icon: string | null;
}) {
  const dataUrl = useModelThumbnail(
    cacheKey,
    () => invoke<ModelData>('load_companion_portrait_model', { rosName }),
    PORTRAIT_TINTS,
    'portrait',
  );
  if (dataUrl) {
    return (
      <img
        src={dataUrl}
        alt=""
        style={{ width: '100%', height: '100%', objectFit: 'cover', display: 'block' }}
      />
    );
  }
  return <FallbackPortrait name={name} icon={icon} loading={dataUrl === null} />;
}

function CompanionCard({
  entry, totalLevel, classLine, subraceName, influence, fallbackIcon, portraitKey, onToggle,
}: {
  entry: RosterEntryInfo;
  totalLevel: number;
  classLine: string;
  subraceName: string | null;
  influence: number | null;
  fallbackIcon: string | null;
  portraitKey: string;
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
            background: 'radial-gradient(120% 100% at 50% 30%, #4a443c, #2b2722)',
            boxShadow: 'inset 0 0 0 1px rgba(45,45,48,0.15)',
          }}
        >
          <CompanionPortrait
            cacheKey={portraitKey}
            rosName={entry.ros_name}
            name={entry.char_name}
            icon={fallbackIcon}
          />
          {totalLevel > 0 && (
            <div
              className="t-semibold"
              title={t('roster.levelLine', { level: totalLevel })}
              style={{
                position: 'absolute', top: 6, right: 6, minWidth: 24, padding: '2px 6px',
                borderRadius: 4, textAlign: 'center', color: '#fff', background: T.accent,
              }}
            >
              {totalLevel}
            </div>
          )}
          <div
            className="party-card-action"
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
        <div className="t-semibold" style={{ color: T.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {entry.char_name}
        </div>
        <div style={{ color: T.textMuted, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {classLine}
        </div>
        {subraceName && (
          <div style={{ color: T.textMuted }}>{subraceName}</div>
        )}
        {influence != null && (
          <div style={{ color: T.textMuted }}>
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
      <span>{t('roster.emptySlot')}</span>
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
    const totalLevel = entry.classes.reduce((sum, c) => sum + c.level, 0);
    return (
      <CompanionCard
        key={entry.ros_name}
        entry={entry}
        portraitKey={`companion-portrait:${characterId ?? 'none'}:${entry.ros_name.toLowerCase()}`}
        fallbackIcon={entry.classes[0] ? (classIcons[entry.classes[0].class_id] ?? null) : null}
        totalLevel={totalLevel}
        classLine={entry.classes
          .map(c => `${display(classNames[c.class_id])} ${c.level}`)
          .join(' / ')}
        subraceName={entry.subrace_name}
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
                className="t-semibold"
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
              <div style={{ color: T.textMuted }}>{t('roster.reservesEmpty')}</div>
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
