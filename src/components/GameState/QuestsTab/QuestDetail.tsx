import { useEffect } from 'react';
import { HTMLTable, NonIdealState } from '@blueprintjs/core';
import { GiScrollQuill } from 'react-icons/gi';
import { T } from '../../theme';
import { KVRow } from '../../shared';
import { GameIcon } from '../../shared/GameIcon';
import { useTranslations } from '@/hooks/useTranslations';
import { useTlkResolver } from '@/hooks/useTlkResolver';
import type { QuestAggregate, SaveGraph } from './types';
import { effectiveSource, selectedRowStyle } from './utils';
import { TransitionsList } from './TransitionsList';
import { LiveOverlay } from './LiveOverlay';

export function QuestDetail({
  quest, graph,
}: {
  quest: QuestAggregate | null;
  graph: SaveGraph;
}) {
  const t = useTranslations();
  const tlk = useTlkResolver();

  useEffect(() => {
    if (quest) tlk.prime(quest.transitions.map(tr => tr.text_strref));
    // `tlk.prime` is a stable useCallback; skip `tlk` from deps to avoid
    // re-firing on every parent render (the resolver returns a fresh object).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [quest]);

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

  return (
    <div>
      <div style={{ padding: '12px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.header')}
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
          <KVRow label={t('gameState.quests.list.tag')} value={<span className="t-mono">{quest.tag}</span>} />
          <KVRow label={t('gameState.quests.list.name')} value={quest.category.name || quest.tag} />
          <KVRow label={t('gameState.quests.list.priority')} value={quest.category.priority || '-'} />
          <KVRow label={t('gameState.quests.list.xp')} value={String(quest.category.xp || 0)} />
          <KVRow
            label={t('gameState.quests.list.live')}
            value={quest.live_state == null ? (
              <span style={{ color: T.textMuted }}>{t('gameState.quests.detail.notStarted')}</span>
            ) : (
              <span>
                <span className="t-mono">{quest.live_state}</span>
                {matchingEntry && <> &mdash; {matchingEntry.text}</>}
                {matchingEntry?.final && <span style={{ color: T.positive, marginLeft: 6 }}>({t('gameState.quests.detail.final')})</span>}
              </span>
            )}
          />
          <KVRow
            label={t('gameState.quests.list.source')}
            value={
              <span style={{ color: source !== 'normal' ? T.negative : T.textMuted }}>
                {t(`gameState.quests.sourceBadge.${source}`)}
              </span>
            }
          />
          <KVRow
            label={t('gameState.quests.list.definedIn')}
            value={quest.defined_in.length > 0 ? quest.defined_in.join(', ') : '-'}
          />
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
        <TransitionsList quest={quest} tlk={tlk} />
      </div>

      <div style={{ padding: '12px 16px' }}>
        <div className="t-bold" style={{ color: T.textMuted, marginBottom: 8 }}>
          {t('gameState.quests.detail.liveOverlay')}
        </div>
        <LiveOverlay quest={quest} graph={graph} />
      </div>
    </div>
  );
}
