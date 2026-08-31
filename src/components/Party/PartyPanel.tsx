import { useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
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

function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0]!.toUpperCase())
    .join('');
}

function CardAvatar({ name, icon, inParty }: { name: string; icon?: string | null; inParty: boolean }) {
  const iconUrl = useIcon(icon);
  const size = 40;
  if (iconUrl) {
    return <img src={iconUrl} alt="" style={{ width: size, height: size, borderRadius: 4, flexShrink: 0 }} />;
  }
  return (
    <div
      style={{
        width: size, height: size, borderRadius: 4, flexShrink: 0,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: inParty ? 'rgba(160,82,45,0.18)' : T.surfaceAlt,
        color: inParty ? T.accent : T.textMuted,
        fontWeight: 600,
      }}
    >
      {initials(name)}
    </div>
  );
}

function CompanionCard({
  entry, classLine, icon, onToggle,
}: {
  entry: RosterEntryInfo;
  classLine: string;
  icon: string | null;
  onToggle: () => void;
}) {
  const t = useTranslations();
  const inParty = entry.staged_in_party ?? entry.in_party;
  const staged = entry.staged_in_party != null;
  return (
    <button
      onClick={onToggle}
      style={{
        display: 'flex', alignItems: 'center', gap: 10,
        padding: '10px 12px', textAlign: 'left', cursor: 'pointer',
        background: inParty ? 'rgba(160,82,45,0.08)' : T.surface,
        border: inParty ? `2px solid ${T.accent}` : `2px solid ${T.borderLight}`,
        borderRadius: 6,
        transition: 'border-color 0.15s, background 0.15s',
      }}
    >
      <CardAvatar name={entry.char_name} icon={icon} inParty={inParty} />
      <div style={{ minWidth: 0, flex: 1 }}>
        <div className="t-md" style={{ fontWeight: 600, color: T.text, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {entry.char_name}
        </div>
        <div className="t-sm" style={{ color: T.textMuted, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {classLine}
        </div>
        <div className="t-sm" style={{ color: inParty ? T.accent : T.textMuted, fontStyle: staged ? 'italic' : 'normal' }}>
          {inParty ? t('roster.inParty') : t('roster.benched')}
          {staged && ` — ${t('roster.pendingSave')}`}
        </div>
      </div>
    </button>
  );
}

/// Save-level party composition: every roster companion as a card, in-party
/// members framed. Clicking toggles staged membership; the save button writes
/// it to roster.rst (+ .ros FactionID mirror).
export function PartyPanel() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { characterId, roster, refreshRoster } = useCharacterContext();
  const [classNames, setClassNames] = useState<Record<number, string | null>>({});
  const [classIcons, setClassIcons] = useState<Record<number, string | null>>({});
  const [partyLimit, setPartyLimit] = useState<number | null>(null);
  const [recognizedRosNames, setRecognizedRosNames] = useState<Set<string> | null>(null);
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
  useEffect(() => {
    if (!characterId) {
      setRecognizedRosNames(null);
      return;
    }
    let cancelled = false;
    gameStateAPI.getCompanionInfluence(characterId)
      .then(response => {
        if (cancelled) return;
        const names = new Set<string>();
        for (const [compId, status] of Object.entries(response.companions)) {
          if (status.source !== 'explicit') continue;
          names.add((EXPLICIT_ROS_NAME_ALIASES[compId] ?? compId).toLowerCase());
        }
        setRecognizedRosNames(names.size > 0 ? names : null);
      })
      .catch(() => setRecognizedRosNames(null));
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

  const { recognizedRoster, otherRoster } = useMemo(() => {
    if (!recognizedRosNames) return { recognizedRoster: roster, otherRoster: [] as RosterEntryInfo[] };
    const recognized: RosterEntryInfo[] = [];
    const other: RosterEntryInfo[] = [];
    for (const entry of roster) {
      (recognizedRosNames.has(entry.ros_name.toLowerCase()) ? recognized : other).push(entry);
    }
    return { recognizedRoster: recognized, otherRoster: other };
  }, [roster, recognizedRosNames]);

  const inPartyCount = recognizedRoster.filter(r => r.staged_in_party ?? r.in_party).length;
  const hasStaged = roster.some(r => r.staged_in_party != null);
  const countLabel = partyLimit != null
    ? t('roster.partyCount', { count: inPartyCount, limit: partyLimit })
    : t('roster.partyCountNoLimit', { count: inPartyCount });

  const renderCard = (entry: RosterEntryInfo) => (
    <CompanionCard
      key={entry.ros_name}
      entry={entry}
      icon={entry.classes[0] ? (classIcons[entry.classes[0].class_id] ?? null) : null}
      classLine={entry.classes
        .map(c => `${display(classNames[c.class_id])} ${c.level}`)
        .join(' / ')}
      onToggle={() => void toggle(entry)}
    />
  );

  return (
    <div style={{ padding: 16, height: '100%', overflowY: 'auto' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 4 }}>
        <h2 className="t-lg" style={{ margin: 0, color: T.text }}>{t('navigation.party')}</h2>
        <span className="t-sm" style={{ color: partyLimit != null && inPartyCount > partyLimit ? T.accent : T.textMuted }}>
          {countLabel}
        </span>
      </div>
      <div className="t-sm" style={{ color: T.textMuted, marginBottom: 12 }}>
        {t('roster.changePartyHint')}
        {hasStaged && <span style={{ fontStyle: 'italic' }}> {t('roster.pendingSaveHint')}</span>}
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(230px, 1fr))', gap: 10 }}>
        {recognizedRoster.map(renderCard)}
      </div>
      {otherRoster.length > 0 && (
        <>
          <div className="t-section" style={{ color: T.accent, marginTop: 20, marginBottom: 4 }}>
            {t('roster.otherRosterEntries')}
          </div>
          <div className="t-sm" style={{ color: T.textMuted, marginBottom: 12 }}>
            {t('roster.otherRosterEntriesHint')}
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(230px, 1fr))', gap: 10 }}>
            {otherRoster.map(renderCard)}
          </div>
        </>
      )}
    </div>
  );
}
