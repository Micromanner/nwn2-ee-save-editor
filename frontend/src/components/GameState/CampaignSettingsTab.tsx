
import { useState, useEffect, useMemo } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { useTranslations } from '@/hooks/useTranslations';
import { Input } from '@/components/ui/Input';
import { Label } from '@/components/ui/Label';
import { Button } from '@/components/ui/Button';
import { Badge } from '@/components/ui/Badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { gameStateAPI, CampaignSettingsResponse, CampaignVariablesResponse, CampaignBackupInfo } from '@/services/gameStateApi';
import { AlertTriangle, Undo2, History, X } from 'lucide-react';

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
  const [backups, setBackups] = useState<CampaignBackupInfo[]>([]);
  const [isRestoreDialogOpen, setIsRestoreDialogOpen] = useState(false);
  const [isLoadingBackups, setIsLoadingBackups] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [restoreError, setRestoreError] = useState<string | null>(null);

  useEffect(() => {
    if (characterId) {
      loadCampaignSettings();
      loadCampaignVariables();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
    } finally {
      setIsLoadingCampaign(false);
    }
  };

  const loadBackups = async () => {
    if (!characterId) return;

    setIsLoadingBackups(true);
    setRestoreError(null);

    try {
      const data = await gameStateAPI.getCampaignBackups(characterId);
      setBackups(data.backups);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to load backups';
      setRestoreError(errorMessage);
    } finally {
      setIsLoadingBackups(false);
    }
  };

  const handleOpenRestoreDialog = () => {
    setIsRestoreDialogOpen(true);
    loadBackups();
  };

  const handleRestoreFromBackup = async (backupPath: string) => {
    if (!characterId) return;

    setIsRestoring(true);
    setRestoreError(null);

    try {
      await gameStateAPI.restoreCampaignFromBackup(characterId, backupPath);
      setIsRestoreDialogOpen(false);
      await loadCampaignSettings();
      setSaveMessage(t('gameState.campaign.restoreSuccess'));
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to restore from backup';
      setRestoreError(errorMessage);
    } finally {
      setIsRestoring(false);
    }
  };

  const formatBackupDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleString();
    } catch {
      return dateStr;
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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
      const response = await gameStateAPI.updateCampaignSettings(characterId, editedSettings as Partial<CampaignSettingsResponse>);

      if (response.warning) {
        setSaveMessage(response.warning);
      }

      await loadCampaignSettings();
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to save changes';
      setError(errorMessage);
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
    } finally {
      setIsSavingCampaign(false);
    }
  };

  const handleRevertCampaignVariable = (name: string) => {
    setEditedCampaignVars(prev => {
      const newVars = { ...prev };
      delete newVars[name];
      return newVars;
    });
  };

  const handleRevertAllCampaignChanges = () => {
    setEditedCampaignVars({});
  };

  const handleRevertAllSettings = () => {
    setEditedSettings({});
  };

  const handleRevertSetting = (field: keyof CampaignSettingsResponse) => {
    setEditedSettings(prev => {
      const newSettings = { ...prev };
      delete newSettings[field];
      return newSettings;
    });
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
              <div className="flex items-center gap-2">
                <Button
                  onClick={handleRevertAllSettings}
                  variant="outline"
                  size="sm"
                  className="text-yellow-500 border-yellow-500/50 hover:bg-yellow-500/10"
                >
                  <Undo2 className="h-4 w-4 mr-2" />
                  {t('common.revertAll')}
                </Button>
                <Button
                  onClick={handleSaveChanges}
                  disabled={isSaving}
                  size="sm"
                >
                  {isSaving ? t('actions.saving') : `${t('actions.save')} ${Object.keys(editedSettings).length} ${t('common.changes')}`}
                </Button>
              </div>
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
                <div className={`relative ${isFieldEdited('level_cap') ? 'bg-yellow-500/5 rounded-lg' : ''}`}>
                  {isFieldEdited('level_cap') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div className="pl-3 py-2">
                    <Label htmlFor="level-cap" className="flex items-center gap-2">
                      {t('gameState.campaign.levelCap')}
                      {isFieldEdited('level_cap') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <div className="flex items-center gap-2 mt-1.5">
                      <Input
                        id="level-cap"
                        type="number"
                        min={1}
                        max={40}
                        value={getCurrentValue('level_cap')}
                        onChange={(e) => handleFieldChange('level_cap', parseInt(e.target.value, 10))}
                        className={`flex-1 ${isFieldEdited('level_cap') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className={`h-9 w-9 shrink-0 ${isFieldEdited('level_cap') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                        onClick={() => handleRevertSetting('level_cap')}
                        title={t('common.revert')}
                      >
                        <Undo2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>

                <div className={`relative ${isFieldEdited('xp_cap') ? 'bg-yellow-500/5 rounded-lg' : ''}`}>
                  {isFieldEdited('xp_cap') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div className="pl-3 py-2">
                    <Label htmlFor="xp-cap" className="flex items-center gap-2">
                      {t('gameState.campaign.xpCap')}
                      {isFieldEdited('xp_cap') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <div className="flex items-center gap-2 mt-1.5">
                      <Input
                        id="xp-cap"
                        type="number"
                        min={0}
                        value={getCurrentValue('xp_cap')}
                        onChange={(e) => handleFieldChange('xp_cap', parseInt(e.target.value, 10))}
                        className={`flex-1 ${isFieldEdited('xp_cap') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className={`h-9 w-9 shrink-0 ${isFieldEdited('xp_cap') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                        onClick={() => handleRevertSetting('xp_cap')}
                        title={t('common.revert')}
                      >
                        <Undo2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className={`relative ${isFieldEdited('companion_xp_weight') ? 'bg-yellow-500/5 rounded-lg' : ''}`}>
                  {isFieldEdited('companion_xp_weight') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div className="pl-3 py-2">
                    <Label htmlFor="companion-xp-weight" className="flex items-center gap-2">
                      {t('gameState.campaign.companionXpWeight')}
                      {isFieldEdited('companion_xp_weight') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <div className="flex items-center gap-2 mt-1.5">
                      <Input
                        id="companion-xp-weight"
                        type="number"
                        min={0}
                        max={1}
                        step={0.1}
                        value={getCurrentValue('companion_xp_weight')}
                        onChange={(e) => handleFieldChange('companion_xp_weight', parseFloat(e.target.value))}
                        className={`flex-1 ${isFieldEdited('companion_xp_weight') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className={`h-9 w-9 shrink-0 ${isFieldEdited('companion_xp_weight') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                        onClick={() => handleRevertSetting('companion_xp_weight')}
                        title={t('common.revert')}
                      >
                        <Undo2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>

                <div className={`relative ${isFieldEdited('henchman_xp_weight') ? 'bg-yellow-500/5 rounded-lg' : ''}`}>
                  {isFieldEdited('henchman_xp_weight') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div className="pl-3 py-2">
                    <Label htmlFor="henchman-xp-weight" className="flex items-center gap-2">
                      {t('gameState.campaign.henchmanXpWeight')}
                      {isFieldEdited('henchman_xp_weight') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <div className="flex items-center gap-2 mt-1.5">
                      <Input
                        id="henchman-xp-weight"
                        type="number"
                        min={0}
                        max={1}
                        step={0.1}
                        value={getCurrentValue('henchman_xp_weight')}
                        onChange={(e) => handleFieldChange('henchman_xp_weight', parseFloat(e.target.value))}
                        className={`flex-1 ${isFieldEdited('henchman_xp_weight') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                      />
                      <Button
                        variant="ghost"
                        size="icon"
                        className={`h-9 w-9 shrink-0 ${isFieldEdited('henchman_xp_weight') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                        onClick={() => handleRevertSetting('henchman_xp_weight')}
                        title={t('common.revert')}
                      >
                        <Undo2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Gameplay Flags */}
            <div className="space-y-4">
              <h3 className="text-sm font-medium uppercase tracking-wider text-[rgb(var(--color-text-muted))] border-b border-[rgb(var(--color-border))] pb-2">
                {t('gameState.campaign.gameplayFlags')}
              </h3>

              <div className="space-y-3">
                <div className={`relative flex items-center justify-between p-3 rounded-lg ${isFieldEdited('attack_neutrals') ? 'bg-yellow-500/5' : 'bg-[rgb(var(--color-surface-secondary))]'}`}>
                  {isFieldEdited('attack_neutrals') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div>
                    <Label htmlFor="attack-neutrals" className="flex items-center gap-2">
                      {t('gameState.campaign.attackNeutrals')}
                      {isFieldEdited('attack_neutrals') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.attackNeutralsDesc')}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      id="attack-neutrals"
                      type="number"
                      min={0}
                      max={1}
                      value={getCurrentValue('attack_neutrals')}
                      onChange={(e) => handleFieldChange('attack_neutrals', parseInt(e.target.value, 10))}
                      className={`w-16 h-8 text-center ${isFieldEdited('attack_neutrals') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      className={`h-8 w-8 shrink-0 ${isFieldEdited('attack_neutrals') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                      onClick={() => handleRevertSetting('attack_neutrals')}
                      title={t('common.revert')}
                    >
                      <Undo2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                <div className={`relative flex items-center justify-between p-3 rounded-lg ${isFieldEdited('auto_xp_award') ? 'bg-yellow-500/5' : 'bg-[rgb(var(--color-surface-secondary))]'}`}>
                  {isFieldEdited('auto_xp_award') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div>
                    <Label htmlFor="auto-xp-award" className="flex items-center gap-2">
                      {t('gameState.campaign.autoXpAward')}
                      {isFieldEdited('auto_xp_award') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.autoXpAwardDesc')}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      id="auto-xp-award"
                      type="number"
                      min={0}
                      max={1}
                      value={getCurrentValue('auto_xp_award')}
                      onChange={(e) => handleFieldChange('auto_xp_award', parseInt(e.target.value, 10))}
                      className={`w-16 h-8 text-center ${isFieldEdited('auto_xp_award') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      className={`h-8 w-8 shrink-0 ${isFieldEdited('auto_xp_award') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                      onClick={() => handleRevertSetting('auto_xp_award')}
                      title={t('common.revert')}
                    >
                      <Undo2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                <div className={`relative flex items-center justify-between p-3 rounded-lg ${isFieldEdited('journal_sync') ? 'bg-yellow-500/5' : 'bg-[rgb(var(--color-surface-secondary))]'}`}>
                  {isFieldEdited('journal_sync') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div>
                    <Label htmlFor="journal-sync" className="flex items-center gap-2">
                      {t('gameState.campaign.journalSync')}
                      {isFieldEdited('journal_sync') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.journalSyncDesc')}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      id="journal-sync"
                      type="number"
                      min={0}
                      max={1}
                      value={getCurrentValue('journal_sync')}
                      onChange={(e) => handleFieldChange('journal_sync', parseInt(e.target.value, 10))}
                      className={`w-16 h-8 text-center ${isFieldEdited('journal_sync') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      className={`h-8 w-8 shrink-0 ${isFieldEdited('journal_sync') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                      onClick={() => handleRevertSetting('journal_sync')}
                      title={t('common.revert')}
                    >
                      <Undo2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                <div className={`relative flex items-center justify-between p-3 rounded-lg ${isFieldEdited('no_char_changing') ? 'bg-yellow-500/5' : 'bg-[rgb(var(--color-surface-secondary))]'}`}>
                  {isFieldEdited('no_char_changing') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div>
                    <Label htmlFor="no-char-changing" className="flex items-center gap-2">
                      {t('gameState.campaign.lockCharChanges')}
                      {isFieldEdited('no_char_changing') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.lockCharChangesDesc')}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      id="no-char-changing"
                      type="number"
                      min={0}
                      max={1}
                      value={getCurrentValue('no_char_changing')}
                      onChange={(e) => handleFieldChange('no_char_changing', parseInt(e.target.value, 10))}
                      className={`w-16 h-8 text-center ${isFieldEdited('no_char_changing') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      className={`h-8 w-8 shrink-0 ${isFieldEdited('no_char_changing') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                      onClick={() => handleRevertSetting('no_char_changing')}
                      title={t('common.revert')}
                    >
                      <Undo2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>

                <div className={`relative flex items-center justify-between p-3 rounded-lg ${isFieldEdited('use_personal_reputation') ? 'bg-yellow-500/5' : 'bg-[rgb(var(--color-surface-secondary))]'}`}>
                  {isFieldEdited('use_personal_reputation') && (
                    <div className="absolute left-0 top-0 bottom-0 w-1 bg-yellow-500 rounded-l" />
                  )}
                  <div>
                    <Label htmlFor="use-personal-rep" className="flex items-center gap-2">
                      {t('gameState.campaign.usePersonalRep')}
                      {isFieldEdited('use_personal_reputation') && (
                        <Badge variant="secondary" className="text-xs h-5 px-1.5 bg-yellow-500/20 text-yellow-500 border-yellow-500/20">Modified</Badge>
                      )}
                    </Label>
                    <p className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                      {t('gameState.campaign.usePersonalRepDesc')}
                    </p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Input
                      id="use-personal-rep"
                      type="number"
                      min={0}
                      max={1}
                      value={getCurrentValue('use_personal_reputation')}
                      onChange={(e) => handleFieldChange('use_personal_reputation', parseInt(e.target.value, 10))}
                      className={`w-16 h-8 text-center ${isFieldEdited('use_personal_reputation') ? 'border-yellow-500/50 focus-visible:ring-yellow-500' : ''}`}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      className={`h-8 w-8 shrink-0 ${isFieldEdited('use_personal_reputation') ? 'text-yellow-500 hover:text-yellow-400 hover:bg-yellow-500/10' : 'invisible'}`}
                      onClick={() => handleRevertSetting('use_personal_reputation')}
                      title={t('common.revert')}
                    >
                      <Undo2 className="h-4 w-4" />
                    </Button>
                  </div>
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
              <div className="md:col-span-2 flex items-center justify-between">
                <div>
                  <span className="text-[rgb(var(--color-text-muted))]">{t('gameState.campaign.campaignFile')}:</span>
                  <span className="ml-2 font-mono text-xs text-[rgb(var(--color-text-muted))]">{settings.campaign_file_path}</span>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleOpenRestoreDialog}
                  className="ml-4"
                >
                  <History className="h-4 w-4 mr-2" />
                  {t('gameState.campaign.restoreBackup')}
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {isRestoreDialogOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 animate-in fade-in duration-200">
          <Card className="w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200 bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] shadow-2xl">
            <div className="p-4 border-b border-[rgb(var(--color-border))] flex justify-between items-center">
              <h2 className="text-lg font-semibold">{t('gameState.campaign.restoreBackupTitle')}</h2>
              <Button variant="ghost" size="sm" onClick={() => setIsRestoreDialogOpen(false)} className="h-8 w-8 p-0">
                <X className="h-4 w-4" />
              </Button>
            </div>

            <div className="p-4">
              <p className="text-sm text-[rgb(var(--color-text-muted))] mb-4">
                {t('gameState.campaign.restoreBackupDesc')}
              </p>

              {restoreError && (
                <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
                  {restoreError}
                </div>
              )}

              {isLoadingBackups ? (
                <div className="py-8 text-center text-[rgb(var(--color-text-muted))]">
                  {t('common.loading')}
                </div>
              ) : backups.length === 0 ? (
                <div className="py-8 text-center text-[rgb(var(--color-text-muted))]">
                  {t('gameState.campaign.noBackups')}
                </div>
              ) : (
                <div className="space-y-2 max-h-[300px] overflow-y-auto">
                  {backups.map((backup) => (
                    <div
                      key={backup.path}
                      className="flex items-center justify-between p-3 rounded-lg bg-[rgb(var(--color-surface-secondary))] hover:bg-[rgb(var(--color-surface-tertiary))] transition-colors"
                    >
                      <div className="flex-1 min-w-0">
                        <div className="font-mono text-sm truncate" title={backup.filename}>
                          {backup.filename}
                        </div>
                        <div className="text-xs text-[rgb(var(--color-text-muted))] mt-1">
                          {formatBackupDate(backup.created)} - {formatBytes(backup.size_bytes)}
                        </div>
                      </div>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleRestoreFromBackup(backup.path)}
                        disabled={isRestoring}
                        className="ml-3"
                      >
                        {isRestoring ? t('common.loading') : t('common.restore')}
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </Card>
        </div>
      )}

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
                <>
                  <Button
                    onClick={handleRevertAllCampaignChanges}
                    variant="outline"
                    size="sm"
                    className="text-yellow-500 border-yellow-500/50 hover:bg-yellow-500/10"
                  >
                    <Undo2 className="h-4 w-4 mr-2" />
                    {t('common.revertAll')}
                  </Button>
                  <Button
                    onClick={handleSaveCampaignChanges}
                    disabled={isSavingCampaign}
                    size="sm"
                    className="min-w-[120px]"
                  >
                    {isSavingCampaign ? t('common.saving') : `${t('common.save')} ${Object.keys(editedCampaignVars).length} ${t('common.changes')}`}
                  </Button>
                </>
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
                  onRevertVariable={handleRevertCampaignVariable}
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
                  onRevertVariable={handleRevertCampaignVariable}
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
                  onRevertVariable={handleRevertCampaignVariable}
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
