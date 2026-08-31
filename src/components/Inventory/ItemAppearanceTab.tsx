import { useEffect, useState, useCallback, useMemo } from 'react';
import { Button, Card, Elevation, Menu, MenuItem, Popover } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import type { ItemAppearance, ItemAppearanceOptions, TintChannels, TintChannel } from '@/lib/bindings';
import { ItemViewer3D } from '../Appearance/ItemViewer3D';
import { ColorPicker } from '../Appearance/ColorPicker';
import { VariantStepper } from '../Appearance/VariantStepper';
import { KVRow, LoadingState } from '../shared';
import { T } from '../theme';

interface ItemAppearanceTabProps {
  appearance: ItemAppearance;
  baseItemId: number;
  onChange: (appearance: ItemAppearance) => void;
}

function SectionHeader({ label }: { label: string }) {
  return (
    <div className="t-section" style={{ color: T.accent, marginBottom: 8, marginTop: 4 }}>
      {label}
    </div>
  );
}

function PartSelect({ value, options, partLabel, onChange }: {
  value: number;
  options: number[];
  partLabel: string;
  onChange: (v: number) => void;
}) {
  return (
    <Popover
      content={
        <Menu style={{ maxHeight: 300, overflowY: 'auto' }}>
          {options.map(v => (
            <MenuItem
              key={v}
              text={`${partLabel} ${v}`}
              active={value === v}
              onClick={() => onChange(v)}
            />
          ))}
        </Menu>
      }
      placement="bottom-end"
      minimal
    >
      <Button
        minimal
        rightIcon="caret-down"
        text={`${partLabel} ${value}`}
        className="t-semibold"
      />
    </Popover>
  );
}

export function ItemAppearanceTab({ appearance, baseItemId, onChange }: ItemAppearanceTabProps) {
  const t = useTranslations();
  const [options, setOptions] = useState<ItemAppearanceOptions | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [refreshPart, setRefreshPart] = useState<{ partIndex: number; key: number } | null>(null);

  useEffect(() => {
    invoke<ItemAppearanceOptions>('get_item_appearance_options', {
      baseItemId,
      armorVisualType: appearance.armor_visual_type,
    })
      .then(setOptions)
      .catch(err => console.error('Failed to load item appearance options:', err));
  }, [baseItemId, appearance.armor_visual_type]);

  const hasMultipleParts = useMemo(() => {
    return options && (options.available_part1.length > 0 || options.available_part2.length > 0 || options.available_part3.length > 0);
  }, [options]);

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

  return (
    <div style={{ display: 'flex', gap: 20, height: '100%', minHeight: 500 }}>
      <div style={{ flex: 1, minWidth: 400 }}>
        <ItemViewer3D
          appearance={appearance}
          baseItemId={baseItemId}
          refreshKey={refreshKey}
          refreshPart={refreshPart}
        />
      </div>

      <div style={{ width: 320, display: 'flex', flexDirection: 'column', gap: 10, overflowY: 'auto' }}>

        {!options && <LoadingState />}

        {hasMultipleParts && options && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.model_parts')} />
            <KVRow
              label={t('appearance.part1')}
              value={
                <PartSelect
                  value={appearance.model_parts[0]}
                  options={options.available_part1}
                  partLabel={t('appearance.part')}
                  onChange={(v) => updatePart(0, v)}
                />
              }
            />
            <KVRow
              label={t('appearance.part2')}
              value={
                <PartSelect
                  value={appearance.model_parts[1]}
                  options={options.available_part2}
                  partLabel={t('appearance.part')}
                  onChange={(v) => updatePart(1, v)}
                />
              }
            />
            <KVRow
              label={t('appearance.part3')}
              value={
                <PartSelect
                  value={appearance.model_parts[2]}
                  options={options.available_part3}
                  partLabel={t('appearance.part')}
                  onChange={(v) => updatePart(2, v)}
                />
              }
            />
          </Card>
        )}

        {options && options.available_variations.length > 0 && (
          <Card elevation={Elevation.ONE} style={{ padding: '12px 16px', background: T.surface }}>
            <SectionHeader label={t('appearance.variation')} />
            <KVRow
              label={t('appearance.variation')}
              value={
                <VariantStepper
                  value={appearance.variation}
                  variants={options.available_variations}
                  onChange={updateVariation}
                />
              }
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
