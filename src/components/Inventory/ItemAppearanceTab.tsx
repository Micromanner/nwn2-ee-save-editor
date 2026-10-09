import { useState, useCallback, useEffect, useRef } from 'react';
import { Button, Card, Elevation, Icon, Switch } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import type {
  AccessorySlot,
  ArmorAccessories,
  ItemAppearance,
  ItemAppearanceOptions,
  TintChannels,
  TintChannel,
} from '@/lib/bindings';
import type { ModelData } from '../ModelViewer/types';
import { tintChannelsToColors } from '../ModelViewer/materials';
import { ACCESSORY_FIELD, ItemViewer3D, swapItemTints } from '../Appearance/ItemViewer3D';
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
    <div className="t-section" style={{ marginBottom: 8, marginTop: 4 }}>
      {label}
    </div>
  );
}

function tintChannelsKey(t: TintChannels): string {
  const c = (ch: { r: number; g: number; b: number }) => `${ch.r},${ch.g},${ch.b}`;
  return `${c(t.channel1)}|${c(t.channel2)}|${c(t.channel3)}`;
}

const PART_LABEL_KEYS = ['appearance.part1', 'appearance.part2', 'appearance.part3'] as const;

/// Left/right pairs, in body order; the six vestigial slots have no meshes
/// and are never offered.
const ACCESSORY_ROWS: { labelKey: string; left: AccessorySlot; right: AccessorySlot }[] = [
  { labelKey: 'appearance.accessorySlot.shoulder', left: 'LtShoulder', right: 'RtShoulder' },
  { labelKey: 'appearance.accessorySlot.arm', left: 'LtArm', right: 'RtArm' },
  { labelKey: 'appearance.accessorySlot.elbow', left: 'LtElbow', right: 'RtElbow' },
  { labelKey: 'appearance.accessorySlot.bracer', left: 'LtBracer', right: 'RtBracer' },
  { labelKey: 'appearance.accessorySlot.leg', left: 'LtLeg', right: 'RtLeg' },
  { labelKey: 'appearance.accessorySlot.knee', left: 'LtKnee', right: 'RtKnee' },
  { labelKey: 'appearance.accessorySlot.shin', left: 'LtShin', right: 'RtShin' },
  { labelKey: 'appearance.accessorySlot.hip', left: 'LtHip', right: 'RtHip' },
];
const ACCESSORY_SLOTS: AccessorySlot[] = ACCESSORY_ROWS.flatMap(row => [row.left, row.right]);

const ZERO_TINTS_KEY = '0,0,0|0,0,0|0,0,0';

const EMPTY_ACCESSORIES: ArmorAccessories = {
  lt_shoulder: null, rt_shoulder: null, lt_arm: null, rt_arm: null,
  lt_elbow: null, rt_elbow: null, lt_bracer: null, rt_bracer: null,
  lt_leg: null, rt_leg: null, lt_knee: null, rt_knee: null,
  lt_shin: null, rt_shin: null, lt_hip: null, rt_hip: null,
  lt_ankle: null, rt_ankle: null, lt_foot: null, rt_foot: null,
  bk_hip: null, ft_hip: null,
};

