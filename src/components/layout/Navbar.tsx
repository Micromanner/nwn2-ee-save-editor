import { useState, useCallback, useEffect } from 'react';
import {
  Button, Navbar as BPNavbar, NavbarDivider, NavbarGroup, Tooltip,
} from '@blueprintjs/core';
import { GiCog, GiExitDoor, GiScrollUnfurled, GiTiedScroll, GiAnticlockwiseRotation, GiClockwiseRotation, GiScrollQuill } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { invoke } from '@tauri-apps/api/core';
import { T } from '../theme';
import { SettingsDialog } from '../Settings/SettingsPanel';
import { GameLaunchDialog, HistoryDialog } from '../shared';
import { useCharacterContext } from '@/contexts/CharacterContext';
import type { SaveCharacterResult } from '@/lib/api/character-state';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useToast } from '@/contexts/ToastContext';
import { TauriAPI } from '@/lib/tauri-api';

interface NavbarProps {
  onBack: () => void;
}

export function Navbar({ onBack }: NavbarProps) {
  const t = useTranslations();
  const [dirty, setDirty] = useState(false);
  useEffect(() => {
    let active = true;
    const check = () => { void invoke<boolean>('has_unsaved_changes').then(value => { if (active) setDirty(value); }).catch(() => {}); };
    check();
    const timer = setInterval(check, 2000);
    window.addEventListener('quest-edits-changed', check);
    return () => { active = false; clearInterval(timer); window.removeEventListener('quest-edits-changed', check); };
  }, []);
  const { handleError } = useErrorHandler();
  const { showToast } = useToast();
  const { character, playerName, historyState, undo, redo, activeSource, sessionKind, refreshRoster, refreshAll } = useCharacterContext();
  const [showSettings, setShowSettings] = useState(false);
  const [showGameLaunch, setShowGameLaunch] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [showHistory, setShowHistory] = useState(false);

  const handleSave = useCallback(async () => {
    setIsSaving(true);
    try {
      const result = await invoke<SaveCharacterResult>('save_character', { filePath: null });
      if (result.warning) {
        console.warn(result.warning);
        showToast(t('roster.syncWarning'), 'warning');
      }
      window.dispatchEvent(new Event('save-changed'));
      window.dispatchEvent(new Event('quest-edits-changed'));
      refreshRoster();
      void refreshAll();
      showToast(t('actions.saveSuccess'), 'success');
      const config = await invoke<{ show_launch_dialog: boolean }>('get_app_config');
      if (config.show_launch_dialog) {
        setShowGameLaunch(true);
      }
    } catch (err) {
      handleError(err);
    } finally {
      setIsSaving(false);
    }
  }, [showToast, t, handleError, refreshRoster, refreshAll]);

  const handleExport = async () => {
    setIsExporting(true);
    try {
      await invoke('export_to_localvault');
      showToast(t('actions.exportSuccess'), 'success');
    } catch (err) {
      handleError(err);
    } finally {
      setIsExporting(false);
    }
  };

  const handleLaunchGame = async (closeEditor: boolean) => {
    await TauriAPI.launchNWN2Game();
    if (closeEditor) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
      if (e.ctrlKey && !e.shiftKey && e.key === 'z') {
        e.preventDefault();
        if (historyState?.can_undo) void undo();
      }
      if (e.ctrlKey && e.shiftKey && (e.key === 'z' || e.key === 'Z')) {
        e.preventDefault();
        if (historyState?.can_redo) void redo();
      }
      if (e.ctrlKey && e.key === 'y') {
        e.preventDefault();
        if (historyState?.can_redo) void redo();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleSave, historyState, undo, redo]);

  return (
    <>
      <BPNavbar className="bp5-dark" style={{ background: T.navbar, paddingLeft: 12, paddingRight: 12, boxShadow: '0 1px 4px rgba(0,0,0,0.15)', position: 'relative', zIndex: 10 }}>
        <NavbarGroup align="left">
          <Button icon={<GameIcon icon={GiExitDoor} size={14} color={T.sidebarText} />} text={t('common.back')} small minimal style={{ color: T.sidebarText }} onClick={onBack} />
        </NavbarGroup>
        <NavbarGroup align="right">
          <Tooltip content={historyState?.undo_label ? t('actions.undoLabel', { action: historyState.undo_label }) : t('actions.undo')} placement="bottom">
            <Button icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} color={T.sidebarText} />} text={t('actions.undo')} small minimal disabled={!historyState?.can_undo} style={{ color: T.sidebarText }} onClick={() => void undo()} />
          </Tooltip>
          <Tooltip content={historyState?.redo_label ? t('actions.redoLabel', { action: historyState.redo_label }) : t('actions.redo')} placement="bottom">
            <Button icon={<GameIcon icon={GiClockwiseRotation} size={14} color={T.sidebarText} />} text={t('actions.redo')} small minimal disabled={!historyState?.can_redo} style={{ color: T.sidebarText }} onClick={() => void redo()} />
          </Tooltip>
          <Button icon={<GameIcon icon={GiScrollQuill} size={14} color={T.sidebarText} />} text={t('actions.history')} small minimal disabled={!character} style={{ color: T.sidebarText }} onClick={() => setShowHistory(true)} />
          <NavbarDivider />
          {activeSource.kind === 'player' && sessionKind === 'save' && (
            <Button icon={<GameIcon icon={GiScrollUnfurled} size={14} color={T.sidebarText} />} text={t('actions.exportCharacter')} small minimal loading={isExporting} style={{ color: T.sidebarText }} onClick={handleExport} />
          )}
          <Button icon={<GameIcon icon={GiTiedScroll} size={14} color={T.sidebarText} />} text={isSaving ? t('actions.saving') : `${t('actions.save')}${dirty ? ' *' : ''}`} small minimal loading={isSaving} style={{ color: T.sidebarText }} onClick={handleSave} />
          <NavbarDivider />
          <Button icon={<GameIcon icon={GiCog} size={14} color={T.sidebarText} />} text={t('common.settings')} small minimal style={{ color: T.sidebarText }} onClick={() => setShowSettings(true)} />
        </NavbarGroup>
      </BPNavbar>
      {showSettings && <SettingsDialog isOpen onClose={() => setShowSettings(false)} />}
      {showHistory && <HistoryDialog isOpen onClose={() => setShowHistory(false)} />}
      <GameLaunchDialog
        isOpen={showGameLaunch}
        onClose={() => setShowGameLaunch(false)}
        onLaunch={handleLaunchGame}
        saveName={playerName ?? character?.name}
      />
    </>
  );
}
