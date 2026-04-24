import { useMemo, useState } from 'react';
import { HTMLSelect, HTMLTable, InputGroup } from '@blueprintjs/core';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import type { QuestSummary } from './types';
import { effectiveSource, selectedRowStyle, type QuestSource } from './utils';

type StatusFilter = 'all' | 'active' | 'complete' | 'untouched';
type SourceFilter = 'all' | QuestSource;
type SortKey = 'name' | 'live';

function statusOf(q: QuestSummary): Exclude<StatusFilter, 'all'> {
  if (q.live_state == null) return 'untouched';
  const matching = q.category.entries.find(e => e.id === q.live_state);
  if (matching?.final) return 'complete';
  return 'active';
}

function EmptyRow({ message }: { message: string }) {
  return (
    <tr><td colSpan={8} style={{ textAlign: 'center', color: T.textMuted, padding: 20 }}>{message}</td></tr>
  );
}

export function QuestList({
  quests, selectedTag, onSelect,
}: {
  quests: QuestSummary[];
  selectedTag: string | null;
  onSelect: (tag: string) => void;
}) {
  const t = useTranslations();
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all');
  const [sortKey, setSortKey] = useState<SortKey>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  const rows = useMemo(() => {
    const needle = search.toLowerCase();
    const filtered = quests.filter(q => {
      if (needle && !q.tag.toLowerCase().includes(needle) && !q.category.name.toLowerCase().includes(needle)) {
        return false;
      }
      if (statusFilter !== 'all' && statusOf(q) !== statusFilter) return false;
      if (sourceFilter !== 'all' && effectiveSource(q) !== sourceFilter) return false;
      return true;
    });
    return filtered.sort((a, b) => {
      let cmp = 0;
      if (sortKey === 'name') {
        cmp = (a.category.name || a.tag).localeCompare(b.category.name || b.tag);
      } else {
        const av = a.live_state ?? -1;
        const bv = b.live_state ?? -1;
        cmp = av - bv;
      }
      return sortDir === 'asc' ? cmp : -cmp;
    });
  }, [quests, search, statusFilter, sourceFilter, sortKey, sortDir]);

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortDir(d => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  };

  const sortIndicator = (key: SortKey) => {
    if (sortKey !== key) return '';
    return sortDir === 'asc' ? ' ▲' : ' ▼';
  };

  const renderBody = () => {
    if (quests.length === 0) return <EmptyRow message={t('gameState.quests.list.emptyAll')} />;
    if (rows.length === 0) return <EmptyRow message={t('gameState.quests.list.emptyFiltered')} />;
    return rows.map(q => {
      const isSelected = q.tag === selectedTag;
      const source = effectiveSource(q);
      const matchingEntry = q.live_state != null ? q.category.entries.find(e => e.id === q.live_state) : null;
      return (
        <tr
          key={q.tag}
          onClick={() => onSelect(q.tag)}
          style={{ cursor: 'pointer', ...selectedRowStyle(isSelected, T.accent) }}
        >
          <td className="t-mono" style={{ color: T.text, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{q.tag}</td>
          <td style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{q.category.name || q.tag}</td>
          <td style={{ color: T.textMuted }}>{q.category.priority || '-'}</td>
          <td style={{ textAlign: 'right' }}>{q.category.xp || 0}</td>
          <td className="t-mono" style={{ color: T.text }}>
            {q.live_state == null && <span style={{ color: T.textMuted }}>-</span>}
            {q.live_state != null && q.live_state}
            {matchingEntry?.final && <span style={{ color: T.positive, marginLeft: 4 }}>({t('gameState.quests.detail.final')})</span>}
          </td>
          <td style={{ color: source !== 'normal' ? T.negative : T.textMuted }}>
            {source !== 'normal' ? t(`gameState.quests.sourceBadge.${source}`) : ''}
          </td>
          <td style={{ textAlign: 'right' }}>{q.transition_count}</td>
          <td title={q.defined_in.join(', ')} style={{ color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {q.defined_in.join(', ') || '-'}
          </td>
        </tr>
      );
    });
  };

  return (
    <div>
      <div style={{ padding: '8px 12px', borderBottom: `1px solid ${T.borderLight}`, display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <InputGroup
          small
          leftIcon="search"
          placeholder={t('gameState.quests.list.searchPlaceholder')}
          value={search}
          onChange={e => setSearch(e.target.value)}
          style={{ flex: 1, minWidth: 140 }}
        />
        <HTMLSelect
          value={statusFilter}
          onChange={e => setStatusFilter(e.target.value as StatusFilter)}
        >
          <option value="all">{t('gameState.quests.list.statusAll')}</option>
          <option value="active">{t('gameState.quests.list.statusActive')}</option>
          <option value="complete">{t('gameState.quests.list.statusComplete')}</option>
          <option value="untouched">{t('gameState.quests.list.statusUntouched')}</option>
        </HTMLSelect>
        <HTMLSelect
          value={sourceFilter}
          onChange={e => setSourceFilter(e.target.value as SourceFilter)}
        >
          <option value="all">{t('gameState.quests.list.sourceAll')}</option>
          <option value="normal">{t('gameState.quests.list.sourceNormal')}</option>
          <option value="live_only">{t('gameState.quests.list.sourceLiveOnly')}</option>
          <option value="transition_only">{t('gameState.quests.list.sourceTransitionOnly')}</option>
        </HTMLSelect>
      </div>

      <HTMLTable compact striped style={{ width: '100%', tableLayout: 'fixed' }}>
        <colgroup>
          <col style={{ width: 120 }} />
          <col />
          <col style={{ width: 70 }} />
          <col style={{ width: 50 }} />
          <col style={{ width: 60 }} />
          <col style={{ width: 80 }} />
          <col style={{ width: 48 }} />
          <col style={{ width: 120 }} />
        </colgroup>
        <thead>
          <tr>
            <th>{t('gameState.quests.list.tag')}</th>
            <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('name')}>
              {t('gameState.quests.list.name')}{sortIndicator('name')}
            </th>
            <th>{t('gameState.quests.list.priority')}</th>
            <th>{t('gameState.quests.list.xp')}</th>
            <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('live')}>
              {t('gameState.quests.list.live')}{sortIndicator('live')}
            </th>
            <th>{t('gameState.quests.list.source')}</th>
            <th>{t('gameState.quests.list.transitions')}</th>
            <th>{t('gameState.quests.list.definedIn')}</th>
          </tr>
        </thead>
        <tbody>{renderBody()}</tbody>
      </HTMLTable>
    </div>
  );
}
