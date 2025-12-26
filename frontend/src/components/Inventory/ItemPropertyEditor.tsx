'use client';

import { useState, useEffect, useMemo } from 'react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Checkbox } from '@/components/ui/Checkbox';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Plus, Trash2, X, Save } from 'lucide-react';
import { inventoryAPI, ItemEditorMetadataResponse, PropertyMetadata } from '@/services/inventoryApi';
import { useTranslations } from '@/hooks/useTranslations';

interface ItemPropertyEditorProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (updatedData: any) => Promise<void>;
  itemData: any; // Raw GFF data
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
  itemIndex,
  slot
}: ItemPropertyEditorProps) {
  const t = useTranslations();
  const [localData, setLocalData] = useState<any>(null);
  const [metadata, setMetadata] = useState<ItemEditorMetadataResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [activeTab, setActiveTab] = useState('basic');

  useEffect(() => {
    if (isOpen && itemData) {
      setLocalData(JSON.parse(JSON.stringify(itemData)));
      loadMetadata();
    }
  }, [isOpen, itemData]);

  const loadMetadata = async () => {
    if (!characterId) return;
    setIsLoading(true);
    try {
      const data = await inventoryAPI.getEditorMetadata(characterId);
      setMetadata(data);
    } catch (error) {
      console.error('Failed to load item editor metadata:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleBasicChange = (field: string, value: any) => {
    setLocalData((prev: any) => ({
      ...prev,
      [field]: value
    }));
  };

  const handleLocalizedChange = (field: string, value: string) => {
    setLocalData((prev: any) => {
      const newData = { ...prev };
      if (!newData[field]) {
        newData[field] = { string_ref: 4294967295, strings: { '0': value } };
      } else {
        newData[field] = {
          ...newData[field],
          strings: { ...newData[field].strings, '0': value }
        };
      }
      return newData;
    });
  };

  const getLocalizedValue = (field: string) => {
    return localData?.[field]?.strings?.['0'] || '';
  };

  const handleAddProperty = () => {
    if (!metadata?.property_types.length) return;
    
    // Default to first property type (usually enhancement)
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

    setLocalData((prev: any) => ({
      ...prev,
      PropertiesList: [...(prev.PropertiesList || []), newProp]
    }));
  };

  const handleRemoveProperty = (index: number) => {
    setLocalData((prev: any) => {
      const newList = [...(prev.PropertiesList || [])];
      newList.splice(index, 1);
      return { ...prev, PropertiesList: newList };
    });
  };

  const handlePropertyChange = (index: number, field: string, value: any) => {
    setLocalData((prev: any) => {
      const newList = [...(prev.PropertiesList || [])];
      const property = { ...newList[index], [field]: value };
      
      // If PropertyName changed, reset subtype/cost_value if necessary
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

  const getCostOptions = (propertyName: number) => {
    if (!metadata) return null;
    const propertyDef = metadata.property_types.find(p => p.id === propertyName);
    return propertyDef?.cost_table_options || null;
  };

  const getParam1Options = (propertyName: number) => {
    if (!metadata) return null;
    const propertyDef = metadata.property_types.find(p => p.id === propertyName);
    return propertyDef?.param1_options || null;
  };

  const handleSave = async () => {
    await onSave(localData);
    onClose();
  };

  if (!isOpen || !localData) return null;

  return (
    <div className="class-modal-overlay">
      <Card className="class-modal-container max-w-3xl h-[85vh]">
        <CardContent padding="p-0" className="flex flex-col h-full">
          {/* Header */}
          <div className="class-modal-header">
            <div className="class-modal-header-row">
              <h3 className="class-modal-title">Edit Item: {getLocalizedValue('LocalizedName') || 'New Item'}</h3>
              <div className="flex gap-2">
                <Button onClick={handleSave} variant="primary" size="sm" className="gap-2">
                  <Save className="w-4 h-4" /> Save
                </Button>
                <Button onClick={onClose} variant="ghost" size="sm" className="p-1">
                  <X className="w-5 h-5" />
                </Button>
              </div>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-hidden">
            <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
              <TabsList className="bg-[rgb(var(--color-surface-2))] border-b border-[rgb(var(--color-surface-border))] rounded-none px-4">
                <TabsTrigger value="basic">Basic Info</TabsTrigger>
                <TabsTrigger value="properties">Properties</TabsTrigger>
                {/* <TabsTrigger value="appearance">Appearance</TabsTrigger> */}
              </TabsList>

              <div className="flex-1 overflow-hidden p-4">
                <ScrollArea className="h-full">
                  <TabsContent value="basic" className="m-0 space-y-6 mr-4">
                    <div className="space-y-4">
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
                  </TabsContent>

                  <TabsContent value="properties" className="m-0 space-y-4 mr-4">
                    <div className="flex justify-between items-center">
                      <h4 className="text-sm font-semibold text-[rgb(var(--color-text-primary))]">Enchantments</h4>
                      <Button onClick={handleAddProperty} variant="outline" size="sm" className="gap-1">
                        <Plus className="w-4 h-4" /> Add Property
                      </Button>
                    </div>

                    <div className="space-y-3">
                      {(localData.PropertiesList || []).map((prop: any, index: number) => {
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
                        <div className="text-center py-8 border-2 border-dashed border-[rgb(var(--color-surface-border))] rounded-lg text-[rgb(var(--color-text-muted))] text-sm">
                          No properties added.
                        </div>
                      )}
                    </div>
                  </TabsContent>

                  {/* <TabsContent value="appearance" className="m-0 mr-4">
                    <div className="py-20 text-center">
                      <div className="text-4xl mb-4">🎨</div>
                      <h4 className="text-lg font-medium text-[rgb(var(--color-text-primary))]">Visual Editor Coming Soon</h4>
                      <p className="text-sm text-[rgb(var(--color-text-muted))] max-w-sm mx-auto mt-2">
                        Appearance editing (model parts and colors) is a complex feature that will be implemented in a future update.
                      </p>
                    </div>
                  </TabsContent> */}
                </ScrollArea>
              </div>
            </Tabs>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
