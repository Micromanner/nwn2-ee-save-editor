import { useState, useMemo, useEffect } from 'react';
import { Button, InputGroup, Icon } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { T } from '../theme';
import { ParchmentDialog, LoadingState } from '../shared';
import { useTranslations } from '@/hooks/useTranslations';
import { stripNwn2Tags } from '@/utils/nwn2Markup';
import { useIcon } from '@/hooks/useIcon';

interface AbilityModifiers {
  Str: number;
  Dex: number;
  Con: number;
  Int: number;
  Wis: number;
  Cha: number;
}

interface RaceInfo {
  id: number;
  name: string;
  icon: string | null;
  description: string | null;
  ability_adjustments: AbilityModifiers;
  favored_class: string | null;
  default_subrace: number | null;
}

interface SubraceInfo {
  id: number;
  name: string;
  label: string;
  icon: string | null;
  description: string | null;
  ability_adjustments: AbilityModifiers;
  favored_class: string | null;
  size: string | null;
}

export interface RaceDialogProps {
  isOpen: boolean;
  currentRaceId: number;
  currentSubrace: string | null;
  onClose: () => void;
  onSelect: (raceId: number, subrace: string | null) => void;
}

interface ParsedDescription {
  lore: string;
  traits: { name: string; text: string }[];
}

function parseRaceDescription(raw: string): ParsedDescription {
  const titlePattern = /^\s*<color[^>]*>\s*<b>[^<]*<\/b>\s*<\/color[^>]*>\s*/i;
  raw = raw.replace(titlePattern, '');

  const traitsMarker = /Racial Traits:?\s*<\/b>\s*<\/color[^>]*>/i;
  const markerMatch = raw.match(traitsMarker);

  if (!markerMatch) {
    return { lore: stripNwn2Tags(raw).trim(), traits: [] };
  }

  const splitIdx = raw.indexOf(markerMatch[0]);
  const lorePart = raw.slice(0, splitIdx);
  const traitsPart = raw.slice(splitIdx + markerMatch[0].length);

  const lore = stripNwn2Tags(lorePart).trim();

  const traitRegex = /-\s*<b>([^<]+):<\/b>\s*/gi;
  const traits: { name: string; text: string }[] = [];
  let match;
  const matches: { name: string; start: number }[] = [];

  while ((match = traitRegex.exec(traitsPart)) !== null) {
    matches.push({ name: match[1].trim(), start: match.index + match[0].length });
  }

  for (let i = 0; i < matches.length; i++) {
    const end = i + 1 < matches.length ? matches[i + 1].start - (traitsPart.slice(0, matches[i + 1].start).match(/-\s*<b>[^<]+:<\/b>\s*$/)?.[0]?.length ?? 0) : traitsPart.length;
    const rawText = traitsPart.slice(matches[i].start, end);
    traits.push({ name: matches[i].name, text: stripNwn2Tags(rawText).trim() });
  }

  return { lore, traits };
}

function formatAbilityMods(mods: AbilityModifiers, t: (key: string) => string): string {
  const parts: string[] = [];
  if (mods.Str) parts.push(`${t('abilityScores.short.str')} ${mods.Str > 0 ? '+' : ''}${mods.Str}`);
  if (mods.Dex) parts.push(`${t('abilityScores.short.dex')} ${mods.Dex > 0 ? '+' : ''}${mods.Dex}`);
  if (mods.Con) parts.push(`${t('abilityScores.short.con')} ${mods.Con > 0 ? '+' : ''}${mods.Con}`);
  if (mods.Int) parts.push(`${t('abilityScores.short.int')} ${mods.Int > 0 ? '+' : ''}${mods.Int}`);
  if (mods.Wis) parts.push(`${t('abilityScores.short.wis')} ${mods.Wis > 0 ? '+' : ''}${mods.Wis}`);
  if (mods.Cha) parts.push(`${t('abilityScores.short.cha')} ${mods.Cha > 0 ? '+' : ''}${mods.Cha}`);
  return parts.join(', ');
}

