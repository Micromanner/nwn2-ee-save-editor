'use client';

import React, { createContext, useContext, useState, useCallback, ReactNode, useEffect } from 'react';
import { useTauri } from '@/providers/TauriProvider';

interface GameLaunchSettings {
  nwn2_installation_path?: string;
  auto_close_on_launch: boolean;
  show_launch_dialog: boolean;
}

interface SettingsContextState {
  gameSettings: GameLaunchSettings;
  isLoading: boolean;
  error: string | null;
  
  // Actions
  updateGameSettings: (settings: Partial<GameLaunchSettings>) => Promise<void>;
  detectGamePath: () => Promise<string | null>;
  resetSettings: () => void;
}

// Default settings
const DEFAULT_GAME_SETTINGS: GameLaunchSettings = {
  auto_close_on_launch: false,
  show_launch_dialog: true,
};

// Create context
const SettingsContext = createContext<SettingsContextState | undefined>(undefined);

// Settings storage key
const SETTINGS_STORAGE_KEY = 'nwn2_editor_settings';

// Provider component
export function SettingsProvider({ children }: { children: ReactNode }) {
  const { api } = useTauri();
  const [gameSettings, setGameSettings] = useState<GameLaunchSettings>(DEFAULT_GAME_SETTINGS);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load settings from localStorage on mount
  useEffect(() => {
    try {
      const savedSettings = localStorage.getItem(SETTINGS_STORAGE_KEY);
      if (savedSettings) {
        const parsed = JSON.parse(savedSettings);
        setGameSettings({ ...DEFAULT_GAME_SETTINGS, ...parsed.gameSettings });
      }
    } catch (err) {
      console.error('Failed to load settings from localStorage:', err);
    }
  }, []);

  // Save settings to localStorage
  const saveSettings = useCallback((settings: GameLaunchSettings) => {
    try {
      const settingsData = { gameSettings: settings };
      localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settingsData));
    } catch (err) {
      console.error('Failed to save settings to localStorage:', err);
    }
  }, []);

  // Update game settings
  const updateGameSettings = useCallback(async (newSettings: Partial<GameLaunchSettings>) => {
    const updatedSettings = { ...gameSettings, ...newSettings };
    setGameSettings(updatedSettings);
    saveSettings(updatedSettings);
  }, [gameSettings, saveSettings]);

  // Auto-detect game installation path
  const detectGamePath = useCallback(async (): Promise<string | null> => {
    if (!api) {
      console.warn('Cannot detect game path: Tauri API not available');
      return null;
    }

    setIsLoading(true);
    setError(null);

    try {
      const detectedPath = await api.detectNWN2Installation();
      
      if (detectedPath) {
        // Auto-save the detected path
        await updateGameSettings({ nwn2_installation_path: detectedPath });
        console.log('Auto-detected and saved NWN2 installation:', detectedPath);
      }
      
      return detectedPath;
    } catch (err) {
      console.error('Failed to detect game path:', err);
      setError(err instanceof Error ? err.message : 'Failed to detect game installation');
      return null;
    } finally {
      setIsLoading(false);
    }
  }, [api, updateGameSettings]);

  // Reset settings to defaults
  const resetSettings = useCallback(() => {
    setGameSettings(DEFAULT_GAME_SETTINGS);
    saveSettings(DEFAULT_GAME_SETTINGS);
    setError(null);
  }, [saveSettings]);

  // Auto-detect game path on first load if not set
  useEffect(() => {
    if (api && !gameSettings.nwn2_installation_path && !isLoading) {
      console.log('Auto-detecting NWN2 installation on first load...');
      detectGamePath().catch(err => {
        console.warn('Auto-detection failed:', err);
      });
    }
  }, [api, gameSettings.nwn2_installation_path, detectGamePath, isLoading]);

  const value: SettingsContextState = {
    gameSettings,
    isLoading,
    error,
    updateGameSettings,
    detectGamePath,
    resetSettings,
  };

  return (
    <SettingsContext.Provider value={value}>
      {children}
    </SettingsContext.Provider>
  );
}

// Hook to use settings context
export function useSettings() {
  const context = useContext(SettingsContext);
  if (context === undefined) {
    throw new Error('useSettings must be used within a SettingsProvider');
  }
  return context;
}