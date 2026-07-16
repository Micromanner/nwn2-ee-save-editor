import { useState, useMemo, useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Button, InputGroup, Tabs, Tab, NonIdealState, Spinner } from '@blueprintjs/core';
import { GiMagnifyingGlass } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { ParchmentDialog, GroupedList } from '../shared';
import type { ListSection } from '../shared';
import { T } from '../theme';
import { ClassDetail } from './ClassDetail';
import type { CategorizedClassesResponse, ClassInfo } from '@/hooks/useClassesLevel';
import { useTranslations } from '@/hooks/useTranslations';
import { display } from '@/utils/dataHelpers';

interface ClassSelectorDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (classInfo: ClassInfo) => void;
  currentClassIds: number[];
  categorizedClasses: CategorizedClassesResponse | null | undefined;
  isChanging: boolean;
  totalLevel: number;
  maxLevel: number;
  maxClasses: number;
  currentClassCount: number;
}

interface PrestigeClassOption {
  id: number;
  can_take: boolean;
  reason: string;
}

function checkCanSelect(
  cls: ClassInfo,
  currentClassIds: number[],
  isChanging: boolean,
  totalLevel: number,
  maxLevel: number,
  maxClasses: number,
  currentClassCount: number,
  prestigeEligibility: Map<number, PrestigeClassOption> | null,
  requirementsNotMetLabel: string,
): { ok: boolean; reason?: string; prestigeBlocked?: boolean } {
  const hasClass = currentClassIds.includes(cls.id);
  if (hasClass && !isChanging) return { ok: false, reason: 'Already have this class' };
  if (!isChanging && currentClassCount >= maxClasses) return { ok: false, reason: `Max ${maxClasses} classes` };
  if (!isChanging && totalLevel >= maxLevel) return { ok: false, reason: 'At max level' };
  const eligibility = prestigeEligibility?.get(cls.id);
  if (eligibility && !eligibility.can_take && !hasClass) {
    return { ok: false, reason: requirementsNotMetLabel, prestigeBlocked: true };
  }
  return { ok: true };
}

