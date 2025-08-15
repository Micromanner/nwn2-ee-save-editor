'use client';

import { HelpCircle, Loader2 } from 'lucide-react';
import { buildIconUrl, getIconCategory } from '@/lib/api/enhanced-icons';
import Image from 'next/image';
import { useState, useEffect } from 'react';
import { useIconCache } from '@/contexts/IconCacheContext';

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
  const checkCacheStatus = iconCache?.checkCacheStatus;
  const [loadError, setLoadError] = useState(false);
  const [retryCount, setRetryCount] = useState(0);
  const [showFallback, setShowFallback] = useState(false);

  // Show fallback immediately if cache is not ready
  useEffect(() => {
    if (useEnhanced && cacheReady === null && !iconUrl) {
      setShowFallback(true);
    }
  }, [useEnhanced, cacheReady, iconUrl]);

  if (!icon) {
    return (
      <div className={`${sizeMap[size].class} bg-[rgb(var(--color-surface-3))] rounded flex items-center justify-center ${className}`}>
        <HelpCircle className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
      </div>
    );
  }

  // If we've determined to show fallback, show it immediately
  if (showFallback) {
    return (
      <div 
        className={`${sizeMap[size].class} rounded flex items-center justify-center ${className} fallback-icon relative`}
        dangerouslySetInnerHTML={{ __html: getFallbackContent(icon) }}
      />
    );
  }
  
  // Build the icon URL
  let fullIconUrl = iconUrl;
  
  if (!fullIconUrl && icon) {
    // If no URL provided, build one using the enhanced system
    if (useEnhanced) {
      try {
        fullIconUrl = buildIconUrl(icon, { useEnhanced: true });
      } catch (error) {
        setShowFallback(true);
        return (
          <div 
            className={`${sizeMap[size].class} rounded flex items-center justify-center ${className} fallback-icon relative`}
            dangerouslySetInnerHTML={{ __html: getFallbackContent(icon) }}
          />
        );
      }
    } else {
      // Legacy mode - try to detect category from prefix
      const category = getIconCategory(icon);
      if (category) {
        try {
          fullIconUrl = buildIconUrl(icon, { useEnhanced: false, category });
        } catch (error) {
          setShowFallback(true);
          return (
            <div 
              className={`${sizeMap[size].class} rounded flex items-center justify-center ${className} fallback-icon relative`}
              dangerouslySetInnerHTML={{ __html: getFallbackContent(icon) }}
            />
          );
        }
      }
    }
  } else if (fullIconUrl && !fullIconUrl.startsWith('http')) {
    // Ensure relative URLs are converted to full URLs
    const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8000/api';
    const baseUrl = apiUrl.replace('/api', '');
    fullIconUrl = `${baseUrl}${fullIconUrl}`;
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
          onError={(e) => {
            // If image fails to load, show fallback immediately
            setShowFallback(true);
          }}
        />
      </div>
    );
  }
  
  // Fallback to styled placeholder
  return (
    <div 
      className={`${sizeMap[size].class} rounded flex items-center justify-center ${className} fallback-icon relative`}
      dangerouslySetInnerHTML={{ __html: getFallbackContent(icon) }}
    />
  );
}

function getFallbackContent(icon: string): string {
  const iconPrefix = icon.slice(0, 3).toLowerCase();
  
  const iconColor = iconPrefix === 'is_' ? 'bg-purple-900/20' : 
                    iconPrefix === 'ife_' ? 'bg-blue-900/20' : 
                    iconPrefix === 'isk_' ? 'bg-green-900/20' : 
                    'bg-[rgb(var(--color-surface-3))]';
  
  const iconSvg = iconPrefix === 'is_' 
    ? '<svg class="w-1/2 h-1/2 text-purple-400 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z" /></svg>'
    : iconPrefix === 'ife_' 
    ? '<svg class="w-1/2 h-1/2 text-blue-400 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.38-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z" /></svg>'
    : iconPrefix === 'isk_' 
    ? '<svg class="w-1/2 h-1/2 text-green-400 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" /></svg>'
    : '<svg class="w-1/2 h-1/2 text-[rgb(var(--color-text-muted))] opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>';
  
  return `<div class="${iconColor} w-full h-full flex items-center justify-center">${iconSvg}</div>`;
}