import { useState, useCallback, useEffect, useRef } from 'react';
import { Card, Elevation } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import type { ItemAppearance, ItemAppearanceOptions, TintChannels, TintChannel } from '@/lib/bindings';
import type { ModelData } from '../ModelViewer/types';
import { tintChannelsToColors } from '../ModelViewer/materials';
import { ItemViewer3D, swapItemTints } from '../Appearance/ItemViewer3D';
import { ColorPicker } from '../Appearance/ColorPicker';
import { ThumbnailGallery } from '../Appearance/ThumbnailGallery';
import { LoadingState } from '../shared';
import { T } from '../theme';

interface ItemAppearanceTabProps {
  appearance: ItemAppearance;
  baseItemId: number;
  options: ItemAppearanceOptions | null;
  // Whether the appearance tab is the selected one. Blueprint mounts hidden
  // tab panels too; galleries wait for the first activation so opening the
  // dialog doesn't kick off every thumbnail load.
  active: boolean;
  onChange: (appearance: ItemAppearance) => void;
}

function SectionHeader({ label }: { label: string }) {
  return (
    <div className="t-section" style={{ color: T.accent, marginBottom: 8, marginTop: 4 }}>
      {label}
    </div>
  );
}

function tintChannelsKey(t: TintChannels): string {
  const c = (ch: { r: number; g: number; b: number }) => `${ch.r},${ch.g},${ch.b}`;
  return `${c(t.channel1)}|${c(t.channel2)}|${c(t.channel3)}`;
}

const PART_LABEL_KEYS = ['appearance.part1', 'appearance.part2', 'appearance.part3'] as const;

export function ItemAppearanceTab({ appearance, baseItemId, options, active, onChange }: ItemAppearanceTabProps) {
  const t = useTranslations();
  const { characterId } = useCharacterContext();
  const [refreshKey, setRefreshKey] = useState(0);
  const [refreshPart, setRefreshPart] = useState<{ partIndex: number; key: number } | null>(null);
  const [activated, setActivated] = useState(active);
  useEffect(() => {
    if (active) setActivated(true);
  }, [active]);

  // Thumbnails render once with the tints active when the tab mounts; later
  // tint edits update only the live viewer (the parent remounts this tab per
  // edited item, so the freeze resets on item switch).
  const frozenTints = useRef(tintChannelsToColors(swapItemTints(appearance.tints))).current;
  const tintKey = useRef(tintChannelsKey(appearance.tints)).current;

  const updatePart = useCallback((idx: number, value: number) => {
    const newParts = [...appearance.model_parts];
    newParts[idx] = value;
    onChange({ ...appearance, model_parts: newParts as [number, number, number] });
    setRefreshPart({ partIndex: idx, key: Date.now() });
  }, [appearance, onChange]);

  const updateVariation = useCallback((value: number) => {
    onChange({ ...appearance, variation: value });
    setRefreshKey(k => k + 1);
  }, [appearance, onChange]);

  const updateTint = useCallback((channel: keyof TintChannels, color: TintChannel) => {
    onChange({
      ...appearance,
      tints: {
        ...appearance.tints,
        [channel]: color
      }
    });
  }, [appearance, onChange]);

  const partLists = options
    ? [options.available_part1, options.available_part2, options.available_part3]
    : [];
  const hasParts = partLists.some(list => list.length > 0);

  return (
    <div style={{ display: 'flex', gap: 20, height: '100%' }}>
      <div style={{ flex: '0 0 480px', height: '100%' }}>
        <ItemViewer3D
          appearance={appearance}
          baseItemId={baseItemId}
          refreshKey={refreshKey}
          refreshPart={refreshPart}
        />
      </div>

      {/* Horizontal padding keeps the cards' elevation shadows from being
          clipped by this scroll container. */}
      <div style={{ flex: 1, height: '100%', display: 'flex', flexDirection: 'column', gap: 10, overflowY: 'auto', padding: '4px 8px 12px 4px' }}>

        {!options && <LoadingState />}

        {activated && hasParts && partLists.map((list, idx) => list.length > 0 && (
          <Card key={idx} elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t(PART_LABEL_KEYS[idx])} />
            <ThumbnailGallery
              values={list}
              selected={appearance.model_parts[idx]}
              onSelect={(v) => updatePart(idx, v)}
              cacheKey={(v) => `part:${baseItemId}:${idx}:${v}:${tintKey}`}
              fetchModel={(v) => invoke<ModelData>('load_item_part', {
                baseItemId,
                partIndex: idx,
                variant: v,
              })}
              tints={frozenTints}
              tileLabel={(v) => `${t('appearance.part')} ${v}`}
            />
          </Card>
        ))}

        {activated && options && options.available_variations.length > 0 && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.variation')} />
            <ThumbnailGallery
              values={options.available_variations}
              selected={appearance.variation}
              onSelect={updateVariation}
              cacheKey={(v) =>
                `variation:${baseItemId}:${characterId ?? 'none'}:${appearance.armor_visual_type ?? 'none'}:${v}:${tintKey}`}
              // Synthetic appearance: variation only, boots/gloves/accessories
              // stripped so armor thumbnails load just the primary mesh.
              fetchModel={(v) => invoke<ModelData>('load_item_model', {
                appearance: {
                  variation: v,
                  model_parts: [v, 0, 0],
                  tints: appearance.tints,
                  armor_visual_type: appearance.armor_visual_type,
                  boots: null,
                  gloves: null,
                },
                baseItemId,
              })}
              tints={frozenTints}
              tileLabel={(v) => `${t('appearance.variation')} ${v}`}
            />
          </Card>
        )}

        {options?.has_model && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.colors')} />
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <ColorPicker
                label={t('appearance.color1')}
                value={appearance.tints.channel1}
                onChange={(c) => updateTint('channel1', c)}
              />
              <ColorPicker
                label={t('appearance.color2')}
                value={appearance.tints.channel2}
                onChange={(c) => updateTint('channel2', c)}
              />
              <ColorPicker
                label={t('appearance.color3')}
                value={appearance.tints.channel3}
                onChange={(c) => updateTint('channel3', c)}
              />
            </div>
          </Card>
        )}

      </div>
    </div>
  );
}
