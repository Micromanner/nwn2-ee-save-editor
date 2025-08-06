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

export function SaveFileSelector() {
  const { isAvailable, isLoading, api } = useTauri();
  const { importCharacter, character, isLoading: characterLoading } = useCharacterContext();
  const { gameSettings } = useSettings();
  
  console.log('🔧 SaveFileSelector: Tauri context state:', { isAvailable, isLoading, hasApi: !!api });
  
  const [selectedFile, setSelectedFile] = useState<SaveFile | null>(null);
  const [saves, setSaves] = useState<SaveFile[]>([]);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [autoScanComplete, setAutoScanComplete] = useState(false);
  const [showLaunchDialog, setShowLaunchDialog] = useState(false);

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
      console.log('🔧 SaveFileSelector: Calling api.findNWN2Saves()...');
      const apiStart = performance.now();
      const foundSaves = await api.findNWN2Saves();
      const apiEnd = performance.now();
      console.log(`✅ SaveFileSelector: API call took ${apiEnd - apiStart}ms`);
      console.log('✅ SaveFileSelector: Found saves:', foundSaves);
      setSaves(foundSaves);
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
      // For now, save with empty updates - the backend will sync current character state
      const result = await CharacterAPI.saveCharacter(character.id, {});
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
  }, [character]);

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
  }, [api]);

  useEffect(() => {
    console.log('🔧 SaveFileSelector: useEffect triggered:', { isAvailable, hasApi: !!api });
    if (isAvailable && api && !autoScanComplete) {
      console.log('✅ SaveFileSelector: Auto-scanning for saves on startup...');
      loadAvailableSaves().finally(() => {
        setAutoScanComplete(true);
      });
    }
  }, [isAvailable, api, autoScanComplete]);

  const handleSelectFile = async () => {
    console.log('🔧 SaveFileSelector: handleSelectFile called');
    console.log('🔧 SaveFileSelector: API available:', !!api);
    
    if (!api) {
        console.error('❌ SaveFileSelector: Tauri API is not available');
        setError("Tauri API is not available. Cannot open file dialog.");
        return;
    };

    try {
      console.log("🔧 SaveFileSelector: Attempting to open file dialog via Tauri...");
      const file = await api.selectSaveFile();
      console.log("✅ SaveFileSelector: File selected successfully:", file);
      setSelectedFile(file);
      await importSaveFile(file);
    } catch (err) {
      console.error('❌ SaveFileSelector: The select_save_file command failed:', err);
      const errorMessage = typeof err === 'string' && err ? err : 'The file selection was cancelled or failed unexpectedly.';
      
      if (typeof err === 'string' && err) {
        setError(`Failed to select save file: ${errorMessage}`);
      }
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

      {character && (
        <div className="p-2 bg-surface-1 text-success rounded text-sm">
          Loaded: {character.name}
        </div>
      )}

      {/* Save and Load buttons */}
      <div className="flex gap-2 mt-4 mb-6">
        <Button
          variant="primary"
          size="md"
          className="flex-1 text-sm"
          disabled={!character || saving || characterLoading}
          onClick={saveCharacter}
        >
          {saving ? 'Saving...' : 'Save'}
        </Button>
        <Button
          variant="outline"
          size="md"
          className="flex-1 text-sm"
          onClick={handleSelectFile}
          disabled={importing || characterLoading}
        >
          {importing ? 'Loading...' : 'Load'}
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
          <div className="space-y-2 max-h-80 overflow-y-auto">
            {saves.slice(0, 5).map((save, index) => (
              <Card
                key={index}
                variant="interactive"
                selected={selectedFile?.path === save.path}
                onClick={() => handleImportSelectedSave(save)}
                className="cursor-pointer flex items-center gap-3"
              >
                <SaveThumbnail 
                  thumbnailPath={save.thumbnail} 
                  size="md" 
                  className="shrink-0"
                />
                <div className="flex flex-col flex-1 min-w-0">
                  <div className="recent-save-name truncate">{save.name}</div>
                  <div className="recent-save-action">
                    {selectedFile?.path === save.path ? 'Loaded' : 'Click to load'}
                  </div>
                </div>
              </Card>
            ))}
          </div>
          {saves.length > 5 && (
            <div className="text-xs text-text-muted mt-2 text-center">
              Showing 5 of {saves.length} saves
            </div>
          )}
        </Card>
      ) : autoScanComplete ? (
        <Card variant="container">
          <div className="recent-saves-header">Last 3 Saved Games</div>
          <div className="text-xs text-text-muted text-center py-4">
            No saves found automatically. Use the Load button above to browse for save files.
          </div>
        </Card>
      ) : null}
      
      {selectedFile && (
        <div className="text-xs text-text-secondary p-2 bg-surface-1 rounded">
          Selected: {selectedFile.name}
        </div>
      )}

      <GameLaunchDialog
        isOpen={showLaunchDialog}
        onClose={() => setShowLaunchDialog(false)}
        onLaunch={handleGameLaunch}
        saveName={character?.name}
        gamePathDetected={!!gameSettings.nwn2_installation_path}
      />
    </div>
  );
}
