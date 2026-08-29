import { useState, useEffect, useCallback, useRef } from 'react';
import { Button, Checkbox, Tab, Tabs } from '@blueprintjs/core';
import { GiTiedScroll } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { DropdownSelect, ParchmentDialog, StepInput, LoadingState } from '../shared';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useInventoryManagement } from '@/hooks/useInventoryManagement';
import { type ItemEditorMetadataResponse, type PropertyMetadata } from '@/services/inventoryApi';
import { stripNwn2Tags, nwn2ToHtml, htmlToNwn2 } from '@/utils/nwn2Markup';
import { ItemAppearanceTab } from './ItemAppearanceTab';
import { IconPickerDialog } from './IconPickerDialog';
import { useIcon } from '@/hooks/useIcon';
import type { ItemAppearance } from '@/lib/bindings';

function sortCostTableEntries(entries: [string, string][]): [string, string][] {
  return entries.sort(([, a], [, b]) => {
    const isDiceA = /\d+d\d+/.test(a);
    const isDiceB = /\d+d\d+/.test(b);
    if (isDiceA && !isDiceB) return -1;
    if (!isDiceA && isDiceB) return 1;
    if (isDiceA && isDiceB) {
      const [numA, dieA] = a.match(/(\d+)d(\d+)/)!.slice(1).map(Number);
      const [numB, dieB] = b.match(/(\d+)d(\d+)/)!.slice(1).map(Number);
      return numA !== numB ? numA - numB : dieA - dieB;
    }
    const numA = parseInt(a.replace(/[^-\d]/g, '')) || 0;
    const numB = parseInt(b.replace(/[^-\d]/g, '')) || 0;
    return numA - numB;
  });
}

// Lowest option a cost table offers. Row 0 of every cost table is "Random",
// which has no TLK strref and makes the game print "Bad StrRef" on the item, so
// the backend drops it - never fall back to 0 here.
function defaultCostValue(meta: PropertyMetadata | undefined): number {
  const keys = Object.keys(meta?.cost_table_options ?? {}).map(Number).filter(n => !Number.isNaN(n));
  return keys.length ? Math.min(...keys) : 0;
}

export interface EditItemDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSaved?: () => void;
  itemData: Record<string, unknown> | null;
  itemIndex?: number | null;
  slot?: string | null;
  resolvedName?: string;
  resolvedDescription?: string;
  resolvedIcon?: string;
  preloadedMetadata?: ItemEditorMetadataResponse | null;
  appearance?: ItemAppearance | null;
  // baseitems.Stacking for this item's base type.
  maxStack?: number;
  // baseitems.PropColumn - which itemprops.2da column governs this base type.
  propColumn?: number;
}

