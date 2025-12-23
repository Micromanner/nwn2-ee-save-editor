'use client';

console.log('📦 ClientOnlyApp: File loaded/parsed');

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useTauri } from '@/providers/TauriProvider';
import DynamicAPI from '@/lib/utils/dynamicApi';
import CustomTitleBar from '@/components/ui/CustomTitleBar';
import Sidebar from '@/components/ui/Sidebar';
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
import SaveFileSelectorWrapper from '@/components/Saves/SaveFileSelectorWrapper';
import TauriInitializer from '@/components/TauriInitializer';
import { CharacterProvider, useCharacterContext } from '@/contexts/CharacterContext';
import { IconCacheProvider } from '@/contexts/IconCacheContext';
import Dashboard from '@/components/Dashboard';
import EditorHeader from '@/components/EditorHeader';
import { CharacterAPI } from '@/services/characterApi';
import { Button } from '@/components/ui/Button';

// Inner component that uses the character context
function AppContent() {
  const t = useTranslations();
  const { isAvailable, isLoading, api } = useTauri();
  const { character, isLoading: characterLoading, loadSubsystem } = useCharacterContext();
  const { clearCharacter, characterId, importCharacter } = useCharacterContext();

  const [activeTab, setActiveTab] = useState('overview');
  // New viewMode state to decouple character presence from UI view
  const [viewMode, setViewMode] = useState<'dashboard' | 'editor'>('dashboard');

  const [currentCompanion, setCurrentCompanion] = useState<{
    name: string;
    portrait?: string;
    isCompanion: boolean;
  } | null>(null);
  const [appReady, setAppReady] = useState(false);
  const [initProgress, setInitProgress] = useState({
    step: 'initializing',
    progress: 0,
    message: 'Starting up...'
  });
  const [backendReady, setBackendReady] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const [showLoadingOverlay, setShowLoadingOverlay] = useState(false);

  // Sync viewMode when character ID changes (new load)
  useEffect(() => {
    if (character?.id) {
       setViewMode('editor');
    }
  }, [character?.id]);

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
    // Clear companion selection to show main character
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
      // Optional: Add toast success
      console.log('Character saved successfully');
    } catch (error) {
       console.error('Failed to save character', error);
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

  // Custom tab change handler that fetches fresh data
  const handleTabChange = async (tabId: string) => {
    // Intercept settings tab to show the full-page settings overlay


    setActiveTab(tabId);
    
    // Fetch fresh data for subsystem-related tabs
    if (character?.id) {
      try {
        switch (tabId) {
          case 'skills':
            console.log('Fetching fresh skills data...');
            await loadSubsystem('skills');
            break;
          case 'classes':
            console.log('Fetching fresh classes data...');
            await loadSubsystem('classes');
            break;
          case 'abilityScores':
            console.log('Fetching fresh ability scores data...');
            await loadSubsystem('abilityScores');
            break;
          case 'feats':
            console.log('Fetching fresh feats data...');
            await loadSubsystem('feats');
            break;
          case 'combat':
            console.log('Fetching fresh combat data...');
            await loadSubsystem('combat');
            break;
          case 'saves':
            console.log('Fetching fresh saves data...');
            await loadSubsystem('saves');
            break;
          case 'spells':
            console.log('Fetching fresh spells data...');
            await loadSubsystem('spells');
            break;
          case 'inventory':
            console.log('Fetching fresh inventory data...');
            await loadSubsystem('inventory');
            break;
          case 'gameState':
            console.log('Fetching fresh game state data...');
            break;
          case 'overview':
            console.log('Fetching fresh overview data...');
            await loadSubsystem('abilityScores');
            await loadSubsystem('combat');
            await loadSubsystem('skills');
            await loadSubsystem('feats');
            await loadSubsystem('saves');
            await loadSubsystem('classes');
            break;
          // For other tabs like 'appearance', etc., no specific fetch needed
          default:
            break;
        }
      } catch (err) {
        console.error(`Failed to fetch data for ${tabId}:`, err);
      }
    }
  };


  // Poll Django initialization status with exponential backoff
  useEffect(() => {
    console.log('🔍 useEffect: Starting, api=', !!api);
    console.log('🔍 useEffect: NEXT_PUBLIC_API_URL =', process.env.NEXT_PUBLIC_API_URL);
    if (!api) return;
    
    let timeoutId: NodeJS.Timeout;
    let isActive = true;
    let pollDelay = 500; // Start at 500ms
    const maxDelay = 5000; // Max 5 seconds between polls
    const maxTotalTime = 60000; // 60 second total timeout
    const startTime = Date.now();
    
    const checkInitStatus = async () => {
      try {
        console.log('🔧 ClientOnlyApp: NEXT_PUBLIC_API_URL =', process.env.NEXT_PUBLIC_API_URL);
        // Initialize DynamicAPI first to get the dynamic port from Tauri
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
      
      // Schedule next poll with exponential backoff
      if (isActive) {
        timeoutId = setTimeout(() => {
          checkInitStatus();
          // Increase delay for next poll (exponential backoff)
          pollDelay = Math.min(pollDelay * 1.5, maxDelay);
        }, pollDelay);
      }
    };
    
    // Start polling
    console.log('🔍 useEffect: About to start first checkInitStatus...');
    checkInitStatus();
    
    return () => {
      isActive = false;
      clearTimeout(timeoutId);
    };
  }, [api]);
  
  // Track overall app readiness
  useEffect(() => {
    if (isAvailable && api && backendReady) {
      // Small delay for smooth transition
      const timer = setTimeout(() => {
        setAppReady(true);
      }, 500);
      return () => clearTimeout(timer);
    }
  }, [isAvailable, api, backendReady]);

  // Show full-page loading screen until app is ready
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
            
            {/* Progress bar */}
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
      {/* Initialize Tauri immediately when app loads */}
      <TauriInitializer />
      
      {/* Custom Title Bar - Fixed at top */}
      <div className="flex-shrink-0">
        <CustomTitleBar />
      </div>
      
      {/* Main Content Area */}
      <div className="flex-1 flex overflow-hidden">
        {(() => {
          // --- BRANCH 1: EDITOR MODE ---
          // Condition: We have a character AND we are explicitly in editor mode.
          // This takes precedence so that if we are editing, we stay in the editor structure.
          if (viewMode === 'editor' && character) {
             return (
              <div className="flex flex-col w-full h-full overflow-hidden">
                 {/* Editor Header - Full Width */}
                 <EditorHeader 
                    characterName={character.name}
                    saveName="Save File" // TODO: Get actual save name
                    onBack={handleEditorBack}
                    onImport={() => console.log('Import clicked')}
                    onExport={() => console.log('Export clicked')}
                    onSave={handleSaveCharacter}
                  />
                  
                 {/* Content Area - Sidebar + Main */}
                 <div className="flex-1 flex overflow-hidden">
                    {/* Left Sidebar */}
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
                            {/* ... Content Renderers ... */}
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
                              <ClassAndLevelsEditor />
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

          // --- BRANCH 2: DASHBOARD SETTINGS OVERLAY ---
          // Condition: NOT in editor mode, and showSettings is strictly true.
          if (showSettings) {
            return (
              <div className="flex flex-col h-full w-full bg-[rgb(var(--color-background))]">
                {/* Settings Header - Overlay Style */}
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
                
                {/* Settings Content - Added proper padding */}
                <div className="flex-1 overflow-y-auto p-6 md:p-8 max-w-5xl mx-auto w-full">
                   <SettingsPage />
                </div>
              </div>
            );
          }
          
          // --- BRANCH 3: DASHBOARD (Default) ---
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

      {/* Loading Overlay Transition */}
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
    </div>
  );
}

// Main export component that provides the context
export default function ClientOnlyApp() {
  console.log('🚀 ClientOnlyApp: Component rendering/mounting');
  const [backendReady, setBackendReady] = useState(false);

  // Backend readiness detection with exponential backoff (same as AppContent)
  useEffect(() => {
    let timeoutId: NodeJS.Timeout;
    let isActive = true;
    let pollDelay = 500; // Start at 500ms
    const maxDelay = 5000; // Max 5 seconds between polls
    const maxTotalTime = 60000; // 60 second total timeout
    const startTime = Date.now();
    
    const checkInitStatus = async () => {
      try {
        // Initialize DynamicAPI first to get the dynamic port from Tauri
        await DynamicAPI.initialize();
        const response = await DynamicAPI.fetch('/system/initialization/status/');
        if (!response.ok) return;
        
        const data = await response.json();
        
        if (isActive && data.stage === 'ready') {
          setBackendReady(true);
          return; // Stop polling
        }
      } catch (_) { // eslint-disable-line @typescript-eslint/no-unused-vars
        // Django might not be ready yet
      }
      
      // Check total timeout
      if (Date.now() - startTime > maxTotalTime) {
        console.error('Backend initialization timeout after 60 seconds');
        return;
      }
      
      // Schedule next poll with exponential backoff
      if (isActive) {
        timeoutId = setTimeout(() => {
          checkInitStatus();
          // Increase delay for next poll (exponential backoff)
          pollDelay = Math.min(pollDelay * 1.5, maxDelay);
        }, pollDelay);
      }
    };
    
    // Start polling
    checkInitStatus();
    
    return () => {
      isActive = false;
      clearTimeout(timeoutId);
    };
  }, []);

  return (
    <IconCacheProvider backendReady={backendReady}>
      <CharacterProvider>
        <AppContent />
      </CharacterProvider>
    </IconCacheProvider>
  );
}