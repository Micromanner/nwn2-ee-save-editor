
import { useState, useEffect, useMemo } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { useTranslations } from '@/hooks/useTranslations';
import { Input } from '@/components/ui/Input';
import { Label } from '@/components/ui/Label';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { Undo2 } from 'lucide-react';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { gameStateAPI, ModuleInfo, ModuleVariablesResponse } from '@/services/gameStateApi';
import { display } from '@/utils/dataHelpers';

import { VariableTable, VariableEdit } from '@/components/ui/VariableTable';

export default function ModuleCampaignTab() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const characterId = character?.id;

  const [moduleInfo, setModuleInfo] = useState<ModuleInfo | null>(null);
  const [moduleVariables, setModuleVariables] = useState<ModuleVariablesResponse | null>(null);
  const [isLoadingModule, setIsLoadingModule] = useState(false);
  const [moduleError, setModuleError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [editedModuleVars, setEditedModuleVars] = useState<Record<string, VariableEdit>>({});
  const [isSavingModule, setIsSavingModule] = useState(false);

  const [availableModules, setAvailableModules] = useState<Array<{id: string, name: string, campaign: string, variable_count: number, is_current: boolean}>>([]);
  const [selectedModuleId, setSelectedModuleId] = useState<string | null>(null);

  useEffect(() => {
    if (characterId) {
      loadAllModules();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [characterId]);

  useEffect(() => {
    if (characterId && selectedModuleId) {
      loadModuleData(selectedModuleId);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [characterId, selectedModuleId]);

  const loadAllModules = async () => {
    if (!characterId) return;

    try {
      const { modules, current_module } = await gameStateAPI.getAllModules(characterId);
      setAvailableModules(modules);
      const initialModule = current_module || (modules.length > 0 ? modules[0].id : null);
      setSelectedModuleId(initialModule);
    } catch {
    }
  };

  const loadModuleData = async (moduleId: string) => {
    if (!characterId) return;

    setIsLoadingModule(true);
    setModuleError(null);

    try {
      const data = await gameStateAPI.getModuleById(characterId, moduleId);

      setModuleInfo({
        module_name: data.module_name,
        area_name: data.area_name,
        campaign: data.campaign,
        entry_area: data.entry_area,
        module_description: data.module_description,
        current_module: data.current_module
      });

      setModuleVariables(data.variables);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load module data';
      setModuleError(errorMessage);
    } finally {
      setIsLoadingModule(false);
    }
  };

  const handleModuleVariableChange = (name: string, value: string, type: 'int' | 'string' | 'float') => {
    let parsedValue: number | string = value;

    if (type === 'int') {
      parsedValue = parseInt(value, 10);
      if (isNaN(parsedValue)) parsedValue = 0;
    } else if (type === 'float') {
      parsedValue = parseFloat(value);
      if (isNaN(parsedValue)) parsedValue = 0.0;
    }

    setEditedModuleVars(prev => ({
      ...prev,
      [name]: { name, value: parsedValue, type }
    }));
  };

  const handleSaveModuleChanges = async () => {
    if (!characterId || Object.keys(editedModuleVars).length === 0) return;

    setIsSavingModule(true);
    setModuleError(null);

    try {
      for (const edit of Object.values(editedModuleVars)) {
        await gameStateAPI.updateModuleVariable(
          characterId,
          edit.name,
          edit.value,
          edit.type,
          selectedModuleId || undefined
        );
      }

      if (selectedModuleId) {
        await loadModuleData(selectedModuleId);
      }
      setEditedModuleVars({});
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to save changes';
      setModuleError(errorMessage);
    } finally {
      setIsSavingModule(false);
    }
  };

  const handleRevertVariable = (name: string) => {
    setEditedModuleVars(prev => {
      const newVars = { ...prev };
      delete newVars[name];
      return newVars;
    });
  };

  const handleRevertAllChanges = () => {
    setEditedModuleVars({});
  };

  const filteredModuleIntegers = useMemo(() => {
    if (!moduleVariables) return [];

    return Object.entries(moduleVariables.integers)
      .filter(([name]) => name.toLowerCase().includes(searchQuery.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [moduleVariables, searchQuery]);

  const filteredModuleStrings = useMemo(() => {
    if (!moduleVariables) return [];

    return Object.entries(moduleVariables.strings)
      .filter(([name]) => name.toLowerCase().includes(searchQuery.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [moduleVariables, searchQuery]);

  const filteredModuleFloats = useMemo(() => {
    if (!moduleVariables) return [];

    return Object.entries(moduleVariables.floats)
      .filter(([name]) => name.toLowerCase().includes(searchQuery.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [moduleVariables, searchQuery]);

  const hasModuleChanges = Object.keys(editedModuleVars).length > 0;

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="pb-4">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>{t('gameState.moduleCampaign.moduleSettings')}</CardTitle>
              <CardDescription>
                {t('gameState.moduleCampaign.moduleInfo')}
              </CardDescription>
            </div>
            {availableModules.length > 0 && (
              <div className="w-72">
                <Select
                  value={selectedModuleId || ''}
                  onValueChange={(value: string) => setSelectedModuleId(value)}
                >
                  <SelectTrigger id="module-select">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {availableModules.map((module) => (
                      <SelectItem key={module.id} value={module.id}>
                        {module.name} {module.is_current ? '(Current)' : ''}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {moduleInfo ? (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-6">
              <div className="space-y-1">
                <Label className="text-[rgb(var(--color-text-muted))] text-xs uppercase tracking-wider">{t('gameState.moduleCampaign.moduleName')}</Label>
                <div className="font-medium truncate" title={String(moduleInfo.module_name)}>{display(moduleInfo.module_name)}</div>
              </div>
              <div className="space-y-1">
                <Label className="text-[rgb(var(--color-text-muted))] text-xs uppercase tracking-wider">{t('gameState.moduleCampaign.campaign')}</Label>
                <div className="font-medium truncate" title={String(moduleInfo.campaign)}>{display(moduleInfo.campaign)}</div>
              </div>
              <div className="space-y-1">
                <Label className="text-[rgb(var(--color-text-muted))] text-xs uppercase tracking-wider">{t('gameState.moduleCampaign.currentArea')}</Label>
                <div className="font-medium truncate" title={String(moduleInfo.area_name)}>{display(moduleInfo.area_name)}</div>
              </div>
              <div className="space-y-1">
                <Label className="text-[rgb(var(--color-text-muted))] text-xs uppercase tracking-wider">{t('gameState.moduleCampaign.entryArea')}</Label>
                <div className="font-medium truncate" title={String(moduleInfo.entry_area)}>{display(moduleInfo.entry_area)}</div>
              </div>
              
              {moduleInfo.current_module && (
                <div className="md:col-span-4 pt-2 border-t border-[rgb(var(--color-border))] mt-2">
                  <div className="flex items-center gap-2 text-sm text-[rgb(var(--color-text-muted))]">
                    <span className="font-medium">ID:</span>
                    <span className="font-mono">{moduleInfo.current_module}</span>
                  </div>
                </div>
              )}
            </div>
          ) : (
            <div className="text-center text-[rgb(var(--color-text-muted))] py-4">
              {t('gameState.moduleCampaign.loadingModuleInfo')}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="flex flex-col min-h-[500px]">
        <CardHeader className="border-b border-[rgb(var(--color-border))]">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                {t('gameState.moduleCampaign.moduleVariables')}
                {moduleVariables && (
                  <Badge variant="secondary">
                    {moduleVariables.total_count}
                  </Badge>
                )}
              </CardTitle>
            </div>
            <div className="flex items-center gap-4">
              <div className="w-64">
                <Input
                  type="text"
                  placeholder="Search variables..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="h-9"
                />
              </div>
              {hasModuleChanges && (
                <>
                  <Button
                    onClick={handleRevertAllChanges}
                    variant="outline"
                    size="sm"
                    className="text-yellow-500 border-yellow-500/50 hover:bg-yellow-500/10"
                  >
                    <Undo2 className="h-4 w-4 mr-2" />
                    {t('common.revertAll')}
                  </Button>
                  <Button
                    onClick={handleSaveModuleChanges}
                    disabled={isSavingModule}
                    size="sm"
                    className="min-w-[120px]"
                  >
                    {isSavingModule ? t('common.saving') : `${t('common.save')} ${Object.keys(editedModuleVars).length} ${t('common.changes')}`}
                  </Button>
                </>
              )}
            </div>
          </div>
        </CardHeader>
        <CardContent className="flex-1 p-0">
          {moduleError && (
            <div className="m-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
              {moduleError}
            </div>
          )}

          {isLoadingModule ? (
            <div className="text-center text-[rgb(var(--color-text-muted))] py-12">
              Loading module variables...
            </div>
          ) : !moduleVariables ? (
            <div className="text-center text-[rgb(var(--color-text-muted))] py-12">
              {t('gameState.moduleCampaign.noVariables')}
            </div>
          ) : (
            <Tabs defaultValue="integers" className="w-full h-full flex flex-col">
              <div className="border-b border-[rgb(var(--color-border))] pb-2">
                <TabsList className="w-full justify-start h-12 bg-transparent p-0 rounded-none gap-2">
                  <TabsTrigger 
                    value="integers" 
                    className="flex-1 h-full rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                  >
                    Integers
                    <Badge variant="secondary" className="ml-2 bg-[rgb(var(--color-surface-primary))]">
                      {filteredModuleIntegers.length}
                    </Badge>
                  </TabsTrigger>
                  <TabsTrigger 
                    value="strings" 
                    className="flex-1 h-full rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                  >
                    Strings
                    <Badge variant="secondary" className="ml-2 bg-[rgb(var(--color-surface-primary))]">
                      {filteredModuleStrings.length}
                    </Badge>
                  </TabsTrigger>
                  <TabsTrigger 
                    value="floats" 
                    className="flex-1 h-full rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                  >
                    Floats
                    <Badge variant="secondary" className="ml-2 bg-[rgb(var(--color-surface-primary))]">
                      {filteredModuleFloats.length}
                    </Badge>
                  </TabsTrigger>
                </TabsList>
              </div>

              <TabsContent value="integers" className="flex-1 min-h-0 p-0">
                <VariableTable
                  variables={filteredModuleIntegers}
                  type="int"
                  editedVars={editedModuleVars}
                  onVariableChange={handleModuleVariableChange}
                  onRevertVariable={handleRevertVariable}
                  searchQuery={searchQuery}
                  className="border-0 rounded-none h-full"
                />
              </TabsContent>

              <TabsContent value="strings" className="flex-1 min-h-0 p-0">
                <VariableTable
                  variables={filteredModuleStrings}
                  type="string"
                  editedVars={editedModuleVars}
                  onVariableChange={handleModuleVariableChange}
                  onRevertVariable={handleRevertVariable}
                  searchQuery={searchQuery}
                  className="border-0 rounded-none h-full"
                />
              </TabsContent>

              <TabsContent value="floats" className="flex-1 min-h-0 p-0">
                <VariableTable
                  variables={filteredModuleFloats}
                  type="float"
                  editedVars={editedModuleVars}
                  onVariableChange={handleModuleVariableChange}
                  onRevertVariable={handleRevertVariable}
                  searchQuery={searchQuery}
                  className="border-0 rounded-none h-full"
                />
              </TabsContent>
            </Tabs>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
