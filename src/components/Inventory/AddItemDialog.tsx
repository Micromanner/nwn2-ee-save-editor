import { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { Button, ButtonGroup, InputGroup, Menu, MenuItem, Popover, Spinner, Tab, Tabs } from '@blueprintjs/core';
import { GiFunnel, GiMagnifyingGlass } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { FixedSizeList as List } from 'react-window';
import { ParchmentDialog } from '../shared';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useInventoryManagement } from '@/hooks/useInventoryManagement';
import { inventoryAPI, BaseItem, ItemTemplate, ITEM_CATEGORIES, CATEGORIES_WITH_SUBS, SUB_CATEGORY_LABELS, isVanillaSource } from '@/services/inventoryApi';
import { display } from '@/utils/dataHelpers';

const LIST_HEIGHT = 380;
const ROW_HEIGHT = 46;

type TabId = 'base' | 'template';
type SourceFilter = 'all' | 'vanilla' | 'override';

interface AddItemDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onItemAdded?: (itemIndex: number) => void;
}

const CATEGORY_ORDER = ['Weapons', 'Armor & Clothing', 'Magic Items', 'Miscellaneous'];

export function AddItemDialog({ isOpen, onClose, onItemAdded }: AddItemDialogProps) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const { characterId } = useCharacterContext();
  const { addItemByBaseType, addItemFromTemplate } = useInventoryManagement();

  const [tab, setTab] = useState<TabId>('base');
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState<string>('all');
  const [subCategory, setSubCategory] = useState<string | null>(null);
  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all');
  const [selectedBaseId, setSelectedBaseId] = useState<number | null>(null);
  const [selectedResref, setSelectedResref] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);

  const [baseItems, setBaseItems] = useState<BaseItem[]>([]);
  const [templates, setTemplates] = useState<ItemTemplate[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const [containerNode, setContainerNode] = useState<HTMLDivElement | null>(null);
  const [listWidth, setListWidth] = useState(680);
  const [listHeight, setListHeight] = useState(LIST_HEIGHT);

  useEffect(() => {
    if (!isOpen || !characterId) return;
    setIsLoading(true);
    Promise.all([
      inventoryAPI.getAllBaseItems(characterId),
      inventoryAPI.getAvailableTemplates(characterId),
    ]).then(([baseRes, templRes]) => {
      setBaseItems(baseRes.base_items);
      setTemplates(templRes.templates);
    }).catch(handleError).finally(() => setIsLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, characterId]);

  useEffect(() => {
    if (!containerNode) return;
    const observer = new ResizeObserver(entries => {
      for (const entry of entries) {
        if (entry.contentRect.width > 0) setListWidth(entry.contentRect.width);
        if (entry.contentRect.height >= LIST_HEIGHT) {
          setListHeight(entry.contentRect.height);
        }
      }
    });
    observer.observe(containerNode);
    return () => observer.disconnect();
  }, [containerNode, isOpen]);

  const handleReset = useCallback(() => {
    setSearch('');
    setCategory('all');
    setSubCategory(null);
    setSourceFilter('all');
    setSelectedBaseId(null);
    setSelectedResref(null);
    setTab('base');
    setListHeight(LIST_HEIGHT);
    setListWidth(680);
  }, []);

  const clearFilters = useCallback(() => {
    setSearch('');
    setCategory('all');
    setSubCategory(null);
    setSourceFilter('all');
  }, []);

  const hasFilters = search.length > 0 || category !== 'all' || sourceFilter !== 'all';

  const baseCategories = useMemo(() => {
    const present = new Set(baseItems.map(i => i.category));
    return CATEGORY_ORDER.filter(c => present.has(c));
  }, [baseItems]);

  const templateCategories = useMemo(() => {
    const counts = new Map<string, number>();
    for (const tmpl of templates) {
      const name = ITEM_CATEGORIES[tmpl.category as keyof typeof ITEM_CATEGORIES] || 'Miscellaneous';
      counts.set(name, (counts.get(name) || 0) + 1);
    }
    return CATEGORY_ORDER.filter(c => counts.has(c)).map(c => [c, counts.get(c)!] as const);
  }, [templates]);

  const availableBaseSubCategories = useMemo(() => {
    if (category === 'all' || !CATEGORIES_WITH_SUBS.has(category)) return [];
    const subs = new Map<string, number>();
    for (const item of baseItems) {
      if (item.category === category && item.subCategory) {
        subs.set(item.subCategory, (subs.get(item.subCategory) || 0) + 1);
      }
    }
    return Array.from(subs.entries()).filter(([, c]) => c > 0).sort(([a], [b]) => {
      const la = SUB_CATEGORY_LABELS[a] || a;
      const lb = SUB_CATEGORY_LABELS[b] || b;
      return la.localeCompare(lb);
    });
  }, [baseItems, category]);

  const availableTemplateSubCategories = useMemo(() => {
    if (category === 'all' || !CATEGORIES_WITH_SUBS.has(category)) return [];
    const subs = new Map<string, number>();
    for (const tmpl of templates) {
      const catName = ITEM_CATEGORIES[tmpl.category as keyof typeof ITEM_CATEGORIES] || 'Miscellaneous';
      if (catName === category && tmpl.sub_category) {
        subs.set(tmpl.sub_category, (subs.get(tmpl.sub_category) || 0) + 1);
      }
    }
    return Array.from(subs.entries()).filter(([, c]) => c > 0).sort(([a], [b]) => {
      const la = SUB_CATEGORY_LABELS[a] || a;
      const lb = SUB_CATEGORY_LABELS[b] || b;
      return la.localeCompare(lb);
    });
  }, [templates, category]);

  const filteredBase = useMemo(() => {
    let items = baseItems;
    if (category !== 'all') items = items.filter(i => i.category === category);
    if (subCategory) items = items.filter(i => i.subCategory === subCategory);
    if (search.trim()) {
      const q = search.toLowerCase();
      items = items.filter(i => i.name.toLowerCase().includes(q));
    }
    return items;
  }, [baseItems, search, category, subCategory]);

  const filteredTemplates = useMemo(() => {
    let items = templates;
    if (category !== 'all') {
      items = items.filter(i => {
        const name = ITEM_CATEGORIES[i.category as keyof typeof ITEM_CATEGORIES] || 'Miscellaneous';
        return name === category;
      });
    }
    if (subCategory) items = items.filter(i => i.sub_category === subCategory);
    if (sourceFilter !== 'all') {
      const wantVanilla = sourceFilter === 'vanilla';
      items = items.filter(i => isVanillaSource(i.source) === wantVanilla);
    }
    if (search.trim()) {
      const q = search.toLowerCase();
      items = items.filter(i => i.name.toLowerCase().includes(q) || i.resref.toLowerCase().includes(q));
    }
    return items;
  }, [templates, search, category, subCategory, sourceFilter]);

  const sourceCounts = useMemo(() => {
    let scoped = templates;
    if (category !== 'all') {
      scoped = scoped.filter(i => {
        const name = ITEM_CATEGORIES[i.category as keyof typeof ITEM_CATEGORIES] || 'Miscellaneous';
        return name === category;
      });
    }
    if (subCategory) scoped = scoped.filter(i => i.sub_category === subCategory);
    let vanilla = 0;
    for (const i of scoped) if (isVanillaSource(i.source)) vanilla++;
    return { all: scoped.length, vanilla, override: scoped.length - vanilla };
  }, [templates, category, subCategory]);

  const canAdd = tab === 'base' ? selectedBaseId !== null : selectedResref !== null;

  const handleAdd = async () => {
    if (isAdding) return;
    setIsAdding(true);
    try {
      let itemIndex: number | undefined;
      if (tab === 'base' && selectedBaseId !== null) {
        const iconTemplate = templates.find(t => t.base_item === selectedBaseId);
        const response = await addItemByBaseType(selectedBaseId, iconTemplate?.resref);
        itemIndex = response.item_index;
      } else if (tab === 'template' && selectedResref) {
        const response = await addItemFromTemplate(selectedResref);
        itemIndex = response.item_index;
      }
      onClose();
      if (itemIndex !== undefined) {
        onItemAdded?.(itemIndex);
      }
    } catch (err) {
      handleError(err);
    } finally {
      setIsAdding(false);
    }
  };

  const categoryLabel = category === 'all'
    ? t('inventory.filterCategory')
    : category;

  const categoryMenu = (
    <Menu>
      <MenuItem text={t('inventory.categoryAll')} active={category === 'all'} onClick={() => { setCategory('all'); setSubCategory(null); }} />
      {(tab === 'base' ? baseCategories : templateCategories.map(([c]) => c)).map(cat => (
        <MenuItem key={cat} text={cat} active={category === cat} onClick={() => { setCategory(cat); setSubCategory(null); }} />
      ))}
    </Menu>
  );

  const activeSubCategories = tab === 'base' ? availableBaseSubCategories : availableTemplateSubCategories;

  const BaseRow = useCallback(({ index, style }: { index: number; style: React.CSSProperties }) => {
    const item = filteredBase[index];
    const selected = selectedBaseId === item.id;
    return (
      <div
        style={{
          ...style,
          display: 'flex', alignItems: 'center', gap: 8,
          padding: '0 12px', cursor: 'pointer',
          background: selected ? `${T.accent}12` : 'transparent',
          borderLeft: selected ? `2px solid ${T.accent}` : '2px solid transparent',
          borderBottom: `1px solid ${T.borderLight}`,
        }}
        onClick={() => setSelectedBaseId(item.id)}
      >
        <div style={{ flex: 1, minWidth: 0 }}>
          <div className={selected ? 't-semibold' : undefined} style={{ color: T.text, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{display(item.name)}</div>
          <div className="t-sm" style={{ color: T.textMuted }}>{item.category}</div>
        </div>
      </div>
    );
  }, [filteredBase, selectedBaseId]);

  const TemplateRow = useCallback(({ index, style }: { index: number; style: React.CSSProperties }) => {
    const item = filteredTemplates[index];
    const selected = selectedResref === item.resref;
    return (
      <div
        style={{
          ...style,
          display: 'flex', alignItems: 'center', gap: 8,
          padding: '0 12px', cursor: 'pointer',
          background: selected ? `${T.accent}12` : 'transparent',
          borderLeft: selected ? `2px solid ${T.accent}` : '2px solid transparent',
          borderBottom: `1px solid ${T.borderLight}`,
        }}
        onClick={() => setSelectedResref(item.resref)}
      >
        <div style={{ flex: 1, minWidth: 0 }}>
          <div className={selected ? 't-semibold' : undefined} style={{ color: T.text, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{display(item.name)}</div>
          <div className="t-sm" style={{ color: T.textMuted }}>
            {ITEM_CATEGORIES[item.category as keyof typeof ITEM_CATEGORIES] || 'Miscellaneous'} - {item.source}
          </div>
        </div>
      </div>
    );
  }, [filteredTemplates, selectedResref]);

  const itemCount = tab === 'base' ? filteredBase.length : filteredTemplates.length;
  const totalBase = baseItems.length;
  const totalTemplates = templates.length;

  const selectedName = tab === 'base'
    ? (selectedBaseId !== null ? display(baseItems.find(i => i.id === selectedBaseId)?.name) : t('inventory.selectNone'))
    : (selectedResref || t('inventory.selectNone'));

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      onClosed={handleReset}
      title={t('inventory.addItem')}
      width={720}
      minHeight={560}
      footerActions={
        <Button
          intent="primary"
          text={isAdding ? t('inventory.addingItem') : t('inventory.addItem')}
          disabled={!canAdd || isAdding}
          loading={isAdding}
          onClick={handleAdd}
        />
      }
      footerLeft={
        <span className="t-sm" style={{ color: T.textMuted }}>
          {t('inventory.selectedId')}: {selectedName}
        </span>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
        <div style={{ padding: '6px 0', borderBottom: `1px solid ${T.borderLight}`, display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
          <Tabs
            id="add-item-tabs"
            selectedTabId={tab}
            onChange={(id) => { setTab(id as TabId); clearFilters(); setSelectedBaseId(null); setSelectedResref(null); }}
            renderActiveTabPanelOnly
          >
            <Tab id="base" title={`${t('inventory.createNewBaseType')} (${totalBase})`} />
            <Tab id="template" title={`${t('inventory.addExistingTemplate')} (${totalTemplates})`} />
          </Tabs>
          <Popover content={categoryMenu} placement="bottom-start" minimal>
            <Button minimal rightIcon="caret-down" text={categoryLabel} />
          </Popover>
          <InputGroup
            leftIcon="search"
            placeholder={tab === 'base' ? t('inventory.searchBaseItems') : t('inventory.searchTemplates')}
            value={search}
            onChange={e => setSearch(e.target.value)}
            rightElement={search ? <Button icon="cross" minimal onClick={() => setSearch('')} /> : undefined}
            style={{ maxWidth: 220 }}
          />
          <Button minimal icon={<GameIcon icon={GiFunnel} size={14} />} text={t('inventory.filterClear')} onClick={clearFilters} disabled={!hasFilters} />
        </div>

        {tab === 'template' && (
          <div style={{ display: 'flex', flexWrap: 'wrap', alignItems: 'center', gap: 4, padding: '6px 0', borderBottom: `1px solid ${T.borderLight}`, flexShrink: 0 }}>
            <ButtonGroup minimal>
              <Button
                small
                intent={sourceFilter === 'all' ? 'primary' : 'none'}
                onClick={() => setSourceFilter('all')}
                text={`${t('inventory.source.all')} (${sourceCounts.all})`}
              />
              <Button
                small
                intent={sourceFilter === 'vanilla' ? 'primary' : 'none'}
                onClick={() => setSourceFilter('vanilla')}
                text={`${t('inventory.source.vanilla')} (${sourceCounts.vanilla})`}
              />
              <Button
                small
                intent={sourceFilter === 'override' ? 'primary' : 'none'}
                disabled={sourceCounts.override === 0}
                onClick={() => setSourceFilter('override')}
                text={`${t('inventory.source.override')} (${sourceCounts.override})`}
              />
            </ButtonGroup>
          </div>
        )}

        {activeSubCategories.length > 1 && (
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, padding: '6px 0', borderBottom: `1px solid ${T.borderLight}`, flexShrink: 0 }}>
            <Button
              small minimal
              active={subCategory === null}
              text={t('inventory.categoryAll')}
              onClick={() => setSubCategory(null)}
              style={{ color: subCategory === null ? T.accent : T.textMuted }}
            />
            {activeSubCategories.map(([sub]) => (
              <Button
                key={sub}
                small minimal
                active={subCategory === sub}
                text={t(SUB_CATEGORY_LABELS[sub] || sub)}
                onClick={() => setSubCategory(sub)}
                style={{ color: subCategory === sub ? T.accent : T.textMuted }}
              />
            ))}
          </div>
        )}

        <div ref={setContainerNode} style={{ flex: 1, minHeight: LIST_HEIGHT }}>
          {isLoading ? (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: listHeight, gap: 10 }}>
              <Spinner size={20} />
              <span style={{ color: T.textMuted }}>{tab === 'base' ? t('inventory.loadingBaseItems') : t('inventory.loadingTemplates')}</span>
            </div>
          ) : itemCount > 0 ? (
            <List
              height={listHeight}
              itemCount={itemCount}
              itemSize={ROW_HEIGHT}
              width={listWidth}
            >
              {tab === 'base' ? BaseRow : TemplateRow}
            </List>
          ) : (
            <div style={{ padding: 32, textAlign: 'center', color: T.textMuted }}>{t('inventory.noItemsMatch')}</div>
          )}
        </div>
      </div>
    </ParchmentDialog>
  );
}
