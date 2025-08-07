'use client';

import { useMemo, useEffect, useState, useCallback } from 'react';
import { VariableSizeList } from 'react-window';
import FeatCard from './FeatCard';

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

interface VirtualizedFeatGridProps {
  feats: FeatInfo[];
  isActive?: boolean;
  height: number; // Container height for virtualization
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

// Hook to detect screen size and calculate columns
function useGridColumns() {
  const [columns, setColumns] = useState(1);

  useEffect(() => {
    const updateColumns = () => {
      const width = window.innerWidth;
      if (width >= 1280) { // xl breakpoint
        setColumns(3);
      } else if (width >= 1024) { // lg breakpoint  
        setColumns(2);
      } else {
        setColumns(1);
      }
    };

    updateColumns();
    window.addEventListener('resize', updateColumns);
    return () => window.removeEventListener('resize', updateColumns);
  }, []);

  return columns;
}

// Group feats into rows based on column count
function useGridRows(feats: FeatInfo[], columns: number) {
  return useMemo(() => {
    const rows: FeatInfo[][] = [];
    for (let i = 0; i < feats.length; i += columns) {
      rows.push(feats.slice(i, i + columns));
    }
    return rows;
  }, [feats, columns]);
}

// Row component for virtualized list
interface RowProps {
  index: number;
  style: React.CSSProperties;
  data: {
    rows: FeatInfo[][];
    columns: number;
    isActive?: boolean;
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
  };
}

const Row = ({ index, style, data }: RowProps) => {
  const { rows, columns, isActive, onDetails, onAdd, onRemove, validationCache, validatingFeatId, onValidate } = data;
  const rowFeats = rows[index];

  if (!rowFeats) return null;

  return (
    <div style={style} className="px-4">
      <div className={`grid gap-3 ${
        columns === 3 ? 'grid-cols-3' : 
        columns === 2 ? 'grid-cols-2' : 
        'grid-cols-1'
      }`}>
        {rowFeats.map((feat, colIndex) => (
          <FeatCard
            key={`${feat.id}-${index}-${colIndex}`}
            feat={feat}
            isActive={isActive}
            viewMode="grid"
            onDetails={onDetails}
            onAdd={onAdd}
            onRemove={onRemove}
            validationState={validationCache?.[feat.id]}
            isValidating={validatingFeatId === feat.id}
            onValidate={onValidate}
          />
        ))}
        {/* Fill empty cells in partial rows */}
        {Array.from({ length: columns - rowFeats.length }).map((_, emptyIndex) => (
          <div key={`empty-${index}-${emptyIndex}`} />
        ))}
      </div>
    </div>
  );
};

export default function VirtualizedFeatGrid({
  feats,
  isActive = false,
  height,
  onDetails,
  onAdd,
  onRemove,
  validationCache,
  validatingFeatId,
  onValidate
}: VirtualizedFeatGridProps) {
  const columns = useGridColumns();
  const rows = useGridRows(feats, columns);

  // Calculate row height - approximately 120px per row based on FeatCard size
  const getItemSize = useCallback(() => {
    return 120 + 12; // Card height + gap
  }, []);

  const itemData = useMemo(() => ({
    rows,
    columns,
    isActive,
    onDetails,
    onAdd,
    onRemove,
    validationCache,
    validatingFeatId,
    onValidate
  }), [rows, columns, isActive, onDetails, onAdd, onRemove, validationCache, validatingFeatId, onValidate]);

  if (feats.length === 0) {
    return null;
  }

  return (
    <VariableSizeList
      height={height}
      width="100%"
      itemCount={rows.length}
      itemSize={getItemSize}
      itemData={itemData}
      overscanCount={2} // Render 2 extra rows above/below viewport
      className="virtualized-grid"
    >
      {Row}
    </VariableSizeList>
  );
}