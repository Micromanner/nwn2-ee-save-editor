import { Fragment, useState, useMemo, useEffect, useRef, useCallback } from 'react';
import {
  Button, Card, Elevation, HTMLTable, InputGroup, Menu, MenuItem,
  NonIdealState, Switch, Tab, Tabs,
} from '@blueprintjs/core';
import { Popover } from '@blueprintjs/core';
import { GiBrokenShield, GiVisoredHelm, GiAnticlockwiseRotation, GiBackwardTime, GiInfo } from 'react-icons/gi';
import { GameIcon } from '../shared/GameIcon';
import { T, formatBytes } from '../theme';
import { formatDateTime } from '@/utils/dateFormat';
import { KVRow, ParchmentDialog, StepInput, LoadingState } from '../shared';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { useErrorHandler } from '@/hooks/useErrorHandler';
import { usePathsChangedEvent } from '@/hooks/usePathsChangedEvent';
import { useTranslations } from '@/hooks/useTranslations';
import { gameStateAPI } from '@/services/gameStateApi';
import { QuestsTab } from './QuestsTab/QuestsTab';
import type {
  CompanionInfluenceData,
  ModuleInfo,
  ModuleSummary,
  ModuleVariablesResponse,
  CampaignVariablesResponse,
  CampaignSettingsResponse,
  CampaignBackupInfo,
} from '@/services/gameStateApi';

const MODIFIED = '#d97706';

