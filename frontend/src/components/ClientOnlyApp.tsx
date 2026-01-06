'use client';

import { useState, useEffect, useCallback } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useTauri } from '@/providers/TauriProvider';
import DynamicAPI from '@/lib/utils/dynamicApi';
import CustomTitleBar from '@/components/ui/CustomTitleBar';
import Sidebar from '@/components/ui/Sidebar';
import { GameLaunchDialog } from '@/components/GameLaunchDialog';
import { getCurrentWindow } from '@tauri-apps/api/window';
import AbilityScoresEditor from '@/components/AbilityScores/AbilityScoresEditor';
import AppearanceEditor from '@/components/Appearance/AppearanceEditor';
import ClassAndLevelsEditor from '@/components/ClassesLevel/ClassAndLevelsEditor';
import InventoryEditor from '@/components/Inventory/InventoryEditor';
import SkillsEditor from '@/components/Skills/SkillsEditor';
import FeatsEditor from '@/components/Feats/FeatsEditor';
import SpellsEditor from '@/components/Spells/SpellsEditor';
import CharacterOverview from '@/components/Overview/CharacterOverview';
import CompanionsView from '@/components/Companions/CompanionsView';
import CharacterBuilder from '@/components/CharacterBuilder';
import GameStateEditor from '@/components/GameState/GameStateEditor';
import SettingsPage from '@/app/settings/page';

import TauriInitializer from '@/components/TauriInitializer';
import { CharacterProvider, useCharacterContext } from '@/contexts/CharacterContext';
import { IconCacheProvider } from '@/contexts/IconCacheContext';
import Dashboard from '@/components/Dashboard';
import EditorHeader from '@/components/EditorHeader';
import { CharacterAPI } from '@/services/characterApi';
import { Button } from '@/components/ui/Button';

import LevelHelperModal from '@/components/ClassesLevel/LevelHelperModal';