export function EditItemDialog({
  isOpen,
  onClose,
  onSaved,
  itemData,
  itemIndex,
  slot,
  resolvedName,
  resolvedDescription,
  resolvedIcon,
  preloadedMetadata,
  appearance,
  maxStack,
  propColumn,
}: EditItemDialogProps) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { characterId } = useCharacterContext();
  const { updateItem } = useInventoryManagement();

  const [tab, setTab] = useState<string>('basic');
  const [localData, setLocalData] = useState<Record<string, unknown> | null>(null);
  const [localAppearance, setLocalAppearance] = useState<ItemAppearance | null>(null);
  const [localIconResref, setLocalIconResref] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [editorInitialized, setEditorInitialized] = useState(false);
  const [iconPickerOpen, setIconPickerOpen] = useState(false);
  const iconPreviewUrl = useIcon(localIconResref);

  const nameRef = useRef<HTMLDivElement>(null);
  const descRef = useRef<HTMLDivElement>(null);
  const prevItemKey = useRef<string | null>(null);

  useEffect(() => {
    if (isOpen) setTab('basic');
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen || !itemData || !characterId) return;

    const itemKey = `${itemIndex ?? ''}:${slot ?? ''}`;
    if (prevItemKey.current !== itemKey) {
      setTab('basic');
      prevItemKey.current = itemKey;
    }

    const data = JSON.parse(JSON.stringify(itemData));
    if (data.StackSize === undefined) data.StackSize = 1;

    if (resolvedName && data.LocalizedName) {
      const substrings = (data.LocalizedName as Record<string, unknown>).substrings as Array<{ string?: string }> | undefined;
      if (!substrings || substrings.length === 0) {
        (data.LocalizedName as Record<string, unknown>).substrings = [{ language: 0, gender: 0, string: resolvedName }];
      }
    }

    if (resolvedDescription && data.DescIdentified) {
      const substrings = (data.DescIdentified as Record<string, unknown>).substrings as Array<{ string?: string }> | undefined;
      if (!substrings || substrings.length === 0) {
        (data.DescIdentified as Record<string, unknown>).substrings = [{ language: 0, gender: 0, string: resolvedDescription }];
      }
    }

    setLocalData(data);
    setLocalAppearance(appearance ? structuredClone(appearance) : null);
    setLocalIconResref(resolvedIcon ?? null);
    setEditorInitialized(false);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, itemData, characterId, resolvedName, resolvedDescription, resolvedIcon]);

  useEffect(() => {
    if (!isOpen || !localData || editorInitialized) return;
    const nameField = localData['LocalizedName'] as Record<string, unknown> | undefined;
    const nameSubstrings = nameField?.substrings as Array<{ string?: string }> | undefined;
    const descField = localData['DescIdentified'] as Record<string, unknown> | undefined;
    const descSubstrings = descField?.substrings as Array<{ string?: string }> | undefined;
    const nameHtml = nwn2ToHtml(nameSubstrings?.[0]?.string || '');
    const descHtml = nwn2ToHtml(descSubstrings?.[0]?.string || '');

    const tryInit = () => {
      if (nameRef.current && descRef.current) {
        nameRef.current.innerHTML = nameHtml;
        descRef.current.innerHTML = descHtml;
        setEditorInitialized(true);
      } else {
        requestAnimationFrame(tryInit);
      }
    };
    requestAnimationFrame(tryInit);
  }, [isOpen, localData, editorInitialized]);

  const syncField = useCallback((field: string, ref: React.RefObject<HTMLDivElement | null>) => {
    if (!ref.current) return;
    const value = htmlToNwn2(ref.current);
    setLocalData(prev => {
      if (!prev) return prev;
      const existing = prev[field] as Record<string, unknown> | undefined;
      if (!existing) {
        return { ...prev, [field]: { string_ref: 4294967295, substrings: [{ language: 0, gender: 0, string: value }] } };
      }
      return { ...prev, [field]: { ...existing, substrings: [{ language: 0, gender: 0, string: value }] } };
    });
  }, []);

  const handleBasicChange = (field: string, value: unknown) => {
    setLocalData(prev => prev ? { ...prev, [field]: value } : prev);
  };

  // itemprops.2da: the engine silently ignores a property the base item's
  // column does not allow, so the picker only offers the ones that work.
  const isPropertyAllowed = useCallback((meta: PropertyMetadata) => {
    if (propColumn === undefined || propColumn < 0) return true;
    return (meta.allowed_columns ?? []).includes(propColumn);
  }, [propColumn]);

  const handleAddProperty = () => {
    if (!preloadedMetadata?.property_types.length) return;
    const firstProp = preloadedMetadata.property_types.find(isPropertyAllowed)
      ?? preloadedMetadata.property_types[0];
    const newProp = {
      PropertyName: firstProp.id,
      Subtype: 0,
      CostTable: firstProp.cost_table_idx ?? 0,
      CostValue: defaultCostValue(firstProp),
      Param1: firstProp.param1_idx ?? 255,
      Param1Value: 0,
      ChanceAppear: 100,
      Useable: true,
      UsesPerDay: 255,
    };
    setLocalData(prev => prev ? { ...prev, PropertiesList: [...((prev.PropertiesList as unknown[]) || []), newProp] } : prev);
  };

  const handleRemoveProperty = (index: number) => {
    setLocalData(prev => {
      if (!prev) return prev;
      const newList = [...((prev.PropertiesList as unknown[]) || [])];
      newList.splice(index, 1);
      return { ...prev, PropertiesList: newList };
    });
  };

  const handlePropertyChange = (index: number, field: string, value: unknown) => {
    setLocalData(prev => {
      if (!prev) return prev;
      const newList = [...((prev.PropertiesList as Record<string, unknown>[]) || [])];
      const property = { ...newList[index], [field]: value };
      if (field === 'PropertyName') {
        const propMeta = preloadedMetadata?.property_types.find(p => p.id === value);
        if (propMeta) {
          property.Subtype = 0;
          property.CostTable = propMeta.cost_table_idx ?? 0;
          property.CostValue = defaultCostValue(propMeta);
          property.Param1 = propMeta.param1_idx ?? 255;
          property.Param1Value = 0;
        }
      }
      newList[index] = property;
      return { ...prev, PropertiesList: newList };
    });
  };

  const handleSave = async () => {
    if (!localData || isSaving) return;
    setIsSaving(true);
    try {
      await updateItem({
        item_index: itemIndex ?? undefined,
        slot: slot ?? undefined,
        item_data: localData,
        appearance: localAppearance,
      });
      onClose();
      onSaved?.();
    } catch (err) {
      handleError(err);
    } finally {
      setIsSaving(false);
    }
  };

  const getLocalizedValue = (field: string) => {
    const fieldData = localData?.[field] as Record<string, unknown> | undefined;
    const substrings = fieldData?.substrings as Array<{ string?: string }> | undefined;
    return substrings?.[0]?.string || '';
  };

  const getSubtypeOptions = (propertyName: number): Record<number, string> | null => {
    const propDef = preloadedMetadata?.property_types.find(p => p.id === propertyName);
    return propDef?.subtype_options || null;
  };

  const getPropMeta = (propertyName: number): PropertyMetadata | undefined =>
    preloadedMetadata?.property_types.find(p => p.id === propertyName);

  const properties = (localData?.PropertiesList as Record<string, unknown>[] | undefined) || [];
  const itemName = stripNwn2Tags(getLocalizedValue('LocalizedName'));
  const stackLimit = Math.max(1, maxStack ?? 1);

  return (
    <>
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      onOpened={undefined}
      title={itemName ? `${t('inventory.editItem')}: ${itemName}` : t('inventory.editItem')}
      width={1160}
      minHeight={720}
      footerActions={
        <Button
          intent="primary"
          icon={<GameIcon icon={GiTiedScroll} size={14} />}
          text={isSaving ? t('inventory.addingItem') : t('inventory.save')}
          onClick={handleSave}
          loading={isSaving}
          disabled={isSaving || !localData}
        />
      }
      footerLeft={
        <span style={{ color: T.textMuted }}>
          {properties.length} {t('inventory.properties')}
        </span>
      }
    >
      {!localData ? (
        <LoadingState height={200} text={t('common.loadingItem')} />
      ) : (
        <Tabs id="edit-item-tab" selectedTabId={tab} onChange={(id) => setTab(id as string)}>
          <Tab id="basic" title={t('inventory.basicInfoTab')} panel={
            <div style={{ display: 'flex', flexDirection: 'column', gap: 14, paddingTop: 4, maxHeight: '58vh', overflowY: 'auto', paddingRight: 8 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <div
                  style={{
                    width: 56, height: 56, flexShrink: 0,
                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                    border: `1px solid ${T.border}`, borderRadius: 3, background: T.surfaceAlt,
                  }}
                >
                  {iconPreviewUrl ? (
                    <img src={iconPreviewUrl} alt="" width={48} height={48} draggable={false} />
                  ) : (
                    <div style={{ width: 48, height: 48, background: T.border, borderRadius: 2 }} />
                  )}
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
                  <Button
                    icon="edit"
                    text={t('inventory.changeIcon')}
                    onClick={() => setIconPickerOpen(true)}
                  />
                  <span className="t-sm" style={{ color: T.textMuted }}>
                    {localIconResref || '-'}
                  </span>
                </div>
              </div>

              <div>
                <label className="t-semibold" style={{ color: T.textMuted, display: 'block', marginBottom: 4 }}>{t('inventory.editor.name')}</label>
                <div
                  ref={nameRef}
                  contentEditable
                  suppressContentEditableWarning
                  onInput={() => syncField('LocalizedName', nameRef)}
                  onKeyDown={(e) => { if (e.key === 'Enter') e.preventDefault(); }}
                  onPaste={(e) => {
                    e.preventDefault();
                    document.execCommand('insertText', false, e.clipboardData.getData('text/plain'));
                  }}
                  style={{
                    width: '100%', borderRadius: 3, border: `1px solid ${T.border}`,
                    background: T.surface, padding: '6px 10px', color: T.text,
                    minHeight: 32, outline: 'none', whiteSpace: 'nowrap', overflow: 'hidden',
                  }}
                />
              </div>

              <div>
                <label className="t-semibold" style={{ color: T.textMuted, display: 'block', marginBottom: 4 }}>{t('inventory.editor.description')}</label>
                <div
                  ref={descRef}
                  contentEditable
                  suppressContentEditableWarning
                  onInput={() => syncField('DescIdentified', descRef)}
                  onPaste={(e) => {
                    e.preventDefault();
                    document.execCommand('insertText', false, e.clipboardData.getData('text/plain'));
                  }}
                  style={{
                    width: '100%', borderRadius: 3, border: `1px solid ${T.border}`,
                    background: T.surfaceAlt, padding: '6px 10px', color: T.text,
                    minHeight: 100, outline: 'none', resize: 'vertical',
                  }}
                />
              </div>

              <div style={{
                display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12,
                padding: '10px 12px', background: T.sectionBg, border: `1px solid ${T.sectionBorder}`, borderRadius: 4,
              }}>
                <div>
                  <label className="t-semibold" style={{ color: T.textMuted, display: 'block', marginBottom: 4 }}>{t('inventory.stackSize')}</label>
                  <StepInput
                    value={(localData.StackSize as number | undefined) ?? 1}
                    onValueChange={(v) => handleBasicChange('StackSize', v)}
                    min={1} max={stackLimit} width={120}
                    disabled={stackLimit <= 1}
                  />
                  {stackLimit <= 1 && (
                    <div className="t-sm" style={{ color: T.textMuted, marginTop: 3 }}>
                      {t('inventory.notStackable')}
                    </div>
                  )}
                </div>
                <div>
                  <label className="t-semibold" style={{ color: T.textMuted, display: 'block', marginBottom: 4 }}>{t('inventory.charges')}</label>
                  <StepInput
                    value={(localData.Charges as number | undefined) ?? 0}
                    onValueChange={(v) => handleBasicChange('Charges', v)}
                    min={0} max={255} width={120}
                  />
                </div>
              </div>

              <div>
                <label className="t-semibold" style={{ color: T.textMuted, display: 'block', marginBottom: 6 }}>Flags</label>
                <div style={{
                  display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 6,
                  padding: '10px 12px', background: T.sectionBg, border: `1px solid ${T.sectionBorder}`, borderRadius: 4,
                }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    {/* Absent Identified counts as identified, as the backend and the game read it. */}
                    <Checkbox
                      checked={(localData.Identified ?? 1) !== 0}
                      onChange={(e) => handleBasicChange('Identified', (e.target as HTMLInputElement).checked ? 1 : 0)}
                      style={{ margin: 0 }}
                    />
                    <span style={{ color: T.text }}>{t('inventory.identified')}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <Checkbox
                      checked={localData.Plot === 1}
                      onChange={(e) => handleBasicChange('Plot', (e.target as HTMLInputElement).checked ? 1 : 0)}
                      style={{ margin: 0 }}
                    />
                    <span style={{ color: T.accent }}>{t('inventory.plotItem')}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <Checkbox
                      checked={localData.Cursed === 1}
                      onChange={(e) => handleBasicChange('Cursed', (e.target as HTMLInputElement).checked ? 1 : 0)}
                      style={{ margin: 0 }}
                    />
                    <span style={{ color: T.negative }}>{t('inventory.cursedItem')}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <Checkbox
                      checked={localData.Stolen === 1}
                      onChange={(e) => handleBasicChange('Stolen', (e.target as HTMLInputElement).checked ? 1 : 0)}
                      style={{ margin: 0 }}
                    />
                    <span style={{ color: T.text }}>{t('inventory.stolenItem')}</span>
                  </div>
                </div>
              </div>
            </div>
          } />

          <Tab id="properties" title={t('inventory.propertiesTab')} panel={
            <div style={{ paddingTop: 4 }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
                <span className="t-semibold" style={{ color: T.text }}>
                  {t('inventory.enchantments')} ({properties.length})
                </span>
                <Button small icon="add" intent="primary" text={t('inventory.addProperty')} onClick={handleAddProperty} disabled={!preloadedMetadata} />
              </div>

              <div style={{ maxHeight: '54vh', overflowY: 'auto', paddingRight: 8 }}>
                {properties.map((prop, i) => {
                  const propMeta = getPropMeta(prop.PropertyName as number);
                  const subtypeOptions = getSubtypeOptions(prop.PropertyName as number);

                  const propAllowed = !propMeta || isPropertyAllowed(propMeta);
                  const propertyItems = (preloadedMetadata?.property_types || [])
                    .filter(pt => pt.id === prop.PropertyName || isPropertyAllowed(pt))
                    .map(pt => ({ id: pt.id, label: pt.label }));
                  const subtypeItems = subtypeOptions
                    ? Object.entries(subtypeOptions).map(([id, label]) => ({ id, label }))
                    : [];
                  const costItems = propMeta?.cost_table_options
                    ? sortCostTableEntries(Object.entries(propMeta.cost_table_options)).map(([id, label]) => ({ id, label }))
                    : [];
                  const param1Items = propMeta?.param1_options
                    ? Object.entries(propMeta.param1_options).map(([id, label]) => ({ id, label }))
                    : [];

                  const propLabel = propertyItems.find(p => p.id === prop.PropertyName)?.label || String(prop.PropertyName);
                  const subtypeLabel = subtypeItems.find(s => String(s.id) === String(prop.Subtype ?? 0))?.label || String(prop.Subtype ?? 0);
                  // A value with no row in the cost table cannot be described by the
                  // game - typically CostValue 0 ("Random") written by an older build.
                  const costEntry = costItems.find(c => String(c.id) === String(prop.CostValue ?? 0));
                  const costInvalid = Boolean(propMeta?.has_cost_table) && !costEntry;
                  const costLabel = costEntry?.label
                    ?? `${t('inventory.propertyValueInvalid')} (${prop.CostValue ?? 0})`;
                  const param1Label = param1Items.find(p => String(p.id) === String(prop.Param1Value ?? 0))?.label || String(prop.Param1Value ?? 0);

                  const colCount = (propMeta?.has_subtype ? 1 : 0) + (propMeta?.has_cost_table ? 1 : 0) + (propMeta?.has_param1 ? 1 : 0);
                  const gridCols = `1fr ${Array(colCount).fill('1fr').join(' ')} 80px`;

                  return (
                    <div key={i} style={{
                      display: 'grid', gridTemplateColumns: gridCols, gap: 8, alignItems: 'end',
                      padding: '8px 0',
                      borderBottom: `1px solid ${T.borderLight}`,
                    }}>
                      <DropdownSelect
                        label={t('inventory.propertyType')}
                        value={propLabel}
                        items={propertyItems}
                        onChange={(id) => handlePropertyChange(i, 'PropertyName', id)}
                        valueColor={propAllowed ? undefined : T.negative}
                      />
                      {!propAllowed && (
                        <div className="t-sm t-semibold" style={{ gridColumn: '1 / -1', color: T.negative, marginTop: -2 }}>
                          {t('inventory.propertyNotSupported')}
                        </div>
                      )}
                      {propMeta?.has_subtype && (
                        <DropdownSelect
                          label={t('inventory.propertySubtype')}
                          value={subtypeLabel}
                          items={subtypeItems}
                          onChange={(id) => handlePropertyChange(i, 'Subtype', id)}
                        />
                      )}
                      {propMeta?.has_cost_table && (
                        <DropdownSelect
                          label={t('inventory.propertyValue')}
                          value={costLabel}
                          items={costItems}
                          valueColor={costInvalid ? T.negative : undefined}
                          onChange={(id) => handlePropertyChange(i, 'CostValue', id)}
                        />
                      )}
                      {propMeta?.has_param1 && (
                        <DropdownSelect
                          label={t('inventory.propertyValue')}
                          value={param1Label}
                          items={param1Items}
                          onChange={(id) => handlePropertyChange(i, 'Param1Value', id)}
                        />
                      )}
                      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                        <Button small minimal icon="trash" intent="danger" onClick={() => handleRemoveProperty(i)} />
                      </div>
                    </div>
                  );
                })}

                {properties.length === 0 && (
                  <div style={{ padding: 32, textAlign: 'center', color: T.textMuted }}>
                    {t('inventory.noPropertiesAdded')}
                  </div>
                )}
              </div>
            </div>
          } />

          <Tab id="appearance" title={t('appearance.title')} panel={
            <div style={{ paddingTop: 4, maxHeight: '58vh', overflowY: 'auto', paddingRight: 8 }}>
              {localAppearance && localData && (
                <ItemAppearanceTab 
                  appearance={localAppearance}
                  baseItemId={Number(localData.BaseItem || 0)}
                  onChange={setLocalAppearance}
                />
              )}
            </div>
          } />
        </Tabs>
      )}
    </ParchmentDialog>
    <IconPickerDialog
      isOpen={iconPickerOpen}
      onClose={() => setIconPickerOpen(false)}
      initialResref={localIconResref}
      onPick={({ rowId, resref }) => {
        setLocalIconResref(resref);
        setLocalData(prev => prev ? { ...prev, Icon: rowId } : prev);
      }}
    />
    </>
  );
}
