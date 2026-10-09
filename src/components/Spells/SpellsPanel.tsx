import { useState, useMemo, useCallback, useEffect } from 'react';
import { Button, InputGroup, Tabs, Tab, Popover, Menu, MenuItem, NonIdealState, Icon } from '@blueprintjs/core';
import { GiVisoredHelm, GiFunnel } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import { T, SPELL_SCHOOL_COLORS } from '../theme';
import { SplitPane, GroupedList, LoadingState } from '../shared';
import type { ListSection } from '../shared';
import { SpellDetail } from './SpellDetail';
import { useSubsystem } from '@/contexts/CharacterContext';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { mapKnownSpellsToSpellInfo, groupMemorizedSpells, mapCasterClasses } from '@/utils/spellUtils';
import type { SpellInfo } from '@/components/Spells/types';
import type { SpellsState } from '@/lib/bindings';
import { display } from '@/utils/dataHelpers';
import { useSpellManagement } from '@/hooks/useSpellManagement';
import { useToast } from '@/contexts/ToastContext';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { useTranslations } from '@/hooks/useTranslations';

type TabId = 'known' | 'prepared' | 'all';

const SPELLS_PER_PAGE = 10000;

const SPELL_SCHOOL_KEYS = [
  'abjuration', 'conjuration', 'divination', 'enchantment',
  'evocation', 'illusion', 'necromancy', 'transmutation',
];

