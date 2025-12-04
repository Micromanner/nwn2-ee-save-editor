'use client';

import { useState, useEffect, useMemo } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { useTranslations } from '@/hooks/useTranslations';
import { Input } from '@/components/ui/Input';
import { Label } from '@/components/ui/Label';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { gameStateAPI, CampaignSettingsResponse, CampaignVariablesResponse } from '@/services/gameStateApi';
import { AlertTriangle } from 'lucide-react';

import { VariableTable, VariableEdit } from '@/components/ui/VariableTable';

export default function CampaignSettingsTab() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const characterId = character?.id;

  const [settings, setSettings] = useState<CampaignSettingsResponse | null>(null);
  const [campaignVariables, setCampaignVariables] = useState<CampaignVariablesResponse | null>(null);
  const [editedSettings, setEditedSettings] = useState<Partial<CampaignSettingsResponse>>({});
  const [editedCampaignVars, setEditedCampaignVars] = useState<Record<string, VariableEdit>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingCampaign, setIsLoadingCampaign] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isSavingCampaign, setIsSavingCampaign] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [campaignError, setCampaignError] = useState<string | null>(null);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    if (characterId) {
      loadCampaignSettings();
      loadCampaignVariables();
    }
  }, [characterId]);

  const loadCampaignSettings = async () => {
    if (!characterId) return;

    setIsLoading(true);
    setError(null);

    try {
      const data = await gameStateAPI.getCampaignSettings(characterId);
      setSettings(data);
      setEditedSettings({});
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load campaign settings';
      setError(errorMessage);
      console.error('Failed to load campaign settings:', err);
    } finally {
      setIsLoading(false);
    }
  };

  const loadCampaignVariables = async () => {
    if (!characterId) return;

    setIsLoadingCampaign(true);
    setCampaignError(null);

    try {
      const data = await gameStateAPI.getCampaignVariables(characterId);
      setCampaignVariables(data);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load campaign variables';
      setCampaignError(errorMessage);
      console.error('Failed to load campaign variables:', err);
    } finally {
      setIsLoadingCampaign(false);
    }
  };

  const handleFieldChange = (field: string, value: number) => {
    setEditedSettings(prev => ({
      ...prev,
      [field]: value
    }));
  };

  const handleSaveChanges = async () => {
    if (!characterId || Object.keys(editedSettings).length === 0) return;

    setIsSaving(true);
    setError(null);
    setSaveMessage(null);

    try {
      const response = await gameStateAPI.updateCampaignSettings(characterId, editedSettings as any);

      if (response.warning) {
        setSaveMessage(response.warning);
      }

      await loadCampaignSettings();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to save changes';
      setError(errorMessage);
      console.error('Failed to save campaign settings:', err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleCampaignVariableChange = (name: string, value: string, type: 'int' | 'string' | 'float') => {
    let parsedValue: number | string = value;

    if (type === 'int') {
      parsedValue = parseInt(value, 10);
      if (isNaN(parsedValue)) parsedValue = 0;
    } else if (type === 'float') {
      parsedValue = parseFloat(value);
      if (isNaN(parsedValue)) parsedValue = 0.0;
    }

    setEditedCampaignVars(prev => ({
      ...prev,
      [name]: { name, value: parsedValue, type }
    }));
  };

  const handleSaveCampaignChanges = async () => {
    if (!characterId || Object.keys(editedCampaignVars).length === 0) return;

    setIsSavingCampaign(true);
    setCampaignError(null);

    try {
      for (const edit of Object.values(editedCampaignVars)) {
        await gameStateAPI.updateCampaignVariable(
          characterId,
          edit.name,
          edit.value,
          edit.type
        );
      }

      await loadCampaignVariables();
      setEditedCampaignVars({});
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to save changes';
      setCampaignError(errorMessage);
      console.error('Failed to save campaign variables:', err);
    } finally {
      setIsSavingCampaign(false);
    }
  };

  const hasUnsavedChanges = Object.keys(editedSettings).length > 0;
  const hasCampaignChanges = Object.keys(editedCampaignVars).length > 0;

  const getCurrentValue = (field: keyof CampaignSettingsResponse): number => {
    if (editedSettings[field] !== undefined) {
      return editedSettings[field] as number;
    }
    return settings?.[field] as number || 0;
  };

  const isFieldEdited = (field: keyof CampaignSettingsResponse): boolean => {
    return editedSettings[field] !== undefined;
  };

  const filteredCampaignIntegers = useMemo(() => {
    if (!campaignVariables) return [];

    return Object.entries(campaignVariables.integers)
      .filter(([name]) => name.toLowerCase().includes(searchQuery.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [campaignVariables, searchQuery]);

  const filteredCampaignStrings = useMemo(() => {
    if (!campaignVariables) return [];

    return Object.entries(campaignVariables.strings)
      .filter(([name]) => name.toLowerCase().includes(searchQuery.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [campaignVariables, searchQuery]);

  const filteredCampaignFloats = useMemo(() => {
    if (!campaignVariables) return [];

    return Object.entries(campaignVariables.floats)
      .filter(([name]) => name.toLowerCase().includes(searchQuery.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b));
  }, [campaignVariables, searchQuery]);

  if (isLoading) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-[rgb(var(--color-text-muted))] py-8">
            {t('common.loading')}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error && !settings) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
            {error}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (!settings) {
    return (
      <Card>
        <CardContent className="pt-6">
          <div className="text-center text-[rgb(var(--color-text-muted))] py-8">
            {t('gameState.campaign.noSettings')}
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <div className="flex items-start justify-between">
            <div className="flex-1">
              <CardTitle>{t('gameState.campaign.campaignSettings')}</CardTitle>
              <CardDescription>
                {settings.display_name || t('gameState.campaign.campaignInfo')}
              </CardDescription>
              {settings.description && (
                <p className="text-sm text-[rgb(var(--color-text-muted))] mt-2">
                  {settings.description}
                </p>
              )}
            </div>
            {hasUnsavedChanges && (
              <Button
                onClick={handleSaveChanges}
                disabled={isSaving}
                size="sm"
              >
                {isSaving ? t('actions.saving') : `${t('actions.save')} ${Object.keys(editedSettings).length} ${t('common.changes')}`}
              </Button>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {saveMessage && (
            <div className="mb-4 p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-lg text-yellow-400 text-sm flex items-start gap-2">
              <AlertTriangle className="w-4 h-4 mt-0.5 flex-shrink-0" />
              <span>{saveMessage}</span>
            </div>
          )}

          {error && (
            <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
              {error}
            </div>
          )}

          <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
            {/* Progression Settings */}
            <div className="space-y-4">
              <h3 className="text-sm font-medium uppercase tracking-wider text-[rgb(var(--color-text-muted))] border-b border-[rgb(var(--color-border))] pb-2">
                Progression
              </h3>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="level-cap" className="flex items-center gap-2">
                    {t('gameState.campaign.levelCap')}
                    {isFieldEdited('level_cap') && (
                      <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                    )}
                  </Label>
                  <Input
                    id="level-cap"
                    type="number"
                    min={1}
                    max={40}
                    value={getCurrentValue('level_cap')}
                    onChange={(e) => handleFieldChange('level_cap', parseInt(e.target.value, 10))}
                    className="mt-1.5"
                  />
                </div>

                <div>
                  <Label htmlFor="xp-cap" className="flex items-center gap-2">
                    {t('gameState.campaign.xpCap')}
                    {isFieldEdited('xp_cap') && (
                      <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                    )}
                  </Label>
                  <Input
                    id="xp-cap"
                    type="number"
                    min={0}
                    value={getCurrentValue('xp_cap')}
                    onChange={(e) => handleFieldChange('xp_cap', parseInt(e.target.value, 10))}
                    className="mt-1.5"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="companion-xp-weight" className="flex items-center gap-2">
                    {t('gameState.campaign.companionXpWeight')}
                    {isFieldEdited('companion_xp_weight') && (
                      <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                    )}
                  </Label>
                  <Input
                    id="companion-xp-weight"
                    type="number"
                    min={0}
                    max={1}
                    step={0.1}
                    value={getCurrentValue('companion_xp_weight')}
                    onChange={(e) => handleFieldChange('companion_xp_weight', parseFloat(e.target.value))}
                    className="mt-1.5"
                  />
                </div>

                <div>
                  <Label htmlFor="henchman-xp-weight" className="flex items-center gap-2">
                    {t('gameState.campaign.henchmanXpWeight')}
                    {isFieldEdited('henchman_xp_weight') && (
                      <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                    )}
                  </Label>
                  <Input
                    id="henchman-xp-weight"
                    type="number"
                    min={0}
                    max={1}
                    step={0.1}
                    value={getCurrentValue('henchman_xp_weight')}
                    onChange={(e) => handleFieldChange('henchman_xp_weight', parseFloat(e.target.value))}
                    className="mt-1.5"
                  />
                </div>
              </div>
            </div>

            {/* Gameplay Flags */}
            <div className="space-y-4">
              <h3 className="text-sm font-medium uppercase tracking-wider text-[rgb(var(--color-text-muted))] border-b border-[rgb(var(--color-border))] pb-2">
                {t('gameState.campaign.gameplayFlags')}
              </h3>

              <div className="space-y-3">
                <div className="flex items-center justify-between p-3 rounded-lg bg-[rgb(var(--color-surface-secondary))]">
                  <div>
                    <Label htmlFor="attack-neutrals" className="flex items-center gap-2">
                      {t('gameState.campaign.attackNeutrals')}
                      {isFieldEdited('attack_neutrals') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.attackNeutralsDesc')}
                    </p>
                  </div>
                  <Input
                    id="attack-neutrals"
                    type="number"
                    min={0}
                    max={1}
                    value={getCurrentValue('attack_neutrals')}
                    onChange={(e) => handleFieldChange('attack_neutrals', parseInt(e.target.value, 10))}
                    className="w-16 h-8 text-center"
                  />
                </div>

                <div className="flex items-center justify-between p-3 rounded-lg bg-[rgb(var(--color-surface-secondary))]">
                  <div>
                    <Label htmlFor="auto-xp-award" className="flex items-center gap-2">
                      {t('gameState.campaign.autoXpAward')}
                      {isFieldEdited('auto_xp_award') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.autoXpAwardDesc')}
                    </p>
                  </div>
                  <Input
                    id="auto-xp-award"
                    type="number"
                    min={0}
                    max={1}
                    value={getCurrentValue('auto_xp_award')}
                    onChange={(e) => handleFieldChange('auto_xp_award', parseInt(e.target.value, 10))}
                    className="w-16 h-8 text-center"
                  />
                </div>

                <div className="flex items-center justify-between p-3 rounded-lg bg-[rgb(var(--color-surface-secondary))]">
                  <div>
                    <Label htmlFor="journal-sync" className="flex items-center gap-2">
                      {t('gameState.campaign.journalSync')}
                      {isFieldEdited('journal_sync') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.journalSyncDesc')}
                    </p>
                  </div>
                  <Input
                    id="journal-sync"
                    type="number"
                    min={0}
                    max={1}
                    value={getCurrentValue('journal_sync')}
                    onChange={(e) => handleFieldChange('journal_sync', parseInt(e.target.value, 10))}
                    className="w-16 h-8 text-center"
                  />
                </div>

                <div className="flex items-center justify-between p-3 rounded-lg bg-[rgb(var(--color-surface-secondary))]">
                  <div>
                    <Label htmlFor="no-char-changing" className="flex items-center gap-2">
                      {t('gameState.campaign.lockCharChanges')}
                      {isFieldEdited('no_char_changing') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.lockCharChangesDesc')}
                    </p>
                  </div>
                  <Input
                    id="no-char-changing"
                    type="number"
                    min={0}
                    max={1}
                    value={getCurrentValue('no_char_changing')}
                    onChange={(e) => handleFieldChange('no_char_changing', parseInt(e.target.value, 10))}
                    className="w-16 h-8 text-center"
                  />
                </div>

                <div className="flex items-center justify-between p-3 rounded-lg bg-[rgb(var(--color-surface-secondary))]">
                  <div>
                    <Label htmlFor="use-personal-rep" className="flex items-center gap-2">
                      {t('gameState.campaign.usePersonalRep')}
                      {isFieldEdited('use_personal_reputation') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5">Mod</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.usePersonalRepDesc')}
                    </p>
                  </div>
                  <Input
                    id="use-personal-rep"
                    type="number"
                    min={0}
                    max={1}
                    value={getCurrentValue('use_personal_reputation')}
                    onChange={(e) => handleFieldChange('use_personal_reputation', parseInt(e.target.value, 10))}
                    className="w-16 h-8 text-center"
                  />
                </div>
              </div>
            </div>
          </div>

          <div className="mt-6 pt-6 border-t border-[rgb(var(--color-border))]">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm">
              <div>
                <span className="text-[rgb(var(--color-text-muted))]">{t('gameState.campaign.startModule')}:</span>
                <span className="ml-2 font-mono">{settings.start_module || '-'}</span>
              </div>
              <div>
                <span className="text-[rgb(var(--color-text-muted))]">{t('gameState.campaign.moduleCount')}:</span>
                <span className="ml-2">{settings.module_names.length}</span>
              </div>
              <div className="md:col-span-2">
                <span className="text-[rgb(var(--color-text-muted))]">{t('gameState.campaign.campaignFile')}:</span>
                <span className="ml-2 font-mono text-xs text-[rgb(var(--color-text-muted))]">{settings.campaign_file_path}</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card className="flex flex-col min-h-[500px]">
        <CardHeader className="border-b border-[rgb(var(--color-border))]">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2">
                {t('gameState.moduleCampaign.campaignVariables')}
                {campaignVariables && (
                  <Badge variant="secondary">
                    {campaignVariables.total_count}
                  </Badge>
                )}
              </CardTitle>
              <CardDescription>
                {t('gameState.moduleCampaign.campaignVariablesDesc')}
              </CardDescription>
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
              {hasCampaignChanges && (
                <Button
                  onClick={handleSaveCampaignChanges}
                  disabled={isSavingCampaign}
                  size="sm"
                  className="min-w-[120px]"
                >
                  {isSavingCampaign ? 'Saving...' : `Save ${Object.keys(editedCampaignVars).length} Changes`}
                </Button>
              )}
            </div>
          </div>
        </CardHeader>
        <CardContent className="flex-1 p-0">
          {campaignError && (
            <div className="m-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
              {campaignError}
            </div>
          )}

          {isLoadingCampaign ? (
            <div className="text-center text-[rgb(var(--color-text-muted))] py-12">
              Loading campaign variables...
            </div>
          ) : !campaignVariables ? (
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
                      {filteredCampaignIntegers.length}
                    </Badge>
                  </TabsTrigger>
                  <TabsTrigger 
                    value="strings" 
                    className="flex-1 h-full rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                  >
                    Strings
                    <Badge variant="secondary" className="ml-2 bg-[rgb(var(--color-surface-primary))]">
                      {filteredCampaignStrings.length}
                    </Badge>
                  </TabsTrigger>
                  <TabsTrigger 
                    value="floats" 
                    className="flex-1 h-full rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                  >
                    Floats
                    <Badge variant="secondary" className="ml-2 bg-[rgb(var(--color-surface-primary))]">
                      {filteredCampaignFloats.length}
                    </Badge>
                  </TabsTrigger>
                </TabsList>
              </div>

              <TabsContent value="integers" className="flex-1 min-h-0 p-0">
                <VariableTable 
                  variables={filteredCampaignIntegers} 
                  type="int" 
                  editedVars={editedCampaignVars} 
                  onVariableChange={handleCampaignVariableChange}
                  searchQuery={searchQuery}
                  className="border-0 rounded-none h-full"
                />
              </TabsContent>

              <TabsContent value="strings" className="flex-1 min-h-0 p-0">
                <VariableTable 
                  variables={filteredCampaignStrings} 
                  type="string" 
                  editedVars={editedCampaignVars} 
                  onVariableChange={handleCampaignVariableChange}
                  searchQuery={searchQuery}
                  className="border-0 rounded-none h-full"
                />
              </TabsContent>

              <TabsContent value="floats" className="flex-1 min-h-0 p-0">
                <VariableTable 
                  variables={filteredCampaignFloats} 
                  type="float" 
                  editedVars={editedCampaignVars} 
                  onVariableChange={handleCampaignVariableChange}
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
