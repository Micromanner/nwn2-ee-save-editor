import { useState, useCallback, useEffect } from 'react';
import {
  Button, Navbar as BPNavbar, NavbarDivider, NavbarGroup, Tooltip,
} from '@blueprintjs/core';
import { GiCog, GiExitDoor, GiScrollUnfurled, GiTiedScroll, GiAnticlockwiseRotation, GiClockwiseRotation } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { invoke } from '@tauri-apps/api/core';
import { T } from '../theme';
import { SettingsDialog } from '../Settings/SettingsPanel';
import { GameLaunchDialog } from '../shared';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useToast } from '@/contexts/ToastContext';
import { TauriAPI } from '@/lib/tauri-api';

interface NavbarProps {
  onBack: () => void;
}

export function Navbar({ onBack }: NavbarProps) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { showToast } = useToast();
  const { character, historyState, undo, redo } = useCharacterContext();
  const [showSettings, setShowSettings] = useState(false);
  const [showGameLaunch, setShowGameLaunch] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  const handleSave = useCallback(async () => {
    setIsSaving(true);
    try {
      await invoke('save_character', { filePath: null });
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
  }, [showToast, t, handleError]);

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
          <Button icon={<GameIcon icon={GiCog} size={14} color={T.sidebarText} />} text={t('common.settings')} small minimal style={{ color: T.sidebarText }} onClick={() => setShowSettings(true)} />
          <Button icon={<GameIcon icon={GiExitDoor} size={14} color={T.sidebarText} />} text={t('common.back')} small minimal style={{ color: T.sidebarText }} onClick={onBack} />
        </NavbarGroup>
        <NavbarGroup align="right">
          <Tooltip content={historyState?.undo_label ? t('actions.undoLabel', { action: historyState.undo_label }) : t('actions.undo')} placement="bottom">
            <Button icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} color={T.sidebarText} />} text={t('actions.undo')} small minimal disabled={!historyState?.can_undo} style={{ color: T.sidebarText }} onClick={() => void undo()} />
          </Tooltip>
          <Tooltip content={historyState?.redo_label ? t('actions.redoLabel', { action: historyState.redo_label }) : t('actions.redo')} placement="bottom">
            <Button icon={<GameIcon icon={GiClockwiseRotation} size={14} color={T.sidebarText} />} text={t('actions.redo')} small minimal disabled={!historyState?.can_redo} style={{ color: T.sidebarText }} onClick={() => void redo()} />
          </Tooltip>
          <NavbarDivider />
          <Button icon={<GameIcon icon={GiScrollUnfurled} size={14} color={T.sidebarText} />} text={t('actions.exportCharacter')} small minimal loading={isExporting} style={{ color: T.sidebarText }} onClick={handleExport} />
          <Button icon={<GameIcon icon={GiTiedScroll} size={14} color={T.sidebarText} />} text={isSaving ? t('actions.saving') : t('actions.save')} small minimal loading={isSaving} style={{ color: T.sidebarText }} onClick={handleSave} />
        </NavbarGroup>
      </BPNavbar>
      {showSettings && <SettingsDialog isOpen onClose={() => setShowSettings(false)} />}
      <GameLaunchDialog
        isOpen={showGameLaunch}
        onClose={() => setShowGameLaunch(false)}
        onLaunch={handleLaunchGame}
        saveName={character?.name}
      />
    </>
  );
}
