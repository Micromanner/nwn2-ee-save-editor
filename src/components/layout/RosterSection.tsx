import { useCallback, useEffect, useState } from 'react';
import { Button, Dialog, DialogBody, DialogFooter } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { T } from '../theme';
import { useCharacterContext } from '@/contexts/CharacterContext';
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

function Avatar({ name, active }: { name: string; active: boolean }) {
  return (
    <div
      style={{
        width: 28, height: 28, borderRadius: '50%', flexShrink: 0,
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        background: active ? 'rgba(160,82,45,0.35)' : 'rgba(255,255,255,0.08)',
        color: active ? T.sidebarAccent : T.sidebarText,
        fontSize: 11, fontWeight: 600,
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
  const { character, roster, activeSource, switchToCompanion, switchToPlayer, refreshRoster } =
    useCharacterContext();
  const [pendingTarget, setPendingTarget] = useState<SwitchTarget | null>(null);
  const [classNames, setClassNames] = useState<Record<number, string>>({});
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

  const playerActive = activeSource.kind === 'player';
  const playerName = character.name || t('roster.player');

  // Name of the character that is CURRENTLY active (i.e. the one with the
  // unsaved changes the dialog is warning about), not the switch target.
  const activeCharacterName = activeSource.kind === 'player'
    ? playerName
    : roster.find(r => r.ros_name === activeSource.rosName)?.char_name ?? playerName;

  return (
    <div style={{ marginTop: 'auto', paddingTop: 8, borderTop: '1px solid rgba(255,255,255,0.1)' }}>
      <div
        className="t-sm"
        style={{ padding: '4px 16px', color: T.sidebarText, opacity: 0.6, textTransform: 'uppercase' }}
      >
        {t('roster.party')}
      </div>

      <button
        style={rowStyle(playerActive, isSwitching)}
        disabled={isSwitching}
        onClick={() => !playerActive && !isSwitching && requestSwitch({ kind: 'player' })}
      >
        <Avatar name={playerName} active={playerActive} />
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
            style={rowStyle(active, isSwitching)}
            disabled={isSwitching}
            onClick={() => !active && !isSwitching && requestSwitch({ kind: 'companion', rosName: entry.ros_name })}
          >
            <Avatar name={entry.char_name} active={active} />
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
