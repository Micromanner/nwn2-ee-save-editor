import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { Button, InputGroup, Spinner } from '@blueprintjs/core';
import { FixedSizeGrid, GridChildComponentProps } from 'react-window';
import { invoke } from '@tauri-apps/api/core';
import { ParchmentDialog } from '../shared';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import { useIcon } from '@/hooks/useIcon';

interface IconOption {
  id: number;
  resref: string;
}

interface IconPickerDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onPick: (selection: { rowId: number; resref: string }) => void;
  initialResref?: string | null;
}

const TILE = 72;
const GRID_MIN_HEIGHT = 440;
const GRID_MIN_WIDTH = 640;
const SEARCH_DEBOUNCE_MS = 150;

type CategoryId =
  | 'all' | 'weapons' | 'armor' | 'apparel' | 'jewelry'
  | 'potions' | 'scrolls' | 'crafting' | 'other';

const WEAPON_PREFIXES = [
  'it_wa_', 'it_wb_', 'it_wc_', 'it_wd_', 'it_we_',
  'it_wm_', 'it_wo_', 'it_wp_', 'it_wr_', 'it_wt_',
  'it_wu_', 'it_st_', 'it_ds_',
];
const ARMOR_PREFIXES = ['it_ah_', 'it_am_', 'it_as_', 'it_ac_', 'it_m_', 'it_ck_', 'it_al_'];
const APPAREL_PREFIXES = ['it_he_', 'it_bo_', 'it_gl_', 'it_be_', 'it_br_'];
const JEWELRY_PREFIXES = ['it_ring', 'it_nk_'];
const POTION_PREFIXES = ['it_pot', 'it_ps_', 'it_alc_', 'it_mortarpestle', 'it_healingmoss'];
const POTION_SUFFIX = 'potion';
const SCROLL_PREFIXES = ['it_s_'];
const SCROLL_SUFFIXES = ['scroll', 'book', 'parch'];
const CRAFTING_PREFIXES = ['it_gem', 'it_ess_', 'it_cft_', 'it_cp_'];

const startsWithAny = (s: string, prefixes: readonly string[]) => prefixes.some(p => s.startsWith(p));
const endsWithAny = (s: string, suffixes: readonly string[]) => suffixes.some(x => s.endsWith(x));

const CATEGORY_PREDICATES: Record<Exclude<CategoryId, 'all' | 'other'>, (r: string) => boolean> = {
  weapons: (r) => startsWithAny(r, WEAPON_PREFIXES),
  armor: (r) => startsWithAny(r, ARMOR_PREFIXES),
  apparel: (r) => startsWithAny(r, APPAREL_PREFIXES),
  jewelry: (r) => startsWithAny(r, JEWELRY_PREFIXES),
  potions: (r) => startsWithAny(r, POTION_PREFIXES) || r.endsWith(POTION_SUFFIX),
  scrolls: (r) => startsWithAny(r, SCROLL_PREFIXES) || endsWithAny(r, SCROLL_SUFFIXES),
  crafting: (r) => startsWithAny(r, CRAFTING_PREFIXES),
};

const CATEGORY_IDS: CategoryId[] = [
  'all', 'weapons', 'armor', 'apparel', 'jewelry', 'potions', 'scrolls', 'crafting', 'other',
];

function matchesCategory(resref: string, category: CategoryId): boolean {
  if (category === 'all') return true;
  if (category === 'other') {
    return !Object.values(CATEGORY_PREDICATES).some(p => p(resref));
  }
  return CATEGORY_PREDICATES[category](resref);
}

function IconTile({ option, selected, onClick }: {
  option: IconOption;
  selected: boolean;
  onClick: () => void;
}) {
  const dataUrl = useIcon(option.resref);
  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onClick}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onClick();
        }
      }}
      title={option.resref}
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 2,
        width: TILE - 6,
        height: TILE - 6,
        margin: 3,
        cursor: 'pointer',
        border: `1px solid ${selected ? T.accent : T.borderLight}`,
        background: selected ? `${T.accent}18` : T.surfaceAlt,
        borderRadius: 3,
        userSelect: 'none',
      }}
    >
      {dataUrl ? (
        <img src={dataUrl} alt="" width={48} height={48} draggable={false} />
      ) : (
        <div style={{ width: 48, height: 48, background: T.border, borderRadius: 2 }} />
      )}
      <div
        className="t-xs"
        style={{
          color: T.textMuted,
          maxWidth: TILE - 10,
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
      >
        {option.resref}
      </div>
    </div>
  );
}

