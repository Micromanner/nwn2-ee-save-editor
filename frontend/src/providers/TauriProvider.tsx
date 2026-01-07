
import React, { createContext, useContext, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
  const [isAvailable, setIsAvailable] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  
  useEffect(() => {
    const checkTauriAvailability = async () => {
      const windowExists = typeof window !== 'undefined';
      
      let tauriExists = false;
      try {
        await invoke('check_fastapi_health');
        tauriExists = true;
      } catch {
        tauriExists = windowExists && '__TAURI__' in window;
      }
      
      if (windowExists && tauriExists) {
        setIsAvailable(true);
        setIsLoading(false);
        return true;
      }
      
      return false;
    };

    const performInitialCheck = async () => {
      if (await checkTauriAvailability()) {
        return;
      }

      const intervalId = setInterval(async () => {
        if (await checkTauriAvailability()) {
          clearInterval(intervalId);
        }
      }, 100);

      const timeoutId = setTimeout(() => {
        clearInterval(intervalId);
        if (!isAvailable) {
          setIsLoading(false);
        }
      }, 3000);

      return () => {
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
