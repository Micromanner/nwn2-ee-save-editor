import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { FixedSizeList as List, ListOnItemsRenderedProps } from 'react-window';
import InfiniteLoader from 'react-window-infinite-loader';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import {
  Button, Dialog, DialogBody, DialogFooter,
  Icon, Spinner, NonIdealState,
} from '@blueprintjs/core';
import { GiOpenFolder, GiExitDoor, GiFullFolder, GiFoldedPaper, GiBrokenShield, GiMagnifyingGlass } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { GameIcon } from '../shared/GameIcon';
import { display, formatNumber } from '@/utils/dataHelpers';
import { T, formatBytes } from '../theme';

type SortField = 'name' | 'date' | 'size' | 'character_name';
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
  canRestore?: boolean;
  refreshKey?: number;
  demoFiles?: FileInfo[];
}

const ITEMS_PER_PAGE = 50;
const ROW_HEIGHT = 40;

export function FileBrowserDialog({
  isOpen,
  onClose,
  mode,
  onSelectFile,
  currentPath = '',
  onPathChange,
  onDeleteBackup,
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
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [showRestoreConfirm, setShowRestoreConfirm] = useState(false);
  const [sortField, setSortField] = useState<SortField>('character_name');
  const [sortDirection, setSortDirection] = useState<SortDirection>('asc');
  const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);
  const listRef = useRef<List>(null);
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
          setContainerHeight(entry.contentRect.height);
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
        }
        const backups = await invoke<BrowseBackupEntry[]>('browse_backups', { path: currentPath });
        if (backupRootPath === '') {
          setBackupRootPath(currentPath);
        }
        const isSaveFolder = !currentPath.replace(/\\/g, '/').replace(/\/$/, '').endsWith('/backups');
        const fileInfos: FileInfo[] = backups.map(b => ({
          name: b.name,
          path: b.path,
          size: b.size,
          modified: String(b.created_at),
          is_directory: !isSaveFolder,
          save_name: b.save_name ?? (b.timestamp ? `Backup ${b.timestamp}` : undefined),
          character_name: b.character_name ?? undefined,
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
  }, [mode, currentPath, onPathChange, files.length, backupRootPath]);

  useEffect(() => {
    if (isOpen) {
      const pathChanged = lastPath.current !== currentPath;
      const refreshRequested = previousRefreshKey.current !== refreshKey;

      if (pathChanged || refreshRequested || files.length === 0) {
        if (pathChanged) {
          setFiles([]);
          setTotalFiles(0);
          setSelectedFile(null);
          lastPath.current = currentPath;
        }
        previousRefreshKey.current = refreshKey;
        loadFiles(true);
      } else {
        listRef.current?.scrollTo(0);
      }
    }
    return () => { abortControllerRef.current?.abort(); };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, currentPath, mode, refreshKey]);

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
      setSortDirection('desc');
    }
  };

  const sortedFiles = useMemo(() => {
    const filtered = mode === 'manage-backups'
      ? files.filter(f => f.name.toLowerCase() !== 'campaign_backups')
      : files;

    return [...filtered].sort((a, b) => {
      if (a.is_directory !== b.is_directory) return a.is_directory ? -1 : 1;
      let cmp = 0;
      switch (sortField) {
        case 'name': cmp = a.name.localeCompare(b.name); break;
        case 'date': cmp = parseFloat(a.modified) - parseFloat(b.modified); break;
        case 'size': cmp = a.size - b.size; break;
        case 'character_name':
          cmp = (a.character_name || '').localeCompare(b.character_name || '');
          if (cmp === 0) return parseFloat(b.modified) - parseFloat(a.modified);
          break;
      }
      return sortDirection === 'asc' ? cmp : -cmp;
    });
  }, [files, sortField, sortDirection, mode]);

  const formatDate = (dateString: string) => {
    const ts = parseFloat(dateString);
    if (isNaN(ts)) return '-';
    return new Date(ts * 1000).toLocaleString();
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
    <span style={{ color: T.textMuted }}>
      {t('fileBrowser.itemCount', {
        loaded: formatNumber(files.length),
        total: formatNumber(totalFiles),
      })}
    </span>
  );

  const footerActions = (
    <>
      {mode === 'manage-backups' && !isBackupRoot && selectedFile && onDeleteBackup && (
        <Button
          intent="danger"
          minimal
          text={t('fileBrowser.delete')}
          onClick={async () => {
            const fileName = selectedFile.name;
            await onDeleteBackup(selectedFile);
            setSuccessMessage(`Backup "${fileName}" deleted successfully`);
            setTimeout(() => setSuccessMessage(null), 3000);
          }}
        />
      )}
      {mode === 'manage-backups' && isBackupRoot && (
        <Button
          intent="primary"
          text={t('fileBrowser.open')}
          disabled={!selectedFile}
          onClick={handleConfirm}
          style={{ background: T.accent }}
        />
      )}
      {(mode === 'load-saves' || mode === 'load-vault' || (mode === 'manage-backups' && !isBackupRoot && canRestore)) && (
        <Button
          intent="primary"
          text={actionLabel}
          disabled={!selectedFile}
          onClick={handleConfirm}
          style={{ background: T.accent }}
        />
      )}
    </>
  );

  return (
    <>
      <Dialog
        isOpen={isOpen}
        onClose={onClose}
        title={title}
        className="bp-app"
        style={{ width: 'min(1400px, 95vw)', height: '80vh', paddingBottom: 0, background: T.surface, display: 'flex', flexDirection: 'column' as const }}
        canOutsideClickClose
      >
        <DialogBody style={{ padding: 0, margin: 0, background: T.surface, display: 'flex', flexDirection: 'column', overflow: 'hidden', flex: 1, minHeight: 0 }}>
          <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
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
            {mode === 'manage-backups' && !isBackupRoot && (
              <Button
                minimal
                small
                icon={<GameIcon icon={GiExitDoor} size={14} />}
                text={t('fileBrowser.back')}
                onClick={() => {
                  setSelectedFile(null);
                  onPathChange?.(backupRootPath);
                }}
              />
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

          {successMessage && (
            <div style={{
              margin: '8px 16px 0',
              padding: '6px 10px',
              borderRadius: 3,
              background: 'rgba(46, 125, 50, 0.1)',
              border: `1px solid ${T.positive}`,
              color: T.positive,
            }}>
              {successMessage}
            </div>
          )}

          <div className="t-bold" style={{
            display: 'flex',
            alignItems: 'center',
            padding: '6px 16px',
            borderBottom: `1px solid ${T.borderLight}`,
            background: T.sectionBg,
            color: T.textMuted,
            userSelect: 'none',
          }}>
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
                <div style={{ flex: 1 }}>
                  {t('fileBrowser.saveName')}
                </div>
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
              <NonIdealState
                icon={<Spinner size={32} />}
                description={t('fileBrowser.loading')}
              />
            ) : error ? (
              <NonIdealState icon={<GameIcon icon={GiBrokenShield} size={40} />} description={error} />
            ) : sortedFiles.length === 0 ? (
              <NonIdealState icon={<GameIcon icon={GiMagnifyingGlass} size={40} />} description={t('fileBrowser.noFiles')} />
            ) : (
              <InfiniteLoader
                key={currentPath}
                isItemLoaded={isItemLoaded}
                itemCount={mode === 'load-saves' ? totalFiles : sortedFiles.length}
                loadMoreItems={loadMoreItems}
                threshold={5}
              >
                {({ onItemsRendered, ref }: { onItemsRendered: (props: ListOnItemsRenderedProps) => void; ref: (r: List | null) => void }) => (
                  <List
                    key={`list-${currentPath}`}
                    ref={(node) => { listRef.current = node; ref(node); }}
                    height={containerHeight}
                    itemCount={mode === 'load-saves' ? totalFiles : sortedFiles.length}
                    itemSize={ROW_HEIGHT}
                    onItemsRendered={onItemsRendered}
                    width="100%"
                  >
                    {({ index, style }) => {
                      const file = sortedFiles[index];
                      if (!file) {
                        return (
                          <div style={{ ...style, display: 'flex', alignItems: 'center', padding: '0 16px' }}>
                            <Spinner size={14} />
                            <span style={{ color: T.textMuted, marginLeft: 8 }}>
                              {t('fileBrowser.loadingMore')}
                            </span>
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
                            </>
                          )}
                          <div style={{ width: 160, color: T.textMuted }}>
                            {formatDate(file.modified)}
                          </div>
                          <div style={{ width: 120, paddingLeft: 24, color: T.textMuted }}>
                            {formatBytes(file.size)}
                          </div>
                        </div>
                      );
                    }}
                  </List>
                )}
              </InfiniteLoader>
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
                <Spinner size={12} />
                {t('fileBrowser.loadingMore')}
              </div>
            )}
          </div>
        </DialogBody>

        <DialogFooter
          style={{ background: T.surfaceAlt, borderTop: `1px solid ${T.borderLight}` }}
          actions={
            <>
              <Button text={t('fileBrowser.cancel')} onClick={onClose} />
              {footerActions}
            </>
          }
        >
          {footerLeft}
        </DialogFooter>
      </Dialog>

      <Dialog
        isOpen={showRestoreConfirm && !!selectedFile}
        onClose={() => setShowRestoreConfirm(false)}
        title={t('fileBrowser.confirmRestore')}
        className="bp-app"
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
    </>
  );
}
