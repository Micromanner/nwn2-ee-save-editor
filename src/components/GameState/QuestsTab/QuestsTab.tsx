import { useEffect, useState } from 'react';
import { Card, Elevation, NonIdealState, ProgressBar } from '@blueprintjs/core';
import { GiBrokenShield } from 'react-icons/gi';
import { GameIcon } from '../../shared/GameIcon';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useQuestTransitions } from '@/hooks/useQuestTransitions';
import { gameStateAPI, type QuestGraphProgress } from '@/services/gameStateApi';
import type { SaveGraph } from './types';
import { CampaignContextCard } from './CampaignContextCard';
import { QuestList } from './QuestList';
import { QuestDetail } from './QuestDetail';

const POLL_INTERVAL_MS = 100;
const INITIAL_PROGRESS: QuestGraphProgress = {
  step: 'starting',
  progress: 0,
  message: '',
};

export function QuestsTab() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const [graph, setGraph] = useState<SaveGraph | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [progress, setProgress] = useState<QuestGraphProgress>(INITIAL_PROGRESS);
  const transitions = useQuestTransitions();

  useEffect(() => {
    let cancelled = false;
    let pollTimer: ReturnType<typeof setTimeout> | null = null;

    const poll = async () => {
      if (cancelled) return;
      try {
        const snapshot = await gameStateAPI.getQuestGraphProgress();
        if (cancelled) return;
        setProgress(prev =>
          prev.step === snapshot.step
          && prev.progress === snapshot.progress
          && prev.message === snapshot.message
            ? prev
            : snapshot,
        );
      } catch (err) {
        console.debug('[QuestsTab] progress poll failed:', err);
      }
      if (!cancelled) {
        pollTimer = setTimeout(poll, POLL_INTERVAL_MS);
      }
    };

    setIsLoading(true);
    setError(null);
    setProgress(INITIAL_PROGRESS);
    poll();

    gameStateAPI.getSaveQuestGraph()
      .then(setGraph)
      .catch(err => {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        handleError(err);
      })
      .finally(() => {
        cancelled = true;
        if (pollTimer) clearTimeout(pollTimer);
        setIsLoading(false);
      });

    return () => {
      cancelled = true;
      if (pollTimer) clearTimeout(pollTimer);
    };
  }, [handleError]);

  if (isLoading && !graph) {
    const isAnimated = progress.step !== 'ready' && progress.step !== 'error';
    return (
      <div style={{ padding: 48, display: 'flex', justifyContent: 'center' }}>
        <div style={{ width: 360, textAlign: 'center' }}>
          <div className="t-md t-bold" style={{ color: T.accent, marginBottom: 12 }}>
            {t('gameState.quests.loadingTitle')}
          </div>
          <ProgressBar
            value={progress.progress / 100}
            intent="primary"
            animate={isAnimated}
            stripes={false}
            style={{ marginBottom: 8 }}
          />
          <div className="t-sm" style={{ color: T.textMuted, minHeight: 18 }}>
            {progress.message}
          </div>
        </div>
      </div>
    );
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
          <div style={{ flex: 3, minWidth: 360, borderRight: `1px solid ${T.borderLight}`, overflowY: 'auto' }}>
            <QuestList
              quests={graph.quests}
              selectedTag={effectiveTag}
              onSelect={setSelectedTag}
            />
          </div>
          <div style={{ flex: 2, minWidth: 360, overflowY: 'auto' }}>
            <QuestDetail quest={selectedQuest} graph={graph} transitions={transitions} />
          </div>
        </div>
      </Card>
    </div>
  );
}
