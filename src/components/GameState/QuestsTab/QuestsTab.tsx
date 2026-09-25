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
import { useModuleNameResolver } from './useModuleNameResolver';
import { T } from '../../theme';
import { GameIcon } from '../../shared/GameIcon';
import { GiAnticlockwiseRotation } from 'react-icons/gi';

const MODIFIED = '#d97706';
import { questDisplayName } from './utils';

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
  const { resolve: resolveModule } = useModuleNameResolver(graph?.modules ?? []);
  const refreshData = useCallback(() => setRefresh(r => r + 1), []);

  useEffect(() => {
    window.addEventListener('save-changed', refreshData);
    return () => window.removeEventListener('save-changed', refreshData);
  }, [refreshData]);

  // Undo and redo can unstage or restage an edit.
  useEffect(() => {
    const reloadPending = () => {
      invoke<PendingQuestEdit[]>('save_list_quest_edits').then(setPending).catch(handleError);
    };
    window.addEventListener('quest-edits-changed', reloadPending);
    return () => window.removeEventListener('quest-edits-changed', reloadPending);
  }, [handleError]);

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
  if (!graph) return <div style={{ padding: '96px 48px', maxWidth: 520, margin: '0 auto', textAlign: 'center' }}>
    <div className="t-semibold" style={{ marginBottom: 16 }}>
      {t('gameState.quests.loadingTitle')}{progress?.message ? `: ${progress.message}` : ''}
    </div>
    <ProgressBar value={(progress?.progress ?? 0) / 100} />
  </div>;
  const blocker = graph.orphans.find(o => o.kind === 'enhanced_edition_required' || o.kind === 'platform_unsupported');
  if (blocker) return <NonIdealState
    icon="warning-sign"
    title={t(`gameState.quests.unavailable.${blocker.kind}.title`)}
    description={t(`gameState.quests.unavailable.${blocker.kind}.description`)}
  />;
  const quests = graph.quests.map(q => {
    const staged = pending.find(p => p.tag === q.tag)?.target;
    return { ...q, live_state: staged ?? q.live_state, pending_state: staged };
  });
  const questNames = new Map(quests.map(q => [q.tag, questDisplayName(q)]));
  const effectiveTag = selectedTag ?? quests[0]?.tag ?? null;
  const selectedQuest = quests.find(q => q.tag === effectiveTag) ?? null;

  return <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
    <CampaignContextCard campaign={graph.campaign} modules={graph.modules} orphans={graph.orphans} />
    {error && <Button onClick={refreshData}>{t('gameState.quests.edit.refresh')}</Button>}
    {pending.length > 0 && <Card style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '8px 16px' }}>
        <span className="t-bold" style={{ color: T.textMuted }}>{t('gameState.quests.edit.pendingTitle', { count: pending.length })}</span>
        <Button small minimal icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} style={{ color: MODIFIED }} disabled={busy} onClick={() => void discard(null)}>{t('gameState.quests.edit.discardAll')}</Button>
      </div>
      {pending.map(edit => <div key={edit.tag} style={{ display: 'flex', alignItems: 'center', gap: 16, padding: '6px 16px', borderTop: `1px solid ${T.borderLight}` }}>
        <span className="t-semibold" style={{ color: T.text, flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{questNames.get(edit.tag) ?? edit.tag}</span>
        <span className="t-mono" style={{ color: T.text }}>{display(edit.saved_state)} → {edit.target}</span>
        {edit.source && <span style={{ color: T.textMuted }}>{resolveModule(edit.source.module)} / {edit.source.dlg} / {edit.source.node}</span>}
        <Button small minimal icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} style={{ color: MODIFIED }} disabled={busy} onClick={() => void discard(edit.tag)} title={t('gameState.quests.edit.discard')} />
      </div>)}
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
