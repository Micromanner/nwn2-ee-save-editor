'use client';

import { useEffect, useCallback } from 'react';
import { buildIconUrl } from '@/lib/api/enhanced-icons';
import { useIconCache } from '@/contexts/IconCacheContext';

interface PreloadOptions {
  enabled?: boolean;
  batchSize?: number;
  delay?: number;
}

export function useIconPreloader(icons: string[], options: PreloadOptions = {}) {
  const { enabled = true, batchSize = 10, delay = 100 } = options;
  const iconCache = useIconCache();
  const cacheReady = iconCache?.cacheReady;
  const preloadedIcons = iconCache?.preloadedIcons || new Set();
  const addPreloadedIcon = iconCache?.addPreloadedIcon;

  const preloadIcon = useCallback((iconName: string): Promise<void> => {
    return new Promise((resolve) => {
      if (!iconName || preloadedIcons.has(iconName)) {
        resolve();
        return;
      }

      const img = new Image();
      const iconUrl = buildIconUrl(iconName, { useEnhanced: true });
      
      img.onload = () => {
        if (addPreloadedIcon) {
          addPreloadedIcon(iconName);
        }
        resolve();
      };
      
      img.onerror = () => {
        // Still mark as "processed" to avoid retrying
        if (addPreloadedIcon) {
          addPreloadedIcon(iconName);
        }
        resolve();
      };
      
      img.src = iconUrl;
    });
  }, [preloadedIcons, addPreloadedIcon]);

  const preloadBatch = useCallback(async (iconBatch: string[]) => {
    const promises = iconBatch.map(icon => preloadIcon(icon));
    await Promise.all(promises);
  }, [preloadIcon]);

  useEffect(() => {
    if (!enabled || !cacheReady || icons.length === 0) {
      return;
    }

    // Filter out already preloaded icons
    const iconsToPreload = icons.filter(icon => !preloadedIcons.has(icon));
    
    if (iconsToPreload.length === 0) {
      return;
    }

    let cancelled = false;

    const preloadInBatches = async () => {
      for (let i = 0; i < iconsToPreload.length; i += batchSize) {
        if (cancelled) break;
        
        const batch = iconsToPreload.slice(i, i + batchSize);
        await preloadBatch(batch);
        
        // Add delay between batches to avoid overwhelming the server
        if (i + batchSize < iconsToPreload.length && delay > 0) {
          await new Promise(resolve => setTimeout(resolve, delay));
        }
      }
    };

    preloadInBatches().catch(console.error);

    return () => {
      cancelled = true;
    };
  }, [icons, enabled, cacheReady, preloadedIcons, batchSize, delay, preloadBatch]);

  return {
    preloadedCount: icons.filter(icon => preloadedIcons.has(icon)).length,
    totalCount: icons.length,
    isPreloading: cacheReady && icons.some(icon => !preloadedIcons.has(icon)),
  };
}