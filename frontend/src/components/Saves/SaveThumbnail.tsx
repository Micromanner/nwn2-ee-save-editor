
import { useState, useEffect } from 'react';
import { TauriAPI } from '@/lib/tauri-api';

interface SaveThumbnailProps {
  thumbnailPath?: string;
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

export function SaveThumbnail({ thumbnailPath, size = 'md', className = '' }: SaveThumbnailProps) {
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(!!thumbnailPath);
  const [error, setError] = useState(false);

  const sizeClasses = {
    sm: 'w-12 h-8',
    md: 'w-16 h-12',
    lg: 'w-32 h-24'
  };

  useEffect(() => {
    let isCancelled = false;

    const loadThumbnail = async () => {
      if (!thumbnailPath) return;

      setLoading(true);
      setError(false);

      try {
        const base64Data = await TauriAPI.getSaveThumbnail(thumbnailPath);

        if (!isCancelled) {
          const dataUrl = `data:image/webp;base64,${base64Data}`;
          setThumbnailUrl(dataUrl);
        }
      } catch {

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
    };
  }, [thumbnailPath]);

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
        <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-[rgb(var(--color-primary))]"></div>
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
    <img
      src={thumbnailUrl}
      alt="Save thumbnail"
      className={`${sizeClasses[size]} object-cover rounded ${className}`}
      onError={() => setError(true)}
    />
  );
}