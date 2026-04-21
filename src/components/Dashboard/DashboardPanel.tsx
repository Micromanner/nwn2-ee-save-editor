import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button, ButtonGroup, Dialog, DialogBody, DialogFooter, Spinner } from '@blueprintjs/core';
import { GiOpenFolder, GiScrollUnfurled, GiBackwardTime, GiCog } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useToast } from '@/contexts/ToastContext';
import { TauriAPI } from '@/lib/tauri-api';
import { CharacterAPI, type SaveCharacterOption } from '@/services/characterApi';
import { T, PATTERN_BG } from '../theme';
import '../blueprint.css';
import { SaveList } from './SaveList';
import type { SaveEntryData } from './SaveEntry';
import { FileBrowserDialog } from './FileBrowserDialog';
import type { FileInfo } from './FileBrowserDialog';
import { SettingsDialog } from '../Settings/SettingsPanel';
import { BackupAPI } from '@/services/backupApi';

export default function DashboardPanel() {
  const t = useTranslations();
  const { character, importCharacter, refreshAll } = useCharacterContext();
  const { handleError } = useErrorHandler();
  const { showToast } = useToast();

  const [saveMode, setSaveMode] = useState<'sp' | 'mp'>('sp');
  const [saves, setSaves] = useState<SaveEntryData[]>([]);
  const [savePaths, setSavePaths] = useState<string[]>([]);
  const [defaultSavePath, setDefaultSavePath] = useState('');
  const [isLoadingSaves, setIsLoadingSaves] = useState(true);
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [isResolvingSaveCharacters, setIsResolvingSaveCharacters] = useState(false);
  const [loadedSavePath, setLoadedSavePath] = useState<string | null>(null);
  const [loadedPlayerIndex, setLoadedPlayerIndex] = useState<number | null>(null);

  const [showVaultBrowser, setShowVaultBrowser] = useState(false);
  const [showBackupBrowser, setShowBackupBrowser] = useState(false);
  const [backupPath, setBackupPath] = useState('');
  const [backupRefreshKey, setBackupRefreshKey] = useState(0);
  const [showSettings, setShowSettings] = useState(false);
  const [showCharacterPicker, setShowCharacterPicker] = useState(false);
  const [saveCharacters, setSaveCharacters] = useState<SaveCharacterOption[]>([]);
  const [pendingSaveSelection, setPendingSaveSelection] = useState<{
    path: string;
    label: string;
  } | null>(null);

  const isBusy = isImporting || isResolvingSaveCharacters;

  useEffect(() => {
    let cancelled = false;

    async function loadSaves() {
      setIsLoadingSaves(true);
      try {
        const [result, defaultPath] = await Promise.all([
          TauriAPI.findNWN2Saves(saveMode),
          invoke<string>('get_default_saves_path', { saveMode }),
        ]);
        if (cancelled) return;

        setDefaultSavePath(defaultPath);
        const paths: string[] = [];
        const entries: SaveEntryData[] = result.map(save => {
          paths.push(save.path);
          return {
            characterName: save.character_name || save.name,
            folderName: save.name,
            date: save.modified
              ? new Date(save.modified * 1000).toLocaleString()
              : '',
            thumbnail: null,
            isActive: save.path === loadedSavePath,
          };
        });

        setSaves(entries);
        setSavePaths(paths);
        setSelectedIndex(null);

        result.forEach((save, i) => {
          if (save.thumbnail) {
            TauriAPI.getSaveThumbnail(save.thumbnail).then(base64 => {
              if (!cancelled) {
                setSaves(prev => prev.map((s, j) =>
                  j === i ? { ...s, thumbnail: base64 } : s
                ));
              }
            }).catch(() => {});
          }
        });
      } catch (err) {
        if (!cancelled) handleError(err);
      } finally {
        if (!cancelled) setIsLoadingSaves(false);
      }
    }

    loadSaves();
    return () => { cancelled = true; };
  }, [handleError, loadedSavePath, saveMode]);

  useEffect(() => {
    if (!character) {
      setLoadedSavePath(null);
      setLoadedPlayerIndex(null);
    }
  }, [character]);

  const confirmSwitch = async (nextLabel: string) => {
    if (!character) return true;
    return TauriAPI.confirmSaveSwitch(character.name || t('character.noCharacter'), nextLabel);
  };

  const importSaveSelection = async (path: string, label: string, playerIndex?: number) => {
    setIsImporting(true);
    try {
      await importCharacter(path, playerIndex);
      setLoadedSavePath(path);
      setLoadedPlayerIndex(playerIndex ?? 0);
      setSaves(prev =>
        prev.map((save, index) => ({ ...save, isActive: savePaths[index] === path })),
      );
      setPendingSaveSelection(null);
      setSaveCharacters([]);
      setShowCharacterPicker(false);
    } catch (err) {
      handleError(err);
    } finally {
      setIsImporting(false);
    }
  };

  const beginImportFlow = async (path: string, label: string) => {
    if (isBusy) return;

    setIsResolvingSaveCharacters(true);
    try {
      const players = await CharacterAPI.listSaveCharacters(path);
      if (players.length <= 1) {
        const nextLabel = players[0]?.name || label;
        const confirmed = await confirmSwitch(nextLabel);
        if (!confirmed) return;

        await importSaveSelection(path, label, players[0]?.player_index);
        return;
      }

      setPendingSaveSelection({ path, label });
      setSaveCharacters(players);
      setShowCharacterPicker(true);
    } catch (err) {
      handleError(err);
    } finally {
      setIsResolvingSaveCharacters(false);
    }
  };

  const handleOpenSave = async () => {
    if (selectedIndex === null || isBusy) return;
    const path = savePaths[selectedIndex];
    if (!path) return;

    await beginImportFlow(path, saves[selectedIndex]?.characterName || saves[selectedIndex]?.folderName || path);
  };

  const handleSelectAndOpen = async (index: number) => {
    setSelectedIndex(index);
    const path = savePaths[index];
    if (!path) return;

    await beginImportFlow(path, saves[index]?.characterName || saves[index]?.folderName || path);
  };

  const handleImportVaultFile = async (file: FileInfo) => {
    setShowVaultBrowser(false);
    setIsImporting(true);
    try {
      await importCharacter(file.path);
    } catch (err) {
      handleError(err);
    } finally {
      setIsImporting(false);
    }
  };

  const handleBrowseFile = async (file: FileInfo) => {
    await beginImportFlow(file.path, file.character_name || file.save_name || file.name);
  };

  return (
    <div
      className="bp-app"
      style={{
        height: '100vh',
        background: T.sidebar,
        padding: 32,
        display: 'flex',
      }}
    >
      <div style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        borderRadius: 8,
        overflow: 'hidden',
        background: T.bg,
        backgroundImage: PATTERN_BG,
        backgroundSize: '200px 200px',
        boxShadow: '0 4px 24px rgba(0, 0, 0, 0.3)',
      }}>
        <div style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          padding: '10px 24px',
          borderBottom: `1px solid ${T.borderLight}`,
        }}>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <ButtonGroup minimal>
              <Button
                small
                active={saveMode === 'sp'}
                disabled={isBusy}
                onClick={() => setSaveMode('sp')}
              >
                {t('dashboard.singlePlayer')}
              </Button>
              <Button
                small
                active={saveMode === 'mp'}
                disabled={isBusy}
                onClick={() => setSaveMode('mp')}
              >
                {t('dashboard.multiplayer')}
              </Button>
            </ButtonGroup>
          </div>

          <div style={{ display: 'flex', gap: 4 }}>
            {selectedIndex !== null && (
              <Button
                small
                intent="primary"
                icon={<GameIcon icon={GiOpenFolder} size={14} />}
                loading={isBusy}
                onClick={handleOpenSave}
                disabled={isBusy}
              >
                {t('dashboard.openSave')}
              </Button>
            )}
            <Button
              minimal
              small
              icon={<GameIcon icon={GiScrollUnfurled} size={14} />}
              onClick={() => setShowVaultBrowser(true)}
              disabled={isBusy}
            >
              {t('actions.importCharacter')}
            </Button>
            <Button
              minimal
              small
              icon={<GameIcon icon={GiOpenFolder} size={14} />}
              onClick={async () => {
                try {
                  const config = await TauriAPI.getPathsConfig();
                  const docsPath = config?.documents_folder?.path;
                  if (docsPath) {
                    await TauriAPI.openFolderInExplorer(docsPath);
                  }
                } catch (err) { handleError(err); }
              }}
            >
              {t('actions.openDocumentsFolder')}
            </Button>
            <Button
              minimal
              small
              icon={<GameIcon icon={GiBackwardTime} size={14} />}
              onClick={async () => {
                try {
                  const path = await invoke<string>('get_default_backups_path');
                  if (path) {
                    setBackupPath(path);
                  }
                } catch {}
                setShowBackupBrowser(true);
              }}
            >
              {t('actions.manageBackups')}
            </Button>
            <Button minimal small icon={<GameIcon icon={GiCog} size={14} />} onClick={() => setShowSettings(true)}>
              {t('navigation.settings')}
            </Button>
          </div>
        </div>

        <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
          {isLoadingSaves ? (
            <div style={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <Spinner size={30} />
            </div>
          ) : (
            <SaveList
              saves={saves}
              selectedIndex={selectedIndex}
              onSelect={setSelectedIndex}
              onDoubleClick={handleSelectAndOpen}
              onBrowseFile={handleBrowseFile}
              defaultBrowsePath={defaultSavePath}
            />
          )}
        </div>
      </div>

      <FileBrowserDialog
        isOpen={showVaultBrowser}
        onClose={() => setShowVaultBrowser(false)}
        mode="load-vault"
        onSelectFile={handleImportVaultFile}
      />

      <FileBrowserDialog
        isOpen={showBackupBrowser}
        onClose={() => setShowBackupBrowser(false)}
        mode="manage-backups"
        currentPath={backupPath}
        onPathChange={setBackupPath}
        refreshKey={backupRefreshKey}
        canRestore
        onSelectFile={async (file) => {
          try {
            await BackupAPI.restoreFromBackup(0, {
              backup_path: file.path,
              confirm_restore: true,
              create_pre_restore_backup: true,
            });
            setShowBackupBrowser(false);
            showToast(t('fileBrowser.restoreSuccess'), 'success');
            await refreshAll();
          } catch (error) {
            handleError(error);
          }
        }}
        onDeleteBackup={async (file) => {
          try {
            await BackupAPI.deleteBackup(file.path);
            showToast(t('fileBrowser.backupDeleted'), 'success');
            setBackupRefreshKey(prev => prev + 1);
          } catch (error) {
            handleError(error);
          }
        }}
      />

      {showSettings && <SettingsDialog isOpen onClose={() => setShowSettings(false)} />}

      <Dialog
        isOpen={showCharacterPicker}
        onClose={() => {
          if (isImporting) return;
          setShowCharacterPicker(false);
          setPendingSaveSelection(null);
          setSaveCharacters([]);
        }}
        title={t('dashboard.chooseCharacter')}
      >
        <DialogBody>
          <div style={{ color: T.textMuted, marginBottom: 16, lineHeight: 1.5 }}>
            {t('dashboard.chooseCharacterHint', {
              save: pendingSaveSelection?.label || t('dashboard.openSave'),
            })}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {saveCharacters.map(player => {
              const classSummary = player.classes.length > 0
                ? player.classes.map(entry => `${entry.name} ${entry.level}`).join(' / ')
                : `Level ${player.total_level}`;
              const isCurrentSelection =
                loadedSavePath === pendingSaveSelection?.path &&
                loadedPlayerIndex === player.player_index;

              return (
                <button
                  key={`${pendingSaveSelection?.path ?? 'save'}-${player.player_index}`}
                  type="button"
                  disabled={isImporting}
                  onClick={async () => {
                    if (!pendingSaveSelection) return;
                    const confirmed = await confirmSwitch(player.name || pendingSaveSelection.label);
                    if (!confirmed) return;
                    await importSaveSelection(
                      pendingSaveSelection.path,
                      pendingSaveSelection.label,
                      player.player_index,
                    );
                  }}
                  style={{
                    width: '100%',
                    textAlign: 'left',
                    padding: '12px 14px',
                    borderRadius: 8,
                    border: isCurrentSelection
                      ? `1px solid ${T.accent}`
                      : `1px solid ${T.borderLight}`,
                    background: isCurrentSelection ? 'rgba(160, 82, 45, 0.10)' : T.surfaceAlt,
                    cursor: isImporting ? 'wait' : 'pointer',
                  }}
                >
                  <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12 }}>
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontWeight: 600, color: T.text }}>{player.name}</div>
                      <div style={{ marginTop: 4, fontSize: 12, color: T.textMuted }}>
                        {player.race}
                      </div>
                      <div style={{ marginTop: 8, fontSize: 12, color: T.textMuted }}>
                        {classSummary}
                      </div>
                    </div>
                    <div style={{ flexShrink: 0, fontSize: 11, color: T.textMuted }}>
                      {t('dashboard.playerSlot', { slot: player.player_index + 1 })}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        </DialogBody>
        <DialogFooter
          actions={(
            <Button
              text={t('actions.cancel')}
              onClick={() => {
                setShowCharacterPicker(false);
                setPendingSaveSelection(null);
                setSaveCharacters([]);
              }}
              disabled={isImporting}
            />
          )}
        />
      </Dialog>
    </div>
  );
}