export function SpellsPanel() {
  const { character, allSpellsCache } = useCharacterContext();

  const spells = useSubsystem('spells');
  const spellsData = spells.data as SpellsState | null;
  const abilitySpells = spellsData?.ability_spells ?? [];
  const { addSpell, removeSpell } = useSpellManagement();
  const { showToast } = useToast();
  const { handleError } = useErrorHandler();
  const t = useTranslations();

  const [tab, setTab] = useState<TabId>('known');
  const [search, setSearch] = useState('');
  const [activeSchool, setActiveSchool] = useState<string>('all');
  const [activeLevel, setActiveLevel] = useState<string>('all');
  const [activeClass, setActiveClass] = useState<string>('all');
  const [selectedSpell, setSelectedSpell] = useState<SpellInfo | null>(null);

  const [allSpells, setAllSpells] = useState<SpellInfo[]>([]);
  const [allSpellsLoading, setAllSpellsLoading] = useState(false);
  const [allSpellsPage, setAllSpellsPage] = useState(1);
  const [_allSpellsHasNext, setAllSpellsHasNext] = useState(false);
  const [_allSpellsHasPrev, setAllSpellsHasPrev] = useState(false);
  const [abilitiesOpen, setAbilitiesOpen] = useState(true);
  const [abilitySelected, setAbilitySelected] = useState(false);

  const debouncedSearch = useDebouncedValue(search, 300);

  useEffect(() => {
    if (character?.id && !spells.data && !spells.isLoading && !spells.error) {
      spells.load();
    }
  }, [character?.id, spells.data, spells.isLoading, spells.error, spells]);

  // Fetch when filters change on the 'all' tab. The preload cache is re-adopted
  // whenever it changes: it is refetched per character on a roster switch and
  // its spell levels come from that character's class columns.
  useEffect(() => {
    const characterId = character?.id;
    if (!characterId) return;

    const hasFilters = activeSchool !== 'all' || activeLevel !== 'all' || activeClass !== 'all' || debouncedSearch.length >= 3;
    if (!hasFilters && allSpellsPage === 1 && allSpellsCache) {
      setAllSpells(allSpellsCache.spells);
      setAllSpellsHasNext(allSpellsCache.pagination.has_next);
      setAllSpellsHasPrev(allSpellsCache.pagination.has_previous);
      return;
    }
    if (tab !== 'all') return;

    const loadAllSpells = async () => {
      setAllSpellsLoading(true);
      try {
        const schoolParam = activeSchool !== 'all' ? activeSchool : undefined;
        const levelParam = activeLevel !== 'all' ? activeLevel : undefined;
        const searchParam = debouncedSearch.length >= 3 ? debouncedSearch : undefined;
        const foundClass = activeClass !== 'all'
          ? casterClasses.find(c => c.name === activeClass)
          : undefined;
        const classParam = foundClass?.class_id;

        const response = await CharacterAPI.getLegitimateSpells({
          page: allSpellsPage,
          limit: SPELLS_PER_PAGE,
          schools: schoolParam,
          levels: levelParam,
          search: searchParam,
          ...(classParam !== undefined ? { class_id: classParam } : {}),
        });

        setAllSpells(response.spells);
        setAllSpellsHasNext(response.pagination.has_next);
        setAllSpellsHasPrev(response.pagination.has_previous);
      } catch {
        setAllSpells([]);
      } finally {
        setAllSpellsLoading(false);
      }
    };

    loadAllSpells();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [character?.id, tab, allSpellsPage, activeSchool, activeLevel, activeClass, debouncedSearch, allSpellsCache]);

  const casterClasses = useMemo(() => mapCasterClasses(spellsData?.spellcasting_classes), [spellsData]);

  const editableClasses = useMemo(
    () => casterClasses.filter(c => c.can_edit_spells),
    [casterClasses]
  );

  const handleAddSpell = useCallback(async (spellId: number, classIndex: number) => {
    try {
      await addSpell(spellId, classIndex);
      showToast(t('placeholders.spellAdded'), 'success');
      setSelectedSpell(null);
    } catch (error) { handleError(error); }
  }, [addSpell, showToast, handleError, t]);

  const handleRemoveSpell = useCallback(async (spellId: number, classIndex: number, spellLevel: number) => {
    try {
      await removeSpell(spellId, classIndex, spellLevel);
      showToast(t('placeholders.spellRemoved'), 'success');
      setSelectedSpell(null);
    } catch (error) { handleError(error); }
  }, [removeSpell, showToast, handleError, t]);

  const clearFilters = useCallback(() => {
    setSearch('');
    setActiveSchool('all');
    setActiveLevel('all');
    setActiveClass('all');
    setAllSpellsPage(1);
  }, []);

  const clientFilter = useCallback((spell: SpellInfo) => {
    if (search.length >= 3 && !spell.name.toLowerCase().includes(search.toLowerCase())) return false;
    if (activeSchool !== 'all' && (spell.school_name || spell.school) !== activeSchool) return false;
    if (activeLevel !== 'all' && spell.level !== Number(activeLevel)) return false;
    if (activeClass !== 'all') {
      const cls = casterClasses.find(c => c.name === activeClass);
      if (cls && spell.class_id !== undefined && spell.class_id !== cls.class_id) return false;
    }
    return true;
  }, [search, activeSchool, activeLevel, activeClass, casterClasses]);

  const knownSpells = useMemo(() => mapKnownSpellsToSpellInfo(spellsData?.known_spells), [spellsData?.known_spells]);
  const preparedSpells = useMemo(() => groupMemorizedSpells(spellsData?.memorized_spells), [spellsData?.memorized_spells]);

  const knownSpellIds = useMemo(() => {
    const ids = new Set<number>();
    for (const ks of spellsData?.known_spells ?? []) ids.add(ks.spell_id);
    return ids;
  }, [spellsData?.known_spells]);

  const allSpellsAvailable = useMemo(
    () => allSpells.filter(s => !knownSpellIds.has(s.id)),
    [allSpells, knownSpellIds]
  );

  const levelLabel = (l: number) => l === 0 ? t('spells.cantrips') : t('spells.levelSpells', { level: l });

  // Per-level cap stats for spontaneous casters (Sorcerer, Bard, etc.).
  // Key = spell level, value = { cap, actual, hasOther } where hasOther means
  // a prepared/all-known class also contributes spells at this level (skip counter).
  const knownLevelStats = useMemo(() => {
    const stats = new Map<number, { cap: number; actual: number; hasOther: boolean }>();
    const spellcastingClasses = spellsData?.spellcasting_classes ?? [];
    const activeClassObj = activeClass !== 'all' ? casterClasses.find(c => c.name === activeClass) : undefined;
    const cappedClassIds = new Set<number>();

    for (const cls of spellcastingClasses) {
      const caps = cls.expected_spells_known;
      if (Object.keys(caps).length === 0) continue;
      if (activeClassObj && cls.class_id !== activeClassObj.class_id) continue;
      cappedClassIds.add(cls.class_id);
      for (const [levelStr, cap] of Object.entries(caps)) {
        const level = Number(levelStr);
        const entry = stats.get(level) ?? { cap: 0, actual: 0, hasOther: false };
        entry.cap += cap;
        stats.set(level, entry);
      }
    }

    for (const ks of spellsData?.known_spells ?? []) {
      if (activeClassObj && ks.class_id !== activeClassObj.class_id) continue;
      const entry = stats.get(ks.level);
      if (!entry) continue;
      if (cappedClassIds.has(ks.class_id)) entry.actual += 1;
      else entry.hasOther = true;
    }

    return stats;
  }, [spellsData?.spellcasting_classes, spellsData?.known_spells, activeClass, casterClasses]);

  const knownSections: ListSection<SpellInfo>[] = useMemo(() => {
    const filtered = knownSpells.filter(clientFilter);
    const grouped = new Map<number, SpellInfo[]>();
    for (const s of filtered) {
      if (!grouped.has(s.level)) grouped.set(s.level, []);
      grouped.get(s.level)!.push(s);
    }
    return [...grouped.entries()]
      .sort(([a], [b]) => a - b)
      .map(([level, items]) => {
        const stat = knownLevelStats.get(level);
        let countLabel: string | undefined;
        let countColor: string | undefined;
        if (stat && !stat.hasOther) {
          countLabel = `${stat.actual}/${stat.cap}`;
          if (stat.actual > stat.cap) countColor = T.negative;
        }
        return {
          key: `lvl-${level}`,
          title: levelLabel(level),
          items,
          countLabel,
          countColor,
        };
      });
  }, [knownSpells, clientFilter, knownLevelStats]);

  const preparedSections: ListSection<SpellInfo>[] = useMemo(() => {
    const filtered = preparedSpells.filter(clientFilter);
    const grouped = new Map<number, SpellInfo[]>();
    for (const s of filtered) {
      if (!grouped.has(s.level)) grouped.set(s.level, []);
      grouped.get(s.level)!.push(s);
    }
    return [...grouped.entries()]
      .sort(([a], [b]) => a - b)
      .map(([level, items]) => ({
        key: `prep-${level}`,
        title: levelLabel(level),
        items,
      }));
  }, [preparedSpells, clientFilter]);

  const allSections: ListSection<SpellInfo>[] = useMemo(() => {
    const grouped = new Map<number, SpellInfo[]>();
    for (const s of allSpellsAvailable) {
      if (!grouped.has(s.level)) grouped.set(s.level, []);
      grouped.get(s.level)!.push(s);
    }
    return [...grouped.entries()]
      .sort(([a], [b]) => a - b)
      .map(([level, items]) => ({
        key: `all-${level}`,
        title: levelLabel(level),
        items,
      }));
  }, [allSpellsAvailable]);

  const hasFilters = search.length > 0 || activeSchool !== 'all' || activeLevel !== 'all' || activeClass !== 'all';

  const sections = tab === 'known' ? knownSections : tab === 'prepared' ? preparedSections : allSections;

  const schoolLabel = activeSchool === 'all' ? t('spells.schoolAll') : t(`spells.schools.${activeSchool.toLowerCase()}`);
  const levelFilterLabel = activeLevel === 'all' ? t('spells.levelAll') : activeLevel === '0' ? t('spells.cantrips') : t('spells.levelSpells', { level: activeLevel });
  const classLabel = activeClass === 'all' ? t('spells.classAll') : activeClass;

  const schoolMenu = (
    <Menu>
      <MenuItem text={t('common.all')} active={activeSchool === 'all'} onClick={() => setActiveSchool('all')} />
      {SPELL_SCHOOL_KEYS.map(s => (
        <MenuItem key={s} text={t(`spells.schools.${s}`)} active={activeSchool.toLowerCase() === s} onClick={() => setActiveSchool(s)} />
      ))}
    </Menu>
  );

  const levelMenu = (
    <Menu>
      <MenuItem text={t('common.all')} active={activeLevel === 'all'} onClick={() => setActiveLevel('all')} />
      <MenuItem text={t('spells.cantrips')} active={activeLevel === '0'} onClick={() => setActiveLevel('0')} />
      {[1, 2, 3, 4, 5, 6, 7, 8, 9].map(l => (
        <MenuItem key={l} text={t('spells.levelSpells', { level: l })} active={activeLevel === String(l)} onClick={() => setActiveLevel(String(l))} />
      ))}
    </Menu>
  );

  const classMenu = (
    <Menu>
      <MenuItem text={t('common.all')} active={activeClass === 'all'} onClick={() => setActiveClass('all')} />
      {casterClasses.map(c => (
        <MenuItem key={c.class_id} text={c.name} active={activeClass === c.name} onClick={() => setActiveClass(c.name)} />
      ))}
    </Menu>
  );

  const renderSpellItem = useCallback((spell: SpellInfo, selected: boolean) => {
    const schoolName = spell.school_name || spell.school;
    const schoolKey = schoolName ? `spells.schools.${schoolName.toLowerCase()}` : '';
    const schoolColor = SPELL_SCHOOL_COLORS[schoolKey] || T.textMuted;
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flex: 1, minWidth: 0 }}>
        <span className={selected ? 't-semibold' : undefined} style={{
          color: T.text,
          flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>
          {display(spell.name)}
        </span>
        {spell.is_domain_spell && (
          <span className="t-medium" style={{ color: '#c62828', flexShrink: 0 }}>{t('spells.domain')}</span>
        )}
        {spell.memorized_count !== undefined && spell.memorized_count > 0 && (
          <span className="t-medium" style={{ color: T.accent, flexShrink: 0 }}>{spell.memorized_count}x</span>
        )}
        <span className="t-medium" style={{ 
          color: schoolColor, 
          flexShrink: 0,
          width: 120,
          textAlign: 'left',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis'
        }}>
          {schoolName ? t(schoolKey) : ''}
        </span>
      </div>
    );
  }, [t]);

  const totalKnown = knownSpells.length;
  const totalPrepared = preparedSpells.length;

  const allSpellsCount = allSpellsAvailable.length;
  const allTabTitle = allSpellsCount > 0
    ? `${t('spells.allSpells')} (${allSpellsCount})`
    : t('spells.allSpells');

  const toolbar = (
    <>
      <Tabs
        id="spell-tabs" selectedTabId={tab}
        onChange={(id) => { setTab(id as TabId); setSelectedSpell(null); setAbilitySelected(false); setAllSpellsPage(1); }}
        renderActiveTabPanelOnly
      >
        <Tab id="known" title={`${t('spells.known')} (${totalKnown})`} />
        <Tab id="prepared" title={`${t('spells.prepared')} (${totalPrepared})`} />
        <Tab id="all" title={allTabTitle} />
      </Tabs>
      {casterClasses.length > 1 && (
        <Popover content={classMenu} placement="bottom-start" minimal>
          <Button minimal rightIcon="caret-down" text={classLabel} />
        </Popover>
      )}
      <Popover content={schoolMenu} placement="bottom-start" minimal>
        <Button minimal rightIcon="caret-down" text={schoolLabel} />
      </Popover>
      <Popover content={levelMenu} placement="bottom-start" minimal>
        <Button minimal rightIcon="caret-down" text={levelFilterLabel} />
      </Popover>
      <InputGroup
        leftIcon="search" placeholder={t('spells.filterSpells')} value={search}
        onChange={e => setSearch(e.target.value)}
        rightElement={search ? <Button icon="cross" minimal onClick={() => setSearch('')} /> : undefined}
        style={{ maxWidth: 220 }}
      />
      <Button minimal icon={<GameIcon icon={GiFunnel} size={14} />} text={t('common.clear')} onClick={clearFilters} disabled={!hasFilters} />
      <div style={{ flex: 1 }} />
    </>
  );

  const isLoadingInitial = spells.isLoading && !spellsData;

  const listContent = () => {
    if (isLoadingInitial) {
      return <LoadingState text={t('common.loadingSpells')} />;
    }

    if (!character) {
      return (
        <NonIdealState
          icon={<GameIcon icon={GiVisoredHelm} size={40} />}
          title={t('common.noCharacterLoaded')}
          description={t('common.loadSaveToView', { section: t('navigation.spells').toLowerCase() })}
        />
      );
    }

    if (tab === 'all' && allSpellsLoading) {
      return <LoadingState text={t('common.loadingSpells')} />;
    }

    if (sections.length === 0) {
      return (
        <div style={{ padding: 24, textAlign: 'center', color: T.textMuted }}>
          {t('common.noMatchFilters', { items: t('navigation.spells').toLowerCase() })}
        </div>
      );
    }

    return (
      <GroupedList
        sections={sections}
        selectedId={abilitySelected ? null : (selectedSpell?.id ?? null)}
        onSelect={(s) => { setSelectedSpell(s); setAbilitySelected(false); }}
        renderItem={renderSpellItem}
      />
    );
  };

  const list = (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ flex: 1, minHeight: 0 }}>
        {listContent()}
      </div>
      {abilitySpells.length > 0 && (
        <>
          <div
            style={{
              display: 'flex', alignItems: 'center', gap: 6,
              padding: '6px 12px',
              background: T.sectionBg,
              borderBottom: `1px solid ${T.sectionBorder}`,
              borderTop: `1px solid ${T.sectionBorder}`,
              cursor: 'pointer',
              userSelect: 'none',
            }}
            onClick={() => setAbilitiesOpen(o => !o)}
          >
            <Icon
              icon={abilitiesOpen ? 'caret-down' : 'caret-right'}
              size={16}
              style={{ color: T.accent, width: 16 }}
            />
            <span className="t-bold" style={{ color: T.accent, flex: 1 }}>{t('spells.specialAbilities')}</span>
            <span style={{ color: T.textMuted }}>{abilitySpells.length}</span>
          </div>
          {abilitiesOpen && (
            <div style={{ maxHeight: '35%', overflowY: 'auto', flexShrink: 0 }}>
              {abilitySpells.map(a => {
                const isSelected = abilitySelected && selectedSpell?.id === a.spell_id;
                return (
                  <div
                    key={a.spell_id}
                    onClick={() => {
                      setSelectedSpell({
                        id: a.spell_id,
                        name: a.name,
                        icon: a.icon,
                        description: a.description,
                        school_name: a.school_name,
                        level: a.innate_level,
                        innate_level: a.innate_level,
                        available_classes: [],
                      });
                      setAbilitySelected(true);
                    }}
                    style={{
                      display: 'flex', alignItems: 'center', gap: 8,
                      padding: '5px 12px 5px 26px',
                      borderBottom: `1px solid ${T.borderLight}`,
                      borderLeft: isSelected ? `2px solid ${T.accent}` : '2px solid transparent',
                      background: isSelected ? `${T.accent}12` : 'transparent',
                      cursor: 'pointer',
                    }}
                  >
                    <span className={isSelected ? 't-semibold' : undefined} style={{ color: T.text, flex: 1 }}>{display(a.name)}</span>
                    {a.school_name && (
                      <span style={{ color: T.textMuted, flexShrink: 0 }}>{t(`spells.schools.${a.school_name.toLowerCase()}`)}</span>
                    )}
                    <span className="t-medium" style={{ color: T.accent, flexShrink: 0 }}>{t('classes.lvl')} {a.innate_level}</span>
                  </div>
                );
              })}
            </div>
          )}
        </>
      )}
    </div>
  );

  const detailMemoizedCount = selectedSpell?.memorized_count;

  return (
    <SplitPane
      toolbar={toolbar}
      list={list}
      detail={
        <SpellDetail
          spell={selectedSpell}
          memorizedCount={detailMemoizedCount}
          isOwned={abilitySelected || tab !== 'all'}
          editableClasses={editableClasses}
          onAdd={handleAddSpell}
          onRemove={handleRemoveSpell}
        />
      }
    />
  );
}
