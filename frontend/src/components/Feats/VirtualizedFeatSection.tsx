'use client';

import { useMemo } from 'react';
import VirtualizedFeatGrid from './VirtualizedFeatGrid';
import VirtualizedFeatList from './VirtualizedFeatList';

interface FeatInfo {
  id: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
  description?: string;
  icon?: string;
  prerequisites?: {
    abilities: Record<string, number>;
    feats: number[];
    class: number;
    level: number;
    bab: number;
    spell_level: number;
  };
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
}

interface VirtualizedFeatSectionProps {
  feats: FeatInfo[];
  isActive?: boolean;
  viewMode: 'grid' | 'list';
  maxHeight?: number; // Maximum height for virtualization container
  onDetails: (feat: FeatInfo) => void;
  onAdd: (featId: number) => void;
  onRemove: (featId: number) => void;
  validationCache?: Record<number, {
    can_take: boolean;
    reason: string;
    has_feat: boolean;
    missing_requirements: string[];
  }>;
  validatingFeatId?: number | null;
  onValidate?: (featId: number) => void;
}

export default function VirtualizedFeatSection({
  feats,
  isActive = false,
  viewMode,
  maxHeight = 400, // Default max height
  onDetails,
  onAdd,
  onRemove,
  validationCache = {},
  validatingFeatId = null,
  onValidate
}: VirtualizedFeatSectionProps) {
  
  // Calculate optimal height for virtualization
  const virtualizedHeight = useMemo(() => {
    if (feats.length === 0) return 0;
    
    if (viewMode === 'list') {
      // List mode: 48px per item
      const totalHeight = feats.length * 48;
      return Math.min(totalHeight, maxHeight);
    } else {
      // Grid mode: estimate rows and height
      // Assume 3 columns max, so divide by 3 and round up for row count
      const estimatedRows = Math.ceil(feats.length / 3);
      const totalHeight = estimatedRows * 132; // 120px card + 12px gap
      return Math.min(totalHeight, maxHeight);
    }
  }, [feats.length, viewMode, maxHeight]);

  // Don't render if no feats
  if (feats.length === 0) {
    return null;
  }

  // Use non-virtualized rendering for small lists (< 20 items) to avoid overhead
  if (feats.length < 20) {
    return (
      <div className={viewMode === 'grid' ? 
        'grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-3' : 
        'space-y-2'
      }>
        {/* Fallback to original FeatCard rendering for small lists */}
        {/* This will be imported from the parent component */}
        {/* For now, let the parent handle small lists */}
      </div>
    );
  }

  return (
    <div className="virtualized-feat-section" style={{ height: virtualizedHeight }}>
      {viewMode === 'grid' ? (
        <VirtualizedFeatGrid
          feats={feats}
          isActive={isActive}
          height={virtualizedHeight}
          onDetails={onDetails}
          onAdd={onAdd}
          onRemove={onRemove}
          validationCache={validationCache}
          validatingFeatId={validatingFeatId}
          onValidate={onValidate}
        />
      ) : (
        <VirtualizedFeatList
          feats={feats}
          isActive={isActive}
          height={virtualizedHeight}
          onDetails={onDetails}
          onAdd={onAdd}
          onRemove={onRemove}
          validationCache={validationCache}
          validatingFeatId={validatingFeatId}
          onValidate={onValidate}
        />
      )}
    </div>
  );
}