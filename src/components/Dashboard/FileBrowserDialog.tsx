import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { List, RowComponentProps, useListRef } from 'react-window';
import { useInfiniteLoader } from 'react-window-infinite-loader';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import {
  Button, ButtonGroup, Checkbox, Dialog, DialogBody, DialogFooter, Divider,
  Icon, InputGroup, Spinner, NonIdealState,
} from '@blueprintjs/core';
import { GiOpenFolder, GiExitDoor, GiFullFolder, GiFoldedPaper, GiBrokenShield, GiMagnifyingGlass } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { useDebouncedValue } from '@/hooks/useDebouncedValue';
import { GameIcon } from '../shared/GameIcon';
import { LoadingState } from '../shared';
import { display, formatNumber } from '@/utils/dataHelpers';
import { T, formatBytes, formatUnixSeconds } from '../theme';
import type { BackupManifest } from '@/services/backupApi';
import { BackupDetailsDialog } from './BackupDetailsDialog';

type SortField = 'name' | 'date' | 'size' | 'character_name' | 'save_name';
type SortDirection = 'asc' | 'desc';

export interface FileInfo {
  name: string;
  path: string;
  size: number;
  modified: string;
  is_directory: boolean;
  save_name?: string;
  character_name?: string;
  display_name?: string;
  changes?: BackupManifest | null;
}

function summarizeChanges(manifest: BackupManifest | null | undefined, t: (key: string, values?: Record<string, unknown>) => string): string {
  if (!manifest) return t('fileBrowser.changesUnknown');
  if (manifest.saves.length === 0) return t('fileBrowser.changesNone');

  let characterCount = 0;
  let questCount = 0;
  let partyCount = 0;
  let variableCount = 0;
  for (const record of manifest.saves) {
    for (const character of record.characters) {
      characterCount += character.changes.length;
    }
    questCount += record.quests.length;
    partyCount += record.party.length;
    variableCount += record.variables.length;
  }

  const parts: string[] = [];
  if (characterCount > 0) parts.push(t('fileBrowser.changesCharacters', { count: characterCount }));
  if (questCount > 0) parts.push(t('fileBrowser.changesQuests', { count: questCount }));
  if (partyCount > 0) parts.push(t('fileBrowser.changesParty', { count: partyCount }));
  if (variableCount > 0) parts.push(t('fileBrowser.changesVariables', { count: variableCount }));

  return parts.length > 0 ? parts.join(', ') : t('fileBrowser.changesNone');
}

interface FileListResponse {
  files: FileInfo[];
  total_count: number;
  path: string;
  current_path: string;
}

interface FileBrowserDialogProps {
  isOpen: boolean;
  onClose: () => void;
  mode: 'load-saves' | 'manage-backups' | 'load-vault';
  onSelectFile?: (file: FileInfo) => void;
  currentPath?: string;
  onPathChange?: (path: string) => void;
  onDeleteBackup?: (file: FileInfo) => Promise<void>;
  onDeleteBackups?: (files: FileInfo[]) => Promise<void>;
  canRestore?: boolean;
  refreshKey?: number;
  demoFiles?: FileInfo[];
}

const ITEMS_PER_PAGE = 50;
const ROW_HEIGHT = 40;
const SEARCH_DEBOUNCE_MS = 250;

