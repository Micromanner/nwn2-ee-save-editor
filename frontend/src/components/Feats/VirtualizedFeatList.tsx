'use client';

import { useMemo } from 'react';
import { FixedSizeList } from 'react-window';
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

interface VirtualizedFeatListProps {
  feats: FeatInfo[];
  isActive?: boolean;
  height: number; // Container height for virtualization
  onDetails: (feat: FeatInfo) => void;
  onAdd: (featId: number) => void;
  onRemove: (featId: number) => void;
}

// Row component for virtualized list
interface RowProps {
  index: number;
  style: React.CSSProperties;
  data: {
    feats: FeatInfo[];
    isActive?: boolean;
    onDetails: (feat: FeatInfo) => void;
    onAdd: (featId: number) => void;
    onRemove: (featId: number) => void;
  };
}

const Row = ({ index, style, data }: RowProps) => {
  const { feats, isActive, onDetails, onAdd, onRemove } = data;
  const feat = feats[index];

  if (!feat) return null;

  return (
    <div style={style} className="px-4">
      <FeatCard
        key={feat.id}
        feat={feat}
        isActive={isActive}
        viewMode="list"
        onDetails={onDetails}
        onAdd={onAdd}
        onRemove={onRemove}
      />
    </div>
  );
};

export default function VirtualizedFeatList({
  feats,
  isActive = false,
  height,
  onDetails,
  onAdd,
  onRemove
}: VirtualizedFeatListProps) {
  const itemData = useMemo(() => ({
    feats,
    isActive,
    onDetails,
    onAdd,
    onRemove
  }), [feats, isActive, onDetails, onAdd, onRemove]);

  if (feats.length === 0) {
    return null;
  }

  return (
    <FixedSizeList
      height={height}
      width="100%"
      itemCount={feats.length}
      itemSize={48} // Fixed height for list items (matches current list view)
      itemData={itemData}
      overscanCount={5} // Render 5 extra items above/below viewport  
      className="virtualized-list"
    >
      {Row}
    </FixedSizeList>
  );
}