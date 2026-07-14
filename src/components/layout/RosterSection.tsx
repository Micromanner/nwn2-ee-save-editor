import { useCallback, useEffect, useState } from 'react';
import { Button, Dialog, DialogBody, DialogFooter } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { T } from '../theme';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useIcon } from '@/hooks/useIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useToast } from '@/contexts/ToastContext';
import { CharacterStateAPI } from '@/lib/api/character-state';

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
  const { showToast } = useToast();
  const {
    character, roster, activeSource, playerName: storedPlayerName, playerClassId,
    isPreloading, isMetadataLoading, switchToCompanion, switchToPlayer, refreshRoster,
  } = useCharacterContext();
  const [pendingTarget, setPendingTarget] = useState<SwitchTarget | null>(null);
  const [classNames, setClassNames] = useState<Record<number, string>>({});
  const [classIcons, setClassIcons] = useState<Record<number, string | null>>({});
  const [isSwitching, setIsSwitching] = useState(false);

  useEffect(() => {
    const ids = [...new Set(roster.flatMap(r => r.classes.map(c => c.class_id)))];
    const missing = ids.filter(id => !(id in classNames));
    if (missing.length === 0) return;
    Promise.allSettled(
      missing.map(async id => [id, await invoke<string>('get_class_name', { classId: id })] as const),
    ).then(results => {
      // Merge only the resolved lookups; rejected ones are cosmetic and keep
      // falling back to the level-only label instead of discarding the batch.
      const fulfilled: Array<readonly [number, string]> = [];
      for (const result of results) {
        if (result.status === 'fulfilled') fulfilled.push(result.value);
      }
      if (fulfilled.length > 0) {
        setClassNames(prev => ({ ...prev, ...Object.fromEntries(fulfilled) }));
      }
    });
  }, [roster, classNames]);

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

  const doSwitch = useCallback(async (target: SwitchTarget, force: boolean) => {
    try {
      if (target.kind === 'player') {
        await switchToPlayer(force);
      } else {
        await switchToCompanion(target.rosName, force);
      }
      if (activeTab === 'appearance' || activeTab === 'models' || activeTab === 'gamestate') {
        onTabChange('overview');
      }
    } catch (error) {
      handleError(error);
    }
  }, [switchToPlayer, switchToCompanion, activeTab, onTabChange, handleError]);

  const requestSwitch = useCallback(async (target: SwitchTarget) => {
    if (isSwitching) return;
    setIsSwitching(true);
    try {
      const dirty = await CharacterStateAPI.hasUnsavedChanges();
      if (dirty) {
        setPendingTarget(target);
      } else {
        await doSwitch(target, false);
      }
    } catch (error) {
      handleError(error);
    } finally {
      setIsSwitching(false);
    }
  }, [doSwitch, handleError, isSwitching]);

  const confirmSave = useCallback(async () => {
    const target = pendingTarget;
    setPendingTarget(null);
    if (!target) return;
    setIsSwitching(true);
    try {
      const result = await CharacterStateAPI.saveCharacter();
      if (result.warning) {
        console.warn(result.warning);
        showToast(t('roster.syncWarning'), 'warning');
      }
      await refreshRoster();
      await doSwitch(target, false);
    } catch (error) {
      handleError(error);
    } finally {
      setIsSwitching(false);
    }
  }, [pendingTarget, doSwitch, refreshRoster, handleError, showToast, t]);

  const confirmDiscard = useCallback(async () => {
    const target = pendingTarget;
    setPendingTarget(null);
    if (!target) return;
    setIsSwitching(true);
    try {
      await doSwitch(target, true);
    } finally {
      setIsSwitching(false);
    }
  }, [pendingTarget, doSwitch]);

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

  // Name of the character that is CURRENTLY active (i.e. the one with the
  // unsaved changes the dialog is warning about), not the switch target.
  const activeCharacterName = activeSource.kind === 'player'
    ? playerName
    : roster.find(r => r.ros_name === activeSource.rosName)?.char_name ?? playerName;

  return (
    <div style={{ marginTop: 'auto', paddingTop: 8, borderTop: '1px solid rgba(255,255,255,0.1)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <div
        className="t-sm"
        style={{ padding: '4px 16px', color: T.sidebarText, opacity: 0.6, textTransform: 'uppercase' }}
      >
        {t('roster.party')}
      </div>

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

        {roster.map(entry => {
          const active = activeSource.kind === 'companion' && activeSource.rosName === entry.ros_name;
          const subtitle = entry.classes.length > 0
            ? entry.classes
                .map(c => {
                  const className = classNames[c.class_id];
                  return className ? `${className} ${c.level}` : t('roster.levelLine', { level: c.level });
                })
                .join(' / ')
            : '';
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
                <div className="t-md" style={{ whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                  {entry.char_name}
                </div>
                {subtitle && (
                  <div className="t-sm" style={{ opacity: 0.6, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                    {subtitle}
                  </div>
                )}
              </div>
            </button>
          );
        })}
      </div>

      <Dialog
        isOpen={pendingTarget !== null}
        onClose={() => setPendingTarget(null)}
        title={t('roster.unsavedTitle')}
      >
        <DialogBody>
          <p>{t('roster.unsavedMessage', { name: activeCharacterName })}</p>
        </DialogBody>
        <DialogFooter
          actions={
            <>
              <Button text={t('roster.cancel')} disabled={isSwitching} onClick={() => setPendingTarget(null)} />
              <Button text={t('roster.discardChanges')} intent="danger" disabled={isSwitching} onClick={confirmDiscard} />
              <Button text={t('roster.saveAndSwitch')} intent="primary" disabled={isSwitching} onClick={confirmSave} />
            </>
          }
        />
      </Dialog>
    </div>
  );
}
