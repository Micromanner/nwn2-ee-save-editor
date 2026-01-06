'use client';

import { buildIconUrl } from '@/lib/api/enhanced-icons';
import Image from 'next/image';
import { useState, useEffect } from 'react';
import { useIconCache } from '@/contexts/IconCacheContext';
import DynamicAPI from '@/lib/utils/dynamicApi';

interface NWN2IconProps {
  icon: string;
  iconUrl?: string;
  alt?: string;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
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
  className = ''
}: NWN2IconProps) {
  const iconCache = useIconCache();
  const cacheReady = iconCache?.cacheReady;
  const [showFallback, setShowFallback] = useState(false);

  useEffect(() => {
    if (cacheReady === null && !iconUrl) {
      setShowFallback(true);
    }
  }, [cacheReady, iconUrl]);

  if (!icon || showFallback) {
    return null;
  }

  let fullIconUrl = iconUrl;

  if (!fullIconUrl && icon) {
    fullIconUrl = buildIconUrl(icon);
    if (!fullIconUrl) {
      return null;
    }
  } else if (fullIconUrl && !fullIconUrl.startsWith('http')) {
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
          onError={() => setShowFallback(true)}
        />
      </div>
    );
  }

  return null;
}

