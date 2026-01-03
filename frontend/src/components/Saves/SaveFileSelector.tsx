'use client';

import { useState, useEffect, useCallback } from 'react';
import { useTauri } from '@/providers/TauriProvider';
import { SaveFile } from '@/lib/tauri-api';
import { SaveThumbnail } from './SaveThumbnail';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useSettings } from '@/contexts/SettingsContext';
import { GameLaunchDialog } from '../GameLaunchDialog';
import { CharacterAPI } from '@/services/characterApi';
import { Button } from '@/components/ui/Button';
import { Card } from '@/components/ui/Card';
import FileBrowserModal from '@/components/FileBrowser/FileBrowserModal';
import { apiClient } from '@/lib/api/client';

export function SaveFileSelector() {
  const { isAvailable, isLoading, api } = useTauri();
  const { importCharacter, character, isLoading: characterLoading } = useCharacterContext();
  const { gameSettings } = useSettings();
  
  console.log('🔧 SaveFileSelector: Tauri context state:', { isAvailable, isLoading, hasApi: !!api });
  
  interface ExtendedSaveFile extends SaveFile {
    character?: string;
  }

  const [selectedFile, setSelectedFile] = useState<SaveFile | null>(null);
  const [saves, setSaves] = useState<ExtendedSaveFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [autoScanComplete, setAutoScanComplete] = useState(false);
  const [showLaunchDialog, setShowLaunchDialog] = useState(false);
  const [showFileBrowser, setShowFileBrowser] = useState(false);
  const [showBackupBrowser, setShowBackupBrowser] = useState(false);
  const [currentPath, setCurrentPath] = useState<string>('');
  const [backupPath, setBackupPath] = useState<string>('');
  const [backupRefreshKey, setBackupRefreshKey] = useState(0);

  const loadAvailableSaves = useCallback(async () => {
    const startTime = performance.now();
    console.log('🔧 SaveFileSelector: loadAvailableSaves called');
    console.log('🔧 SaveFileSelector: API in loadAvailableSaves:', !!api);
    
    if (!api) {
      console.log('❌ SaveFileSelector: No API available in loadAvailableSaves');
      return;
    }
    
    setLoading(true);
    setError(null);
    try {
      console.log('🔧 SaveFileSelector: Calling backend /saves/list...');
      const apiStart = performance.now();
      
      // Call backend API instead of Tauri API
      const response = await apiClient.get<any>('/saves/list?limit=10'); // limit 10 for "Last 3" + buffer
      
      const apiEnd = performance.now();
      console.log(`✅ SaveFileSelector: API call took ${apiEnd - apiStart}ms`);
      
      if (response && response.files) {
        const mappedSaves: ExtendedSaveFile[] = response.files
            .filter((f: any) => f.is_directory) 
            .map((f: any) => ({
                name: f.name,
                path: f.path,
                thumbnail: f.thumbnail || '', 
                character: f.character_name,
                modified: f.modified ? parseFloat(f.modified) * 1000 : undefined
            }));
            
        console.log('✅ SaveFileSelector: Found saves:', mappedSaves);
        setSaves(mappedSaves);
      }
    } catch (err) {
      console.error('❌ SaveFileSelector: Failed to find saves:', err);
      const errorMessage = typeof err === 'string' ? err : 'An unknown error occurred while finding save files.';
      setError(`Failed to find save files. Please check if NWN2 save directory exists. Details: ${errorMessage}`);
    } finally {
      setLoading(false);
      const endTime = performance.now();
      console.log(`🔧 SaveFileSelector: Total loadAvailableSaves took ${endTime - startTime}ms`);
    }
  }, [api]);

  const importSaveFile = useCallback(async (saveFile: SaveFile) => {
    setImporting(true);
    setError(null);

    try {
      await importCharacter(saveFile.path);
      console.log('Save imported successfully');
      setError(null);
    } catch (err) {
      console.error('Failed to import save:', err);
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Failed to import save file. Please check the file and try again.');
      }
    } finally {
      setImporting(false);
    }
  }, [importCharacter]);

  const saveCharacter = useCallback(async () => {
    if (!character?.id) {
      setError('No character loaded to save');
      return;
    }

    setSaving(true);
    setError(null);

    try {
      // Send sync signal to save current character state
      const result = await CharacterAPI.saveCharacter(character.id, { sync_current_state: true });
      console.log('Character saved successfully:', result);
      
      // Show launch dialog after successful save (if enabled in settings)
      if (gameSettings.show_launch_dialog) {
        setShowLaunchDialog(true);
      }
    } catch (err) {
      console.error('Failed to save character:', err);
      if (err instanceof Error) {
        setError(err.message);
      } else {
        setError('Failed to save character. Please try again.');
      }
    } finally {
      setSaving(false);
    }
  }, [character, gameSettings.show_launch_dialog]);

  const handleGameLaunch = useCallback(async (closeEditor: boolean) => {
    if (!api) {
      setError('Cannot launch game: Tauri API not available');
      return;
    }

    try {
      // Launch the game with stored path preference
      await api.launchNWN2Game(gameSettings.nwn2_installation_path);
      console.log('Game launched successfully');
      
      // Close the dialog
      setShowLaunchDialog(false);
      
      // Close the editor if requested
      if (closeEditor) {
        // Use Tauri to close the application
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().close();
      }
    } catch (err) {
      console.error('Failed to launch game:', err);
      setError(err instanceof Error ? err.message : 'Failed to launch NWN2:EE');
      setShowLaunchDialog(false);
    }
  }, [api, gameSettings.nwn2_installation_path]);

  const handleOpenBackupsFolder = useCallback(() => {
    // Clear any previous messages
    setSuccessMessage(null);
    setError(null);

    // If a save is selected, show backups for that specific save
    // Otherwise, show all backups in the main backups directory
    let backupsPath = '';

    if (selectedFile) {
      // Show backups for the selected save
      const saveDir = selectedFile.path.replace(/[\/\\][^\/\\]*$/, '');
      backupsPath = saveDir + (saveDir.includes('\\') ? '\\backups' : '/backups');
    } else {
      // Show all backups (empty path will use default from backend)
      backupsPath = '';
    }

    setBackupPath(backupsPath);
    setShowBackupBrowser(true);
  }, [selectedFile]);

  const handleBackupSelect = useCallback(async (file: { path: string; name: string }) => {
    try {
      await apiClient.post('/backups/restore', {
        backup_path: file.path,
        save_path: selectedFile?.path || null,
        create_pre_restore_backup: true,
        confirm_restore: true
      });

      console.log('Backup restored successfully');
      setShowBackupBrowser(false);

      if (selectedFile) {
         await importSaveFile(selectedFile);
      } else {
         setSuccessMessage('Backup restored successfully. Please load the save file.');
         setTimeout(() => setSuccessMessage(null), 5000);
      }
    } catch (err) {
      console.error('Failed to restore backup:', err);
      setError(err instanceof Error ? err.message : 'Failed to restore backup');
    }
  }, [selectedFile, importSaveFile]);

  const handleDeleteBackup = useCallback(async (file: { path: string; name: string }) => {
    try {
      await apiClient.delete('/backups/delete', {
        backup_path: file.path
      });

      setBackupRefreshKey(prev => prev + 1);
    } catch (err) {
      console.error('Failed to delete backup:', err);
      setError(err instanceof Error ? err.message : 'Failed to delete backup');
    }
  }, []);

  useEffect(() => {
    console.log('🔧 SaveFileSelector: useEffect triggered:', { isAvailable, hasApi: !!api });
    if (isAvailable && api && !autoScanComplete) {
      console.log('✅ SaveFileSelector: Auto-scanning for saves on startup...');
      loadAvailableSaves().finally(() => {
        setAutoScanComplete(true);
      });
    }
  }, [isAvailable, api, autoScanComplete, loadAvailableSaves]);

  // Expose backup handler to parent - always available
  useEffect(() => {
    // Always expose the function, even without a save selected
    (window as Window & { __openBackups?: () => void }).__openBackups = handleOpenBackupsFolder;

    return () => {
      delete (window as Window & { __openBackups?: () => void }).__openBackups;
    };
  }, [handleOpenBackupsFolder]);

  const handleSelectFile = async () => {
    setShowFileBrowser(true);
  };

  const handleFileBrowserSelect = async (file: { path: string; name: string }) => {
    try {
      const saveFile: SaveFile = {
        name: file.name,
        path: file.path,
        thumbnail: ''
      };
      setSelectedFile(saveFile);
      await importSaveFile(saveFile);
    } catch (err) {
      console.error('Failed to import save file:', err);
      setError(err instanceof Error ? err.message : 'Failed to import save file');
    }
  };

  const handleImportSelectedSave = async (save: SaveFile) => {
    // If clicking the same save that's already loaded, do nothing
    if (selectedFile?.path === save.path && character) {
      return;
    }
    
    // If a save is already loaded and user clicks a different one, show confirmation
    if (selectedFile && selectedFile.path !== save.path && character && api) {
      const confirmed = await api.confirmSaveSwitch(selectedFile.name, save.name);
      if (!confirmed) {
        return; // User cancelled
      }
    }
    
    // Normal import flow
    setSelectedFile(save);
    await importSaveFile(save);
  };

  // const handleImportLatestSave = async () => {
  //   if (saves.length === 0) return;
  //   
  //   const latestSave = saves.reduce((latest, current) => {
  //     const latestMatch = latest.name.match(/(\d{2}-\d{2}-\d{4}-\d{2}-\d{2})/);
  //     const currentMatch = current.name.match(/(\d{2}-\d{2}-\d{4}-\d{2}-\d{2})/);
  //     
  //     if (!latestMatch) return current;
  //     if (!currentMatch) return latest;
  //     
  //     const parseDate = (dateStr: string) => {
  //       const [day, month, year, hour, minute] = dateStr.split('-');
  //       return `${year}-${month}-${day}T${hour}:${minute}:00`;
  //     };
  //     
  //     const latestDate = new Date(parseDate(latestMatch[1]));
  //     const currentDate = new Date(parseDate(currentMatch[1]));
  //
  //     return currentDate > latestDate ? current : latest;
  //   });
  //   
  //   await handleImportSelectedSave(latestSave);
  // };

  if (isLoading) {
    return <div className="text-sm text-text-muted">Initializing...</div>;
  }

  if (!isAvailable) {
    return <div className="text-sm text-error">Desktop mode unavailable</div>;
  }

  return (
    <div className="space-y-3">
      {error && (
        <div className="p-2 bg-surface-1 text-error rounded text-sm">
          {error}
        </div>
      )}

      {successMessage && (
        <div className="p-2 bg-surface-1 text-success rounded text-sm">
          {successMessage}
        </div>
      )}

      {character && (
        <div className="p-2 bg-surface-1 text-success rounded text-sm">
          Loaded: {character.name}
        </div>
      )}

      {/* Browse button */}
      <div className="flex gap-2 mt-4 mb-6">
        <Button
          variant="outline"
          size="md"
          className="flex-1 text-sm h-10"
          onClick={handleSelectFile}
          disabled={importing || characterLoading}
        >
          {importing ? 'Loading...' : 'Browse...'}
        </Button>
      </div>

      {/* Auto-detected saves */}
      {loading && !autoScanComplete ? (
        <div className="text-xs text-text-muted">Scanning for saves...</div>
      ) : saves.length > 0 ? (
        <Card variant="container">
          <div className="recent-saves-header">
            Last 3 Saved Games
          </div>
          <div className="space-y-2 max-h-[600px] overflow-y-auto">
            {saves.slice(0, 3).map((save, index) => (
              <Card
                key={index}
                variant="interactive"
                selected={selectedFile?.path === save.path}
                onClick={() => handleImportSelectedSave(save)}
                className="cursor-pointer flex items-center gap-4 p-3"
              >
                <SaveThumbnail 
                  thumbnailPath={save.thumbnail} 
                  size="lg" 
                  className="shrink-0"
                />
                <div className="flex flex-col flex-1 min-w-0">
                  <div className="recent-save-name truncate">
                    {save.character ? (
                        <div className="flex flex-col">
                            <span className="font-semibold text-[rgb(var(--color-primary))]">{save.character}</span>
                            <span className="text-xs text-[rgb(var(--color-text-muted))] opacity-75">{save.name}</span>
                        </div>
                    ) : save.name}
                  </div>
                  {save.modified && (
                    <div className="text-xs text-[rgb(var(--color-text-muted))] mt-0.5">
                       {new Date(save.modified).toLocaleString()}
                    </div>
                  )}
                  <div className="recent-save-action mt-1">
                    {selectedFile?.path === save.path ? 'Loaded' : 'Click to load'}
                  </div>
                </div>
              </Card>
            ))}
          </div>
        </Card>
      ) : autoScanComplete ? (
        <Card variant="container">
          <div className="recent-saves-header">Last 3 Saved Games</div>
          <div className="text-xs text-text-muted text-center py-4">
            No saves found automatically. Use the Load button above to browse for save files.
          </div>
        </Card>
      ) : null}

      <GameLaunchDialog
        isOpen={showLaunchDialog}
        onClose={() => setShowLaunchDialog(false)}
        onLaunch={handleGameLaunch}
        saveName={character?.name}
        gamePathDetected={!!gameSettings.nwn2_installation_path}
      />

      <FileBrowserModal
        isOpen={showFileBrowser}
        onClose={() => setShowFileBrowser(false)}
        mode="load-saves"
        onSelectFile={handleFileBrowserSelect}
        currentPath={currentPath}
        onPathChange={setCurrentPath}
      />

      <FileBrowserModal
        isOpen={showBackupBrowser}
        onClose={() => setShowBackupBrowser(false)}
        mode="manage-backups"
        currentPath={backupPath}
        onPathChange={setBackupPath}
        onDeleteBackup={handleDeleteBackup}
        canRestore={true}
        refreshKey={backupRefreshKey}
        onSelectFile={handleBackupSelect}
      />
    </div>
  );
}
