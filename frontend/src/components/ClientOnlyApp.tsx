'use client';

console.log('📦 ClientOnlyApp: File loaded/parsed');

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useTauri } from '@/providers/TauriProvider';
import { tauriCompatibleFetch } from '@/lib/utils/tauriFetch';
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
import SettingsPage from '@/app/settings/page';
import { Button } from '@/components/ui/Button';
import SaveFileSelectorWrapper from '@/components/Saves/SaveFileSelectorWrapper';
import TauriInitializer from '@/components/TauriInitializer';
import { CharacterProvider, useCharacterContext } from '@/contexts/CharacterContext';
import { IconCacheProvider } from '@/contexts/IconCacheContext';

// Inner component that uses the character context
function AppContent() {
  const t = useTranslations();
  const { isAvailable, isLoading, api } = useTauri();
  const { character, isLoading: characterLoading, loadSubsystem } = useCharacterContext();
  const [activeTab, setActiveTab] = useState('overview');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
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

  // Custom tab change handler that fetches fresh data
  const handleTabChange = async (tabId: string) => {
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
          case 'overview':
            console.log('Fetching fresh overview data...');
            await loadSubsystem('abilityScores');
            await loadSubsystem('combat');
            await loadSubsystem('skills');
            await loadSubsystem('feats');
            await loadSubsystem('saves');
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
      
      {/* Main Content Area - Takes remaining height */}
      <div className="flex-1 flex overflow-hidden">
      {/* Sidebar - Full Height */}
      <Sidebar 
        activeTab={activeTab} 
        onTabChange={handleTabChange}
        isCollapsed={sidebarCollapsed}
        onCollapsedChange={setSidebarCollapsed}
        currentCharacter={currentCharacter}
        onBackToMain={handleBackToMain}
        isLoading={characterLoading}
      />
      
      <div className="flex-1 flex overflow-hidden">
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
            
            {activeTab === 'settings' && <SettingsPage />}
          </div>
        </main>
        
        {/* Right Sidebar - File Operations */}
        <div className="w-64 bg-[rgb(var(--color-surface-1))] border-l border-[rgb(var(--color-surface-border)/0.6)] flex flex-col shadow-elevation-2">
          <div className="p-4 space-y-3">
            <h3 className="text-base font-bold text-text-primary mb-3 text-center">
              Save Files
            </h3>
            <div className="h-px mb-4 separator-primary"></div>
            
            {/* Save File Selector - Always shown */}
            <SaveFileSelectorWrapper />
            
            {/* Quick Actions */}
            <div className="mt-10 space-y-3">
              <h3 className="text-base font-bold text-text-primary mb-3 text-center">
                Character Actions
              </h3>
              <div className="h-px separator-primary"></div>
              <div className="flex gap-2 mt-3">
                <Button variant="outline" className="flex-1 text-sm">
                  Import
                </Button>
                <Button variant="outline" className="flex-1 text-sm">
                  Export
                </Button>
              </div>
            </div>

            {/* Save Management - Always visible */}
            <div className="mt-6 space-y-3">
              <h3 className="text-base font-bold text-text-primary mb-3 text-center">
                Save Management
              </h3>
              <div className="h-px separator-primary"></div>
              <div className="mt-3">
                <Button
                  variant="outline"
                  className="w-full text-sm"
                  onClick={() => {
                    if ((window as any).__openBackups) {
                      (window as any).__openBackups();
                    }
                  }}
                >
                  Manage Backups
                </Button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
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