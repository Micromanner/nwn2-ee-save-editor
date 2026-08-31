import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GiThreeFriends } from 'react-icons/gi';
import { T } from '../theme';
import { GameIcon } from '../shared/GameIcon';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useIcon } from '@/hooks/useIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';

const PARTY_GAME_ICON = 'b_partyselect';

type SwitchTarget = { kind: 'player' } | { kind: 'companion'; rosName: string };

function initials(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map(part => part[0]!.toUpperCase())
    .join('');
}

function Avatar({ name, icon, active }: { name: string; icon?: string | null; active: boolean }) {
  const iconUrl = useIcon(icon);
  if (iconUrl) {
    return (
      <img
        src={iconUrl}
        alt=""
        style={{
          width: 'var(--icon-nav)', height: 'var(--icon-nav)',
          borderRadius: 4, flexShrink: 0, opacity: active ? 1 : 0.85,
        }}
      />
    );
  }
  return (
    <div
      style={{
        width: 'var(--icon-nav)', height: 'var(--icon-nav)', borderRadius: 4, flexShrink: 0,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: active ? 'rgba(160,82,45,0.35)' : 'rgba(255,255,255,0.08)',
        color: active ? T.sidebarAccent : T.sidebarText,
        fontSize: 'var(--font-sm)', fontWeight: 600,
      }}
    >
      {initials(name)}
    </div>
  );
}

interface RosterSectionProps {
  activeTab: string;
  onTabChange: (id: string) => void;
}

export function RosterSection({ activeTab, onTabChange }: RosterSectionProps) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const {
    character, roster, activeSource, playerName: storedPlayerName, playerClassId,
    isPreloading, isMetadataLoading, switchToCompanion, switchToPlayer, sessionKind,
  } = useCharacterContext();
  const partyIconUrl = useIcon(PARTY_GAME_ICON);
  const [classIcons, setClassIcons] = useState<Record<number, string | null>>({});
  const [isSwitching, setIsSwitching] = useState(false);

  useEffect(() => {
    const ids = [...new Set([
      ...roster.flatMap(r => r.classes.map(c => c.class_id)),
      ...(playerClassId != null ? [playerClassId] : []),
    ])];
    const missing = ids.filter(id => !(id in classIcons));
    if (missing.length === 0) return;
    Promise.allSettled(
      missing.map(async id => [id, await invoke<string | null>('get_class_icon', { classId: id })] as const),
    ).then(results => {
      // Failed lookups are stored as null so they are not refetched forever;
      // null falls back to the initials avatar.
      const entries = results.map((result, i) =>
        result.status === 'fulfilled' ? result.value : ([missing[i]!, null] as const),
      );
      setClassIcons(prev => ({ ...prev, ...Object.fromEntries(entries) }));
    });
  }, [roster, playerClassId, classIcons]);

  const requestSwitch = useCallback(async (target: SwitchTarget) => {
    if (isSwitching) return;
    setIsSwitching(true);
    try {
      if (target.kind === 'player') {
        await switchToPlayer();
      } else {
        await switchToCompanion(target.rosName);
      }
      if (activeTab === 'gamestate') {
        onTabChange('overview');
      }
    } catch (error) {
      handleError(error);
    } finally {
      setIsSwitching(false);
    }
  }, [switchToPlayer, switchToCompanion, activeTab, onTabChange, handleError, isSwitching]);

  if (!character) return null;

  const rowStyle = (active: boolean, disabled: boolean): React.CSSProperties => ({
    display: 'flex', alignItems: 'center', gap: 8,
    width: '100%', padding: '6px 16px',
    border: 'none', cursor: active || disabled ? 'default' : 'pointer',
    opacity: disabled && !active ? 0.6 : 1,
    background: active ? 'rgba(160,82,45,0.12)' : 'transparent',
    color: active ? T.sidebarAccent : T.sidebarText,
    textAlign: 'left',
    borderLeft: active ? `2px solid ${T.sidebarAccent}` : '2px solid transparent',
    transition: 'all 0.15s',
  });

  // Lock switching while a switch is in flight OR while background preloads
  // are still running — a switch mid-preload lets stale data through.
  const switchLocked = isSwitching || isPreloading || isMetadataLoading;

  const playerActive = activeSource.kind === 'player';
  // The player's own name, independent of which character is currently
  // loaded — `character.name` would show the active companion's name here.
  const playerName = storedPlayerName || t('roster.player');


  return (
    <div style={{ marginTop: 'auto', paddingTop: 8, borderTop: '1px solid rgba(255,255,255,0.1)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      {sessionKind !== 'vault' && (
        <button
          onClick={() => onTabChange('party')}
          className="t-md"
          style={{
            display: 'flex', alignItems: 'center', gap: 8,
            width: '100%', padding: '8px 16px',
            border: 'none', cursor: 'pointer',
            background: activeTab === 'party' ? 'rgba(160,82,45,0.12)' : 'transparent',
            color: activeTab === 'party' ? T.sidebarAccent : T.sidebarText,
            textAlign: 'left',
            borderLeft: activeTab === 'party' ? `2px solid ${T.sidebarAccent}` : '2px solid transparent',
            transition: 'all 0.15s',
          }}
        >
          {partyIconUrl ? (
            <img src={partyIconUrl} alt="" style={{ width: 'var(--icon-nav)', height: 'var(--icon-nav)', borderRadius: 2, flexShrink: 0 }} />
          ) : (
            <GameIcon icon={GiThreeFriends} style={{ width: 'var(--icon-nav)', height: 'var(--icon-nav)', flexShrink: 0 }} />
          )}
          {t('navigation.party')}
        </button>
      )}

      <div style={{ height: 1, background: 'rgba(255,255,255,0.1)', margin: '8px 0' }} />

      <div className="sidebar-scroll" style={{ overflowY: 'auto', minHeight: 0 }}>
        <button
          style={rowStyle(playerActive, switchLocked)}
          disabled={switchLocked}
          onClick={() => !playerActive && !switchLocked && requestSwitch({ kind: 'player' })}
        >
          <Avatar
            name={playerName}
            icon={playerClassId != null ? classIcons[playerClassId] : null}
            active={playerActive}
          />
          <div style={{ minWidth: 0 }}>
            <div className="t-md" style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
              {playerName}
            </div>
          </div>
        </button>

        {[...roster]
          .sort((a, b) => Number(b.staged_in_party ?? b.in_party) - Number(a.staged_in_party ?? a.in_party))
          .map(entry => {
          const inParty = entry.staged_in_party ?? entry.in_party;
          const active = activeSource.kind === 'companion' && activeSource.rosName === entry.ros_name;
          return (
            <button
              key={entry.ros_name}
              style={rowStyle(active, switchLocked)}
              disabled={switchLocked}
              onClick={() => !active && !switchLocked && requestSwitch({ kind: 'companion', rosName: entry.ros_name })}
            >
              <Avatar
                name={entry.char_name}
                icon={entry.classes[0] ? classIcons[entry.classes[0].class_id] : null}
                active={active}
              />
              <div style={{ minWidth: 0 }}>
                <div
                  className="t-md"
                  title={inParty ? t('roster.inParty') : undefined}
                  style={{
                    whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
                    ...(inParty ? {
                      textDecoration: 'underline',
                      textDecorationColor: T.sidebarAccent,
                      textUnderlineOffset: 3,
                      // IBM Plex Serif overflows Blueprint's line box; without
                      // this the offset underline is clipped by overflow:hidden.
                      paddingBottom: 4,
                    } : {}),
                  }}
                >
                  {entry.char_name}
                </div>
              </div>
            </button>
          );
        })}
      </div>

    </div>
  );
}
