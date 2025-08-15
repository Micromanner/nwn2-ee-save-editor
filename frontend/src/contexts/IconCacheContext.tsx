'use client';

import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import { fetchIconStats } from '@/lib/api/enhanced-icons';

interface IconCacheStats {
  initialized: boolean;
  initializing: boolean;
  statistics: {
    base_count: number;
    override_count: number;
    workshop_count: number;
    hak_count: number;
    module_count: number;
    total_count: number;
    total_size: number;
  };
  format: string;
  mimetype: string;
}

interface IconCacheContextValue {
  cacheStats: IconCacheStats | null;
  cacheReady: boolean | null;
  isInitializing: boolean;
  checkCacheStatus: () => Promise<void>;
  preloadedIcons: Set<string>;
  addPreloadedIcon: (iconName: string) => void;
}

const IconCacheContext = createContext<IconCacheContextValue | undefined>(undefined);

export function useIconCache() {
  const context = useContext(IconCacheContext);
  if (context === undefined) {
    throw new Error('useIconCache must be used within an IconCacheProvider');
  }
  return context;
}

interface IconCacheProviderProps {
  children: React.ReactNode;
  backendReady?: boolean;
}

export function IconCacheProvider({ children, backendReady = false }: IconCacheProviderProps) {
  const [cacheStats, setCacheStats] = useState<IconCacheStats | null>(null);
  const [cacheReady, setCacheReady] = useState<boolean | null>(null);
  const [isInitializing, setIsInitializing] = useState(false);
  const [preloadedIcons, setPreloadedIcons] = useState<Set<string>>(new Set());

  const checkCacheStatus = useCallback(async () => {
    try {
      const stats = await fetchIconStats();
      setCacheStats(stats);
      setCacheReady(stats.initialized);
      setIsInitializing(stats.initializing);
      
      // If cache is still initializing, keep checking
      if (!stats.initialized && stats.initializing) {
        setTimeout(checkCacheStatus, 3000);
      } else if (!stats.initialized && !stats.initializing) {
        // Cache failed to initialize, retry
        setTimeout(checkCacheStatus, 5000);
      }
    } catch (error) {
      console.error('Failed to fetch icon cache stats:', error);
      setCacheReady(false);
      setIsInitializing(false);
      // Retry on error
      setTimeout(checkCacheStatus, 5000);
    }
  }, []);

  const addPreloadedIcon = useCallback((iconName: string) => {
    setPreloadedIcons(prev => new Set([...prev, iconName]));
  }, []);

  // Removed automatic cache status check at startup
  // Cache will only be checked when explicitly requested

  const value: IconCacheContextValue = {
    cacheStats,
    cacheReady,
    isInitializing,
    checkCacheStatus,
    preloadedIcons,
    addPreloadedIcon,
  };

  return (
    <IconCacheContext.Provider value={value}>
      {children}
    </IconCacheContext.Provider>
  );
}