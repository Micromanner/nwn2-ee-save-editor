import { useEffect, useState } from 'react';
import { Card, Elevation, NonIdealState, Spinner } from '@blueprintjs/core';
import { GiBrokenShield } from 'react-icons/gi';
import { GameIcon } from '../../shared/GameIcon';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { gameStateAPI } from '@/services/gameStateApi';
import type { SaveGraph } from './types';
import { CampaignContextCard } from './CampaignContextCard';
import { QuestList } from './QuestList';
import { QuestDetail } from './QuestDetail';

export function QuestsTab() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const [graph, setGraph] = useState<SaveGraph | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTag, setSelectedTag] = useState<string | null>(null);

  useEffect(() => {
    setIsLoading(true);
    setError(null);
    gameStateAPI.getSaveQuestGraph()
      .then(setGraph)
      .catch(err => {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        handleError(err);
      })
      .finally(() => setIsLoading(false));
  }, [handleError]);

  if (isLoading && !graph) {
    return <div style={{ padding: 48, textAlign: 'center' }}><Spinner size={32} /></div>;
  }

  if (error && !graph) {
    return (
      <div style={{ padding: 24 }}>
        <NonIdealState
          icon={<GameIcon icon={GiBrokenShield} size={40} />}
          title={t('gameState.quests.failedToLoad')}
          description={error}
        />
      </div>
    );
  }

  if (!graph) return null;

  const effectiveTag = selectedTag ?? graph.quests[0]?.tag ?? null;
  const selectedQuest = graph.quests.find(q => q.tag === effectiveTag) ?? null;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      <CampaignContextCard campaign={graph.campaign} modules={graph.modules} orphans={graph.orphans} />

      <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
        <div style={{ display: 'flex', minHeight: 480, maxHeight: 'calc(100vh - 280px)' }}>
          <div style={{ width: '40%', minWidth: 360, borderRight: `1px solid ${T.borderLight}`, overflowY: 'auto' }}>
            <QuestList
              quests={graph.quests}
              selectedTag={effectiveTag}
              onSelect={setSelectedTag}
            />
          </div>
          <div style={{ flex: 1, overflowY: 'auto' }}>
            <QuestDetail quest={selectedQuest} graph={graph} />
          </div>
        </div>
      </Card>
    </div>
  );
}
