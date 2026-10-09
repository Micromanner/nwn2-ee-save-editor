import { useState, useMemo, useCallback, useEffect } from 'react';
import { Icon } from '@blueprintjs/core';
import { List, RowComponentProps } from 'react-window';
import { T } from '../theme';
import { useFontSize } from '@/providers/FontSizeProvider';

export interface ListSection<T> {
  key: string;
  title: string;
  items: T[];
  countLabel?: string;
  countColor?: string;
}

interface GroupedListProps<T extends { id: number }> {
  sections: ListSection<T>[];
  selectedId: number | null;
  onSelect: (item: T) => void;
  renderItem: (item: T, selected: boolean) => React.ReactNode;
}

type FlatRow<T> =
  | { type: 'header'; sectionKey: string; title: string; count: number; countLabel?: string; countColor?: string }
  | { type: 'item'; item: T };

const ROW_HEIGHTS = {
  small:  { header: 28, item: 27 },
  medium: { header: 32, item: 31 },
  large:  { header: 38, item: 36 },
} as const;

export function GroupedList<T extends { id: number }>({ sections, selectedId, onSelect, renderItem }: GroupedListProps<T>) {
  const { fontSize } = useFontSize();
  const heights = ROW_HEIGHTS[fontSize];
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [containerHeight, setContainerHeight] = useState(400);
  const [containerNode, setContainerNode] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!containerNode) return;
    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        setContainerHeight(entry.contentRect.height);
      }
    });
    observer.observe(containerNode);
    return () => observer.disconnect();
  }, [containerNode]);

  const toggleSection = useCallback((key: string) => {
    setCollapsed(prev => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key); else next.add(key);
      return next;
    });
  }, []);

  const flatRows: FlatRow<T>[] = useMemo(() => {
    const rows: FlatRow<T>[] = [];
    for (const section of sections) {
      rows.push({
        type: 'header',
        sectionKey: section.key,
        title: section.title,
        count: section.items.length,
        countLabel: section.countLabel,
        countColor: section.countColor,
      });
      if (!collapsed.has(section.key)) {
        for (const item of section.items) {
          rows.push({ type: 'item', item });
        }
      }
    }
    return rows;
  }, [sections, collapsed]);

  const getItemSize = useCallback((index: number) => {
    return flatRows[index].type === 'header' ? heights.header : heights.item;
  }, [flatRows, heights]);

  const Row = useCallback(({ index, style }: RowComponentProps) => {
    const row = flatRows[index];
    if (row.type === 'header') {
      const isCollapsed = collapsed.has(row.sectionKey);
      return (
        <div
          style={{ ...style, display: 'flex', alignItems: 'center', gap: 6, padding: '0 12px', cursor: 'pointer', userSelect: 'none', background: T.sectionBg, borderBottom: `1px solid ${T.sectionBorder}` }}
          onClick={() => toggleSection(row.sectionKey)}
        >
          <Icon
            icon={isCollapsed ? 'caret-right' : 'caret-down'}
            size={16}
            style={{ color: T.accent, width: 16 }}
          />
          <span className="t-bold" style={{ color: T.accent, flex: 1 }}>
            {row.title}
          </span>
          <span style={{ color: row.countColor ?? T.textMuted }}>{row.countLabel ?? row.count}</span>
        </div>
      );
    }

    const selected = selectedId === row.item.id;
    return (
      <div
        style={{
          ...style,
          padding: '0 12px 0 28px',
          display: 'flex',
          alignItems: 'center',
          cursor: 'pointer',
          borderBottom: `1px solid ${T.borderLight}`,
          background: selected ? `${T.accent}12` : 'transparent',
          borderLeft: selected ? `2px solid ${T.accent}` : '2px solid transparent',
        }}
        onClick={() => onSelect(row.item)}
      >
        {renderItem(row.item, selected)}
      </div>
    );
  }, [flatRows, collapsed, selectedId, onSelect, renderItem, toggleSection]);

  return (
    <div ref={setContainerNode} style={{ height: '100%' }}>
      <List
        style={{ height: containerHeight, width: '100%' }}
        rowCount={flatRows.length}
        rowHeight={getItemSize}
        rowComponent={Row}
        rowProps={{}}
        overscanCount={20}
      />
    </div>
  );
}