export function FileBrowserDialog({
  isOpen,
  onClose,
  mode,
  onSelectFile,
  currentPath = '',
  onPathChange,
  onDeleteBackup,
  onDeleteBackups,
  canRestore = true,
  refreshKey = 0,
  demoFiles,
}: FileBrowserDialogProps) {
  const t = useTranslations();

  const [files, setFiles] = useState<FileInfo[]>([]);
  const [totalFiles, setTotalFiles] = useState(0);
  const [loading, setLoading] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showRestoreConfirm, setShowRestoreConfirm] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [showDetailsDialog, setShowDetailsDialog] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);
  const [sortField, setSortField] = useState<SortField>('name');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebouncedValue(search, SEARCH_DEBOUNCE_MS);
  const lastSearch = useRef(debouncedSearch);
  const listRef = useListRef(null);
  const [containerNode, setContainerNode] = useState<HTMLDivElement | null>(null);
  const previousRefreshKey = useRef(refreshKey);
  const lastPath = useRef(currentPath);

  const [containerHeight, setContainerHeight] = useState(400);
  const abortControllerRef = useRef<AbortController | null>(null);
  const [backupRootPath, setBackupRootPath] = useState('');

  const isBackupRoot = mode === 'manage-backups' && (
    backupRootPath === '' || currentPath === backupRootPath
  );

  useEffect(() => {
    if (!containerNode) return;
    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.height > 0) {
          setContainerHeight(Math.floor(entry.contentRect.height));
        }
      }
    });
    observer.observe(containerNode);
    return () => observer.disconnect();
  }, [containerNode, isOpen]);

  const loadFiles = useCallback(async (isInitial = true) => {
    if (demoFiles) {
      setFiles(demoFiles);
      setTotalFiles(demoFiles.length);
      return;
    }

    if (isInitial) {
      setLoading(true);
      setError(null);
    } else {
      setLoadingMore(true);
    }

    try {
      if (mode === 'manage-backups') {
        interface BrowseBackupEntry {
          name: string;
          path: string;
          size: number;
          timestamp: string;
          created_at: number;
          character_name: string | null;
          save_name: string | null;
          changes: BackupManifest | null;
        }
        const backups = await invoke<BrowseBackupEntry[]>('browse_backups', { path: currentPath });
        if (backupRootPath === '') {
          setBackupRootPath(currentPath);
        }
        const isSaveFolder = !currentPath.replace(/\\/g, '/').replace(/\/$/, '').toLowerCase().endsWith('/backups');
        const fileInfos: FileInfo[] = backups.map(b => ({
          name: b.name,
          path: b.path,
          size: b.size,
          modified: String(b.created_at),
          is_directory: !isSaveFolder,
          save_name: b.save_name ?? (b.timestamp ? `Backup ${b.timestamp}` : undefined),
          character_name: b.character_name ?? undefined,
          changes: b.changes,
        }));
        setFiles(fileInfos);
        setTotalFiles(fileInfos.length);
      } else if (mode === 'load-vault') {
        interface BrowseVaultResponse {
          files: { name: string; path: string; size: number; modified: number }[];
          total_count: number;
          path: string;
        }
        const response = await invoke<BrowseVaultResponse>('browse_localvault');
        const fileInfos: FileInfo[] = response.files.map(f => ({
          name: f.name,
          path: f.path,
          size: f.size,
          modified: String(f.modified),
          is_directory: false,
          display_name: f.name,
        }));
        setFiles(fileInfos);
        setTotalFiles(fileInfos.length);
        if (response.path && response.path !== currentPath) {
          onPathChange?.(response.path);
        }
      } else {
        interface BrowseSavesResponse {
          files: {
            name: string; path: string; size: number; modified: number;
            is_directory: boolean; save_name: string | null;
            character_name: string | null; thumbnail: string | null;
          }[];
          total_count: number;
          path: string;
          current_path: string;
        }
        const response = await invoke<BrowseSavesResponse>('browse_saves', {
          path: currentPath || null,
          limit: ITEMS_PER_PAGE,
          offset: isInitial ? 0 : files.length,
          search: debouncedSearch || null,
        });
        const fileInfos: FileInfo[] = response.files.map(f => ({
          name: f.name,
          path: f.path,
          size: f.size,
          modified: String(f.modified),
          is_directory: f.is_directory,
          save_name: f.save_name ?? undefined,
          character_name: f.character_name ?? undefined,
        }));
        if (isInitial) {
          setFiles(fileInfos);
        } else {
          setFiles(prev => [...prev, ...fileInfos]);
        }
        setTotalFiles(response.total_count);
        if (response.current_path && response.current_path !== currentPath) {
          lastPath.current = response.current_path;
          onPathChange?.(response.current_path);
        }
      }
    } catch (err) {
      console.error('Failed to load files:', err);
      setError(err instanceof Error ? err.message : 'Failed to read directory');
      setFiles([]);
      setTotalFiles(0);
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, [mode, currentPath, onPathChange, files.length, backupRootPath, debouncedSearch]);

  useEffect(() => {
    if (isOpen) {
      const pathChanged = lastPath.current !== currentPath;
      const refreshRequested = previousRefreshKey.current !== refreshKey;
      const searchChanged = mode === 'load-saves' && lastSearch.current !== debouncedSearch;

      if (pathChanged || refreshRequested || searchChanged || files.length === 0) {
        if (pathChanged) {
          setFiles([]);
          setTotalFiles(0);
          setSelectedFile(null);
          setSelectedPaths(new Set());
          lastPath.current = currentPath;
        }
        if (refreshRequested) {
          setSelectedPaths(new Set());
          setSelectedFile(null);
        }
        previousRefreshKey.current = refreshKey;
        lastSearch.current = debouncedSearch;
        loadFiles(true);
      } else {
        listRef.current?.scrollToRow({ index: 0 });
      }
    }
    return () => { abortControllerRef.current?.abort(); };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, currentPath, mode, refreshKey, debouncedSearch]);

  const isItemLoaded = (index: number) => index < files.length;
  const loadMoreItems = () => {
    if (loadingMore || files.length >= totalFiles) return Promise.resolve();
    return loadFiles(false);
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(prev => (prev === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortField(field);
      setSortDirection(field === 'character_name' ? 'asc' : 'desc');
    }
  };

  const isBackupMode = mode === 'manage-backups';

  const sortedFiles = useMemo(() => {
    const query = debouncedSearch.trim().toLowerCase();
    const filtered = files.filter(f =>
      !(isBackupMode && f.name.toLowerCase() === 'campaign_backups')
      && (!query || (f.save_name || f.display_name || f.name).toLowerCase().includes(query)),
    );

    return [...filtered].sort((a, b) => {
      if (a.is_directory !== b.is_directory) return a.is_directory ? -1 : 1;
      let cmp = 0;
      switch (sortField) {
        case 'name':
          cmp = (a.display_name || a.name).localeCompare(b.display_name || b.name, undefined, { numeric: true, sensitivity: 'base' });
          break;
        case 'date':
          cmp = parseFloat(a.modified) - parseFloat(b.modified);
          break;
        case 'size':
          cmp = a.size - b.size;
          break;
        case 'character_name':
          cmp = (a.character_name || '').localeCompare(b.character_name || '', undefined, { numeric: true, sensitivity: 'base' });
          if (cmp === 0) return parseFloat(b.modified) - parseFloat(a.modified);
          break;
        case 'save_name':
          cmp = (a.save_name || a.display_name || a.name).localeCompare(b.save_name || b.display_name || b.name, undefined, { numeric: true, sensitivity: 'base' });
          if (cmp === 0) return parseFloat(b.modified) - parseFloat(a.modified);
          break;
      }
      return sortDirection === 'asc' ? cmp : -cmp;
    });
  }, [files, sortField, sortDirection, isBackupMode, debouncedSearch]);

  const rowCount = mode === 'load-saves' ? totalFiles : sortedFiles.length;
  const onRowsRendered = useInfiniteLoader({
    isRowLoaded: isItemLoaded,
    loadMoreRows: loadMoreItems,
    rowCount,
    threshold: 5,
  });

  const FileRow = ({ index, style }: RowComponentProps) => {
    const file = sortedFiles[index];
    if (!file) {
      return (
        <div style={{ ...style, padding: '0 16px' }}>
          <LoadingState variant="inline" text={t('fileBrowser.loadingMore')} />
        </div>
      );
    }

    const isSelected = selectedFile?.path === file.path;

    return (
      <div
        style={{
          ...style,
          display: 'flex',
          alignItems: 'center',
          padding: '0 16px',
          cursor: 'pointer',
          borderBottom: `1px solid ${T.borderLight}`,
          borderLeft: isSelected ? `3px solid ${T.accent}` : '3px solid transparent',
          background: isSelected ? 'rgba(160, 82, 45, 0.08)' : 'transparent',


        }}
        onClick={() => handleFileClick(file)}
        onDoubleClick={() => {
          if (mode === 'manage-backups' && file.is_directory) {
            setSelectedFile(null);
            onPathChange?.(file.path);
          } else if (onSelectFile) {
            setSelectedFile(file);
            if (mode === 'manage-backups') {
              setShowRestoreConfirm(true);
            } else {
              onSelectFile(file);
              onClose();
            }
          }
        }}
      >
        {isBackupMode && (
          <div
            style={{ width: 32, display: 'flex', alignItems: 'center' }}
            onClick={(e) => e.stopPropagation()}
          >
            <Checkbox
              style={{ margin: 0 }}
              checked={selectedPaths.has(file.path)}
              onChange={() => handleToggleSelect(file.path)}
            />
          </div>
        )}
        <div style={{ ...(isVault ? { flex: 3 } : { width: 260 }), display: 'flex', alignItems: 'center', gap: 6, overflow: 'hidden' }}>
          <GameIcon
            icon={file.is_directory ? GiFullFolder : GiFoldedPaper}
            size={14}
            color={T.textMuted}
          />
          <span className="t-medium" style={{
            color: isSelected ? T.accent : T.text,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {display(file.display_name || file.name)}
          </span>
        </div>
        {!isVault && (
          <>
            <div style={{ width: 240, color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {display(file.character_name)}
            </div>
            <div style={{ flex: 1, color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {display(file.save_name)}
            </div>
            {isBackupMode && !isBackupRoot && (
              <div style={{ width: 220, color: T.textMuted, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {summarizeChanges(file.changes, t)}
              </div>
            )}
          </>
        )}
        <div style={{ width: 160, color: T.textMuted }}>
          {formatUnixSeconds(file.modified)}
        </div>
        <div style={{ width: 120, paddingLeft: 24, color: T.textMuted }}>
          {formatBytes(file.size)}
        </div>
      </div>
    );
  };

  const handleToggleSelect = (path: string) => {
    setSelectedPaths(prev => {
      const next = new Set(prev);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  };

  const handleToggleSelectAll = () => {
    if (selectedPaths.size === sortedFiles.length) {
      setSelectedPaths(new Set());
    } else {
      setSelectedPaths(new Set(sortedFiles.map(f => f.path)));
    }
  };

  const handleSelectExceptNewest = () => {
    const byDateDesc = [...sortedFiles].sort((a, b) => parseFloat(b.modified) - parseFloat(a.modified));
    const older = byDateDesc.slice(1).map(f => f.path);
    setSelectedPaths(new Set(older));
  };

  const handleSelectOlderThan3 = () => {
    const byDateDesc = [...sortedFiles].sort((a, b) => parseFloat(b.modified) - parseFloat(a.modified));
    const older = byDateDesc.slice(3).map(f => f.path);
    setSelectedPaths(new Set(older));
  };

  const selectedFilesList = useMemo(() => {
    if (selectedPaths.size > 0) {
      return sortedFiles.filter(f => selectedPaths.has(f.path));
    }
    return selectedFile ? [selectedFile] : [];
  }, [sortedFiles, selectedPaths, selectedFile]);

  const selectedTotalBytes = useMemo(() => {
    return selectedFilesList.reduce((sum, f) => sum + (f.size || 0), 0);
  }, [selectedFilesList]);

  const totalListedBytes = useMemo(() => {
    return files.reduce((sum, f) => sum + (f.size || 0), 0);
  }, [files]);

  const summaryLabel = useMemo(() => {
    const count = files.length;
    const size = formatBytes(totalListedBytes);
    if (mode === 'manage-backups') {
      return t('fileBrowser.totalBackupsSummary', { count, size });
    }
    if (mode === 'load-vault') {
      return t('fileBrowser.totalVaultSummary', { count, size });
    }
    if (files.length < totalFiles) {
      return t('fileBrowser.itemCount', {
        loaded: formatNumber(files.length),
        total: formatNumber(totalFiles),
      });
    }
    return t('fileBrowser.totalSavesSummary', { count, size });
  }, [mode, files.length, totalFiles, totalListedBytes, t]);

  const handleDeleteExecute = async () => {
    if (selectedFilesList.length === 0) return;
    setIsDeleting(true);
    try {
      if (onDeleteBackups) {
        await onDeleteBackups(selectedFilesList);
      } else if (onDeleteBackup) {
        for (const f of selectedFilesList) {
          await onDeleteBackup(f);
        }
      }
      setSelectedPaths(new Set());
      setSelectedFile(null);
      setShowDeleteConfirm(false);
    } catch (err) {
      console.error('Failed to delete backup(s):', err);
      setError(err instanceof Error ? err.message : 'Failed to delete backup(s)');
    } finally {
      setIsDeleting(false);
    }
  };

  const handleFileClick = (file: FileInfo) => setSelectedFile(file);

  const handleConfirm = () => {
    if (!selectedFile) return;
    if (mode === 'manage-backups' && selectedFile.is_directory) {
      setSelectedFile(null);
      onPathChange?.(selectedFile.path);
      return;
    }
    if (!onSelectFile) return;
    if (mode === 'manage-backups') {
      setShowRestoreConfirm(true);
    } else {
      onSelectFile(selectedFile);
      onClose();
    }
  };

  const handleRestoreConfirmed = () => {
    if (selectedFile && onSelectFile) {
      onSelectFile(selectedFile);
      setShowRestoreConfirm(false);
      onClose();
    }
  };

  const handleChangeLocation = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: t('fileBrowser.selectSaveLocation'),
      });
      if (selected && typeof selected === 'string') {
        onPathChange?.(selected);
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
    }
  };

  const title =
    mode === 'load-vault'  ? t('fileBrowser.importFromVault') :
    mode === 'load-saves'  ? t('fileBrowser.loadSave') :
                             t('fileBrowser.manageBackups');

  const actionLabel =
    mode === 'load-vault'  ? t('fileBrowser.import') :
    mode === 'load-saves'  ? t('fileBrowser.load') :
                             t('fileBrowser.restore');

  const locationLabel =
    mode === 'load-vault'  ? t('fileBrowser.vaultLocation') :
    mode === 'load-saves'  ? t('fileBrowser.saveLocation') :
                             t('fileBrowser.backupLocation');

  const dateLabel =
    mode === 'manage-backups' ? t('fileBrowser.created') : t('fileBrowser.modified');

  const nameLabel =
    mode === 'load-vault' ? t('fileBrowser.characterName') : t('fileBrowser.folderName');

  const isVault = mode === 'load-vault';

  const renderSortIcon = (field: SortField) => {
    if (sortField !== field) return null;
    return <Icon icon={sortDirection === 'asc' ? 'chevron-up' : 'chevron-down'} size={12} />;
  };

  const footerLeft = (
    <div>
      {mode === 'manage-backups' && !isBackupRoot && (
        <Button
          minimal
          icon={<GameIcon icon={GiExitDoor} size={14} />}
          text={t('fileBrowser.back')}
          onClick={() => {
            setSelectedFile(null);
            onPathChange?.(backupRootPath);
          }}
        />
      )}
    </div>
  );

  const footerActions = (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
      {isBackupMode && (
        <>
          <ButtonGroup minimal>
            <Button
              text={selectedPaths.size === sortedFiles.length && sortedFiles.length > 0
                ? t('fileBrowser.deselectAll')
                : t('fileBrowser.selectAll')}
              disabled={sortedFiles.length === 0}
              onClick={handleToggleSelectAll}
            />
            {!isBackupRoot && sortedFiles.length > 1 && (
              <Button
                text={t('fileBrowser.selectExceptNewest')}
                onClick={handleSelectExceptNewest}
              />
            )}
            {!isBackupRoot && sortedFiles.length > 3 && (
              <Button
                text={t('fileBrowser.selectOlderThan3')}
                onClick={handleSelectOlderThan3}
              />
            )}
          </ButtonGroup>
          <Divider style={{ height: 20, margin: '0 6px' }} />
        </>
      )}
      <Button text={t('fileBrowser.cancel')} onClick={onClose} />
      {isBackupMode && !isBackupRoot && (
        <Button
          text={t('fileBrowser.details')}
          disabled={!selectedFile}
          onClick={() => setShowDetailsDialog(true)}
        />
      )}
      {isBackupMode && (onDeleteBackup || onDeleteBackups) && (
        selectedPaths.size > 0 ? (
          <Button
            intent="danger"
            text={t('fileBrowser.deleteSelected', { count: selectedPaths.size })}
            onClick={() => setShowDeleteConfirm(true)}
          />
        ) : selectedFile ? (
          <Button
            intent="danger"
            text={isBackupRoot ? t('fileBrowser.deleteSaveBackups') : t('fileBrowser.delete')}
            onClick={() => setShowDeleteConfirm(true)}
          />
        ) : null
      )}
      {isBackupMode && isBackupRoot && (
        <Button
          intent="primary"
          text={t('fileBrowser.open')}
          disabled={!selectedFile}
          onClick={handleConfirm}
          style={{ background: T.accent }}
        />
      )}
      {(mode === 'load-saves' || mode === 'load-vault' || (isBackupMode && !isBackupRoot && canRestore)) && (
        <Button
          intent="primary"
          text={actionLabel}
          disabled={!selectedFile}
          onClick={handleConfirm}
          style={{ background: T.accent }}
        />
      )}
    </div>
  );

  return (
    <>
      <Dialog
        isOpen={isOpen}
        onClose={onClose}
        title={title}
        className="file-browser-dialog"
        transitionDuration={0}
        style={{
          width: 'round(nearest, min(1400px, 95vw), 2px)',
          height: 'round(nearest, 80vh, 2px)',
          paddingBottom: 0,
          background: T.surface,
          display: 'flex',
          flexDirection: 'column' as const,
          transform: 'none',
        }}
        canOutsideClickClose
      >
        <DialogBody useOverflowScrollContainer={false} style={{ padding: 0, margin: 0, background: T.surface, display: 'flex', flexDirection: 'column', overflow: 'hidden', flex: 1, minHeight: 0 }}>
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            padding: '8px 16px',
            borderBottom: `1px solid ${T.borderLight}`,
            background: T.surfaceAlt,
          }}>
            <GameIcon icon={GiOpenFolder} size={14} color={T.textMuted} />
            <span style={{ color: T.textMuted }}>{locationLabel}</span>
            <span className="t-mono" style={{
              color: T.text,
              flex: 1,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}>
              {currentPath || t('fileBrowser.defaultLocation')}
            </span>
            <div style={{ display: 'flex', alignItems: 'center', gap: 12, flexShrink: 0 }}>
              <InputGroup
                small
                leftIcon="search"
                placeholder={t('fileBrowser.searchSaveName')}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                rightElement={search ? <Button icon="cross" minimal small onClick={() => setSearch('')} /> : undefined}
                style={{ width: 220 }}
              />
              {isBackupMode && selectedPaths.size > 0 ? (
                <span style={{ color: T.accent, fontWeight: 600, fontSize: 13 }}>
                  {t('fileBrowser.selectedCount', {
                    count: selectedPaths.size,
                    size: formatBytes(selectedTotalBytes),
                  })}
                </span>
              ) : (
                <span style={{ color: T.textMuted, fontSize: 13 }}>
                  {summaryLabel}
                </span>
              )}
              {mode !== 'manage-backups' && (
                <Button
                  minimal
                  small
                  icon={<GameIcon icon={GiOpenFolder} size={14} />}
                  text={t('fileBrowser.changeLocation')}
                  onClick={handleChangeLocation}
                />
              )}
            </div>
          </div>

          <div className="t-bold" style={{
            display: 'flex',
            alignItems: 'center',
            padding: '6px 16px',
            borderBottom: `1px solid ${T.borderLight}`,
            background: T.sectionBg,
            color: T.textMuted,
            userSelect: 'none',
          }}>
            {isBackupMode && (
              <div style={{ width: 32, display: 'flex', alignItems: 'center' }}>
                <Checkbox
                  style={{ margin: 0 }}
                  checked={sortedFiles.length > 0 && selectedPaths.size === sortedFiles.length}
                  indeterminate={selectedPaths.size > 0 && selectedPaths.size < sortedFiles.length}
                  onChange={handleToggleSelectAll}
                />
              </div>
            )}
            <div
              style={{ ...(isVault ? { flex: 3 } : { width: 260 }), cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4 }}
              onClick={() => handleSort('name')}
            >
              {nameLabel} {renderSortIcon('name')}
            </div>
            {!isVault && (
              <>
                <div
                  style={{ width: 240, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4 }}
                  onClick={() => handleSort('character_name')}
                >
                  {t('fileBrowser.character')} {renderSortIcon('character_name')}
                </div>
                <div
                  style={{ flex: 1, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4 }}
                  onClick={() => handleSort('save_name')}
                >
                  {t('fileBrowser.saveName')} {renderSortIcon('save_name')}
                </div>
                {isBackupMode && !isBackupRoot && (
                  <div style={{ width: 220 }}>
                    {t('fileBrowser.changes')}
                  </div>
                )}
              </>
            )}
            <div
              style={{ width: 160, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4 }}
              onClick={() => handleSort('date')}
            >
              {dateLabel} {renderSortIcon('date')}
            </div>
            <div
              style={{ width: 120, paddingLeft: 24, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 4 }}
              onClick={() => handleSort('size')}
            >
              {t('fileBrowser.size')} {renderSortIcon('size')}
            </div>
          </div>

          <div ref={setContainerNode} style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
            {loading && files.length === 0 ? (
              <LoadingState text={t('fileBrowser.loading')} />
            ) : error ? (
              <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} description={error} />
            ) : sortedFiles.length === 0 ? (
              <NonIdealState icon={<GameIcon icon={GiMagnifyingGlass} size={40} />} description={t('fileBrowser.noFiles')} />
            ) : (
              <List
                key={`list-${currentPath}`}
                listRef={listRef}
                style={{ height: containerHeight, width: '100%' }}
                rowCount={rowCount}
                rowHeight={ROW_HEIGHT}
                rowComponent={FileRow}
                rowProps={{}}
                onRowsRendered={onRowsRendered}
              />
            )}

            {loadingMore && files.length > 0 && (
              <div style={{
                position: 'absolute',
                bottom: 8,
                left: '50%',
                transform: 'translateX(-50%)',
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                padding: '4px 12px',
                borderRadius: 12,
                background: T.surface,
                border: `1px solid ${T.borderLight}`,
                boxShadow: '0 2px 8px rgba(0,0,0,0.15)',
                color: T.textMuted,
                zIndex: 10,
              }}>
                <Spinner className="nwn-spinner" size={12} />
                {t('fileBrowser.loadingMore')}
              </div>
            )}
          </div>
        </DialogBody>

        <DialogFooter
          style={{ background: T.surfaceAlt, borderTop: `1px solid ${T.borderLight}` }}
          actions={footerActions}
        >
          {footerLeft}
        </DialogFooter>
      </Dialog>

      <Dialog
        isOpen={showRestoreConfirm && !!selectedFile}
        onClose={() => setShowRestoreConfirm(false)}
        title={t('fileBrowser.confirmRestore')}
        transitionDuration={0}
        style={{ width: 440, paddingBottom: 0, background: T.surface }}
      >
        <DialogBody style={{ background: T.surface, margin: 0, padding: 16 }}>
          <p className="t-body" style={{ color: T.textMuted, margin: 0 }}>
            {t('fileBrowser.confirmRestoreMessage', {
              name: selectedFile?.save_name?.replace('Backup of ', '') || selectedFile?.name || '',
            })}
          </p>
          <p className="t-body t-medium" style={{
            color: T.negative,
            marginTop: 12,
            marginBottom: 0,
          }}>
            {t('fileBrowser.confirmRestoreWarning')}
          </p>
          <p className="t-body" style={{ color: T.textMuted, marginTop: 12, marginBottom: 0 }}>
            {summarizeChanges(selectedFile?.changes, t)}
          </p>
        </DialogBody>
        <DialogFooter
          style={{ background: T.surfaceAlt, borderTop: `1px solid ${T.borderLight}` }}
          actions={
            <>
              <Button text={t('fileBrowser.cancel')} onClick={() => setShowRestoreConfirm(false)} />
              <Button
                intent="primary"
                text={t('fileBrowser.restoreBackup')}
                onClick={handleRestoreConfirmed}
                style={{ background: T.accent }}
              />
            </>
          }
        />
      </Dialog>

      <Dialog
        isOpen={showDeleteConfirm && selectedFilesList.length > 0}
        onClose={() => !isDeleting && setShowDeleteConfirm(false)}
        title={t('fileBrowser.confirmDelete')}
        transitionDuration={0}
        style={{ width: 460, paddingBottom: 0, background: T.surface }}
      >
        <DialogBody style={{ background: T.surface, margin: 0, padding: 16 }}>
          <p className="t-body" style={{ color: T.text, margin: 0 }}>
            {isBackupRoot
              ? t('fileBrowser.confirmDeleteRootMessage', {
                  count: selectedFilesList.length,
                  size: formatBytes(selectedTotalBytes),
                })
              : t('fileBrowser.confirmDeleteMessage', {
                  count: selectedFilesList.length,
                  size: formatBytes(selectedTotalBytes),
                })}
          </p>
          <p className="t-body t-medium" style={{
            color: T.negative,
            marginTop: 12,
            marginBottom: 0,
          }}>
            {t('fileBrowser.confirmDeleteWarning')}
          </p>
        </DialogBody>
        <DialogFooter
          style={{ background: T.surfaceAlt, borderTop: `1px solid ${T.borderLight}` }}
          actions={
            <>
              <Button
                text={t('fileBrowser.cancel')}
                disabled={isDeleting}
                onClick={() => setShowDeleteConfirm(false)}
              />
              <Button
                intent="danger"
                text={t('fileBrowser.delete')}
                loading={isDeleting}
                onClick={handleDeleteExecute}
              />
            </>
          }
        />
      </Dialog>

      <BackupDetailsDialog
        isOpen={showDetailsDialog}
        onClose={() => setShowDetailsDialog(false)}
        file={selectedFile}
      />
    </>
  );
}
