'use client';

import { useState, useEffect } from 'react';
import { TauriAPI } from '@/lib/tauri-api';

interface SaveThumbnailProps {
  thumbnailPath?: string;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function SaveThumbnail({ thumbnailPath, size = 'md', className = '' }: SaveThumbnailProps) {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);

  const sizeClasses = {
    sm: 'w-12 h-8',
    md: 'w-16 h-12',
    lg: 'w-24 h-18'
  };

  useEffect(() => {
    let isCancelled = false;

    const loadThumbnail = async () => {
      if (!thumbnailPath) return;

      setLoading(true);
      setError(false);

      try {
        console.log('🖼️ Loading thumbnail from:', thumbnailPath);
        const base64Data = await TauriAPI.getSaveThumbnail(thumbnailPath);
        console.log('📦 Received base64 data:', base64Data.length, 'characters');
        
        if (!isCancelled) {
          // Create data URL from base64 string (now WebP format)
          const dataUrl = `data:image/webp;base64,${base64Data}`;
          console.log('🔗 Created WebP data URL (first 100 chars):', dataUrl.substring(0, 100));
          setThumbnailUrl(dataUrl);
        }
      } catch (err) {
        console.error('❌ Failed to load thumbnail:', err);
        if (!isCancelled) {
          setError(true);
        }
      } finally {
        if (!isCancelled) {
          setLoading(false);
        }
      }
    };

    loadThumbnail();

    return () => {
      isCancelled = true;
      if (thumbnailUrl) {
        URL.revokeObjectURL(thumbnailUrl);
      }
    };
  }, [thumbnailPath, thumbnailUrl]);

  // Cleanup URL on unmount
  useEffect(() => {
    return () => {
      if (thumbnailUrl) {
        URL.revokeObjectURL(thumbnailUrl);
      }
    };
  }, [thumbnailUrl]);

  if (!thumbnailPath) {
    return (
      <div className={`${sizeClasses[size]} bg-surface-2 rounded flex items-center justify-center ${className}`}>
        <div className="text-xs text-text-muted">No preview</div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className={`${sizeClasses[size]} bg-surface-2 rounded flex items-center justify-center ${className}`}>
        <div className="text-xs text-text-muted">Loading...</div>
      </div>
    );
  }

  if (error || !thumbnailUrl) {
    return (
      <div className={`${sizeClasses[size]} bg-surface-2 rounded flex items-center justify-center ${className}`}>
        <div className="text-xs text-text-muted">Error</div>
      </div>
    );
  }

  return (
    // eslint-disable-next-line @next/next/no-img-element
    <img
      src={thumbnailUrl}
      alt="Save thumbnail"
      className={`${sizeClasses[size]} object-cover rounded ${className}`}
      onError={() => setError(true)}
    />
  );
}