'use client';

import React, { createContext, useContext, useEffect, useState } from 'react';
import { TauriAPI } from '@/lib/tauri-api';

interface TauriContextType {
  isAvailable: boolean;
  isLoading: boolean;
  api: typeof TauriAPI | null;
}

const TauriContext = createContext<TauriContextType>({
  isAvailable: false,
  isLoading: true,
  api: null,
});

export const useTauri = () => {
  const context = useContext(TauriContext);
  if (!context) {
    throw new Error('useTauri must be used within a TauriProvider');
  }
  return context;
};

interface TauriProviderProps {
  children: React.ReactNode;
}

export function TauriProvider({ children }: TauriProviderProps) {
  console.log('🏗️ TauriProvider: Component rendering/mounting');
  console.log('🏗️ TauriProvider: Window exists at render:', typeof window !== 'undefined');
  console.log('🏗️ TauriProvider: User agent at render:', typeof window !== 'undefined' ? window.navigator.userAgent : 'N/A');
  
  const [isAvailable, setIsAvailable] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  
  useEffect(() => {
    console.log('🔧 TauriProvider: useEffect started');
    console.log('🔧 TauriProvider: Window object exists:', typeof window !== 'undefined');
    
    if (typeof window !== 'undefined') {
      console.log('🔧 TauriProvider: User agent:', window.navigator.userAgent);
      console.log('🔧 TauriProvider: Window keys count:', Object.keys(window).length);
      console.log('🔧 TauriProvider: First 10 window keys:', Object.keys(window).slice(0, 10));
    }
    
    const checkTauriAvailability = async () => {
      const windowExists = typeof window !== 'undefined';
      
      // Modern Tauri 2.x detection - try to import invoke function
      let tauriExists = false;
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        // Test if we can actually call invoke (this will only work in Tauri context)
        await invoke('check_django_health');
        tauriExists = true;
        console.log('✅ TauriProvider: Tauri 2.x context detected via API import!');
      } catch (error) {
        // Fallback: check for legacy __TAURI__ object
        tauriExists = windowExists && '__TAURI__' in window;
        console.log('🔧 TauriProvider: Modern Tauri detection failed, checking legacy __TAURI__:', tauriExists);
      }
      
      console.log('🔧 TauriProvider: Check - Window exists:', windowExists);
      console.log('🔧 TauriProvider: Check - Tauri available:', tauriExists);
      
      if (windowExists && tauriExists) {
        console.log('✅ TauriProvider: Tauri context successfully detected!');
        console.log('✅ TauriProvider: Desktop authentication handled automatically by backend middleware');
        setIsAvailable(true);
        setIsLoading(false);
        return true;
      }
      
      console.log('❌ TauriProvider: Tauri context not found in this check');
      return false;
    };

    const performInitialCheck = async () => {
      if (await checkTauriAvailability()) {
        return;
      }

      console.log('⏳ TauriProvider: Initial check failed. Starting polling every 100ms...');
      let pollCount = 0;
      const intervalId = setInterval(async () => {
        pollCount++;
        console.log(`🔄 TauriProvider: Poll attempt ${pollCount}/30`);
        
        if (await checkTauriAvailability()) {
          console.log('✅ TauriProvider: Found Tauri context during polling!');
          clearInterval(intervalId);
        }
      }, 100);

      const timeoutId = setTimeout(() => {
        console.log('⏰ TauriProvider: 3 second timeout reached');
        clearInterval(intervalId);
        if (!isAvailable) {
          console.error('❌ TauriProvider: Failed to detect Tauri context after 3 seconds. Assuming web mode.');
          console.log('🔧 TauriProvider: Final window check - keys containing TAURI:', 
            typeof window !== 'undefined' ? Object.keys(window).filter(k => k.toLowerCase().includes('tauri')) : 'N/A');
          setIsLoading(false);
        }
      }, 3000);

      return () => {
        console.log('🧹 TauriProvider: Cleanup - clearing intervals and timeouts');
        clearInterval(intervalId);
        clearTimeout(timeoutId);
      };
    };

    performInitialCheck();
  }, [isAvailable]);

  const value: TauriContextType = {
    isAvailable,
    isLoading,
    api: isAvailable ? TauriAPI : null,
  };

  return (
    <TauriContext.Provider value={value}>
      {children}
    </TauriContext.Provider>
  );
}