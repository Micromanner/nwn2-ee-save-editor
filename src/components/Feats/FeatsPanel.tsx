import { useState, useMemo, useCallback, useEffect } from 'react';
import { Button, InputGroup, Tabs, Tab, Popover, Menu, MenuItem, NonIdealState } from '@blueprintjs/core';
import { GiBrokenShield, GiVisoredHelm, GiMagnifyingGlass, GiFunnel } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import { T, FEAT_TYPE_COLORS } from '../theme';
import { SplitPane, GroupedList, LoadingState } from '../shared';
import type { ListSection } from '../shared';
import { FeatDetail } from './FeatDetail';
import { useSubsystem, useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { aggregateFeats, filterFeatsByType, sortFeats, FEAT_TYPE_LABELS, getFeatTypeLabel } from '@/utils/featUtils';
import type { FeatInfo } from '@/lib/bindings';
import type { FeatsState } from '@/components/Feats/types';
import { FEAT_TYPES } from '@/components/Feats/types';
import { display } from '@/utils/dataHelpers';
import { useFeatManagement } from '@/hooks/useFeatManagement';
import { useToast } from '@/contexts/ToastContext';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';

type TabId = 'my' | 'all';

const FEAT_TYPE_OPTIONS: { labelKey: string; value: number }[] = [
  { labelKey: 'feats.categories.general', value: FEAT_TYPES.GENERAL },
  { labelKey: 'feats.categories.proficiency', value: FEAT_TYPES.PROFICIENCY },
  { labelKey: 'feats.categories.skillSave', value: FEAT_TYPES.SKILL_SAVE },
  { labelKey: 'feats.categories.metamagic', value: FEAT_TYPES.METAMAGIC },
  { labelKey: 'feats.categories.divine', value: FEAT_TYPES.DIVINE },
  { labelKey: 'feats.categories.epic', value: FEAT_TYPES.EPIC },
  { labelKey: 'feats.categories.class', value: FEAT_TYPES.CLASS },
  { labelKey: 'feats.categories.background', value: FEAT_TYPES.BACKGROUND },
  { labelKey: 'feats.categories.spellcasting', value: FEAT_TYPES.SPELLCASTING },
  { labelKey: 'feats.categories.history', value: FEAT_TYPES.HISTORY },
  { labelKey: 'feats.categories.heritage', value: FEAT_TYPES.HERITAGE },
  { labelKey: 'feats.categories.itemCreation', value: FEAT_TYPES.ITEM_CREATION },
  { labelKey: 'feats.categories.racial', value: FEAT_TYPES.RACIAL },
  { labelKey: 'feats.categories.domain', value: FEAT_TYPES.DOMAIN },
];


const MY_FEATS_SECTIONS: { key: keyof FeatsState['summary']; titleKey: string }[] = [
  { key: 'general_feats', titleKey: 'feats.generalFeats' },
  { key: 'class_feats', titleKey: 'feats.classFeats' },
  { key: 'background_feats', titleKey: 'feats.backgroundFeats' },
  { key: 'domain_feats', titleKey: 'feats.domainFeats' },
  { key: 'custom_feats', titleKey: 'feats.customFeats' },
];

const FEATS_PER_PAGE = 10000;

export function FeatsPanel() {
  const { character, allFeatsCache } = useCharacterContext();
  const featsSubsystem = useSubsystem('feats');
  const featsData = featsSubsystem.data as FeatsState | null;
  const isLoading = featsSubsystem.isLoading;
  const loadError = featsSubsystem.error;
  const { addFeat, removeFeat } = useFeatManagement({ autoLoadFeats: false });
  const { showToast } = useToast();
  const { handleError } = useErrorHandler();
  const t = useTranslations();

  const [tab, setTab] = useState<TabId>('my');
  const [search, setSearch] = useState('');
  const [activeTypeBit, setActiveTypeBit] = useState<number | null>(null);
  const [selectedFeat, setSelectedFeat] = useState<FeatInfo | null>(null);

  const [allFeats, setAllFeats] = useState<FeatInfo[]>([]);
  const [allFeatsLoading, setAllFeatsLoading] = useState(false);
  const [allFeatsError, setAllFeatsError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [_hasNext, setHasNext] = useState(false);
  const [_hasPrevious, setHasPrevious] = useState(false);
  const [allFeatsTotal, setAllFeatsTotal] = useState(0);

  const debouncedSearch = useDebouncedValue(search, 300);

  useEffect(() => {
    if (character && !featsData && !featsSubsystem.isLoading && !featsSubsystem.error) {
      featsSubsystem.load();
    }
  }, [character, featsData, featsSubsystem]);

  // Fetch when filters change on the 'all' tab. The preload cache is re-adopted
  // whenever it changes (refetched per character on a roster switch).
  useEffect(() => {
    if (!character?.id) return;

    const hasFilters = activeTypeBit != null || debouncedSearch.length >= 3;
    if (!hasFilters && currentPage === 1 && allFeatsCache) {
      setAllFeats(allFeatsCache.feats);
      setAllFeatsTotal(allFeatsCache.pagination.total);
      setHasNext(allFeatsCache.pagination.has_next);
      setHasPrevious(allFeatsCache.pagination.has_previous);
      setAllFeatsLoading(false);
      setAllFeatsError(null);
      return;
    }
    if (tab !== 'all') {
      setAllFeatsLoading(false);
      return;
    }

    let cancelled = false;
    setAllFeatsLoading(true);
    setAllFeatsError(null);

    const featTypeBitmask = activeTypeBit ?? undefined;
    const searchParam = debouncedSearch.length >= 3 ? debouncedSearch : undefined;

    CharacterAPI.getLegitimateFeats({
      page: currentPage,
      limit: FEATS_PER_PAGE,
      featType: featTypeBitmask,
      search: searchParam,
    }).then(response => {
      if (!cancelled) {
        setAllFeats(response.feats);
        setAllFeatsTotal(response.pagination.total);
        setHasNext(response.pagination.has_next);
        setHasPrevious(response.pagination.has_previous);
      }
    }).catch(err => {
      if (!cancelled) {
        setAllFeatsError(err instanceof Error ? err.message : t('common.failedToLoad', { section: t('navigation.feats').toLowerCase() }));
      }
    }).finally(() => {
      if (!cancelled) setAllFeatsLoading(false);
    });

    return () => { cancelled = true; };
  }, [tab, character?.id, currentPage, activeTypeBit, debouncedSearch, allFeatsCache, t]);

  useEffect(() => {
    setCurrentPage(1);
  }, [tab, activeTypeBit, debouncedSearch]);

  const clearFilters = useCallback(() => {
    setSearch('');
    setActiveTypeBit(null);
  }, []);

  const handleAddFeat = useCallback(async (featId: number) => {
    try {
      const response = await addFeat(featId);

      let toastMsg = t('placeholders.featAdded') as string;

      const parts: string[] = [];
      if (response.auto_added_feats.length > 0) {
        const featNames = response.auto_added_feats.map(f => f.label).join(', ');
        parts.push(t('placeholders.autoAddedFeats', { feats: featNames }) as string);
      }
      if (response.auto_modified_abilities.length > 0) {
        const abilityChanges = response.auto_modified_abilities.map(a => `${t(`abilityScores.short.${a.ability.slice(0, 3).toLowerCase()}`)} ${a.old_value} -> ${a.new_value}`).join(', ');
        parts.push(t('placeholders.autoModifiedAbilities', { abilities: abilityChanges }) as string);
      }

      if (parts.length > 0) {
        toastMsg = t('placeholders.featAddedWithPrereqs', { details: parts.join('; ') }) as string;
      }

      showToast(toastMsg, 'success');
      setSelectedFeat(null);
    } catch (error) { handleError(error); }
  }, [addFeat, showToast, handleError, t]);

  const handleRemoveFeat = useCallback(async (featId: number) => {
    try {
      await removeFeat(featId);
      showToast(t('placeholders.featRemoved'), 'success');
    } catch (error) { handleError(error); }
  }, [removeFeat, showToast, handleError, t]);

  const allMyFeats = useMemo(() => aggregateFeats(featsData?.summary), [featsData]);

  const ownedFeatIds = useMemo(() => {
    const ids = new Set<number>();
    for (const f of allMyFeats) ids.add(f.id);
    return ids;
  }, [allMyFeats]);

  const allFeatsAvailable = useMemo(
    () => allFeats.filter(f => !ownedFeatIds.has(f.id)),
    [allFeats, ownedFeatIds]
  );

  const mySections: ListSection<FeatInfo>[] = useMemo(() => {
    const typeFilter = activeTypeBit !== null ? new Set([activeTypeBit]) : new Set<number>();
    const searchLower = search.toLowerCase();

    return MY_FEATS_SECTIONS.flatMap(({ key, titleKey }) => {
      const raw = (featsData?.summary?.[key] as FeatInfo[] | undefined) ?? [];
      let items = typeFilter.size > 0 ? filterFeatsByType(raw, typeFilter) : raw;
      if (search.length >= 3) {
        items = items.filter(f => f.name.toLowerCase().includes(searchLower));
      }
      items = sortFeats(items, 'name');
      if (items.length === 0) return [];
      return [{ key, title: t(titleKey), items }];
    });
  }, [featsData, activeTypeBit, search, t]);

  const allSections: ListSection<FeatInfo>[] = useMemo(() => {
    const grouped = new Map<string, FeatInfo[]>();
    for (const f of allFeatsAvailable) {
      const labelKey = getFeatTypeLabel(f.type);
      if (!grouped.has(labelKey)) grouped.set(labelKey, []);
      grouped.get(labelKey)!.push(f);
    }
    return [...grouped.entries()].map(([labelKey, items]) => ({
      key: labelKey,
      title: `${t(labelKey)} ${t('feats.feats')}`,
      items,
    }));
  }, [allFeatsAvailable, t]);

  const totalOwned = allMyFeats.length;
  const hasFilters = search.length > 0 || activeTypeBit !== null;
  const sections = tab === 'my' ? mySections : allSections;

  const renderFeatItem = useCallback((feat: FeatInfo, selected: boolean) => {
    const labelKey = getFeatTypeLabel(feat.type);
    const typeColor = FEAT_TYPE_COLORS[labelKey] || T.textMuted;
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
        <span className={selected ? 't-semibold' : undefined} style={{
          color: T.text,
          flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>
          {display(feat.name)}
        </span>
        <span className="t-medium" style={{ 
          color: typeColor, 
          flexShrink: 0,
          width: 120,
          textAlign: 'left',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis'
        }}>
          {t(labelKey)}
        </span>
      </div>
    );
  }, [t]);

  const typeLabel = activeTypeBit === null
    ? t('common.typeAll')
    : (t(FEAT_TYPE_LABELS[activeTypeBit]) || t('common.typeAll'));

  const typeMenu = (
    <Menu>
      <MenuItem text={t('common.all')} active={activeTypeBit === null} onClick={() => setActiveTypeBit(null)} />
      {FEAT_TYPE_OPTIONS.map(opt => (
        <MenuItem
          key={opt.value}
          text={t(opt.labelKey)}
          active={activeTypeBit === opt.value}
          onClick={() => setActiveTypeBit(opt.value)}
        />
      ))}
    </Menu>
  );

  const allFeatsCount = allFeatsAvailable.length;
  const allTabTitle = allFeatsCount > 0
    ? `${t('feats.allFeats')} (${allFeatsCount})`
    : t('feats.allFeats');

  const toolbar = (
    <>
      <Tabs
        id="feat-tabs" selectedTabId={tab}
        onChange={(id) => { setTab(id as TabId); setSelectedFeat(null); }}
        renderActiveTabPanelOnly
      >
        <Tab id="my" title={`${t('feats.myFeats')} (${totalOwned})`} />
        <Tab id="all" title={allTabTitle} />
      </Tabs>
      <Popover content={typeMenu} placement="bottom-start" minimal>
        <Button minimal rightIcon="caret-down" text={typeLabel} />
      </Popover>
      <InputGroup
        leftIcon="search" placeholder={t('feats.filterFeats')} value={search}
        onChange={e => setSearch(e.target.value)}
        rightElement={search ? <Button icon="cross" minimal onClick={() => setSearch('')} /> : undefined}
        style={{ maxWidth: 220 }}
      />
      <Button minimal icon={<GameIcon icon={GiFunnel} size={14} />} text={t('common.clear')} onClick={clearFilters} disabled={!hasFilters} />
      <div style={{ flex: 1 }} />
    </>
  );

  const renderList = () => {
    if (tab === 'my') {
      if (isLoading && !featsData) {
        return <LoadingState text={t('feats.loadingFeats')} />;
      }
      if (loadError) {
        return (
          <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} title={t('common.failedToLoad', { section: t('navigation.feats').toLowerCase() })} description={loadError} />
        );
      }
      if (!character || !featsData) {
        return (
          <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('common.noCharacterLoaded')} description={t('common.loadSaveToView', { section: t('navigation.feats').toLowerCase() })} />
        );
      }
      if (sections.length === 0) {
        return (
          <NonIdealState icon={<GameIcon icon={GiMagnifyingGlass} size={40} />} title={t('common.noMatchFilters', { items: t('navigation.feats').toLowerCase() })} action={<Button minimal text={t('common.clearFilters')} onClick={clearFilters} />} />
        );
      }
    }

    if (tab === 'all') {
      if (!character) {
        return (
          <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('common.noCharacterLoaded')} description={t('common.loadSaveToView', { section: t('navigation.feats').toLowerCase() })} />
        );
      }
      if (allFeatsLoading) {
        return <LoadingState text={t('feats.loadingFeats')} />;
      }
      if (allFeatsError) {
        return (
          <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} title={t('common.failedToLoad', { section: t('navigation.feats').toLowerCase() })} description={allFeatsError} />
        );
      }
      if (sections.length === 0) {
        return (
          <NonIdealState icon={<GameIcon icon={GiMagnifyingGlass} size={40} />} title={t('common.noMatchFilters', { items: t('navigation.feats').toLowerCase() })} action={<Button minimal text={t('common.clearFilters')} onClick={clearFilters} />} />
        );
      }
    }

    return (
      <GroupedList
        sections={sections}
        selectedId={selectedFeat?.id ?? null}
        onSelect={setSelectedFeat}
        renderItem={renderFeatItem}
      />
    );
  };

  return (
    <SplitPane
      toolbar={toolbar}
      list={renderList()}
      detail={
        <FeatDetail
          feat={selectedFeat}
          isOwned={tab === 'my'}
          onAdd={handleAddFeat}
          onRemove={handleRemoveFeat}
        />
      }
    />
  );
}
