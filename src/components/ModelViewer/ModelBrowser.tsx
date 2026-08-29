import { useState, useEffect, useCallback, useMemo, memo } from 'react';
import { InputGroup, NonIdealState, Tag } from '@blueprintjs/core';
import { LoadingState } from '@/components/shared';
import { GiBrokenShield, GiCube, GiMagnifyingGlass } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { FixedSizeList as List, ListChildComponentProps } from 'react-window';
import { invoke } from '@tauri-apps/api/core';
import { useTranslations } from '@/hooks/useTranslations';
import { AssetViewer3D } from './AssetViewer3D';

interface ModelEntry {
  filename: string;
  resref: string;
  zip_source: string;
}

interface RowData {
  items: ModelEntry[];
  selectedResref: string | null;
  onSelect: (resref: string) => void;
}

const ROW_HEIGHT = 28;

const Row = memo(({ index, style, data }: ListChildComponentProps<RowData>) => {
  const m = data.items[index];
  return (
    <div
      style={{
        ...style,
        display: 'flex',
        alignItems: 'center',
        cursor: 'pointer',
        padding: '0 8px',
        background: m.resref === data.selectedResref ? 'rgba(160, 82, 45, 0.15)' : undefined,
        borderBottom: '1px solid rgba(0,0,0,0.08)',
      }}
      onClick={() => data.onSelect(m.resref)}
    >
      <code className="t-base" style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{m.resref}</code>
      <Tag minimal className="t-xs" style={{ flexShrink: 0 }}>{m.zip_source}</Tag>
    </div>
  );
});

function useContainerHeight() {
  const [height, setHeight] = useState(0);
  const [node, setNode] = useState<HTMLDivElement | null>(null);
  useEffect(() => {
    if (!node) return;
    setHeight(node.clientHeight);
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setHeight(entry.contentRect.height);
      }
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, [node]);
  return { ref: setNode, height };
}

export function ModelBrowser() {
  const t = useTranslations();
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [loading, setLoading] = useState(false);
  const [selectedResref, setSelectedResref] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { ref: listContainerRef, height: listHeight } = useContainerHeight();

  useEffect(() => {
    setLoading(true);
    invoke<ModelEntry[]>('list_available_models')
      .then((entries) => setModels(entries))
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(search), 150);
    return () => clearTimeout(timer);
  }, [search]);

  const filtered = useMemo(() => {
    if (!debouncedSearch) return models;
    const q = debouncedSearch.toLowerCase();
    return models.filter((m) =>
      m.resref.toLowerCase().includes(q) || m.zip_source.toLowerCase().includes(q)
    );
  }, [debouncedSearch, models]);

  const handleSelect = useCallback((resref: string) => {
    setSelectedResref(resref);
  }, []);

  const itemData = useMemo<RowData>(() => ({
    items: filtered,
    selectedResref,
    onSelect: handleSelect,
  }), [filtered, selectedResref, handleSelect]);

  if (loading) {
    return <LoadingState text={t('common.loadingModels')} />;
  }

  if (error) {
    return <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} title={t('modelViewer.error')} description={error} />;
  }

  return (
    <div style={{ display: 'flex', height: '100%', overflow: 'hidden' }}>
      <div style={{ width: 350, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 8, padding: 12 }}>
        <InputGroup
          leftIcon="search"
          placeholder={t('modelViewer.searchPlaceholder')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <Tag minimal>{filtered.length} {t('modelViewer.modelsFound')}</Tag>
        <div
          ref={listContainerRef}
          style={{
            flex: 1,
            minHeight: 0,
            overflow: 'hidden',
            borderTop: '1px solid rgba(0,0,0,0.15)',
            borderBottom: '1px solid rgba(0,0,0,0.15)',
          }}
        >
          <List
            height={listHeight || 400}
            itemCount={filtered.length}
            itemSize={ROW_HEIGHT}
            itemData={itemData}
            width="100%"
          >
            {Row}
          </List>
        </div>
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        {selectedResref ? (
          <AssetViewer3D resref={selectedResref} />
        ) : (
          <NonIdealState icon={<GameIcon icon={GiCube} size={40} />} title={t('modelViewer.selectModel')} />
        )}
      </div>
    </div>
  );
}