export function IconPickerDialog({ isOpen, onClose, onPick, initialResref }: IconPickerDialogProps) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();

  const [options, setOptions] = useState<IconOption[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebouncedValue(search, SEARCH_DEBOUNCE_MS);
  const [category, setCategory] = useState<CategoryId>('all');
  const [selectedId, setSelectedId] = useState<number | null>(null);

  const [containerNode, setContainerNode] = useState<HTMLDivElement | null>(null);
  const [gridWidth, setGridWidth] = useState(GRID_MIN_WIDTH);
  const [gridHeight, setGridHeight] = useState(GRID_MIN_HEIGHT);

  const hasLoaded = useRef(false);

  useEffect(() => {
    if (!isOpen || hasLoaded.current) return;
    hasLoaded.current = true;
    setIsLoading(true);
    invoke<IconOption[]>('get_available_icons')
      .then((result) => setOptions(result))
      .catch(handleError)
      .finally(() => setIsLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const initial = initialResref
      ? options.find((o) => o.resref === initialResref)
      : undefined;
    setSelectedId(initial?.id ?? null);
    setSearch('');
    setCategory('all');
  }, [isOpen, initialResref, options]);

  useEffect(() => {
    if (!containerNode) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        if (width > 0) setGridWidth(width);
        if (height > 0) setGridHeight(Math.max(GRID_MIN_HEIGHT, height));
      }
    });
    observer.observe(containerNode);
    return () => observer.disconnect();
  }, [containerNode]);

  const filtered = useMemo(() => {
    const q = debouncedSearch.trim().toLowerCase();
    return options.filter((o) => {
      if (category !== 'all' && !matchesCategory(o.resref, category)) return false;
      if (q && !o.resref.toLowerCase().includes(q)) return false;
      return true;
    });
  }, [options, debouncedSearch, category]);

  const selectedResref = useMemo(
    () => (selectedId === null ? null : options.find((o) => o.id === selectedId)?.resref ?? null),
    [options, selectedId],
  );

  const columnCount = Math.max(1, Math.floor(gridWidth / TILE));
  const rowCount = Math.ceil(filtered.length / columnCount);

  const Cell = useCallback(({ columnIndex, rowIndex, style }: GridChildComponentProps) => {
    const index = rowIndex * columnCount + columnIndex;
    if (index >= filtered.length) return null;
    const option = filtered[index];
    const selected = selectedId === option.id;
    return (
      <div style={style}>
        <IconTile
          option={option}
          selected={selected}
          onClick={() => setSelectedId(option.id)}
        />
      </div>
    );
  }, [filtered, columnCount, selectedId]);

  const canConfirm = selectedId !== null && selectedResref !== null;

  const handleConfirm = () => {
    if (!canConfirm) return;
    onPick({ rowId: selectedId!, resref: selectedResref! });
    onClose();
  };

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('inventory.iconPickerTitle')}
      width={720}
      minHeight={560}
      footerActions={
        <Button
          intent="primary"
          text={t('inventory.save')}
          onClick={handleConfirm}
          disabled={!canConfirm}
        />
      }
      footerLeft={
        <span className="t-sm" style={{ color: T.textMuted }}>
          {filtered.length}{filtered.length !== options.length ? ` / ${options.length}` : ''}
          {selectedResref ? ` - ${selectedResref}` : ''}
        </span>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 8 }}>
        <InputGroup
          leftIcon="search"
          placeholder={t('inventory.searchIcons')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          rightElement={search ? <Button icon="cross" minimal onClick={() => setSearch('')} /> : undefined}
          disabled={isLoading}
        />
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
          {CATEGORY_IDS.map((id) => (
            <Button
              key={id}
              small
              minimal
              active={category === id}
              text={t(`inventory.iconCategories.${id}`)}
              onClick={() => setCategory(id)}
              disabled={isLoading}
            />
          ))}
        </div>
        <div
          ref={setContainerNode}
          style={{
            flex: 1,
            minHeight: GRID_MIN_HEIGHT,
            border: `1px solid ${T.borderLight}`,
            borderRadius: 3,
            background: T.surface,
            overflow: 'hidden',
          }}
        >
          {isLoading ? (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100%', gap: 10 }}>
              <Spinner size={20} />
              <span style={{ color: T.textMuted }}>{t('inventory.loadingIcons')}</span>
            </div>
          ) : filtered.length === 0 ? (
            <div style={{ padding: 32, textAlign: 'center', color: T.textMuted }}>
              {t('inventory.noIconsMatch')}
            </div>
          ) : (
            <FixedSizeGrid
              columnCount={columnCount}
              columnWidth={TILE}
              rowCount={rowCount}
              rowHeight={TILE}
              width={gridWidth}
              height={gridHeight}
              overscanRowCount={2}
            >
              {Cell}
            </FixedSizeGrid>
          )}
        </div>
      </div>
    </ParchmentDialog>
  );
}