export function RaceDialog({ isOpen, currentRaceId, currentSubrace, onClose, onSelect }: RaceDialogProps) {
  const t = useTranslations();
  const [search, setSearch] = useState('');
  const [races, setRaces] = useState<RaceInfo[]>([]);
  const [loadingRaces, setLoadingRaces] = useState(false);
  const [expandedRaceId, setExpandedRaceId] = useState<number | null>(null);
  const [subraces, setSubraces] = useState<SubraceInfo[]>([]);
  const [loadingSubraces, setLoadingSubraces] = useState(false);
  const [selectedRaceId, setSelectedRaceId] = useState<number | null>(null);
  const [selectedSubrace, setSelectedSubrace] = useState<string | null>(null);

  useEffect(() => {
    if (!isOpen) return;
    setLoadingRaces(true);
    invoke<RaceInfo[]>('get_available_races')
      .then(res => setRaces(res))
      .catch(() => setRaces([]))
      .finally(() => setLoadingRaces(false));
  }, [isOpen]);

  const handleOpen = () => {
    setSearch('');
    setExpandedRaceId(currentRaceId ?? null);
    setSelectedRaceId(currentRaceId ?? null);
    setSelectedSubrace(currentSubrace ?? null);
    setSubraces([]);
  };

  useEffect(() => {
    if (!isOpen || expandedRaceId === null) return;
    setLoadingSubraces(true);
    invoke<SubraceInfo[]>('get_subraces_for_race', { raceId: expandedRaceId })
      .then(res => {
        setSubraces(res);
        if (res.length === 1) {
          setSelectedSubrace(res[0].name);
        }
      })
      .catch(() => setSubraces([]))
      .finally(() => setLoadingSubraces(false));
  }, [isOpen, expandedRaceId]);

  const hasMultipleSubraces = expandedRaceId !== null && subraces.length > 1;
  const singleSubrace = expandedRaceId !== null && subraces.length === 1 ? subraces[0] : null;

  const handleRaceClick = (raceId: number) => {
    if (expandedRaceId === raceId) {
      setExpandedRaceId(null);
      return;
    }
    setExpandedRaceId(raceId);
    setSelectedRaceId(raceId);
    setSelectedSubrace(null);
  };

  const handleSubraceClick = (sub: SubraceInfo) => {
    setSelectedRaceId(expandedRaceId);
    setSelectedSubrace(sub.name);
  };

  const filtered = useMemo(() => {
    if (!search) return races;
    const q = search.toLowerCase();
    return races.filter(r => r.name.toLowerCase().includes(q));
  }, [search, races]);

  const detail = races.find(r => r.id === selectedRaceId) ?? null;
  const selectedSub = useMemo(() => {
    if (selectedSubrace === null) return null;
    return subraces.find(s => s.name === selectedSubrace) ?? null;
  }, [selectedSubrace, subraces]);

  const displayDescription = selectedSub?.description ?? detail?.description ?? null;
  const displayName = selectedSub ? selectedSub.name : detail?.name ?? null;

  const handleConfirm = () => {
    if (selectedRaceId !== null) {
      const subLabel = selectedSub?.label ?? null;
      onSelect(selectedRaceId, subLabel);
    }
    onClose();
  };

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      onOpened={handleOpen}
      title={t('race.selectTitle')}
      width={780}
      height={720}
      footerActions={
        <Button
          text={t('race.confirm')}
          intent="primary"
          onClick={handleConfirm}
          style={{ background: T.accent }}
          disabled={selectedRaceId === null}
        />
      }
    >
      <div style={{ display: 'flex', gap: 0, margin: -16, flex: 1, minHeight: 0, overflow: 'hidden' }}>

        <div style={{ width: 260, borderRight: `1px solid ${T.borderLight}`, display: 'flex', flexDirection: 'column', background: T.surfaceAlt }}>
          <div style={{ padding: 8 }}>
            <InputGroup
              leftIcon="search"
              placeholder={t('race.searchPlaceholder')}
              small
              value={search}
              onChange={e => setSearch(e.target.value)}
              rightElement={search ? <Button icon="cross" small minimal onClick={() => setSearch('')} /> : undefined}
            />
          </div>
          <div style={{ flex: 1, overflowY: 'auto' }}>
            {loadingRaces ? (
              <LoadingState height={200} text={t('common.loadingRaces')} />
            ) : (
              <>
                {filtered.map(r => {
                  const isExpanded = expandedRaceId === r.id;
                  const isActive = selectedRaceId === r.id && (selectedSubrace === null || (isExpanded && singleSubrace !== null));
                  const showChildren = isExpanded && !loadingSubraces && hasMultipleSubraces;
                  return (
                    <div key={r.id}>
                      <button
                        onClick={() => handleRaceClick(r.id)}
                        className={isExpanded ? 't-semibold' : undefined}
                        style={{
                          display: 'flex', alignItems: 'center', gap: 6,
                          width: '100%', textAlign: 'left',
                          padding: '6px 12px', border: 'none', cursor: 'pointer',
                          background: isActive ? `${T.accent}15` : 'transparent',
                          borderLeft: isActive ? `2px solid ${T.accent}` : '2px solid transparent',
                          color: isActive ? T.accent : T.text,
                        }}
                      >
                        <Icon
                          icon={isExpanded ? 'caret-down' : 'caret-right'}
                          size={16}
                          style={{ color: T.accent }}
                        />
                        {r.name}
                      </button>
                      {isExpanded && loadingSubraces && (
                        <div style={{ padding: '0 24px' }}>
                          <LoadingState variant="inline" text={t('common.loadingSubraces')} />
                        </div>
                      )}
                      {showChildren && subraces.map(sub => {
                        const isSubSelected = selectedSubrace === sub.name;
                        const modsStr = formatAbilityMods(sub.ability_adjustments, t);
                        return (
                          <button
                            key={sub.id}
                            onClick={() => handleSubraceClick(sub)}
                            className={isSubSelected ? 't-semibold' : undefined}
                            style={{
                              display: 'block', width: '100%', textAlign: 'left',
                              padding: '4px 12px 4px 30px', border: 'none', cursor: 'pointer',
                              background: isSubSelected ? `${T.accent}15` : 'transparent',
                              borderLeft: isSubSelected ? `2px solid ${T.accent}` : '2px solid transparent',
                              color: isSubSelected ? T.accent : T.textMuted,
                            }}
                          >
                            <div>{sub.name}</div>
                            {modsStr && (
                              <div style={{ opacity: 0.7 }}>{modsStr}</div>
                            )}
                          </button>
                        );
                      })}
                    </div>
                  );
                })}
                {filtered.length === 0 && !loadingRaces && (
                  <div className="t-center" style={{ padding: 16, color: T.textMuted }}>
                    {t('race.noMatch')}
                  </div>
                )}
              </>
            )}
          </div>
        </div>

        <div style={{ flex: 1, padding: 16, overflowY: 'auto' }}>
          {displayName ? (
            <div>
              <RaceDetailHeader name={displayName} icon={selectedSub?.icon ?? detail?.icon ?? null} />
              <RaceDescription text={displayDescription} />
            </div>
          ) : (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%' }}>
              <div className="t-center" style={{ color: T.textMuted }}>
                {t('race.selectRace')}
              </div>
            </div>
          )}
        </div>

      </div>
    </ParchmentDialog>
  );
}

function RaceDetailHeader({ name, icon }: { name: string; icon: string | null }) {
  const iconUrl = useIcon(icon);
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
      {iconUrl && <img src={iconUrl} alt="" width={32} height={32} style={{ borderRadius: 4, flexShrink: 0 }} />}
      <div className="t-bold" style={{ color: T.text }}>{name}</div>
    </div>
  );
}

function RaceDescription({ text }: { text: string | null }) {
  if (!text) return null;
  const { lore, traits } = parseRaceDescription(text);
  return (
    <div>
      {lore && (
        <div className="t-body" style={{ color: T.textMuted, marginBottom: traits.length > 0 ? 12 : 0 }}>
          {lore}
        </div>
      )}
      {traits.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          {traits.map((trait, i) => (
            <div key={i} style={{ display: 'flex', gap: 8 }}>
              <span style={{ color: T.accent, flexShrink: 0 }}>-</span>
              <div>
                <span className="t-bold" style={{ color: T.text }}>{trait.name}: </span>
                <span style={{ color: T.textMuted }}>{trait.text}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