export function ItemAppearanceTab({ appearance, baseItemId, options, active, onChange }: ItemAppearanceTabProps) {
  const t = useTranslations();
  const { characterId } = useCharacterContext();
  const [refreshKey, setRefreshKey] = useState(0);
  const [refreshPart, setRefreshPart] = useState<{ partIndex: number; key: number } | null>(null);
  const [refreshAccessory, setRefreshAccessory] = useState<{ slot: AccessorySlot; key: number } | null>(null);
  const [hoveredChannel, setHoveredChannel] = useState<keyof TintChannels | null>(null);
  const [hoveredTarget, setHoveredTarget] = useState<AccessorySlot | null>(null);
  const [hoveredSlot, setHoveredSlot] = useState<AccessorySlot | null>(null);
  const [activated, setActivated] = useState(active);
  useEffect(() => {
    if (active) setActivated(true);
  }, [active]);

  // Slots with their own colors. A slot starts custom when its saved tint
  // differs from the armor's (almost every game item), so a master color edit
  // only moves slots that already matched the armor. All-zero tints render
  // in the armor colors and count as following it.
  const [customTintSlots, setCustomTintSlots] = useState<Set<AccessorySlot>>(() => {
    const armorKey = tintChannelsKey(appearance.tints);
    return new Set(ACCESSORY_SLOTS.filter(slot => {
      const data = appearance.accessories[ACCESSORY_FIELD[slot]];
      if (!data || data.id <= 0) return false;
      const key = tintChannelsKey(data.tints);
      return key !== ZERO_TINTS_KEY && key !== armorKey;
    }));
  });

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
    if (value === appearance.variation) return;
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
    const nextTints = {
      ...appearance.tints,
      [channel]: color,
    };
    const nextAccessories = { ...appearance.accessories };
    let changedAcc = false;
    for (const slot of ACCESSORY_SLOTS) {
      if (customTintSlots.has(slot)) continue;
      const key = ACCESSORY_FIELD[slot];
      const current = nextAccessories[key];
      if (current && current.id > 0 && tintChannelsKey(current.tints) !== ZERO_TINTS_KEY) {
        nextAccessories[key] = {
          ...current,
          tints: { ...current.tints, [channel]: color },
        };
        changedAcc = true;
      }
    }
    onChange({
      ...appearance,
      tints: nextTints,
      accessories: changedAcc ? nextAccessories : appearance.accessories,
    });
  }, [appearance, customTintSlots, onChange]);

  // A slot going from empty to a mesh takes the armor's own colors; swapping
  // the mesh of an already-set slot keeps that slot's tints. Picking "none"
  // on a slot the item never had leaves the AC* field absent.
  const updateAccessory = useCallback((slot: AccessorySlot, id: number) => {
    const key = ACCESSORY_FIELD[slot];
    const current = appearance.accessories[key];
    if (!current && id === 0) return;
    const isCustom = customTintSlots.has(slot);
    const tints = isCustom && current && current.id > 0 ? current.tints : appearance.tints;
    onChange({
      ...appearance,
      accessories: { ...appearance.accessories, [key]: { id, tints } },
    });
    setRefreshAccessory({ slot, key: Date.now() });
  }, [appearance, customTintSlots, onChange]);

  const pairedSlotOf = useCallback((slot: AccessorySlot): AccessorySlot | null => {
    const row = ACCESSORY_ROWS.find(r => r.left === slot || r.right === slot);
    if (!row) return null;
    return slot === row.left ? row.right : row.left;
  }, []);

  const toggleCustom = useCallback((slot: AccessorySlot) => {
    setCustomTintSlots(prev => {
      const next = new Set(prev);
      const isNowCustom = !next.has(slot);
      if (isNowCustom) {
        next.add(slot);
      } else {
        next.delete(slot);
        const key = ACCESSORY_FIELD[slot];
        const current = appearance.accessories[key];
        if (current) {
          onChange({
            ...appearance,
            accessories: {
              ...appearance.accessories,
              [key]: { ...current, tints: appearance.tints },
            },
          });
        }
      }
      return next;
    });
  }, [appearance, onChange]);

  const updateSlotTint = useCallback((slot: AccessorySlot, channel: keyof TintChannels, color: TintChannel) => {
    const key = ACCESSORY_FIELD[slot];
    const current = appearance.accessories[key];
    if (!current) return;
    const nextTints = { ...current.tints, [channel]: color };
    onChange({
      ...appearance,
      accessories: {
        ...appearance.accessories,
        [key]: { ...current, tints: nextTints },
      },
    });
  }, [appearance, onChange]);

  const copyToPair = useCallback((sourceSlot: AccessorySlot, targetSlot: AccessorySlot) => {
    const srcKey = ACCESSORY_FIELD[sourceSlot];
    const dstKey = ACCESSORY_FIELD[targetSlot];
    const srcData = appearance.accessories[srcKey];
    const dstData = appearance.accessories[dstKey];
    if (!srcData || !dstData || dstData.id === 0) return;

    setCustomTintSlots(prev => new Set(prev).add(targetSlot));
    onChange({
      ...appearance,
      accessories: {
        ...appearance.accessories,
        [dstKey]: { ...dstData, tints: { ...srcData.tints } },
      },
    });
  }, [appearance, onChange]);

  const resetToArmor = useCallback((slot: AccessorySlot) => {
    setCustomTintSlots(prev => {
      const next = new Set(prev);
      next.delete(slot);
      return next;
    });
    const key = ACCESSORY_FIELD[slot];
    const current = appearance.accessories[key];
    if (current) {
      onChange({
        ...appearance,
        accessories: {
          ...appearance.accessories,
          [key]: { ...current, tints: appearance.tints },
        },
      });
    }
  }, [appearance, onChange]);

  const accessoryOptions = options?.available_accessories ?? [];
  const accessoryIds = (slot: AccessorySlot) =>
    accessoryOptions.find(o => o.slot === slot)?.ids ?? [];
  const accessoryId = (slot: AccessorySlot) => appearance.accessories[ACCESSORY_FIELD[slot]]?.id ?? 0;
  const rowOf = (slot: AccessorySlot) => ACCESSORY_ROWS.find(r => r.left === slot || r.right === slot);
  const slotLabel = (slot: AccessorySlot) => {
    const row = rowOf(slot);
    if (!row) return slot;
    return t('appearance.accessorySide', {
      slot: t(row.labelKey),
      side: t(slot === row.left ? 'appearance.accessoryLeft' : 'appearance.accessoryRight'),
    });
  };

  // The gallery shows one slot at a time: the first set one on open, then
  // whichever slot button was clicked last.
  const [activeSlot, setActiveSlot] = useState<AccessorySlot>(() => {
    const firstSet = ACCESSORY_SLOTS
      .find(slot => (appearance.accessories[ACCESSORY_FIELD[slot]]?.id ?? 0) > 0);
    return firstSet ?? 'LtShoulder';
  });
  const activeIds = accessoryIds(activeSlot);
  const activeData = appearance.accessories[ACCESSORY_FIELD[activeSlot]];
  const activeId = accessoryId(activeSlot);
  const activeTints = activeData && activeData.id > 0 && tintChannelsKey(activeData.tints) !== ZERO_TINTS_KEY ? activeData.tints : appearance.tints;
  const activeTintKey = tintChannelsKey(activeTints);

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
          refreshAccessory={refreshAccessory}
          highlightChannel={hoveredChannel}
          highlightTarget={hoveredTarget}
          selectedAccessorySlot={hoveredSlot}
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
                  accessories: EMPTY_ACCESSORIES,
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

        {activated && accessoryOptions.length > 0 && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.accessories')} />
            {/* The slot list sets the height; the gallery scrolls inside it so
                every slot stays clickable while browsing a long list. */}
            <div style={{ display: 'flex', gap: 14, alignItems: 'stretch' }}>
              <div style={{ flex: '0 0 200px', border: `1px solid ${T.border}`, borderRadius: 3, overflow: 'hidden', background: T.surfaceAlt }}>
                <div className="t-semibold" style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 10px', color: T.textMuted, background: T.sectionBg, borderBottom: `1px solid ${T.border}` }}>
                  <span>{t('appearance.accessorySlotColumn')}</span>
                  <span>{t('appearance.accessoryCurrentColumn')}</span>
                </div>
                {ACCESSORY_SLOTS.map(slot => {
                  const id = accessoryId(slot);
                  const isActive = slot === activeSlot;
                  const disabled = accessoryIds(slot).length === 0;
                  return (
                    <div
                      key={slot}
                      role="button"
                      tabIndex={disabled ? -1 : 0}
                      onClick={() => { if (!disabled) setActiveSlot(slot); }}
                      onKeyDown={(e) => {
                        if (!disabled && (e.key === 'Enter' || e.key === ' ')) {
                          e.preventDefault();
                          setActiveSlot(slot);
                        }
                      }}
                      onMouseEnter={() => setHoveredSlot(slot)}
                      onMouseLeave={() => setHoveredSlot(s => (s === slot ? null : s))}
                      style={{
                        display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                        padding: '4px 10px', cursor: disabled ? 'default' : 'pointer',
                        opacity: disabled ? 0.4 : 1, userSelect: 'none',
                        background: isActive ? T.accent : undefined,
                        color: isActive ? '#fff' : T.text,
                        borderBottom: `1px solid ${T.borderLight}`,
                      }}
                    >
                      <span>{slotLabel(slot)}</span>
                      <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
                        <span
                          className="t-semibold"
                          title={id > 0 ? t('appearance.accessoryCurrentTitle', { id: String(id) }) : t('appearance.none')}
                          style={{
                            minWidth: 28, textAlign: 'center', padding: '0 6px', borderRadius: 10,
                            background: id > 0 ? (isActive ? 'rgba(255,255,255,0.25)' : `${T.accent}22`) : undefined,
                            color: id > 0 ? (isActive ? '#fff' : T.accent) : (isActive ? 'rgba(255,255,255,0.7)' : T.textMuted),
                          }}
                        >
                          {id > 0 ? `#${id}` : '-'}
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
              <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 8 }}>
                {activeId > 0 && (
                  <div style={{
                    display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 10,
                    padding: '6px 10px', background: T.surfaceAlt,
                    border: `1px solid ${T.borderLight}`, borderRadius: 3,
                  }}>
                    <Switch
                      checked={customTintSlots.has(activeSlot)}
                      label={t('appearance.customColors')}
                      onChange={() => toggleCustom(activeSlot)}
                      style={{ margin: 0 }}
                    />
                    {customTintSlots.has(activeSlot) ? (
                      <>
                        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                          {(['channel1', 'channel2', 'channel3'] as const).map(channel => (
                            <div
                              key={channel}
                              onMouseEnter={() => {
                                setHoveredChannel(channel);
                                setHoveredTarget(activeSlot);
                              }}
                              onMouseLeave={() => {
                                setHoveredChannel(c => (c === channel ? null : c));
                                setHoveredTarget(null);
                              }}
                            >
                              <ColorPicker
                                value={activeTints[channel]}
                                onChange={c => updateSlotTint(activeSlot, channel, c)}
                              />
                            </div>
                          ))}
                        </div>
                        {pairedSlotOf(activeSlot) && accessoryId(pairedSlotOf(activeSlot)!) > 0 && (
                          <Button
                            small
                            minimal
                            icon="duplicate"
                            text={t('appearance.copyToPair', { pair: slotLabel(pairedSlotOf(activeSlot)!) })}
                            onClick={() => copyToPair(activeSlot, pairedSlotOf(activeSlot)!)}
                          />
                        )}
                        <Button
                          small
                          minimal
                          icon="undo"
                          text={t('appearance.resetToArmor')}
                          onClick={() => resetToArmor(activeSlot)}
                        />
                      </>
                    ) : (
                      <div style={{ display: 'flex', alignItems: 'center', gap: 6, color: T.textMuted }}>
                        <span>{t('appearance.usingArmorColors')}</span>
                        <div style={{ display: 'inline-flex', gap: 4 }}>
                          {(['channel1', 'channel2', 'channel3'] as const).map(ch => (
                            <span
                              key={ch}
                              style={{
                                width: 14,
                                height: 14,
                                borderRadius: '50%',
                                background: `rgb(${activeTints[ch].r}, ${activeTints[ch].g}, ${activeTints[ch].b})`,
                                border: `1px solid ${T.border}`,
                                display: 'inline-block',
                              }}
                            />
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
                <div style={{ flex: 1, minWidth: 0, position: 'relative', minHeight: 300 }}>
                  <div style={{ position: 'absolute', inset: 0, overflowY: 'auto', padding: '0 4px' }}>
                    <ThumbnailGallery
                      values={activeIds}
                      selected={activeId}
                      onSelect={(id) => updateAccessory(activeSlot, id)}
                      cacheKey={(id) => `accessory:${characterId ?? 'none'}:${activeSlot}:${id}:${activeTintKey}`}
                      fetchModel={(id) => invoke<ModelData>('load_accessory_model', {
                        slot: activeSlot,
                        id,
                        tints: activeTints,
                      })}
                      tints={frozenTints}
                      tileLabel={(id) => String(id)}
                      pose="armor"
                      noneLabel={t('appearance.none')}
                      centered
                    />
                  </div>
                </div>
              </div>
            </div>
            <div style={{ color: T.textMuted, marginTop: 8, display: 'flex', alignItems: 'center', gap: 5 }}>
              <Icon icon="info-sign" size={14} style={{ color: T.textMuted }} />
              {t('appearance.accessoriesHint')}
            </div>
          </Card>
        )}

        {options?.has_model && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.colors')} />
            <div style={{ display: 'flex', gap: 12 }}>
              {(['channel1', 'channel2', 'channel3'] as const).map((channel) => (
                <div
                  key={channel}
                  onMouseEnter={() => {
                    setHoveredChannel(channel);
                    setHoveredTarget(null);
                  }}
                  onMouseLeave={() => setHoveredChannel((c) => (c === channel ? null : c))}
                >
                  <ColorPicker
                    value={appearance.tints[channel]}
                    onChange={(c) => updateTint(channel, c)}
                  />
                </div>
              ))}
            </div>
            <div style={{ color: T.textMuted, marginTop: 8, display: 'flex', alignItems: 'center', gap: 5 }}>
              <Icon icon="info-sign" size={14} style={{ color: T.textMuted }} />
              {t('appearance.colorsHoverHint')}
            </div>
          </Card>
        )}

      </div>
    </div>
  );
}
