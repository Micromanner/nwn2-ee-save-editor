import { useEffect, useRef } from 'react';
import { useModelThumbnail } from '@/hooks/useModelThumbnail';
import type { TintColors } from '../ModelViewer/materials';
import type { ModelData } from '../ModelViewer/types';
import type { PreviewPose } from '@/lib/bindings';
import { T } from '../theme';

const TILE_SIZE = 96;

interface ThumbnailGalleryProps {
  values: number[];
  selected: number;
  onSelect: (value: number) => void;
  cacheKey: (value: number) => string;
  fetchModel: (value: number) => Promise<ModelData>;
  tints: TintColors;
  tileLabel: (value: number) => string;
  pose?: PreviewPose | null;
}

function Tile({ value, selected, onSelect, cacheKey, fetchModel, tints, label, pose }: {
  value: number;
  selected: boolean;
  onSelect: (value: number) => void;
  cacheKey: string;
  fetchModel: (value: number) => Promise<ModelData>;
  tints: TintColors;
  label: string;
  pose?: PreviewPose | null;
}) {
  const dataUrl = useModelThumbnail(cacheKey, () => fetchModel(value), tints, 'model', pose);
  const ref = useRef<HTMLDivElement>(null);

  // Bring the item's current variation into view once its image lands, so the
  // user can see what they already have without scanning the whole grid.
  useEffect(() => {
    if (selected && dataUrl) ref.current?.scrollIntoView({ block: 'nearest' });
    // Only on the first render of the selected tile's image.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dataUrl]);

  // A variant whose model does not resolve renders nothing at all rather than
  // offering a click that would blank the preview.
  if (dataUrl === '') return null;

  return (
    <div
      ref={ref}
      role="button"
      tabIndex={0}
      title={label}
      onClick={() => onSelect(value)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onSelect(value);
        }
      }}
      style={{
        width: TILE_SIZE,
        height: TILE_SIZE,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: 'pointer',
        border: `1px solid ${selected ? T.accent : T.borderLight}`,
        background: selected ? `${T.accent}18` : T.surfaceAlt,
        borderRadius: 3,
        userSelect: 'none',
        overflow: 'hidden',
      }}
    >
      {dataUrl === null ? (
        <div style={{ width: TILE_SIZE - 16, height: TILE_SIZE - 16, background: T.border, borderRadius: 2 }} />
      ) : (
        <img src={dataUrl} alt={label} width={TILE_SIZE - 4} height={TILE_SIZE - 4} draggable={false} />
      )}
    </div>
  );
}

// Grid of clickable model thumbnails; the selected value gets the accent
// border. Rendering/caching lives in useModelThumbnail - this component is
// presentational and reusable outside the item editor.
export function ThumbnailGallery({
  values,
  selected,
  onSelect,
  cacheKey,
  fetchModel,
  tints,
  tileLabel,
  pose,
}: ThumbnailGalleryProps) {
  if (values.length === 0) return null;

  return (
    <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
      {values.map((v) => (
        <Tile
          key={v}
          value={v}
          selected={v === selected}
          onSelect={onSelect}
          cacheKey={cacheKey(v)}
          fetchModel={fetchModel}
          tints={tints}
          label={tileLabel(v)}
          pose={pose}
        />
      ))}
    </div>
  );
}