function AppContent() {
  const t = useTranslations();
  const { isAvailable, isLoading, api } = useTauri();
  const { character, isLoading: characterLoading, loadSubsystem } = useCharacterContext();
  const { clearCharacter, characterId, importCharacter, loadCharacter } = useCharacterContext();

  const [activeTab, setActiveTab] = useState('overview');
  const [viewMode, setViewMode] = useState<'dashboard' | 'editor'>('dashboard');

  const [currentCompanion, setCurrentCompanion] = useState<{
    name: string;
    portrait?: string;
    isCompanion: boolean;
  } | null>(null);

  // Level Helper State - just controls visibility, data is fetched dynamically by the modal
  const [showLevelHelper, setShowLevelHelper] = useState(false);

  // When a level is gained, show the helper modal
  // The modal itself will query subsystems for current available points
  const handleLevelGains = useCallback(() => {
    setShowLevelHelper(true);
  }, []);
  const [appReady, setAppReady] = useState(false);
  const [initProgress, setInitProgress] = useState({
    step: 'initializing',
    progress: 0,
    message: 'Starting up...'
  });
  const [backendReady, setBackendReady] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const [showLoadingOverlay, setShowLoadingOverlay] = useState(false);
  const [showLaunchDialog, setShowLaunchDialog] = useState(false);

  // Sync viewMode when character ID changes (new load)
  useEffect(() => {
    if (character?.id) {
       setViewMode('editor');
    }
  }, [character?.id]);

  const [feedback, setFeedback] = useState<{type: 'success' | 'error', message: string} | null>(null);

  useEffect(() => {
    setShowLoadingOverlay(characterLoading);
  }, [characterLoading]);

  // Create the current character object for the sidebar
  const currentCharacter = currentCompanion || (character ? {
    name: character.name || 'Unknown Character',
    portrait: character.portrait,
    customPortrait: character.customPortrait,
    isCompanion: false
  } : null);

  // Mock function to simulate loading a companion
  const handleLoadCompanion = (companionName: string) => {
    setCurrentCompanion({
      name: companionName,
      isCompanion: true
    });
  };

  const handleBackToMain = () => {
    setCurrentCompanion(null);
  };

  const handleEditorBack = () => {
    // Minimize to dashboard instead of clearing
    setViewMode('dashboard');
    setActiveTab('overview'); // Reset tab optionally, or keep it
  };

  const handleContinueEditing = () => {
    setViewMode('editor');
  };

  const handleCloseSession = () => {
    clearCharacter();
    setViewMode('dashboard');
    setActiveTab('overview');
  };
  
  const handleCloseSettings = () => {
    setShowSettings(false);
  };

  const handleSaveCharacter = async () => {
    if (!characterId) return;
    try {
      await CharacterAPI.saveCharacter(characterId);
      // Refresh to clear dirty state
      await loadCharacter(characterId);
      
      setShowLaunchDialog(true);
    } catch (error) {
       console.error('Failed to save character', error);
       setFeedback({ type: 'error', message: error instanceof Error ? error.message : 'Failed to save character' });
       setTimeout(() => setFeedback(null), 5000);
    }
  };

  const handleLaunchGame = async (closeEditor: boolean) => {
    try {
      await api?.launchNWN2Game();
      if (closeEditor) {
        try {
           await getCurrentWindow().close();
        } catch (e) {
           console.error("Failed to close window", e);
        }
      }
      setShowLaunchDialog(false);
    } catch (error) {
      console.error("Failed to launch game", error);
      setFeedback({ type: 'error', message: "Failed to launch game" });
    }
  };

  const handleOpenBackups = () => {
    (window as Window & { __openBackups?: () => void }).__openBackups?.();
  };

  const handleOpenFolder = async () => {
      try {
        const saves = await api?.findNWN2Saves();
        if (saves && saves.length > 0) {
          const firstSavePath = saves[0].path;
          const separator = firstSavePath.includes('\\') ? '\\' : '/';
          const lastSeparatorIndex = firstSavePath.lastIndexOf(separator);
          const folderPath = firstSavePath.substring(0, lastSeparatorIndex);
          
          await api?.openFolderInExplorer(folderPath);
        } else {
             console.warn("No saves found to determine folder path.");
        }
      } catch (err) {
        console.error("Failed to open saves folder:", err);
      }
  };

  const handleImportCharacter = async () => {
    try {
      const filePath = await api?.selectCharacterFile();
      if (filePath) {
        await importCharacter(filePath);
      }
    } catch (error) {
       console.error("Failed to import character:", error);
    }
  };

  const handleSettings = () => {
      setShowSettings(true);
  };

  const handleTabChange = async (tabId: string) => {
    setActiveTab(tabId);
    
    // Fetch fresh data for subsystem-related tabs
    if (character?.id) {
      try {
        switch (tabId) {
          case 'skills':
            await loadSubsystem('skills');
            break;
          case 'classes':
            await loadSubsystem('classes');
            break;
          case 'abilityScores':
            await loadSubsystem('abilityScores');
            break;
          case 'feats':
            await loadSubsystem('feats');
            break;
          case 'combat':
            await loadSubsystem('combat');
            break;
          case 'saves':
            await loadSubsystem('saves');
            break;
          case 'spells':
            await loadSubsystem('spells');
            break;
          case 'inventory':
            await loadSubsystem('inventory');
            break;
          case 'gameState':
            break;
          case 'overview':
            await loadSubsystem('abilityScores');
            await loadSubsystem('combat');
            await loadSubsystem('skills');
            await loadSubsystem('feats');
            await loadSubsystem('saves');
            await loadSubsystem('classes');
            break;
          default:
            break;
        }
      } catch (err) {
        console.error(`Failed to fetch data for ${tabId}:`, err);
      }
    }
  };


  useEffect(() => {
    if (!api) return;
    
    let timeoutId: NodeJS.Timeout;
    let isActive = true;
    let pollDelay = 500; // Start at 500ms
    const maxDelay = 5000; // Max 5 seconds between polls
    const maxTotalTime = 60000; // 60 second total timeout
    const startTime = Date.now();
    
    const checkInitStatus = async () => {
      try {
        await DynamicAPI.initialize();
        const response = await DynamicAPI.fetch('/system/initialization/status/');
        if (!response.ok) return;
        
        const data = await response.json();
        
        if (isActive) {
          setInitProgress({
            step: data.stage,
            progress: data.progress,
            message: data.message
          });
          
          if (data.stage === 'ready') {
            setBackendReady(true);
            return; // Stop polling
          }
        }
      } catch (_) { // eslint-disable-line @typescript-eslint/no-unused-vars
        // Django might not be ready yet
      }
      
      // Check total timeout
      if (Date.now() - startTime > maxTotalTime) {
        console.error('Backend initialization timeout after 60 seconds');
        return;
      }
      
      if (isActive) {
        timeoutId = setTimeout(() => {
          checkInitStatus();
          pollDelay = Math.min(pollDelay * 1.5, maxDelay);
        }, pollDelay);
      }
    };

    checkInitStatus();
    
    return () => {
      isActive = false;
      clearTimeout(timeoutId);
    };
  }, [api]);

  useEffect(() => {
    if (isAvailable && api && backendReady) {
      const timer = setTimeout(() => {
        setAppReady(true);
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [isAvailable, api, backendReady]);

  if (!appReady || isLoading || !isAvailable || !api) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-[rgb(var(--color-background))]">
        <div className="max-w-md w-full mx-4">
          <div className="bg-[rgb(var(--color-surface-1))] rounded-lg shadow-lg border border-[rgb(var(--color-surface-border))] p-8 text-center">
            <div className="mb-6">
              <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-[rgb(var(--color-surface-2))] flex items-center justify-center">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
              </div>
              <h1 className="text-2xl font-bold text-[rgb(var(--color-text-primary))] mb-2">NWN2 Save Editor</h1>
              <p className="text-[rgb(var(--color-text-secondary))]">{initProgress.message}</p>
            </div>

            <div className="w-full bg-[rgb(var(--color-surface-2))] rounded-full h-3 mb-4">
              <div 
                className="bg-[rgb(var(--color-primary))] h-3 rounded-full transition-all duration-500 ease-out"
                style={{ width: `${initProgress.progress}%` }}
              ></div>
            </div>
            
            <div className="text-sm text-[rgb(var(--color-text-muted))]">
              {initProgress.step === 'icon_cache' && 'Loading icons...'}
              {initProgress.step === 'game_data' && 'Loading game data...'}
              {initProgress.step === 'resource_manager' && 'Initializing...'}
              {initProgress.step === 'ready' && 'Starting application...'}
              {initProgress.step === 'initializing' && 'Starting up...'}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      <TauriInitializer />

      <div className="flex-shrink-0">
        <CustomTitleBar />
      </div>

      <div className="flex-1 flex overflow-hidden">
        {(() => {
          if (viewMode === 'editor' && character) {
             return (
              <div className="flex flex-col w-full h-full overflow-hidden">
                 <EditorHeader
                    characterName={character.name}
                    saveName="Save File"
                    onBack={handleEditorBack}
                    onImport={() => {}}
                    onExport={() => {}}
                    onSave={handleSaveCharacter}
                    isModified={character.has_unsaved_changes}
                  />

                 <div className="flex-1 flex overflow-hidden">
                    <Sidebar 
                      activeTab={activeTab === 'dashboard_minimized' ? 'overview' : activeTab} 
                      onTabChange={handleTabChange}
                      currentCharacter={currentCharacter}
                      onBackToMain={handleBackToMain}
                      isLoading={characterLoading}
                    />
                    
                    <div className="flex-1 flex flex-col overflow-hidden">
                      <main className="flex-1 bg-[rgb(var(--color-background))] overflow-y-auto">
                        <div className="p-6">
                          {activeTab === 'overview' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.overview')}</h2>
                              <CharacterOverview onNavigate={setActiveTab} />
                            </div>
                          )}
                          
                          {activeTab === 'character-builder' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.characterBuilder')}</h2>
                              <CharacterBuilder />
                            </div>
                          )}
                          
                          {activeTab === 'abilityScores' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.abilityScores')}</h2>
                              <AbilityScoresEditor />
                            </div>
                          )}
                          
                          {activeTab === 'appearance' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.appearance')}</h2>
                              <AppearanceEditor />
                            </div>
                          )}
                          
                          {activeTab === 'classes' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.classes')}</h2>
                              <ClassAndLevelsEditor 
                                onNavigate={(path) => {
                                  // Simple mapping: remove leading slash
                                  const tabId = path.replace(/^\//, '');
                                  setActiveTab(tabId);
                                }} 
                                onLevelGains={handleLevelGains}
                              />
                            </div>
                          )}
                          
                          {activeTab === 'skills' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.skills')}</h2>
                              <SkillsEditor />
                            </div>
                          )}
                          
                          {activeTab === 'feats' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.feats')}</h2>
                              <FeatsEditor />
                            </div>
                          )}
                          
                          {activeTab === 'spells' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.spells')}</h2>
                              <SpellsEditor />
                            </div>
                          )}
                          
                          {activeTab === 'inventory' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.inventory')}</h2>
                              <InventoryEditor />
                            </div>
                          )}
                          
                          {activeTab === 'companions' && <CompanionsView onLoadCompanion={handleLoadCompanion} currentCharacterName={currentCharacter?.name} />}
    
                          {activeTab === 'gameState' && (
                            <div className="space-y-6">
                              <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">{t('navigation.gameState')}</h2>
                              <GameStateEditor />
                            </div>
                          )}

                          {activeTab === 'settings' && (
                            <div className="space-y-6">

                               <SettingsPage />
                            </div>
                          )}
                        </div>
                      </main>
                    </div>
                 </div>
              </div>
             );
          }

          if (showSettings) {
            return (
              <div className="flex flex-col h-full w-full bg-[rgb(var(--color-background))]">
                <div className="h-14 flex items-center px-4 bg-[rgb(var(--color-surface-2))] border-b border-[rgb(var(--color-surface-border))] shadow-sm flex-shrink-0">
                  <div className="flex items-center w-full">
                    <Button 
                      variant="ghost" 
                      onClick={handleCloseSettings}
                      className="text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))]"
                    >
                      <span className="mr-2">←</span> Back to Dashboard
                    </Button>
                    <div className="h-6 w-px bg-[rgb(var(--color-surface-border))] mx-4"></div>
                    <span className="text-sm font-medium text-[rgb(var(--color-text-secondary))]">Application Settings</span>
                  </div>
                </div>

                <div className="flex-1 overflow-y-auto p-6 md:p-8 max-w-5xl mx-auto w-full">
                   <SettingsPage />
                </div>
              </div>
            );
          }

          return (
            <Dashboard 
              onOpenBackups={handleOpenBackups}
              onOpenFolder={handleOpenFolder}
              onSettings={handleSettings}
              onImportCharacter={handleImportCharacter}
              activeCharacter={character ? { name: character.name } : undefined}
              onContinueEditing={handleContinueEditing}
              onCloseSession={handleCloseSession}
            />
          );
        })()}
      </div>

      {showLoadingOverlay && (
        <div className="fixed inset-0 z-[9999] flex flex-col items-center justify-center bg-[rgb(var(--color-background))] animate-in fade-in duration-200">
          <div className="flex flex-col items-center gap-4">
            <div className="w-12 h-12 rounded-full border-2 border-[rgb(var(--color-primary)/0.2)] border-t-[rgb(var(--color-primary))] animate-spin"></div>
            <div className="text-[rgb(var(--color-text-primary))] font-medium animate-pulse">
              Loading save
            </div>
          </div>
        </div>
      )}

      {feedback && (
        <div className="fixed top-20 right-8 z-[10000] animate-in fade-in slide-in-from-right-8 duration-300">
          <div className={`rounded-lg shadow-lg border p-4 flex items-center gap-3 ${
            feedback.type === 'success' 
              ? 'bg-green-900/90 border-green-500 text-green-100' 
              : 'bg-red-900/90 border-red-500 text-red-100'
          }`}>
             <div className={`p-1 rounded-full ${feedback.type === 'success' ? 'bg-green-500/20' : 'bg-red-500/20'}`}>
                {feedback.type === 'success' ? (
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" /></svg>
                ) : (
                  <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
                )}
             </div>
             <div>
               <h4 className="font-semibold">{feedback.type === 'success' ? 'Success' : 'Error'}</h4>
               <p className="text-sm opacity-90">{feedback.message}</p>
             </div>
             <button onClick={() => setFeedback(null)} className="ml-4 hover:opacity-75">
               <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
             </button>
          </div>
        </div>
      )}

      <LevelHelperModal
        isOpen={showLevelHelper}
        onClose={() => setShowLevelHelper(false)}
        className=""
        onNavigate={(path) => {
           const tabId = path.replace(/^\//, '');
           setActiveTab(tabId);
        }}
      />

      <GameLaunchDialog
        isOpen={showLaunchDialog}
        onClose={() => setShowLaunchDialog(false)}
        onLaunch={handleLaunchGame}
        saveName={character?.name}
      />
    </div>
  );
}

