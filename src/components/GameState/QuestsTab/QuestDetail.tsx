import { useEffect, useState } from 'react';
import { Button, Callout, Icon, NonIdealState, Switch, Tag, Tooltip } from '@blueprintjs/core';
import { LoadingState } from '@/components/shared';
import { GiScrollQuill } from 'react-icons/gi';
import { T } from '../../theme';
import { GameIcon } from '../../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useTlkResolver } from '@/hooks/useTlkResolver';
import type { QuestTransitionsResolver } from '@/hooks/useQuestTransitions';
import type { PendingQuestEdit, QuestSummary, SaveGraph, TransitionNode } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';
import { effectiveSource, questDisplayName, questStates, questStatus } from './utils';
import { TransitionsList } from './TransitionsList';
import { LiveOverlay } from './LiveOverlay';
import { QuestEditDialog } from './QuestEditDialog';

export function QuestDetail({
  quest, graph, transitions, pending, onApplied,
}: {
  quest: QuestSummary | null;
  graph: SaveGraph;
  transitions: QuestTransitionsResolver;
  pending: PendingQuestEdit[];
  onApplied: () => void;
}) {
  const t = useTranslations();
  const [target, setTarget] = useState<number | null>(null);
  const [technical, setTechnical] = useState(false);
  const tlk = useTlkResolver();
  const { resolveMany } = useModuleNameResolver(graph.modules);

  const entry = quest ? transitions.get(quest.tag) : undefined;
  const loadedTransitions: TransitionNode[] | null =
    entry?.state === 'resolved' ? entry.transitions : null;

  useEffect(() => {
    if (quest) transitions.load(quest.tag);
    // `transitions.load` is a stable useCallback; omitted from deps so this only
    // re-fires when the selected quest actually changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [quest?.tag]);

  useEffect(() => {
    if (loadedTransitions) {
      tlk.prime(loadedTransitions.map(tr => tr.text_strref));
    }
    // `tlk.prime` is a stable useCallback; skip `tlk` from deps to avoid
    // re-firing on every parent render (the resolver returns a fresh object).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadedTransitions]);

  if (!quest) {
    return (
      <div style={{ padding: 48 }}>
        <NonIdealState
          icon={<GameIcon icon={GiScrollQuill} size={40} />}
          title={t('gameState.quests.detail.selectQuest')}
        />
      </div>
    );
  }

  const status = questStatus(quest);
  const currentEntry = quest.live_state != null
    ? quest.category.entries.find(e => e.id === quest.live_state)
    : null;
  const source = effectiveSource(quest);
  const states = questStates(quest, loadedTransitions ?? []);
  const entryLabel = (state: number) => t('gameState.quests.detail.entry', { n: state });
  const statusColor = status === 'complete' ? T.positive : status === 'active' ? T.text : T.textMuted;

  return (
    <div>
      <div style={{ padding: '12px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
        <div className="t-bold" style={{ color: T.text, marginBottom: 8 }}>{questDisplayName(quest)}</div>
        <div style={{ display: 'grid', gridTemplateColumns: 'auto 1fr', columnGap: 16, rowGap: 4, alignItems: 'baseline' }}>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.status')}</span>
          <span className="t-semibold" style={{ color: statusColor }}>{t(`gameState.quests.status.${status}`)}</span>

          {quest.live_state != null && <>
            <span style={{ color: T.textMuted }}>{t('gameState.quests.list.currentEntry')}</span>
            <span style={{ color: T.text }}>
              <span className="t-semibold">{entryLabel(quest.live_state)}</span>
              {currentEntry && <>: {currentEntry.text}</>}
            </span>
          </>}

          <span style={{ color: T.textMuted }}>{t('gameState.quests.detail.xpReward')}</span>
          <span className="t-semibold" style={{ color: T.text }}>{String(quest.category.xp || 0)}</span>

          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.module')}</span>
          <span className="t-semibold" style={{ color: T.text }}>{quest.defined_in.length === 0 ? '-' : resolveMany(quest.defined_in)}</span>
        </div>
        {technical && <div className="t-mono" style={{ color: T.textMuted, marginTop: 8 }}>
          {quest.tag}
          {quest.defined_in.map(id => <span key={id}> &middot; {id}</span>)}
          {source !== 'normal' && <> &middot; <span style={{ color: T.negative }}>{t(`gameState.quests.sourceBadge.${source}`)}</span></>}
        </div>}
      </div>

      <div style={{ padding: '12px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
        {quest.pending_state !== undefined && (
          <Callout intent="primary" style={{ marginBottom: 12 }}>
            {t('gameState.quests.detail.pendingBanner', { entry: entryLabel(quest.pending_state) })}
          </Callout>
        )}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 12, marginBottom: 4 }}>
          <span className="t-bold" style={{ color: T.textMuted, display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            {t('gameState.quests.detail.entries')}
            <Tooltip content={t('gameState.quests.edit.orderExplanation')} placement="top" targetProps={{ style: { display: 'inline-flex' } }}>
              <Icon icon="info-sign" size={12} style={{ color: T.textMuted }} />
            </Tooltip>
          </span>
          <Switch checked={technical} onChange={e => setTechnical(e.currentTarget.checked)} label={t('gameState.quests.detail.showScriptDetails')} style={{ margin: 0, whiteSpace: 'nowrap' }} />
        </div>
        {!entry || entry.state === 'pending' ? <LoadingState variant="inline" /> : entry.state === 'error' ?
          <p>{t('gameState.quests.detail.transitionsFailed')}: {entry.message}</p> : null}
        {states.map(state => {
            const definition = quest.category.entries.find(e => e.id === state);
            const isCurrent = state === quest.live_state;
            const isPending = state === quest.pending_state;
            const routes = (loadedTransitions ?? []).filter(tr => tr.new_state === state);
            return <div key={state} style={{ display: 'flex', gap: 10, alignItems: 'flex-start', borderTop: `1px solid ${T.borderLight}`, padding: '8px 0' }}>
              <span className="t-mono" style={{ color: T.textMuted, minWidth: 28, textAlign: 'right', paddingTop: 1 }}>{state}</span>
              <div style={{ flex: 1, minWidth: 0 }}>
                <span style={{ color: definition ? T.text : T.textMuted, fontStyle: definition ? 'normal' : 'italic' }}>
                  {definition?.text || t('gameState.quests.edit.undefinedState')}
                </span>
                {isCurrent && <Tag minimal intent="primary" style={{ marginLeft: 6 }}>{t('gameState.quests.detail.current')}</Tag>}
                {isPending && <Tag minimal intent="warning" style={{ marginLeft: 6 }}>{t('gameState.quests.edit.pending')}</Tag>}
                {definition?.final && <Tag minimal intent="success" style={{ marginLeft: 6 }}>{t('gameState.quests.detail.endsQuest')}</Tag>}
                {technical && <div style={{ marginTop: 6 }}>
                  <TransitionsList quest={quest} transitions={routes} tlk={tlk} modules={graph.modules} />
                </div>}
              </div>
              <Button small intent="primary" disabled={isCurrent && !isPending} onClick={() => setTarget(state)} style={{ flexShrink: 0 }}>
                {t('gameState.quests.edit.jump')}
              </Button>
            </div>;
          })}
      </div>

      {technical && <div style={{ padding: '12px 16px' }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.relatedVariables')}
        </div>
        <LiveOverlay tag={quest.tag} pending={pending} modules={graph.modules} />
      </div>}
      {target !== null && <QuestEditDialog key={`${quest.tag}:${target}`} quest={quest} target={target} modules={graph.modules}
        transitions={loadedTransitions ?? []} pending={pending} tlk={tlk} onClose={() => setTarget(null)} onApplied={onApplied} />}
    </div>
  );
}