export function ClassSelectorDialog({
  isOpen, onClose, onSelect, currentClassIds, categorizedClasses, isChanging,
  totalLevel, maxLevel, maxClasses, currentClassCount,
}: ClassSelectorDialogProps) {
  const t = useTranslations();
  const [search, setSearch] = useState('');
  const [tab, setTab] = useState<string>('base');
  const [selectedClass, setSelectedClass] = useState<ClassInfo | null>(null);
  const [prestigeEligibility, setPrestigeEligibility] = useState<Map<number, PrestigeClassOption> | null>(null);

  useEffect(() => {
    if (!isOpen) {
      setPrestigeEligibility(null);
      return;
    }
    const ctrl = { cancelled: false };
    invoke<PrestigeClassOption[]>('get_available_prestige_classes')
      .then((options) => {
        if (!ctrl.cancelled) {
          setPrestigeEligibility(new Map(options.map(o => [o.id, o])));
        }
      })
      .catch((err) => {
        console.error('get_available_prestige_classes failed:', err);
      });
    return () => { ctrl.cancelled = true; };
  }, [isOpen]);

  const buildSections = useCallback((groups: Record<string, ClassInfo[]>): ListSection<ClassInfo>[] => {
    return Object.entries(groups).flatMap(([focusKey, classList]) => {
      if (!classList.length) return [];
      const focusName = categorizedClasses?.focus_info[focusKey]?.name || focusKey;
      return [{ key: focusKey, title: focusName, items: classList }];
    });
  }, [categorizedClasses]);

  const allClasses = useMemo((): ClassInfo[] => {
    if (!categorizedClasses) return [];
    const result: ClassInfo[] = [];
    for (const focusClasses of Object.values(categorizedClasses.categories.base)) result.push(...focusClasses);
    for (const focusClasses of Object.values(categorizedClasses.categories.prestige)) result.push(...focusClasses);
    for (const focusClasses of Object.values(categorizedClasses.categories.npc)) result.push(...focusClasses);
    return result;
  }, [categorizedClasses]);

  const sections: ListSection<ClassInfo>[] = useMemo(() => {
    if (!categorizedClasses) return [];

    if (search.trim()) {
      const q = search.toLowerCase();
      const filtered = allClasses.filter(c =>
        c.name.toLowerCase().includes(q) || c.label.toLowerCase().includes(q)
      );
      return filtered.length > 0 ? [{ key: 'search', title: `Results for "${search}"`, items: filtered }] : [];
    }

    const groups = categorizedClasses.categories[tab as keyof typeof categorizedClasses.categories] ?? {};
    return buildSections(groups);
  }, [categorizedClasses, search, tab, allClasses, buildSections]);

  const requirementsNotMetLabel = t('classes.requirementsNotMet');

  const selectedValidity = useMemo(() => {
    if (!selectedClass) return { ok: false };
    return checkCanSelect(selectedClass, currentClassIds, isChanging, totalLevel, maxLevel, maxClasses, currentClassCount, prestigeEligibility, requirementsNotMetLabel);
  }, [selectedClass, currentClassIds, isChanging, totalLevel, maxLevel, maxClasses, currentClassCount, prestigeEligibility, requirementsNotMetLabel]);

  const renderClassItem = useCallback((cls: ClassInfo, selected: boolean) => {
    const { ok, reason } = checkCanSelect(cls, currentClassIds, isChanging, totalLevel, maxLevel, maxClasses, currentClassCount, prestigeEligibility, requirementsNotMetLabel);
    return (
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, opacity: ok ? 1 : 0.5 }}>
        <span className={selected ? 't-semibold' : undefined} style={{
          color: T.text,
          flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
        }}>
          {display(cls.name)}
        </span>
        {!ok && reason && (
          <span className="t-xs" style={{ color: T.negative, flexShrink: 0 }}>{reason}</span>
        )}
      </div>
    );
  }, [currentClassIds, isChanging, totalLevel, maxLevel, maxClasses, currentClassCount, prestigeEligibility, requirementsNotMetLabel]);

  const totalClasses = allClasses.length;

  const toolbar = (
    <>
      {!search.trim() && (
        <Tabs
          id="class-type-tabs" selectedTabId={tab}
          onChange={(id) => { setTab(id as string); setSelectedClass(null); }}
          renderActiveTabPanelOnly
        >
          <Tab id="base" title={t('classes.base')} />
          <Tab id="prestige" title={t('classes.prestige')} />
          <Tab id="npc" title={t('classes.npc')} />
        </Tabs>
      )}
      <InputGroup
        leftIcon="search" placeholder={t('classes.searchClasses')} value={search}
        onChange={e => setSearch(e.target.value)}
        rightElement={search ? <Button icon="cross" minimal onClick={() => setSearch('')} /> : undefined}
        style={{ maxWidth: 200 }}
      />
      <div style={{ flex: 1 }} />
      <span className="t-sm" style={{ color: T.textMuted }}>
        {totalClasses} classes | {currentClassCount}/{maxClasses} slots
      </span>
    </>
  );

  const renderList = () => {
    if (!categorizedClasses) {
      return <NonIdealState icon={<Spinner size={30} />} title={t('common.loading')} />;
    }
    if (sections.length === 0) {
      return <NonIdealState icon={<GameIcon icon={GiMagnifyingGlass} size={40} />} title={t('classes.noClassesFound')} />;
    }
    return (
      <GroupedList
        sections={sections}
        selectedId={selectedClass?.id ?? null}
        onSelect={setSelectedClass}
        renderItem={renderClassItem}
      />
    );
  };

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      title={isChanging ? t('classes.changeClass') : t('classes.selectClass')}
      width={820}
      minHeight={650}
      footerActions={
        <Button
          intent="primary"
          disabled={!selectedClass || !selectedValidity.ok}
          onClick={() => selectedClass && onSelect(selectedClass)}
        >
          {t('classes.selectClass')}
        </Button>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, paddingBottom: 8, borderBottom: `1px solid ${T.borderLight}` }}>
          {toolbar}
        </div>
        <div style={{ display: 'flex', flex: 1, minHeight: 0, marginTop: 8 }}>
          <div style={{ width: 340, borderRight: `1px solid ${T.borderLight}`, overflow: 'auto' }}>
            {renderList()}
          </div>
          <div style={{ flex: 1, overflow: 'auto' }}>
            <ClassDetail
              cls={selectedClass}
              canSelect={selectedValidity.ok}
              selectReason={selectedValidity.prestigeBlocked
                ? t('classes.prestigeCrashWarning')
                : selectedValidity.reason}
            />
          </div>
        </div>
      </div>
    </ParchmentDialog>
  );
}
