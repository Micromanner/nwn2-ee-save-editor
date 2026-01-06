'use client';

import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Checkbox } from '@/components/ui/Checkbox';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Plus, Trash2, X, Save } from 'lucide-react';
import { inventoryAPI, ItemEditorMetadataResponse } from '@/services/inventoryApi';
import { useTranslations } from '@/hooks/useTranslations';

interface ItemPropertyEditorProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (updatedData: Record<string, unknown>) => Promise<void>;
  itemData: Record<string, unknown>;
  characterId: number | undefined;
  itemIndex?: number | null;
  slot?: string | null;
}

export default function ItemPropertyEditor({
  isOpen,
  onClose,
  onSave,
  itemData,
  characterId,
  itemIndex: _itemIndex,
  slot: _slot
}: ItemPropertyEditorProps) {
  useTranslations();
  const [localData, setLocalData] = useState<Record<string, unknown> | null>(null);
  const [metadata, setMetadata] = useState<ItemEditorMetadataResponse | null>(null);
  const [, setIsLoading] = useState(true);
  const [activeTab, setActiveTab] = useState('basic');

  const loadMetadata = useCallback(async () => {
    if (!characterId) return;
    setIsLoading(true);
    try {
      const data = await inventoryAPI.getEditorMetadata(characterId);
      setMetadata(data);
    } catch {
    } finally {
      setIsLoading(false);
    }
  }, [characterId]);

  useEffect(() => {
    if (isOpen && itemData) {
      setLocalData(JSON.parse(JSON.stringify(itemData)));
      loadMetadata();
    }
  }, [isOpen, itemData, loadMetadata]);

  const handleBasicChange = (field: string, value: unknown) => {
    setLocalData((prev: Record<string, unknown> | null) => ({
      ...prev,
      [field]: value
    }));
  };

  const handleLocalizedChange = (field: string, value: string) => {
    setLocalData((prev: Record<string, unknown> | null) => {
      const newData = { ...prev } as Record<string, unknown>;
      const existingField = newData[field] as Record<string, unknown> | undefined;
      if (!existingField) {
        newData[field] = { string_ref: 4294967295, strings: { '0': value } };
      } else {
        newData[field] = {
          ...existingField,
          strings: { ...(existingField.strings as Record<string, string>), '0': value }
        };
      }
      return newData;
    });
  };

  const getLocalizedValue = (field: string) => {
    const fieldData = localData?.[field] as Record<string, unknown> | undefined;
    const strings = fieldData?.strings as Record<string, string> | undefined;
    return strings?.['0'] || '';
  };

  const handleAddProperty = () => {
    if (!metadata?.property_types.length) return;

    const firstProp = metadata.property_types[0];
    
    const newProp = {
      PropertyName: firstProp.id,
      Subtype: 0,
      CostTable: 0,
      CostValue: 0,
      Param1: firstProp.param1_idx ?? 255,
      Param1Value: 0,
      ChancesAppear: 100,
      Useable: true,
      SpellID: 65535,
      UsesPerDay: 255
    };

    setLocalData((prev: Record<string, unknown> | null) => ({
      ...prev,
      PropertiesList: [...((prev?.PropertiesList as unknown[]) || []), newProp]
    }));
  };

  const handleRemoveProperty = (index: number) => {
    setLocalData((prev: Record<string, unknown> | null) => {
      const newList = [...((prev?.PropertiesList as unknown[]) || [])];
      newList.splice(index, 1);
      return { ...prev, PropertiesList: newList };
    });
  };

  const handlePropertyChange = (index: number, field: string, value: unknown) => {
    setLocalData((prev: Record<string, unknown> | null) => {
      const newList = [...((prev?.PropertiesList as Record<string, unknown>[]) || [])];
      const property = { ...newList[index], [field]: value };

      if (field === 'PropertyName') {
        const propMeta = metadata?.property_types.find(p => p.id === value);
        if (propMeta) {
          property.Subtype = 0;
          property.CostTable = propMeta.cost_table_idx ?? 0;
          property.CostValue = 0;
          property.Param1 = propMeta.param1_idx ?? 255;
          property.Param1Value = 0;
        }
      }
      
      newList[index] = property;
      return { ...prev, PropertiesList: newList };
    });
  };

  const getSubtypeOptions = (propertyName: number) => {
    if (!metadata) return null;
    const propertyDef = metadata.property_types.find(p => p.id === propertyName);
    return propertyDef?.subtype_options || null;
  };





  const handleSave = async () => {
    await onSave(localData);
    onClose();
  };

  if (!isOpen || !localData) return null;

  return (
    <div className="add-item-modal-overlay">
      <Card className="add-item-modal-container">
        <CardContent padding="p-0" className="flex flex-col h-full">
          {/* Header */}
          <div className="add-item-modal-header">
            <div className="add-item-modal-header-row">
              <h3 className="add-item-modal-title">Edit Item: {getLocalizedValue('LocalizedName') || 'New Item'}</h3>
              <Button onClick={onClose} variant="ghost" size="sm" className="add-item-modal-close-button">
                <X className="w-4 h-4" />
              </Button>
            </div>
          </div>

          {/* Tabs */}
          <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col overflow-hidden">
            <div className="add-item-modal-tabs">
              <TabsList className="w-full flex bg-transparent p-0 gap-2">
                <TabsTrigger
                  value="basic"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  Basic Info
                </TabsTrigger>
                <TabsTrigger
                  value="properties"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  Properties
                </TabsTrigger>
              </TabsList>
            </div>

            {/* Content */}
            <div className="add-item-modal-content">
              <TabsContent value="basic" className="add-item-modal-tab-content">
                <ScrollArea className="h-full">
                  <div className="space-y-4 p-4">
                    <div>
                      <label className="text-sm font-medium text-[rgb(var(--color-text-muted))] mb-1 block">Name</label>
                      <Input
                        value={getLocalizedValue('LocalizedName')}
                        onChange={(e) => handleLocalizedChange('LocalizedName', e.target.value)}
                      />
                    </div>
                    <div>
                      <label className="text-sm font-medium text-[rgb(var(--color-text-muted))] mb-1 block">Description</label>
                      <textarea
                        className="w-full bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded-md p-2 text-sm text-[rgb(var(--color-text-primary))] outline-none min-h-[100px]"
                        value={getLocalizedValue('Description')}
                        onChange={(e) => handleLocalizedChange('Description', e.target.value)}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <div>
                        <label className="text-sm font-medium text-[rgb(var(--color-text-muted))] mb-1 block">Stack Size</label>
                        <Input
                          type="number"
                          value={localData.StackSize || 1}
                          onChange={(e) => handleBasicChange('StackSize', parseInt(e.target.value))}
                        />
                      </div>
                      <div>
                        <label className="text-sm font-medium text-[rgb(var(--color-text-muted))] mb-1 block">Charges</label>
                        <Input
                          type="number"
                          value={localData.Charges || 0}
                          onChange={(e) => handleBasicChange('Charges', parseInt(e.target.value))}
                        />
                      </div>
                    </div>

                    <div className="grid grid-cols-2 gap-2">
                      <div className="flex items-center gap-2 p-2 bg-[rgb(var(--color-surface-2))] rounded">
                        <Checkbox
                          checked={localData.Identified === 1}
                          onCheckedChange={(checked) => handleBasicChange('Identified', checked ? 1 : 0)}
                        />
                        <span className="text-sm">Identified</span>
                      </div>
                      <div className="flex items-center gap-2 p-2 bg-[rgb(var(--color-surface-2))] rounded">
                        <Checkbox
                          checked={localData.Plot === 1}
                          onCheckedChange={(checked) => handleBasicChange('Plot', checked ? 1 : 0)}
                        />
                        <span className="text-sm text-[rgb(var(--color-warning))]">Plot Item</span>
                      </div>
                      <div className="flex items-center gap-2 p-2 bg-[rgb(var(--color-surface-2))] rounded">
                        <Checkbox
                          checked={localData.Cursed === 1}
                          onCheckedChange={(checked) => handleBasicChange('Cursed', checked ? 1 : 0)}
                        />
                        <span className="text-sm text-[rgb(var(--color-danger))]">Cursed</span>
                      </div>
                      <div className="flex items-center gap-2 p-2 bg-[rgb(var(--color-surface-2))] rounded">
                        <Checkbox
                          checked={localData.Stolen === 1}
                          onCheckedChange={(checked) => handleBasicChange('Stolen', checked ? 1 : 0)}
                        />
                        <span className="text-sm">Stolen</span>
                      </div>
                    </div>
                  </div>
                </ScrollArea>
              </TabsContent>

              <TabsContent value="properties" className="add-item-modal-tab-content">
                <div className="add-item-modal-category-filter">
                  <span className="text-sm font-semibold text-[rgb(var(--color-text-primary))]">Enchantments</span>
                  <Button onClick={handleAddProperty} variant="outline" size="sm" className="gap-1 ml-auto">
                    <Plus className="w-4 h-4" /> Add Property
                  </Button>
                </div>
                <ScrollArea className="flex-1">
                  <div className="space-y-3 p-4">
                    {((localData?.PropertiesList as Array<{ PropertyName: number; Subtype?: number; CostValue?: number; Param1Value?: number }>) || []).map((prop, index) => {
                      const subtypeOptions = getSubtypeOptions(prop.PropertyName);
                      const propMeta = metadata?.property_types.find(p => p.id === prop.PropertyName);

                      return (
                        <Card key={index} className="bg-[rgb(var(--color-surface-2))] border-[rgb(var(--color-surface-border))]">
                          <CardContent className="p-3">
                            <div className="flex justify-between items-start gap-4">
                              <div className="flex-1 grid grid-cols-4 gap-3 items-end">
                                <div className="col-span-1">
                                  <label className="text-[10px] text-[rgb(var(--color-text-muted))] uppercase tracking-wider block mb-1">
                                    Property Type
                                  </label>
                                  <select
                                    className="w-full bg-[rgb(var(--color-surface-3))] border border-[rgb(var(--color-surface-border))] rounded p-1.5 text-sm outline-none"
                                    value={prop.PropertyName || 0}
                                    onChange={(e) => handlePropertyChange(index, 'PropertyName', parseInt(e.target.value))}
                                  >
                                    {metadata?.property_types.map(pt => (
                                      <option key={pt.id} value={pt.id}>{pt.label}</option>
                                    ))}
                                  </select>
                                </div>

                                <div className={`col-span-1 ${!propMeta?.has_subtype ? "opacity-30 pointer-events-none" : ""}`}>
                                  <label className="text-[10px] text-[rgb(var(--color-text-muted))] uppercase tracking-wider block mb-1">
                                    Subtype {!propMeta?.has_subtype && "(N/A)"}
                                  </label>
                                  {subtypeOptions ? (
                                    <select
                                      className="w-full bg-[rgb(var(--color-surface-3))] border border-[rgb(var(--color-surface-border))] rounded p-1.5 text-sm outline-none"
                                      value={prop.Subtype || 0}
                                      onChange={(e) => handlePropertyChange(index, 'Subtype', parseInt(e.target.value))}
                                      disabled={!propMeta?.has_subtype}
                                    >
                                      {Object.entries(subtypeOptions).map(([id, label]) => (
                                        <option key={id} value={id}>{label}</option>
                                      ))}
                                    </select>
                                  ) : (
                                    <Input
                                      type="number"
                                      className="h-9"
                                      value={prop.Subtype || 0}
                                      onChange={(e) => handlePropertyChange(index, 'Subtype', parseInt(e.target.value))}
                                      disabled={!propMeta?.has_subtype}
                                    />
                                  )}
                                </div>

                                <div className={`col-span-1 ${!propMeta?.has_cost_table ? "hidden" : ""}`}>
                                  <label className="text-[10px] text-[rgb(var(--color-text-muted))] uppercase tracking-wider block mb-1">
                                    {propMeta?.has_param1 ? "Value" : "Value / Bonus"}
                                  </label>
                                  {propMeta?.cost_table_options ? (
                                    <select
                                      className="w-full bg-[rgb(var(--color-surface-3))] border border-[rgb(var(--color-surface-border))] rounded p-1.5 text-sm outline-none"
                                      value={prop.CostValue || 0}
                                      onChange={(e) => handlePropertyChange(index, 'CostValue', parseInt(e.target.value))}
                                    >
                                      {Object.entries(propMeta.cost_table_options).map(([id, label]) => (
                                        <option key={id} value={id}>{label}</option>
                                      ))}
                                    </select>
                                  ) : (
                                    <Input
                                      type="number"
                                      className="h-9"
                                      value={prop.CostValue || 0}
                                      onChange={(e) => handlePropertyChange(index, 'CostValue', parseInt(e.target.value))}
                                    />
                                  )}
                                </div>

                                <div className={`col-span-1 ${!propMeta?.has_param1 ? "hidden" : ""}`}>
                                  <label className="text-[10px] text-[rgb(var(--color-text-muted))] uppercase tracking-wider block mb-1">
                                    {propMeta?.has_cost_table ? "Modifier" : "Value / Bonus"}
                                  </label>
                                  {propMeta?.param1_options ? (
                                    <select
                                      className="w-full bg-[rgb(var(--color-surface-3))] border border-[rgb(var(--color-surface-border))] rounded p-1.5 text-sm outline-none"
                                      value={prop.Param1Value || 0}
                                      onChange={(e) => handlePropertyChange(index, 'Param1Value', parseInt(e.target.value))}
                                    >
                                      {Object.entries(propMeta.param1_options).map(([id, label]) => (
                                        <option key={id} value={id}>{label}</option>
                                      ))}
                                    </select>
                                  ) : (
                                    <Input
                                      type="number"
                                      className="h-9"
                                      value={prop.Param1Value || 0}
                                      onChange={(e) => handlePropertyChange(index, 'Param1Value', parseInt(e.target.value))}
                                    />
                                  )}
                                </div>
                              </div>
                              <Button
                                onClick={() => handleRemoveProperty(index)}
                                variant="ghost"
                                size="sm"
                                className="text-[rgb(var(--color-danger))] hover:bg-[rgb(var(--color-danger)/0.1)] p-1 h-auto mt-1"
                              >
                                <Trash2 className="w-4 h-4" />
                              </Button>
                            </div>
                          </CardContent>
                        </Card>
                      );
                    })}
                    {(localData.PropertiesList || []).length === 0 && (
                      <div className="add-item-modal-empty">
                        No properties added. Click "Add Property" to add enchantments.
                      </div>
                    )}
                  </div>
                </ScrollArea>
              </TabsContent>
            </div>
          </Tabs>

          {/* Footer */}
          <div className="add-item-modal-footer">
            <div className="add-item-modal-footer-info">
              {(localData.PropertiesList || []).length} properties
            </div>
            <div className="add-item-modal-footer-actions">
              <Button variant="outline" onClick={onClose}>
                Cancel
              </Button>
              <Button onClick={handleSave} className="gap-2">
                <Save className="w-4 h-4" /> Save
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
