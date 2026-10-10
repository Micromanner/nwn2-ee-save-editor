import { useCallback, useMemo, useState } from 'react';
import { HTMLTable, Icon, NonIdealState } from '@blueprintjs/core';
import { GiSwapBag } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { fmtNum } from '../shared';
import { T } from '../theme';
import type { FullInventoryItem } from '@/lib/bindings';
import { display } from '@/utils/dataHelpers';
import { renderNwn2Markup } from '@/utils/nwn2Markup';
import { useTranslations } from '@/hooks/useTranslations';
import { categoryLabel } from './inventoryLabels';

interface BackpackTableProps {
  items: FullInventoryItem[];
  selectedKey: string | null;
  onSelectItem: (item: FullInventoryItem) => void;
}

/** Identifies a backpack item, or an item inside a bag, across reloads. */
export function backpackItemKey(item: Pick<FullInventoryItem, 'index' | 'container_index'>): string {
  return `${item.container_index ?? ''}:${item.index}`;
}

type SortKey = 'name' | 'category' | 'stack_size' | 'weight' | 'value';

interface SortState {
  key: SortKey;
  dir: 'asc' | 'desc';
}

function compareItems(a: FullInventoryItem, b: FullInventoryItem, key: SortKey): number {
  if (key === 'name' || key === 'category') {
    return (a[key] ?? '').localeCompare(b[key] ?? '', undefined, { sensitivity: 'base' });
  }
  return (a[key] ?? 0) - (b[key] ?? 0);
}

export function BackpackTable({ items, selectedKey, onSelectItem }: BackpackTableProps) {
  const t = useTranslations();
  const [sort, setSort] = useState<SortState | null>(null);
  const [openBags, setOpenBags] = useState<Set<number>>(new Set());

  const sortItems = useCallback((list: FullInventoryItem[]) => {
    if (!sort) return list;
    const sign = sort.dir === 'asc' ? 1 : -1;
    return [...list].sort((a, b) => sign * compareItems(a, b, sort.key) || a.index - b.index);
  }, [sort]);

  const sortedItems = useMemo(() => sortItems(items), [items, sortItems]);

  const toggleBag = (index: number) => {
    setOpenBags(prev => {
      const next = new Set(prev);
      if (!next.delete(index)) next.add(index);
      return next;
    });
  };

  const toggleSort = (key: SortKey) => {
    setSort(prev => {
      if (prev?.key !== key) return { key, dir: 'asc' };
      if (prev.dir === 'asc') return { key, dir: 'desc' };
      return null;
    });
  };

  const header = (key: SortKey, label: string, align?: 'center' | 'right') => (
    <th onClick={() => toggleSort(key)} style={{ textAlign: align, cursor: 'pointer', userSelect: 'none', whiteSpace: 'nowrap', position: 'sticky', top: 0, zIndex: 1, background: T.surface }}>
      {label}
      {sort?.key === key && (
        <Icon icon={sort.dir === 'asc' ? 'caret-up' : 'caret-down'} size={12} style={{ marginLeft: 2, verticalAlign: 'middle' }} />
      )}
    </th>
  );

  const renderRow = (item: FullInventoryItem, isOpen: boolean) => {
    const key = backpackItemKey(item);
    const selected = selectedKey === key;
    const inBag = item.container_index !== null;
    const isBag = item.is_container;
    return (
      <tr
        key={key}
        onClick={() => onSelectItem(item)}
        className={selected ? 't-bold' : undefined}
        style={selected ? { background: 'rgba(160, 82, 45, 0.1)' } : undefined}
      >
        <td className={!selected && item.is_custom ? 't-semibold' : undefined} style={{
          color: item.is_custom ? T.accent : T.text,
          overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
          paddingLeft: inBag ? 28 : undefined,
        }}>
          {isBag && (
            <Icon
              icon={isOpen ? 'caret-down' : 'caret-right'}
              size={14}
              style={{ marginRight: 4, verticalAlign: 'middle', cursor: 'pointer', color: T.textMuted }}
              onClick={(e) => { e.stopPropagation(); toggleBag(item.index); }}
            />
          )}
          {renderNwn2Markup(display(item.name))}
          {isBag && <span style={{ color: T.textMuted }}> ({item.contents.length})</span>}
        </td>
        <td style={{ color: T.textMuted }}>{display(categoryLabel(t, item.category))}</td>
        <td style={{ textAlign: 'center' }}>{item.stack_size > 1 ? item.stack_size : ''}</td>
        <td style={{ textAlign: 'right', color: T.textMuted }}>
          {item.weight > 0 ? t('inventory.weightValue', { value: item.weight.toFixed(1) }) : '-'}
        </td>
        <td style={{ textAlign: 'right', color: T.textMuted }}>
          {item.value > 0 ? t('inventory.goldValue', { value: fmtNum(item.value) }) : '-'}
        </td>
      </tr>
    );
  };

  if (items.length === 0) {
    return (
      <NonIdealState
        icon={<GameIcon icon={GiSwapBag} size={40} />}
        title={t('inventory.backpackEmpty')}
        description={t('inventory.backpackEmptyDescription')}
      />
    );
  }

  return (
    <HTMLTable compact striped bordered interactive style={{ width: '100%', tableLayout: 'fixed' }}>
      <colgroup>
        <col />
        <col style={{ width: 120 }} />
        <col style={{ width: 56 }} />
        <col style={{ width: 84 }} />
        <col style={{ width: 116 }} />
      </colgroup>
      <thead>
        <tr>
          {header('name', t('inventory.item'))}
          {header('category', t('abilityScores.type'))}
          {header('stack_size', t('inventory.qty'), 'center')}
          {header('weight', t('inventory.weight'), 'right')}
          {header('value', t('inventory.value'), 'right')}
        </tr>
      </thead>
      <tbody>
        {sortedItems.flatMap((item) => {
          const isOpen = openBags.has(item.index);
          const rows = [renderRow(item, isOpen)];
          if (isOpen) {
            rows.push(...sortItems(item.contents).map(child => renderRow(child, false)));
            if (item.contents.length === 0) {
              rows.push(
                <tr key={`${item.index}:empty`}>
                  <td colSpan={5} style={{ paddingLeft: 28, color: T.textMuted, fontStyle: 'italic' }}>
                    {t('inventory.bagEmpty')}
                  </td>
                </tr>
              );
            }
          }
          return rows;
        })}
      </tbody>
    </HTMLTable>
  );
}