function RestoreBackupDialog({
  isOpen,
  onClose,
  onRestore,
  backups,
  isLoading,
  isRestoring,
}: {
  isOpen: boolean;
  onClose: () => void;
  onRestore: (path: string) => void;
  backups: CampaignBackupInfo[];
  isLoading: boolean;
  isRestoring: boolean;
}) {
  const t = useTranslations();
  const [selected, setSelected] = useState<string | null>(null);

  const selectedName = backups.find(b => b.path === selected)?.filename;

  return (
    <ParchmentDialog
      isOpen={isOpen}
      onClose={onClose}
      onOpened={() => setSelected(null)}
      title={t('common.restoreBackup')}
      width={780}
      minHeight={480}
      footerActions={
        <Button
          text={isRestoring ? t('common.restoring') : t('common.restore')}
          intent="primary"
          disabled={!selected || isRestoring}
          onClick={() => selected && onRestore(selected)}
        />
      }
      footerLeft={
        <span style={{ color: T.textMuted }}>
          {t('common.selected')}: {selectedName || t('common.none')}
        </span>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', margin: -16 }}>
        <div style={{ padding: '8px 12px', borderBottom: `1px solid ${T.borderLight}`, color: T.textMuted }}>
          {t('common.restoreBackupPrompt')}
        </div>
        <div style={{ overflowY: 'auto', maxHeight: 380, paddingLeft: 16 }}>
          {isLoading ? (
            <LoadingState height={160} text={t('common.loadingBackups')} />
          ) : backups.length === 0 ? (
            <div style={{ padding: 32, textAlign: 'center', color: T.textMuted }}>{t('common.noBackups')}</div>
          ) : backups.map(b => {
            const isActive = selected === b.path;
            return (
              <div
                key={b.path}
                onClick={() => setSelected(b.path)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  padding: '8px 12px', cursor: 'pointer',
                  background: isActive ? `${T.accent}12` : 'transparent',
                  borderLeft: isActive ? `2px solid ${T.accent}` : '2px solid transparent',
                  borderBottom: `1px solid ${T.borderLight}`,
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div className={`t-mono${isActive ? ' t-semibold' : ''}`} style={{
                    color: isActive ? T.accent : T.text,
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                  }}>
                    {b.filename}
                  </div>
                  <div style={{ color: T.textMuted, marginTop: 2 }}>
                    {formatDateTime(b.created_at * 1000)} &mdash; {formatBytes(b.size_bytes)}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </ParchmentDialog>
  );
}

function InfluenceSlider({ value, onChange }: { value: number; onChange: (v: number) => void }) {
  const pct = ((value + 100) / 200) * 100;
  const handleMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    const track = e.currentTarget;
    const update = (clientX: number) => {
      const rect = track.getBoundingClientRect();
      const ratio = Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
      onChange(Math.round(ratio * 200 - 100));
    };
    update(e.clientX);
    const onMove = (ev: MouseEvent) => update(ev.clientX);
    const onUp = () => { window.removeEventListener('mousemove', onMove); window.removeEventListener('mouseup', onUp); };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  };

  return (
    <div
      onMouseDown={handleMouseDown}
      style={{ flex: 1, height: 20, display: 'flex', alignItems: 'center', cursor: 'pointer', position: 'relative' }}
    >
      <div style={{ position: 'absolute', left: 0, right: 0, height: 4, background: '#d9d0c1', borderRadius: 2 }} />
      <div style={{ position: 'absolute', left: 0, width: `${pct}%`, height: 4, background: T.accent, borderRadius: 2 }} />
      <div style={{
        position: 'absolute', left: `calc(${pct}% - 7px)`,
        width: 14, height: 14, borderRadius: '50%',
        background: T.accent, border: '1px solid #8a4525',
        boxShadow: '0 1px 2px rgba(0,0,0,0.15)',
      }} />
    </div>
  );
}

function ReputationTab({ characterId, globalsVersion }: { characterId: number; globalsVersion: number }) {
  const t = useTranslations();
  const [companions, setCompanions] = useState<Record<string, CompanionInfluenceData>>({});
  const initialRef = useRef<Record<string, CompanionInfluenceData> | null>(null);
  const [influences, setInfluences] = useState<Record<string, number>>({});
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saveVersion, setSaveVersion] = useState(0);
  const { handleError } = useErrorHandler();

  useEffect(() => {
    const onChange = () => setSaveVersion(v => v + 1);
    window.addEventListener('save-changed', onChange);
    window.addEventListener('quest-edits-changed', onChange);
    return () => {
      window.removeEventListener('save-changed', onChange);
      window.removeEventListener('quest-edits-changed', onChange);
    };
  }, []);

  useEffect(() => {
    setIsLoading(true);
    setError(null);
    gameStateAPI.getCompanionInfluence()
      .then(res => {
        setCompanions(res.companions);
        initialRef.current = res.companions;
        const map: Record<string, number> = {};
        Object.entries(res.companions).forEach(([id, data]) => {
          map[id] = data.influence ?? 0;
        });
        setInfluences(map);
      })
      .catch(err => setError(err instanceof Error ? err.message : t('gameState.companions.failedToLoad')))
      .finally(() => setIsLoading(false));
  }, [characterId, globalsVersion, saveVersion]);

  const hasChanges = Object.entries(influences).some(
    ([id, val]) => val !== (initialRef.current?.[id]?.influence ?? 0)
  );

  const handleSave = async () => {
    if (!hasChanges) return;
    setIsSaving(true);
    try {
      const updates = Object.entries(influences)
        .filter(([id, val]) => val !== (companions[id]?.influence ?? 0))
        .map(([id, val]) => gameStateAPI.updateCompanionInfluence(companions[id].influence_var, val));
      const results = await Promise.allSettled(updates);
      const res = await gameStateAPI.getCompanionInfluence();
      setCompanions(res.companions);
      initialRef.current = res.companions;
      const failed = results.find((r): r is PromiseRejectedResult => r.status === 'rejected');
      if (failed) {
        handleError(failed.reason);
      } else {
        const map: Record<string, number> = {};
        Object.entries(res.companions).forEach(([id, data]) => {
          map[id] = data.influence ?? 0;
        });
        setInfluences(map);
      }
    } catch (err) {
      handleError(err);
    } finally {
      setIsSaving(false);
    }
  };

  const handleRevertAll = () => {
    if (!initialRef.current) return;
    const map: Record<string, number> = {};
    Object.entries(initialRef.current).forEach(([id, data]) => {
      map[id] = data.influence ?? 0;
    });
    setInfluences(map);
  };

  if (isLoading) {
    return (
      <LoadingState text={t('common.loadingSection', { section: t('navigation.gameState').toLowerCase() })} />
    );
  }

  if (error) {
    return (
      <div style={{ padding: 24 }}>
        <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} title={t('gameState.companions.failedToLoad')} description={error} />
      </div>
    );
  }

  const companionEntries = Object.entries(companions);

  if (companionEntries.length === 0) {
    return (
      <div style={{ padding: 24 }}>
        <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('gameState.companions.noData')} description={t('gameState.companions.noDataDescription')} />
      </div>
    );
  }

  return (
    <>
      {(hasChanges || isSaving) && (
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, padding: '8px 16px', borderBottom: `1px solid ${T.borderLight}` }}>
          <Button small minimal icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} onClick={handleRevertAll} style={{ color: MODIFIED }} disabled={isSaving}>{t('common.revertAll')}</Button>
          <Button small intent="primary" loading={isSaving} onClick={handleSave}>{t('common.save')}</Button>
        </div>
      )}

      {companionEntries.map(([companionId, companion]) => {
        const current = influences[companionId] ?? (companion.influence ?? 0);
        const initial = initialRef.current?.[companionId]?.influence ?? 0;
        const isModified = current !== initial;

        return (
          <div
            key={companionId}
            style={{
              borderBottom: `1px solid ${T.borderLight}`,
              padding: '10px 16px',
              borderLeft: isModified ? `3px solid ${MODIFIED}` : '3px solid transparent',
              background: isModified ? `${MODIFIED}10` : undefined,
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <span className="t-semibold" style={{ color: T.text }}>{companion.name || companionId}</span>
                {isModified && (
                  <span className="t-semibold" style={{ color: MODIFIED }}>{t('common.modified')}</span>
                )}
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <input
                  type="number"
                  className="bp6-input t-semibold t-center"
                  value={current}
                  min={-100}
                  max={100}
                  onChange={e => {
                    const v = parseInt(e.target.value, 10);
                    if (!isNaN(v)) setInfluences(prev => ({ ...prev, [companionId]: Math.max(-100, Math.min(100, v)) }));
                  }}
                  style={{
                    width: 56, height: 24,
                    background: T.surface, border: `1px solid ${isModified ? MODIFIED : T.borderLight}`,
                    borderRadius: 3, color: T.text,
                  }}
                />
                {isModified && (
                  <Button
                    small minimal icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />}
                    style={{ color: MODIFIED }}
                    onClick={() => setInfluences(prev => ({ ...prev, [companionId]: initial }))}
                  />
                )}
              </div>
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ color: T.textMuted, minWidth: 20 }}>-100</span>
              <InfluenceSlider value={current} onChange={v => setInfluences(prev => ({ ...prev, [companionId]: v }))} />
              <span style={{ color: T.textMuted, minWidth: 20, textAlign: 'right' }}>+100</span>
            </div>
          </div>
        );
      })}
    </>
  );
}

