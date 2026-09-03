import { useState, useCallback, useEffect, useRef } from 'react';
import { Card, Elevation, Icon } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import type { ItemAppearance, ItemAppearanceOptions, TintChannels, TintChannel } from '@/lib/bindings';
import type { ModelData } from '../ModelViewer/types';
import { tintChannelsToColors } from '../ModelViewer/materials';
import { ItemViewer3D, swapItemTints } from '../Appearance/ItemViewer3D';
import { ColorPicker } from '../Appearance/ColorPicker';
import { ThumbnailGallery } from '../Appearance/ThumbnailGallery';
import { DropdownSelect } from '../shared/DropdownSelect';
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
  const [hoveredChannel, setHoveredChannel] = useState<keyof TintChannels | null>(null);
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

  // Modeltype-0 items (shields, bows, crossbows) resolve their mesh from
  // ModelPart1 and treat Variation as a mirror, so both have to move or the
  // live preview keeps rendering the old variant.
  const variationField = options?.variation_field ?? 'variation';
  const updateVariation = useCallback((value: number) => {
    const next = { ...appearance, variation: value };
    if (variationField === 'model_part1') {
      next.model_parts = [value, appearance.model_parts[1], appearance.model_parts[2]];
    }
    onChange(next);
    setRefreshKey(k => k + 1);
  }, [appearance, onChange, variationField]);

  // ArmorVisualType is the armor.2da material; the parent refetches the
  // options for the new material, and the effect below moves the variation
  // onto that material's first mesh when the old one has no counterpart.
  const updateMaterial = useCallback((id: number) => {
    onChange({ ...appearance, armor_visual_type: id });
    setRefreshKey(k => k + 1);
  }, [appearance, onChange]);

  const materials = options?.available_materials ?? [];
  const variations = options?.available_variations ?? [];
  useEffect(() => {
    if (materials.length === 0 || variations.length === 0) return;
    if (variations.includes(appearance.variation)) return;
    onChange({ ...appearance, variation: variations[0] });
    setRefreshKey(k => k + 1);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [options]);

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
          highlightChannel={hoveredChannel}
          pose={options?.preview_pose}
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
              cacheKey={(v) => `part:${baseItemId}:${idx}:${v}:${tintKey}:${options?.preview_pose ?? 'auto'}`}
              fetchModel={(v) => invoke<ModelData>('load_item_part', {
                baseItemId,
                partIndex: idx,
                variant: v,
              })}
              tints={frozenTints}
              tileLabel={(v) => `${t('appearance.part')} ${v}`}
              pose={options?.preview_pose}
            />
          </Card>
        ))}

        {activated && options && (materials.length > 0 || options.available_variations.length > 0) && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.variation')} />
            {materials.length > 0 && (
              <div style={{ marginBottom: 10 }}>
                <DropdownSelect
                  label={t('appearance.material')}
                  value={materials.find(m => m.id === appearance.armor_visual_type)?.name ?? '-'}
                  items={materials.map(m => ({ id: m.id, label: m.name }))}
                  onChange={updateMaterial}
                  width={220}
                  small
                />
              </div>
            )}
            {options.available_variations.length > 0 && (
            <ThumbnailGallery
              values={options.available_variations}
              selected={appearance.variation}
              onSelect={updateVariation}
              cacheKey={(v) =>
                `variation:${baseItemId}:${characterId ?? 'none'}:${appearance.armor_visual_type ?? 'none'}:${v}:${tintKey}:${options.preview_pose}`}
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
              pose={options.preview_pose}
            />
            )}
          </Card>
        )}

        {options?.has_model && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.colors')} />
            <div style={{ display: 'flex', gap: 12 }}>
              {(['channel1', 'channel2', 'channel3'] as const).map((channel) => (
                <div
                  key={channel}
                  onMouseEnter={() => setHoveredChannel(channel)}
                  onMouseLeave={() => setHoveredChannel((c) => (c === channel ? null : c))}
                >
                  <ColorPicker
                    value={appearance.tints[channel]}
                    onChange={(c) => updateTint(channel, c)}
                  />
                </div>
              ))}
            </div>
            <div className="t-base" style={{ color: T.textMuted, marginTop: 8, display: 'flex', alignItems: 'center', gap: 5 }}>
              <Icon icon="info-sign" size={12} style={{ color: T.textMuted }} />
              {t('appearance.colorsHoverHint')}
            </div>
          </Card>
        )}

      </div>
    </div>
  );
}
