import { useCallback, useEffect, useState } from 'react';
import { Button, Card, NonIdealState, ProgressBar } from '@blueprintjs/core';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useQuestTransitions } from '@/hooks/useQuestTransitions';
import { gameStateAPI, type QuestGraphProgress } from '@/services/gameStateApi';
import { display } from '@/utils/dataHelpers';
import type { PendingQuestEdit, SaveGraph } from './types';
import { CampaignContextCard } from './CampaignContextCard';
import { QuestList } from './QuestList';
import { QuestDetail } from './QuestDetail';
import { T } from '../../theme';

export function QuestsTab() {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const [graph, setGraph] = useState<SaveGraph | null>(null);
  const [pending, setPending] = useState<PendingQuestEdit[]>([]);
  const [selectedTag, setSelectedTag] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<QuestGraphProgress | null>(null);
  const [refresh, setRefresh] = useState(0);
  const transitions = useQuestTransitions();
  const refreshData = useCallback(() => setRefresh(r => r + 1), []);

  useEffect(() => {
    window.addEventListener('save-changed', refreshData);
    return () => window.removeEventListener('save-changed', refreshData);
  }, [refreshData]);

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      try {
        const value = await gameStateAPI.getQuestGraphProgress();
        if (active) setProgress(value);
      } catch { /* Main request reports loading failures. */ }
      if (active) timer = setTimeout(poll, 250);
    };
    setError(false); setBusy(true);
    void poll();
    Promise.all([gameStateAPI.getSaveQuestGraph(), invoke<PendingQuestEdit[]>('save_list_quest_edits')])
      .then(([value, edits]) => { if (active) { setGraph(value); setPending(edits); } })
      .catch(error => { if (active) { setError(true); handleError(error); } })
      .finally(() => { if (active) setBusy(false); active = false; clearTimeout(timer); });
    return () => { active = false; clearTimeout(timer); };
  }, [handleError, refresh]);

  const discard = async (tag: string | null) => {
    try {
      await invoke('save_discard_quest_edits', { tag });
      window.dispatchEvent(new Event('quest-edits-changed'));
      refreshData();
    } catch (error) { handleError(error); }
  };

  if (!graph && error) return <NonIdealState title={t('gameState.quests.failedToLoad')}
    action={<Button onClick={refreshData}>{t('gameState.quests.edit.refresh')}</Button>} />;
  if (!graph) return <div style={{ padding: 48 }}>
    <p>{t('gameState.quests.loadingTitle')}</p>
    <ProgressBar value={(progress?.progress ?? 0) / 100} />
    <p>{progress?.message}</p>
  </div>;
  const quests = graph.quests.map(q => ({ ...q, pending_state: pending.find(p => p.tag === q.tag)?.target }));
  const effectiveTag = selectedTag ?? quests[0]?.tag ?? null;
  const selectedQuest = quests.find(q => q.tag === effectiveTag) ?? null;

  return <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
    <CampaignContextCard campaign={graph.campaign} modules={graph.modules} orphans={graph.orphans} />
    {error && <Button onClick={refreshData}>{t('gameState.quests.edit.refresh')}</Button>}
    {pending.length > 0 && <Card>
      <strong>{t('gameState.quests.edit.pendingTitle')}</strong>
      <p>{t('gameState.quests.edit.stagingExplanation')}</p>
      {pending.map(edit => <details key={edit.tag} style={{ marginBottom: 8 }}>
        <summary>{edit.tag}: {display(edit.saved_state)} → {edit.target}</summary>
        {edit.source && <p>{edit.source.module} / {edit.source.dlg} / {edit.source.node}</p>}
        {edit.writes.map((w, i) => <p key={i}>{w.key.module ?? t('gameState.quests.edit.campaign')} / {w.key.name} ({w.key.value_type}): {display(w.saved)} → {display(w.after)}</p>)}
        <Button small disabled={busy} onClick={() => void discard(edit.tag)}>{t('gameState.quests.edit.discard')}</Button>
      </details>)}
      <Button small disabled={busy} onClick={() => void discard(null)}>{t('gameState.quests.edit.discardAll')}</Button>
    </Card>}
    <Card style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
      <div style={{ display: 'flex', minHeight: 480, maxHeight: 'calc(100vh - 280px)' }}>
        <div style={{ flex: 3, minWidth: 360, borderRight: `1px solid ${T.borderLight}`, overflowY: 'auto' }}>
          <QuestList quests={quests} modules={graph.modules} selectedTag={effectiveTag} onSelect={setSelectedTag} />
        </div>
        <div style={{ flex: 2, minWidth: 360, overflowY: 'auto' }}>
          <QuestDetail key={effectiveTag} quest={selectedQuest} graph={graph} transitions={transitions} pending={pending} onApplied={refreshData} />
        </div>
      </div>
    </Card>
  </div>;
}
