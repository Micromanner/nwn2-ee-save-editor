'use client';

import { useState, useEffect } from 'react';

interface CharacterPortraitProps {
  portrait?: string | null;
  customPortrait?: string | null;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  className?: string;
  fallbackIcon?: React.ReactNode;
}

const sizeMap = {
  sm: { width: 32, height: 32 },
  md: { width: 48, height: 48 },
  lg: { width: 64, height: 64 },
  xl: { width: 96, height: 96 },
};

export default function CharacterPortrait({ 
  portrait, 
  customPortrait, 
  size = 'md', 
  className = '',
  fallbackIcon
}: CharacterPortraitProps) {
  const [imageUrl, setImageUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  
  const dimensions = sizeMap[size];
  
  useEffect(() => {
    // Reset state when portrait changes
    setError(false);
    setImageUrl(null);
    
    // Determine which portrait to use (custom takes precedence)
    const portraitId = customPortrait || portrait;
    
    if (!portraitId) {
      return;
    }
    
    // Build the icon URL using the v2 endpoint
    // Portrait IDs in NWN2 are typically like "po_hu_f_99_" or custom names
    const iconPath = `portraits/${portraitId}`;
    const url = `http://localhost:8000/api/gamedata/icons/v2/${iconPath}/`;
    
    setLoading(true);
    setImageUrl(url);
  }, [portrait, customPortrait]);
  
  const handleError = () => {
    setError(true);
    setLoading(false);
    
    // Try alternative portrait path if first attempt fails
    const portraitId = customPortrait || portrait;
    if (portraitId && imageUrl && !imageUrl.includes('_s')) {
      // Try with _s suffix for small portrait version
      const alternativeUrl = imageUrl.replace(`/${portraitId}/`, `/${portraitId}_s/`);
      setImageUrl(alternativeUrl);
      setError(false); // Reset error to try again
    }
  };
  
  const handleLoad = () => {
    setLoading(false);
  };
  
  // Show fallback if no portrait or error
  if (!imageUrl || (error && !loading)) {
    return (
      <div 
        className={`flex items-center justify-center bg-gradient-to-br from-[rgb(var(--color-primary)/0.2)] to-[rgb(var(--color-primary)/0.1)] rounded-lg border border-[rgb(var(--color-primary)/0.3)] ${className}`}
        style={{ width: dimensions.width, height: dimensions.height }}
      >
        {fallbackIcon || (
          <svg 
            className={`text-[rgb(var(--color-primary))] ${size === 'sm' ? 'w-4 h-4' : size === 'md' ? 'w-6 h-6' : size === 'lg' ? 'w-8 h-8' : 'w-12 h-12'}`} 
            fill="none" 
            stroke="currentColor" 
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
        )}
      </div>
    );
  }
  
  return (
    <div 
      className={`relative overflow-hidden rounded-lg ${className}`}
      style={{ width: dimensions.width, height: dimensions.height }}
    >
      {loading && (
        <div className="absolute inset-0 flex items-center justify-center bg-[rgb(var(--color-surface-2))]">
          <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-[rgb(var(--color-primary))]"></div>
        </div>
      )}
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img
        src={imageUrl}
        alt="Character Portrait"
        width={dimensions.width}
        height={dimensions.height}
        className="w-full h-full object-cover"
        onError={handleError}
        onLoad={handleLoad}
        style={{ display: loading ? 'none' : 'block' }}
      />
    </div>
  );
}