export default function ClientOnlyApp() {
  const [, setBackendReady] = useState(false);

  useEffect(() => {
    let timeoutId: NodeJS.Timeout;
    let isActive = true;
    let pollDelay = 500;
    const maxDelay = 5000;
    const maxTotalTime = 60000;
    const startTime = Date.now();

    const checkInitStatus = async () => {
      try {
        await DynamicAPI.initialize();
        const response = await DynamicAPI.fetch('/system/initialization/status/');
        if (!response.ok) return;

        const data = await response.json();

        if (isActive && data.stage === 'ready') {
          setBackendReady(true);
          return;
        }
      } catch (_) { // eslint-disable-line @typescript-eslint/no-unused-vars
      }

      if (Date.now() - startTime > maxTotalTime) {
        console.error('Backend initialization timeout after 60 seconds');
        return;
      }

      if (isActive) {
        timeoutId = setTimeout(() => {
          checkInitStatus();
          pollDelay = Math.min(pollDelay * 1.5, maxDelay);
        }, pollDelay);
      }
    };

    checkInitStatus();

    return () => {
      isActive = false;
      clearTimeout(timeoutId);
    };
  }, []);

  return (
    <IconCacheProvider>
      <CharacterProvider>
        <AppContent />
      </CharacterProvider>
    </IconCacheProvider>
  );
}