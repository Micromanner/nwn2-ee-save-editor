import { useModelThumbnail } from '@/hooks/useModelThumbnail';
import type { TintColors } from '../ModelViewer/materials';
import type { ModelData } from '../ModelViewer/types';
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
}

function Tile({ value, selected, onSelect, cacheKey, fetchModel, tints, label }: {
  value: number;
  selected: boolean;
  onSelect: (value: number) => void;
  cacheKey: string;
  fetchModel: (value: number) => Promise<ModelData>;
  tints: TintColors;
  label: string;
}) {
  const dataUrl = useModelThumbnail(cacheKey, () => fetchModel(value), tints);

  return (
    <div
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
      ) : dataUrl === '' ? (
        <span className="t-sm t-semibold" style={{ color: T.textMuted }}>{value}</span>
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
        />
      ))}
    </div>
  );
}
