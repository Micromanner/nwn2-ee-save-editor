import { useMemo, useState } from 'react';
import { Button, HTMLTable, InputGroup, Menu, MenuItem, Popover } from '@blueprintjs/core';
import { T } from '../../theme';
import { useTranslations } from '@/hooks/useTranslations';
import type { AggregatedModule, QuestSummary } from './types';
import { useModuleNameResolver } from './useModuleNameResolver';
import { questDisplayName, questStatus, selectedRowStyle, type QuestStatus } from './utils';

type StatusFilter = 'all' | QuestStatus;
type SortKey = 'name' | 'status';

const STATUS_ORDER: Record<QuestStatus, number> = { active: 0, complete: 1, untouched: 2 };

function EmptyRow({ message }: { message: string }) {
  return (
    <tr><td colSpan={5} style={{ textAlign: 'center', color: T.textMuted, padding: 20 }}>{message}</td></tr>
  );
}

export function QuestList({
  quests, modules, selectedTag, onSelect,
}: {
  quests: QuestSummary[];
  modules: AggregatedModule[];
  selectedTag: string | null;
  onSelect: (tag: string) => void;
}) {
  const t = useTranslations();
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [moduleFilter, setModuleFilter] = useState<string>('all');
  const [sortKey, setSortKey] = useState<SortKey>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  const { resolve, resolveMany } = useModuleNameResolver(modules);

  const rows = useMemo(() => {
    const needle = search.toLowerCase();
    const filtered = quests.filter(q => {
      if (needle && !q.tag.toLowerCase().includes(needle) && !questDisplayName(q).toLowerCase().includes(needle)) {
        return false;
      }
      if (statusFilter !== 'all' && questStatus(q) !== statusFilter) return false;
      if (moduleFilter !== 'all' && !q.defined_in.includes(moduleFilter)) return false;
      return true;
    });
    return filtered.sort((a, b) => {
      const cmp = sortKey === 'name'
        ? questDisplayName(a).localeCompare(questDisplayName(b))
        : STATUS_ORDER[questStatus(a)] - STATUS_ORDER[questStatus(b)];
      return sortDir === 'asc' ? cmp : -cmp;
    });
  }, [quests, search, statusFilter, moduleFilter, sortKey, sortDir]);

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

  const statusOptions: { value: StatusFilter; labelKey: string }[] = [
    { value: 'all', labelKey: 'gameState.quests.list.statusAll' },
    { value: 'active', labelKey: 'gameState.quests.status.active' },
    { value: 'complete', labelKey: 'gameState.quests.status.complete' },
    { value: 'untouched', labelKey: 'gameState.quests.status.untouched' },
  ];
  const moduleOptions = modules.filter(m => m.journal_category_tags.length > 0);

  const statusMenu = (
    <Menu>
      {statusOptions.map(o => (
        <MenuItem
          key={o.value}
          text={t(o.labelKey)}
          active={statusFilter === o.value}
          onClick={() => setStatusFilter(o.value)}
        />
      ))}
    </Menu>
  );

  const moduleMenu = (
    <Menu>
      <MenuItem text={t('gameState.quests.list.moduleAll')} active={moduleFilter === 'all'} onClick={() => setModuleFilter('all')} />
      {moduleOptions.map(m => (
        <MenuItem key={m.name} text={resolve(m.name)} active={moduleFilter === m.name} onClick={() => setModuleFilter(m.name)} />
      ))}
    </Menu>
  );

  const statusLabel = t(statusOptions.find(o => o.value === statusFilter)!.labelKey);
  const moduleLabel = moduleFilter === 'all' ? t('gameState.quests.list.moduleAll') : resolve(moduleFilter);

  const renderBody = () => {
    if (quests.length === 0) return <EmptyRow message={t('gameState.quests.list.emptyAll')} />;
    if (rows.length === 0) return <EmptyRow message={t('gameState.quests.list.emptyFiltered')} />;
    return rows.map(q => {
      const isSelected = q.tag === selectedTag;
      const status = questStatus(q);
      const currentEntry = q.live_state != null ? q.category.entries.find(e => e.id === q.live_state) : null;
      const statusColor = status === 'complete' ? T.positive : status === 'active' ? T.text : T.textMuted;
      return (
        <tr
          key={q.tag}
          onClick={() => onSelect(q.tag)}
          style={{ cursor: 'pointer', ...selectedRowStyle(isSelected, T.accent) }}
        >
          <td style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{questDisplayName(q)}</td>
          <td style={{ color: statusColor }}>
            {t(`gameState.quests.status.${status}`)}
            {q.pending_state !== undefined && <span style={{ color: T.accent }}> *</span>}
          </td>
          <td style={{ color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={currentEntry?.text}>
            {currentEntry?.text ?? (q.live_state != null ? t('gameState.quests.edit.undefinedState') : '')}
          </td>
          <td style={{ textAlign: 'right' }}>{q.category.xp || 0}</td>
          <td title={resolveMany(q.defined_in)} style={{ color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {q.defined_in.length === 0 ? '-' : resolveMany(q.defined_in)}
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
        <Popover content={statusMenu} placement="bottom-start" minimal>
          <Button minimal rightIcon="caret-down" text={statusLabel} />
        </Popover>
        <Popover content={moduleMenu} placement="bottom-start" minimal>
          <Button minimal rightIcon="caret-down" text={moduleLabel} />
        </Popover>
      </div>

      <HTMLTable compact striped style={{ width: '100%', tableLayout: 'fixed' }}>
        <colgroup>
          <col style={{ width: 200 }} />
          <col style={{ width: 96 }} />
          <col />
          <col style={{ width: 50 }} />
          <col style={{ width: 120 }} />
        </colgroup>
        <thead>
          <tr>
            <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('name')}>
              {t('gameState.quests.list.name')}{sortIndicator('name')}
            </th>
            <th style={{ cursor: 'pointer' }} onClick={() => toggleSort('status')}>
              {t('gameState.quests.list.status')}{sortIndicator('status')}
            </th>
            <th>{t('gameState.quests.list.currentEntry')}</th>
            <th>{t('gameState.quests.list.xp')}</th>
            <th>{t('gameState.quests.list.module')}</th>
          </tr>
        </thead>
        <tbody>{renderBody()}</tbody>
      </HTMLTable>
    </div>
  );
}