type VarEntry = { name: string; value: number | string };
type VarType = 'int' | 'string' | 'float' | 'bool';
type VarEdits = Partial<Record<VarType, Record<string, string>>>;

function VariableTable({ variables, search, type, edits, onEdit, onRevert }: {
  variables: VarEntry[];
  search: string;
  type: VarType;
  edits: VarEdits;
  onEdit: (type: VarType, name: string, value: string) => void;
  onRevert: (type: VarType, name: string) => void;
}) {
  const t = useTranslations();
  const typeEdits = edits[type] ?? {};
  const filtered = useMemo(() =>
    variables.filter(v => v.name.toLowerCase().includes(search.toLowerCase())),
    [variables, search],
  );

  return (
    <HTMLTable compact striped bordered style={{ width: '100%', tableLayout: 'fixed' }}>
      <colgroup><col /><col style={{ width: 200 }} /><col style={{ width: 260 }} /><col style={{ width: 40 }} /></colgroup>
      <thead>
        <tr><th>{t('gameState.variables.variable')}</th><th>{t('gameState.variables.current')}</th><th>{t('gameState.variables.value')}</th><th /></tr>
      </thead>
      <tbody>
        {filtered.length === 0 ? (
          <tr><td colSpan={4} style={{ textAlign: 'center', color: T.textMuted, padding: 20 }}>{t('gameState.variables.noVariables')}</td></tr>
        ) : filtered.map(v => {
          const isModified = v.name in typeEdits;
          const value = isModified ? typeEdits[v.name] : String(v.value);
          return (
            <tr key={v.name} style={{ borderLeft: isModified ? `3px solid ${MODIFIED}` : undefined, background: isModified ? `${MODIFIED}10` : undefined }}>
              <td className="t-mono" style={{ color: T.text }}>{v.name}</td>
              <td className="t-mono" style={{ color: T.textMuted }}>{String(v.value)}</td>
              <td>
                {type === 'bool' ? (
                  <Switch
                    checked={value !== '0'}
                    onChange={e => onEdit(type, v.name, e.currentTarget.checked ? '1' : '0')}
                    style={{ margin: 0 }}
                  />
                ) : (
                  <input
                    className="bp6-input t-mono"
                    type={type === 'string' ? 'text' : 'number'}
                    value={value}
                    onChange={e => onEdit(type, v.name, e.target.value)}
                    style={{
                      width: '100%', height: 24,
                      background: T.surface, border: `1px solid ${isModified ? MODIFIED : T.borderLight}`,
                      borderRadius: 3, color: T.text, padding: '2px 6px',
                    }}
                  />
                )}
              </td>
              <td style={{ textAlign: 'center' }}>
                {isModified && (
                  <Button small minimal icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} style={{ color: MODIFIED }} onClick={() => onRevert(type, v.name)} />
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </HTMLTable>
  );
}

function VariableSection({
  integers, strings, floats, booleans, edits, onEdit, onRevert, onRevertAll, onSave, isSaving, changeCount, warningText,
  onRestoreClick,
}: {
  integers: VarEntry[];
  strings: VarEntry[];
  floats: VarEntry[];
  booleans?: VarEntry[];
  edits: VarEdits;
  onEdit: (type: VarType, name: string, value: string) => void;
  onRevert: (type: VarType, name: string) => void;
  onRevertAll: () => void;
  onSave: () => void;
  isSaving: boolean;
  changeCount: number;
  warningText?: string;
  onRestoreClick?: () => void;
}) {
  const t = useTranslations();
  const [search, setSearch] = useState('');
  const total = integers.length + strings.length + floats.length + (booleans?.length ?? 0);

  return (
    <>
      <div style={{ padding: '10px 16px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <span className="t-semibold" style={{ color: T.textMuted }}>{total} {t('common.variables')}</span>
          {(changeCount > 0 || isSaving) && (
            <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
              <span className="t-semibold" style={{ color: MODIFIED }}>{changeCount} {t('common.modified')}</span>
              <Button small minimal icon={<GameIcon icon={GiAnticlockwiseRotation} size={14} />} onClick={onRevertAll} style={{ color: MODIFIED }} disabled={isSaving}>{t('common.revertAll')}</Button>
              <Button small intent="primary" loading={isSaving} onClick={onSave}>{t('common.save')}</Button>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {onRestoreClick && (
            <Button small minimal icon={<GameIcon icon={GiBackwardTime} size={14} />} style={{ color: T.textMuted }} onClick={onRestoreClick}>{t('common.restoreBackup')}</Button>
          )}
          <InputGroup
            small
            leftIcon="search"
            placeholder={t('common.search')}
            value={search}
            onChange={e => setSearch(e.target.value)}
            style={{ width: 180 }}
          />
        </div>
      </div>
      {warningText && changeCount > 0 && (
        <div style={{ margin: '0 16px', padding: '8px 12px', background: `${MODIFIED}12`, border: `1px solid ${MODIFIED}40`, borderRadius: 4, display: 'flex', alignItems: 'center', gap: 8 }}>
          <span className="t-semibold" style={{ color: MODIFIED }}>&#9888;</span>
          <span style={{ color: T.text }}>{warningText}</span>
        </div>
      )}
      <div style={{ padding: '0 16px 16px' }}>
        <Tabs id={`vars-${total}`} defaultSelectedTabId="integers" renderActiveTabPanelOnly>
          <Tab id="integers" title={`${t('gameState.variables.integers')} (${integers.length})`}
            panel={<div style={{ paddingTop: 8 }}><VariableTable variables={integers} search={search} type="int" edits={edits} onEdit={onEdit} onRevert={onRevert} /></div>}
          />
          <Tab id="strings" title={`${t('gameState.variables.strings')} (${strings.length})`}
            panel={<div style={{ paddingTop: 8 }}><VariableTable variables={strings} search={search} type="string" edits={edits} onEdit={onEdit} onRevert={onRevert} /></div>}
          />
          <Tab id="floats" title={`${t('gameState.variables.floats')} (${floats.length})`}
            panel={<div style={{ paddingTop: 8 }}><VariableTable variables={floats} search={search} type="float" edits={edits} onEdit={onEdit} onRevert={onRevert} /></div>}
          />
          {booleans && (
            <Tab id="booleans" title={`${t('gameState.variables.booleans')} (${booleans.length})`}
              panel={<div style={{ paddingTop: 8 }}><VariableTable variables={booleans} search={search} type="bool" edits={edits} onEdit={onEdit} onRevert={onRevert} /></div>}
            />
          )}
        </Tabs>
      </div>
    </>
  );
}

function useVariableEdits(onSaveCallback: (updates: Array<[string, string, VarType]>) => Promise<void>) {
  const { handleError } = useErrorHandler();
  const [edits, setEdits] = useState<VarEdits>({});
  const [isSaving, setIsSaving] = useState(false);

  const onEdit = (type: VarType, name: string, value: string) => {
    setEdits(prev => ({ ...prev, [type]: { ...prev[type], [name]: value } }));
  };

  const onRevert = (type: VarType, name: string) => {
    setEdits(prev => {
      const typeEdits = { ...prev[type] };
      delete typeEdits[name];
      return { ...prev, [type]: typeEdits };
    });
  };

  const onRevertAll = () => setEdits({});

  const updates = (Object.entries(edits) as Array<[VarType, Record<string, string>]>)
    .flatMap(([type, byName]) => Object.entries(byName).map(([name, value]): [string, string, VarType] => [name, value, type]));

  const onSave = async () => {
    setIsSaving(true);
    try {
      await onSaveCallback(updates);
      setEdits({});
    } catch (err) {
      handleError(err);
    } finally {
      setIsSaving(false);
    }
  };

  return { edits, onEdit, onRevert, onRevertAll, onSave, isSaving, changeCount: updates.length };
}

function useBackupDialog(
  listBackups: () => Promise<CampaignBackupInfo[]>,
  restoreBackup: (path: string) => Promise<void>,
  onRestored: () => void,
) {
  const { handleError } = useErrorHandler();
  const [isOpen, setIsOpen] = useState(false);
  const [backups, setBackups] = useState<CampaignBackupInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);

  const open = () => {
    setIsOpen(true);
    setIsLoading(true);
    setBackups([]);
    listBackups()
      .then(setBackups)
      .catch(handleError)
      .finally(() => setIsLoading(false));
  };

  const close = () => setIsOpen(false);

  const restore = async (path: string) => {
    setIsRestoring(true);
    try {
      await restoreBackup(path);
      setIsOpen(false);
      onRestored();
    } catch (err) {
      handleError(err);
    } finally {
      setIsRestoring(false);
    }
  };

  return { isOpen, backups, isLoading, isRestoring, open, close, restore };
}

function recordToVarEntries(record: Record<string, number | string>): VarEntry[] {
  return Object.entries(record)
    .map(([name, value]) => ({ name, value }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function ModuleInfoSection({ characterId, onModuleIdChange }: { characterId: number; onModuleIdChange?: (id: string | null) => void }) {
  const t = useTranslations();
  const { handleError } = useErrorHandler();
  const [moduleInfo, setModuleInfo] = useState<ModuleInfo | null>(null);
  const [availableModules, setAvailableModules] = useState<ModuleSummary[]>([]);
  const [selectedModuleId, setSelectedModuleId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    setIsLoading(true);
    gameStateAPI.listModules()
      .then(modules => {
        setAvailableModules(modules);
        const current = modules.find(m => m.is_current);
        const initial = current?.id || (modules.length > 0 ? modules[0].id : null);
        setSelectedModuleId(initial);
        onModuleIdChange?.(initial);
      })
      .catch(handleError)
      .finally(() => setIsLoading(false));
  }, [characterId, handleError]);

  useEffect(() => {
    if (!selectedModuleId) return;
    setIsLoading(true);
    gameStateAPI.getModuleById(selectedModuleId)
      .then(data => setModuleInfo(data))
      .catch(handleError)
      .finally(() => setIsLoading(false));
  }, [characterId, selectedModuleId, handleError]);

  const handleModuleSelect = (id: string) => {
    setSelectedModuleId(id);
    onModuleIdChange?.(id);
  };

  const selectedLabel = availableModules.find(m => m.id === selectedModuleId)?.name || selectedModuleId || '-';

  if (isLoading && !moduleInfo) {
    return <LoadingState text={t('common.loadingModule')} />;
  }

  const moduleMenu = (
    <Menu style={{ maxHeight: 300, overflowY: 'auto' }}>
      {availableModules.map(m => (
        <MenuItem
          key={m.id}
          text={`${m.name}${m.is_current ? ` (${t('gameState.module.current')})` : ''}`}
          active={m.id === selectedModuleId}
          onClick={() => handleModuleSelect(m.id)}
        />
      ))}
    </Menu>
  );

  return (
    <>
      {availableModules.length > 0 && (
        <div style={{ padding: '10px 16px', borderBottom: `1px solid ${T.borderLight}`, display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span className="t-semibold" style={{ color: T.textMuted }}>{t('gameState.module.module')}</span>
          <Popover content={moduleMenu} placement="bottom-end" minimal>
            <Button
              minimal
              rightIcon="caret-down"
              text={selectedLabel}
              style={{ textAlign: 'left', border: `1px solid ${T.border}`, background: T.surface, minWidth: 260 }}
            />
          </Popover>
        </div>
      )}
      <div style={{ padding: '10px 16px', display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
        <KVRow label={t('gameState.module.module')} value={moduleInfo?.module_name || '-'} />
        <KVRow label={t('gameState.module.campaign')} value={moduleInfo?.campaign || '-'} />
        <KVRow label={t('gameState.module.currentArea')} value={moduleInfo?.area_name || '-'} />
        <KVRow label={t('gameState.module.entryArea')} value={moduleInfo?.entry_area || '-'} />
      </div>
    </>
  );
}

function ModuleVariablesSection({ characterId, moduleId }: { characterId: number; moduleId?: string | null }) {
  const t = useTranslations();
  const [vars, setVars] = useState<ModuleVariablesResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const requestRef = useRef(0);
  const loadVars = useCallback(() => {
    const request = ++requestRef.current;
    setIsLoading(true);
    setError(null);
    gameStateAPI.getModuleVariables(moduleId)
      .then(data => { if (request === requestRef.current) setVars(data); })
      .catch(err => { if (request === requestRef.current) setError(err instanceof Error ? err.message : t('common.failedToLoad', { section: t('gameState.tabs.moduleAndVariables') })); })
      .finally(() => { if (request === requestRef.current) setIsLoading(false); });
  }, [characterId, moduleId]);

  useEffect(() => { loadVars(); }, [loadVars]);

  const varEdits = useVariableEdits(async (updates) => {
    await gameStateAPI.batchUpdateModuleVariables(updates, moduleId || undefined);
    loadVars();
  });

  const backupDialog = useBackupDialog(
    () => gameStateAPI.getModuleBackups(moduleId),
    path => gameStateAPI.restoreModuleBackup(path),
    loadVars,
  );

  const integers = vars ? recordToVarEntries(vars.integers) : [];
  const strings = vars ? recordToVarEntries(vars.strings) : [];
  const floats = vars ? recordToVarEntries(vars.floats) : [];

  if (isLoading && !vars) {
    return <LoadingState text={t('common.loadingVariables')} />;
  }

  if (error && !vars) {
    return <div style={{ padding: 16, color: T.negative }}>{error}</div>;
  }

  return (
    <>
      <VariableSection
        integers={integers}
        strings={strings}
        floats={floats}
        warningText={t('gameState.warnings.moduleVariables')}
        onRestoreClick={backupDialog.open}
        {...varEdits}
      />
      <RestoreBackupDialog
        isOpen={backupDialog.isOpen}
        onClose={backupDialog.close}
        onRestore={backupDialog.restore}
        backups={backupDialog.backups}
        isLoading={backupDialog.isLoading}
        isRestoring={backupDialog.isRestoring}
      />
    </>
  );
}

function CampaignSettingsSection({ characterId }: { characterId: number }) {
  const t = useTranslations();
  const [settings, setSettings] = useState<CampaignSettingsResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [campaignGuid, setCampaignGuid] = useState('');

  const [levelCap, setLevelCap] = useState(0);
  const [xpCap, setXpCap] = useState(0);
  const [compXp, setCompXp] = useState(0);
  const [henchXp, setHenchXp] = useState(0);
  const [attackNeutrals, setAttackNeutrals] = useState(false);
  const [autoXp, setAutoXp] = useState(false);
  const [journalSync, setJournalSync] = useState(false);
  const [noCharChange, setNoCharChange] = useState(false);
  const [personalRep, setPersonalRep] = useState(false);

  const populateFromSettings = (s: CampaignSettingsResponse) => {
    setLevelCap(s.level_cap ?? 0);
    setXpCap(s.xp_cap ?? 0);
    setCompXp(s.companion_xp_weight ?? 0);
    setHenchXp(s.henchman_xp_weight ?? 0);
    setAttackNeutrals(s.attack_neutrals);
    setAutoXp(s.auto_xp_award);
    setJournalSync(s.journal_sync);
    setNoCharChange(s.no_char_changing);
    setPersonalRep(s.use_personal_reputation);
  };

  useEffect(() => {
    setIsLoading(true);
    setError(null);
    gameStateAPI.getCampaignSettings()
      .then(s => { setSettings(s); setCampaignGuid(s.guid || ''); populateFromSettings(s); })
      .catch(err => setError(err instanceof Error ? err.message : t('gameState.campaign.failedToLoad')))
      .finally(() => setIsLoading(false));
  }, [characterId]);

  const reloadSettings = useCallback(() => {
    gameStateAPI.getCampaignSettings()
      .then(s => { setSettings(s); populateFromSettings(s); })
      .catch(() => {});
  }, [characterId]);

  const backupDialog = useBackupDialog(
    () => gameStateAPI.getCampaignBackups(campaignGuid),
    path => gameStateAPI.restoreCampaignFromBackup(path, campaignGuid),
    reloadSettings,
  );

  const hasChanges = settings !== null && (
    levelCap !== (settings.level_cap ?? 0) ||
    xpCap !== (settings.xp_cap ?? 0) ||
    compXp !== (settings.companion_xp_weight ?? 0) ||
    henchXp !== (settings.henchman_xp_weight ?? 0) ||
    attackNeutrals !== settings.attack_neutrals ||
    autoXp !== settings.auto_xp_award ||
    journalSync !== settings.journal_sync ||
    noCharChange !== settings.no_char_changing ||
    personalRep !== settings.use_personal_reputation
  );

  const handleRevert = () => {
    if (settings) populateFromSettings(settings);
  };

  const handleSave = async () => {
    if (!settings) return;
    setIsSaving(true);
    setError(null);
    try {
      const updated = {
        ...settings,
        level_cap: levelCap,
        xp_cap: xpCap,
        companion_xp_weight: compXp,
        henchman_xp_weight: henchXp,
        attack_neutrals: attackNeutrals,
        auto_xp_award: autoXp,
        journal_sync: journalSync,
        no_char_changing: noCharChange,
        use_personal_reputation: personalRep,
      };
      await gameStateAPI.updateCampaignSettings(updated);
      setSettings(updated);
    } catch (err) {
      const msg = err instanceof Error ? err.message : typeof err === 'object' && err !== null ? JSON.stringify(err) : String(err);
      setError(msg);
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading) {
    return <LoadingState text={t('common.loadingCampaignSettings')} />;
  }

  if (error && !settings) {
    return <div style={{ padding: 16 }}><NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} title={t('gameState.campaign.failedToLoad')} description={error} /></div>;
  }

  if (!settings) {
    return <div style={{ padding: 16 }}><NonIdealState icon={<GameIcon icon={GiInfo} size={40} />} title={t('gameState.campaign.noCampaign')} description={t('gameState.campaign.noCampaignDescription')} /></div>;
  }

  return (
    <>
      {error && (
        <div style={{ margin: '8px 16px', padding: '6px 10px', background: '#c6282810', border: `1px solid #c6282840`, borderRadius: 3, color: T.negative }}>
          {error}
        </div>
      )}
      <div style={{ padding: '10px 16px' }}>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
          <div>
            <div className="t-bold" style={{ color: T.textMuted, borderBottom: `1px solid ${T.borderLight}`, paddingBottom: 4, marginBottom: 10 }}>
              {t('gameState.campaign.progression')}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color: T.textMuted }}>{t('gameState.campaign.levelCap')}</span>
                <StepInput value={levelCap} onValueChange={setLevelCap} min={1} max={40} width={100} />
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color: T.textMuted }}>{t('gameState.campaign.xpCap')}</span>
                <StepInput value={xpCap} onValueChange={setXpCap} min={0} max={2000000} step={1000} width={120} />
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color: T.textMuted }}>{t('gameState.campaign.companionXpWeight')}</span>
                <StepInput value={compXp} onValueChange={setCompXp} min={0} max={1} step={0.05} width={100} />
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color: T.textMuted }}>{t('gameState.campaign.henchmanXpWeight')}</span>
                <StepInput value={henchXp} onValueChange={setHenchXp} min={0} max={1} step={0.05} width={100} />
              </div>
            </div>
          </div>

          <div>
            <div className="t-bold" style={{ color: T.textMuted, borderBottom: `1px solid ${T.borderLight}`, paddingBottom: 4, marginBottom: 10 }}>
              {t('gameState.campaign.gameplayFlags')}
            </div>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
              <Switch checked={attackNeutrals} onChange={() => setAttackNeutrals(v => !v)} label={t('gameState.campaign.attackNeutrals')} style={{ marginBottom: 0 }} />
              <Switch checked={autoXp} onChange={() => setAutoXp(v => !v)} label={t('gameState.campaign.autoXpAward')} style={{ marginBottom: 0 }} />
              <Switch checked={journalSync} onChange={() => setJournalSync(v => !v)} label={t('gameState.campaign.journalSync')} style={{ marginBottom: 0 }} />
              <Switch checked={noCharChange} onChange={() => setNoCharChange(v => !v)} label={t('gameState.campaign.lockCharChanges')} style={{ marginBottom: 0 }} />
              <Switch checked={personalRep} onChange={() => setPersonalRep(v => !v)} label={t('gameState.campaign.usePersonalRep')} style={{ marginBottom: 0 }} />
            </div>
          </div>
        </div>

        {hasChanges && (
          <div style={{ marginTop: 12, padding: '8px 12px', background: `${MODIFIED}12`, border: `1px solid ${MODIFIED}40`, borderRadius: 4, display: 'flex', alignItems: 'center', gap: 8 }}>
            <span className="t-semibold" style={{ color: MODIFIED }}>&#9888;</span>
            <span style={{ color: T.text }}>{t('gameState.campaign.sharedWarning')}</span>
          </div>
        )}

        <div style={{ marginTop: 12, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <Button small minimal icon={<GameIcon icon={GiBackwardTime} size={14} />} style={{ color: T.textMuted }} onClick={backupDialog.open}>{t('common.restoreBackup')}</Button>
          {hasChanges && (
            <div style={{ display: 'flex', gap: 8 }}>
              <Button minimal onClick={handleRevert}>{t('gameState.campaign.revertSettings')}</Button>
              <Button intent="primary" loading={isSaving} onClick={handleSave}>{t('gameState.campaign.saveSettings')}</Button>
            </div>
          )}
        </div>
      </div>

      <div style={{ borderTop: `1px solid ${T.borderLight}`, padding: '10px 16px' }}>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '4px 16px' }}>
          <KVRow label={t('gameState.campaign.startModule')} value={<span className="t-mono">{settings.start_module || '-'}</span>} />
          <KVRow label={t('gameState.campaign.moduleCount')} value={String(settings.module_names?.length ?? 0)} />
        </div>
        <div style={{ marginTop: 6, color: T.textMuted }}>
          <span className="t-semibold">{t('gameState.campaign.campaignFile')}: </span>
          <span className="t-mono">{settings.campaign_file_path || '-'}</span>
          {settings.description && (
            <span> &mdash; {settings.description}</span>
          )}
        </div>
      </div>
      <RestoreBackupDialog
        isOpen={backupDialog.isOpen}
        onClose={backupDialog.close}
        onRestore={backupDialog.restore}
        backups={backupDialog.backups}
        isLoading={backupDialog.isLoading}
        isRestoring={backupDialog.isRestoring}
      />
    </>
  );
}

function CampaignVariablesSection({ characterId, onGlobalsChanged }: { characterId: number; onGlobalsChanged: () => void }) {
  const t = useTranslations();
  const [vars, setVars] = useState<CampaignVariablesResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadVars = useCallback(() => {
    setIsLoading(true);
    setError(null);
    gameStateAPI.getCampaignVariables()
      .then(data => setVars(data))
      .catch(err => setError(err instanceof Error ? err.message : t('common.failedToLoad', { section: t('gameState.tabs.campaignAndVariables') })))
      .finally(() => setIsLoading(false));
  }, [characterId]);

  useEffect(() => { loadVars(); }, [loadVars]);

  const varEdits = useVariableEdits(async (updates) => {
    await gameStateAPI.batchUpdateCampaignVariables(updates);
    loadVars();
    onGlobalsChanged();
  });

  const backupDialog = useBackupDialog(
    () => gameStateAPI.getCampaignVariableBackups(),
    path => gameStateAPI.restoreCampaignVariableBackup(path),
    () => {
      loadVars();
      onGlobalsChanged();
    },
  );

  const integers = vars ? recordToVarEntries(vars.integers) : [];
  const strings = vars ? recordToVarEntries(vars.strings) : [];
  const floats = vars ? recordToVarEntries(vars.floats) : [];
  const booleans = vars ? recordToVarEntries(vars.booleans) : [];

  if (isLoading && !vars) {
    return <LoadingState text={t('common.loadingVariables')} />;
  }

  if (error && !vars) {
    return <div style={{ padding: 16, color: T.negative }}>{error}</div>;
  }

  return (
    <>
      <VariableSection
        integers={integers}
        strings={strings}
        floats={floats}
        booleans={booleans}
        warningText={t('gameState.warnings.campaignVariables')}
        onRestoreClick={backupDialog.open}
        {...varEdits}
      />
      <RestoreBackupDialog
        isOpen={backupDialog.isOpen}
        onClose={backupDialog.close}
        onRestore={backupDialog.restore}
        backups={backupDialog.backups}
        isLoading={backupDialog.isLoading}
        isRestoring={backupDialog.isRestoring}
      />
    </>
  );
}

export function GameStatePanel() {
  const t = useTranslations();
  const { character } = useCharacterContext();
  const characterId = character?.id;
  const [activeTab, setActiveTab] = useState<string>('reputation');
  const [campaignId, setCampaignId] = useState<string | null>(null);
  const [selectedModuleId, setSelectedModuleId] = useState<string | null>(null);
  const [pathsVersion, setPathsVersion] = useState(0);
  const [globalsVersion, setGlobalsVersion] = useState(0);

  usePathsChangedEvent(() => setPathsVersion(v => v + 1));

  useEffect(() => {
    if (!characterId) return;
    gameStateAPI.getCampaignSettings()
      .then(s => setCampaignId(s.guid || null))
      .catch(() => setCampaignId(null));
  }, [characterId, pathsVersion]);

  if (!characterId) {
    return (
      <div style={{ padding: 48 }}>
        <NonIdealState icon={<GameIcon icon={GiVisoredHelm} size={40} />} title={t('common.noCharacterLoaded')} description={t('common.loadSaveToView', { section: t('navigation.gameState').toLowerCase() })} />
      </div>
    );
  }

  return (
    <div style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
      <Fragment key={pathsVersion}>
        <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
          <div style={{ padding: '10px 16px 0' }}>
            <Tabs
              id="gamestate-tabs"
              selectedTabId={activeTab}
              onChange={(newTab) => setActiveTab(newTab as string)}
              large
            >
              <Tab id="reputation" title={t('gameState.tabs.companionInfluence')} />
              <Tab id="moduleVars" title={t('gameState.tabs.moduleAndVariables')} />
              <Tab id="campaignSettings" title={t('gameState.tabs.campaignAndVariables')} />
              <Tab id="quests" title={t('gameState.tabs.quests')} />
            </Tabs>
          </div>

          <div style={{ display: activeTab === 'reputation' ? undefined : 'none' }}>
            <ReputationTab characterId={characterId} globalsVersion={globalsVersion} />
          </div>
          <div style={{ display: activeTab === 'moduleVars' ? undefined : 'none' }}>
            <ModuleInfoSection characterId={characterId} onModuleIdChange={setSelectedModuleId} />
          </div>
          <div style={{ display: activeTab === 'campaignSettings' ? undefined : 'none' }}>
            <CampaignSettingsSection characterId={characterId} />
          </div>
        </Card>

        <div style={{ display: activeTab === 'moduleVars' ? undefined : 'none' }}>
          <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
            <ModuleVariablesSection characterId={characterId} moduleId={selectedModuleId} />
          </Card>
        </div>

        {campaignId && (
          <div style={{ display: activeTab === 'campaignSettings' ? undefined : 'none' }}>
            <Card elevation={Elevation.ONE} style={{ padding: 0, background: T.surface, overflow: 'hidden' }}>
              <CampaignVariablesSection characterId={characterId} onGlobalsChanged={() => setGlobalsVersion(v => v + 1)} />
            </Card>
          </div>
        )}

        {activeTab === 'quests' && <QuestsTab />}
      </Fragment>
    </div>
  );
}
