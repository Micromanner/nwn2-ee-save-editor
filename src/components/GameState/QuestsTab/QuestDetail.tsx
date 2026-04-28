import { useEffect } from 'react';
import { HTMLTable, NonIdealState, Spinner } from '@blueprintjs/core';
import { GiScrollQuill } from 'react-icons/gi';
import { T } from '../../theme';
import { GameIcon } from '../../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useTlkResolver } from '@/hooks/useTlkResolver';
import type { QuestTransitionsResolver } from '@/hooks/useQuestTransitions';
import type { QuestSummary, SaveGraph, TransitionNode } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';
import { effectiveSource, selectedRowStyle } from './utils';
import { TransitionsList } from './TransitionsList';
import { LiveOverlay } from './LiveOverlay';

export function QuestDetail({
  quest, graph, transitions,
}: {
  quest: QuestSummary | null;
  graph: SaveGraph;
  transitions: QuestTransitionsResolver;
}) {
  const t = useTranslations();
  const tlk = useTlkResolver();

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

  const { resolveMany } = useModuleNameResolver(graph.modules);
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
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.entries')}
        </div>
        {quest.category.entries.length === 0 ? (
          <div style={{ color: T.textMuted, fontStyle: 'italic' }}>{t('gameState.quests.detail.noEntries')}</div>
        ) : (
          <HTMLTable compact striped bordered style={{ width: '100%', tableLayout: 'fixed' }}>
            <colgroup><col style={{ width: 60 }} /><col /><col style={{ width: 64 }} /></colgroup>
            <thead>
              <tr>
                <th>{t('gameState.quests.list.live')}</th>
                <th>{t('gameState.quests.list.name')}</th>
                <th>{t('gameState.quests.detail.final')}</th>
              </tr>
            </thead>
            <tbody>
              {quest.category.entries.slice().sort((a, b) => a.id - b.id).map(e => {
                const isCurrent = e.id === quest.live_state;
                return (
                  <tr
                    key={e.id}
                    style={selectedRowStyle(isCurrent, T.accent)}
                  >
                    <td className="t-mono" style={{ textAlign: 'right' }}>{e.id}</td>
                    <td>{e.text}</td>
                    <td style={{ textAlign: 'center', color: e.final ? T.positive : T.textMuted }}>
                      {e.final ? t('gameState.quests.detail.final') : '-'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </HTMLTable>
        )}
      </div>

      <div style={{ padding: '12px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.transitions')}
        </div>
        {renderTransitionsSection(entry, quest, loadedTransitions, tlk, t)}
      </div>

      <div style={{ padding: '12px 16px' }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.liveOverlay')}
        </div>
        <LiveOverlay transitions={loadedTransitions ?? []} graph={graph} />
      </div>
    </div>
  );
}

function renderTransitionsSection(
  entry: ReturnType<QuestTransitionsResolver['get']>,
  quest: QuestSummary,
  loaded: TransitionNode[] | null,
  tlk: ReturnType<typeof useTlkResolver>,
  t: ReturnType<typeof useTranslations>,
) {
  if (!entry || entry.state === 'pending') {
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, color: T.textMuted }}>
        <Spinner size={16} /> <span>{t('gameState.quests.detail.loadingTransitions')}</span>
      </div>
    );
  }
  if (entry.state === 'error') {
    return (
      <div style={{ color: T.negative }}>
        {t('gameState.quests.detail.transitionsFailed')}: {entry.message}
      </div>
    );
  }
  return <TransitionsList quest={quest} transitions={loaded ?? []} tlk={tlk} modules={graph.modules} />;
}
