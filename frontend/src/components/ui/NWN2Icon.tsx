'use client';

import { buildIconUrl, getIconCategory } from '@/lib/api/enhanced-icons';
import Image from 'next/image';
import { useState, useEffect } from 'react';
import { useIconCache } from '@/contexts/IconCacheContext';
import DynamicAPI from '@/lib/utils/dynamicApi';

interface NWN2IconProps {
  icon: string;
  iconUrl?: string;  // URL to actual icon image (can be legacy or enhanced)
  alt?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
  useEnhanced?: boolean; // Use enhanced icon system
}

const sizeMap = {
  sm: { class: 'w-8 h-8', px: 32 },
  md: { class: 'w-10 h-10', px: 40 }, 
  lg: { class: 'w-12 h-12', px: 48 },
  xl: { class: 'w-14 h-14', px: 56 }
};


export default function NWN2Icon({ 
  icon, 
  iconUrl, 
  alt, 
  size = 'md', 
  className = '',
  useEnhanced = true
}: NWN2IconProps) {
  const iconCache = useIconCache();
  const cacheReady = iconCache?.cacheReady;
  const _checkCacheStatus = iconCache?.checkCacheStatus; // eslint-disable-line @typescript-eslint/no-unused-vars
  const [_loadError, _setLoadError] = useState(false); // eslint-disable-line @typescript-eslint/no-unused-vars
  const [showFallback, setShowFallback] = useState(false);

  // Show fallback immediately if cache is not ready
  useEffect(() => {
    if (useEnhanced && cacheReady === null && !iconUrl) {
      setShowFallback(true);
    }
  }, [useEnhanced, cacheReady, iconUrl]);

  if (!icon) {
    return null;
  }

  // If we've determined to show fallback, don't render anything
  if (showFallback) {
    return null;
  }
  
  // Build the icon URL
  let fullIconUrl = iconUrl;
  
  if (!fullIconUrl && icon) {
    // If no URL provided, build one using the enhanced system
    if (useEnhanced) {
      try {
        fullIconUrl = buildIconUrl(icon, { useEnhanced: true });
      } catch (_error) { // eslint-disable-line @typescript-eslint/no-unused-vars
        return null;
      }
    } else {
      // Legacy mode - try to detect category from prefix
      const category = getIconCategory(icon);
      if (category) {
        try {
          fullIconUrl = buildIconUrl(icon, { useEnhanced: false, category });
        } catch (_error) { // eslint-disable-line @typescript-eslint/no-unused-vars
          return null;
        }
      }
    }
  } else if (fullIconUrl && !fullIconUrl.startsWith('http')) {
    // Ensure relative URLs are converted to full URLs
    const cachedBase = DynamicAPI.getCachedBaseUrl();
    if (!cachedBase) return null;
    fullIconUrl = `${cachedBase}${fullIconUrl}`;
  }
  
  if (fullIconUrl) {
    const sizeConfig = sizeMap[size];
    return (
      <div className={`${sizeConfig.class} rounded ${className} icon-container`}>
        <Image 
          src={fullIconUrl} 
          alt={alt || icon}
          width={sizeConfig.px}
          height={sizeConfig.px}
          className="w-full h-full object-cover"
          onError={(_e) => { // eslint-disable-line @typescript-eslint/no-unused-vars
            // If image fails to load, don't render anything
            setShowFallback(true);
          }}
        />
      </div>
    );
  }
  
  // No icon available
  return null;
}

