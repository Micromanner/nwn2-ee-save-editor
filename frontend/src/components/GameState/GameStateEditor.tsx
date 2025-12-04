'use client';

import { useState } from 'react';
import { Card } from '@/components/ui/Card';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/Tabs';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { AlertCircle } from 'lucide-react';
import ReputationInfluenceTab from './ReputationInfluenceTab';
// FEATURE ON HOLD: Quest mapping produces duplicate/incorrect mappings due to NWN2's
// script architecture. Variables like "CallumState" cannot be reliably linked to journal
// entries like "LogramEyegouger" because they're set via area scripts, not dialogues.
// See backend/docs/plans and reports/QUEST_MAPPING_UX_PROBLEM.md for full analysis.
// import QuestsEditor from './QuestsEditor';
import ModuleCampaignTab from './ModuleCampaignTab';
import CampaignSettingsTab from './CampaignSettingsTab';

type GameStateTab = 'reputation' | 'moduleVariables' | 'campaignSettings';

export default function GameStateEditor() {
  const t = useTranslations();
  const { character, isLoading: characterLoading, error: characterError } = useCharacterContext();
  const [activeTab, setActiveTab] = useState<GameStateTab>('reputation');

  const isLoading = characterLoading;
  const error = characterError;

  if (!character) {
    return (
      <Card className="p-8">
        <div className="text-center text-[rgb(var(--color-text-muted))]">
          {t('placeholders.noCharacterOverview')}
        </div>
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="p-8">
        <div className="flex items-center gap-3 text-red-500">
          <AlertCircle className="w-5 h-5" />
          <span>{error}</span>
        </div>
      </Card>
    );
  }

  if (isLoading) {
    return (
      <Card className="p-8">
        <div className="flex items-center justify-center">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
        </div>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as GameStateTab)}>
        <TabsList className="w-full flex bg-transparent p-0 gap-2">
          <TabsTrigger 
            value="reputation"
            className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
          >
            {t('gameState.tabs.reputation')}
          </TabsTrigger>
          {/* FEATURE ON HOLD: Quest tab disabled - see comment at top of file */}
          {/* <TabsTrigger value="quests">
            {t('gameState.tabs.quests')}
          </TabsTrigger> */}
          <TabsTrigger 
            value="moduleVariables"
            className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
          >
            {t('gameState.tabs.moduleVariables')}
          </TabsTrigger>
          <TabsTrigger 
            value="campaignSettings"
            className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
          >
            Campaign & Variables
          </TabsTrigger>
        </TabsList>

        <TabsContent value="reputation" className="mt-4">
          <ReputationInfluenceTab />
        </TabsContent>

        {/* FEATURE ON HOLD: Quest content disabled - see comment at top of file */}
        {/* <TabsContent value="quests" className="mt-4">
          <QuestsEditor />
        </TabsContent> */}

        <TabsContent value="moduleVariables" className="mt-4">
          <ModuleCampaignTab />
        </TabsContent>

        <TabsContent value="campaignSettings" className="mt-4">
          <CampaignSettingsTab />
        </TabsContent>
      </Tabs>
    </div>
  );
}
