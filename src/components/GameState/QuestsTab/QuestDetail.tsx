import { useEffect, useState } from 'react';
import { Button, InputGroup, NonIdealState } from '@blueprintjs/core';
import { LoadingState } from '@/components/shared';
import { GiScrollQuill } from 'react-icons/gi';
import { T } from '../../theme';
import { GameIcon } from '../../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useTlkResolver } from '@/hooks/useTlkResolver';
import type { QuestTransitionsResolver } from '@/hooks/useQuestTransitions';
import type { PendingQuestEdit, QuestSummary, SaveGraph, TransitionNode } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';
import { effectiveSource, questStates } from './utils';
import { TransitionsList } from './TransitionsList';
import { LiveOverlay } from './LiveOverlay';
import { QuestEditDialog } from './QuestEditDialog';
import { display } from '@/utils/dataHelpers';

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
  const [numeric, setNumeric] = useState('');
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

  const matchingEntry = quest.live_state != null
    ? quest.category.entries.find(e => e.id === quest.live_state)
    : null;
  const source = effectiveSource(quest);

  const resolvedDefinedIn = quest.defined_in.length === 0 ? '-' : resolveMany(quest.defined_in);

  return (
    <div>
      <div style={{ padding: '12px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.header')}
        </div>
        <div className="t-md" style={{ display: 'grid', gridTemplateColumns: 'auto 1fr auto 1fr', columnGap: 16, rowGap: 4, alignItems: 'baseline' }}>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.tag')}</span>
          <span className="t-semibold t-mono" style={{ color: T.text }}>{quest.tag}</span>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.name')}</span>
          <span className="t-semibold" style={{ color: T.text }}>{quest.category.name || quest.tag}</span>

          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.priority')}</span>
          <span className="t-semibold" style={{ color: T.text }}>{quest.category.priority || '-'}</span>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.xp')}</span>
          <span className="t-semibold" style={{ color: T.text }}>{String(quest.category.xp || 0)}</span>

          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.source')}</span>
          <span className="t-semibold" style={{ color: source !== 'normal' ? T.negative : T.text }}>
            {t(`gameState.quests.sourceBadge.${source}`)}
          </span>
          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.definedIn')}</span>
          <span className="t-semibold" title={quest.defined_in.join(', ')} style={{ color: T.text }}>{resolvedDefinedIn}</span>

          <span style={{ color: T.textMuted }}>{t('gameState.quests.list.live')}</span>
          <span className="t-semibold" style={{ gridColumn: '2 / -1', color: T.text }}>
            {quest.live_state == null ? (
              <span style={{ color: T.textMuted }}>{t('gameState.quests.detail.notStarted')}</span>
            ) : (
              <>
                <span className="t-mono">{quest.live_state}</span>
                {matchingEntry && <> &mdash; {matchingEntry.text}</>}
                {matchingEntry?.final && <span style={{ color: T.positive, marginLeft: 6 }}>({t('gameState.quests.detail.final')})</span>}
              </>
            )}
          </span>
        </div>
      </div>

      <div style={{ padding: '12px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
        <p>{t('gameState.quests.edit.pending')}: {display(quest.pending_state)}</p>
        <p>{t('gameState.quests.edit.orderExplanation')}</p>
        {!entry || entry.state === 'pending' ? <LoadingState variant="inline" /> : entry.state === 'error' ?
          <p>{t('gameState.quests.detail.transitionsFailed')}: {entry.message}</p> : null}
        {questStates(quest, loadedTransitions ?? []).map(state => {
            const definition = quest.category.entries.find(e => e.id === state);
            return <details key={state} style={{ borderTop: `1px solid ${T.borderLight}`, padding: '8px 0' }}>
              <summary style={{ cursor: 'pointer' }}>
                <strong>{state}</strong> &mdash; {definition?.text || t('gameState.quests.edit.undefinedState')}
                {definition?.final && <span> ({t('gameState.quests.detail.final')})</span>}
                {state === quest.live_state && <span> ({t('gameState.quests.edit.saved')})</span>}
                {state === quest.pending_state && <span> ({t('gameState.quests.edit.pending')})</span>}
              </summary>
              <Button small intent="primary" onClick={() => setTarget(state)} style={{ margin: '8px 0' }}>
                {t('gameState.quests.edit.setState')}
              </Button>
              <TransitionsList quest={quest} transitions={(loadedTransitions ?? []).filter(tr => tr.new_state === state)} tlk={tlk} modules={graph.modules} />
            </details>;
          })}
        <details style={{ marginTop: 12 }}>
          <summary>{t('gameState.quests.edit.advanced')}</summary>
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <InputGroup type="number" min={0} max={4294967295} step={1} value={numeric} onChange={e => setNumeric(e.target.value)} aria-label={t('gameState.quests.edit.numericState')} />
            <Button disabled={!/^\d+$/.test(numeric) || !Number.isSafeInteger(Number(numeric)) || Number(numeric) > 4294967295}
              onClick={() => setTarget(Number(numeric))}>{t('gameState.quests.edit.setState')}</Button>
          </div>
        </details>
      </div>

      <div style={{ padding: '12px 16px' }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.liveOverlay')}
        </div>
        <LiveOverlay tag={quest.tag} pending={pending} />
      </div>
      {target !== null && <QuestEditDialog key={`${quest.tag}:${target}`} quest={quest} target={target}
        transitions={loadedTransitions ?? []} pending={pending} tlk={tlk} onClose={() => setTarget(null)} onApplied={onApplied} />}
    </div>
  );
}
