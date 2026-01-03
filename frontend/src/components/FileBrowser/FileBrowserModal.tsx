import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { FixedSizeList as List, ListOnItemsRenderedProps } from 'react-window';
import InfiniteLoader from 'react-window-infinite-loader';
import { open } from '@tauri-apps/plugin-dialog';
import { useTranslations } from '@/hooks/useTranslations';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { apiClient } from '@/lib/api/client';
import { display, formatNumber } from '@/utils/dataHelpers';

const X = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
  </svg>
);

const FolderIcon = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
  </svg>
);

const ChevronUp = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
  </svg>
);

const ChevronDown = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
  </svg>
);

type SortField = 'name' | 'date' | 'size' | 'character_name';
type SortDirection = 'asc' | 'desc';

interface FileInfo {
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

interface FileBrowserModalProps {
  isOpen: boolean;
  onClose: () => void;
  mode: 'load-saves' | 'manage-backups';
  onSelectFile?: (file: FileInfo) => void;
  currentPath?: string;
  onPathChange?: (path: string) => void;
  onDeleteBackup?: (file: FileInfo) => Promise<void>;
  canRestore?: boolean;
  refreshKey?: number;
}

export default function FileBrowserModal({
  isOpen,
  onClose,
  mode,
  onSelectFile,
  currentPath = '',
  onPathChange,
  onDeleteBackup,
  canRestore = true,
  refreshKey = 0
}: FileBrowserModalProps) {
  useTranslations();

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
  const containerRef = useRef<HTMLDivElement>(null);
  const previousRefreshKey = useRef(refreshKey);
  const lastPath = useRef(currentPath);

  const [containerHeight, setContainerHeight] = useState(600);
  const ITEMS_PER_PAGE = 50;
  const abortControllerRef = useRef<AbortController | null>(null);
  const currentRequestRef = useRef<string | null>(null);

  // Track container height correctly
  useEffect(() => {
    if (!containerRef.current) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        if (entry.contentRect.height > 0) {
          setContainerHeight(entry.contentRect.height);
        }
      }
    });

    observer.observe(containerRef.current);
    return () => observer.disconnect();
  }, [isOpen]);

  const loadFiles = useCallback(async (isInitial = true, forceRefresh = false) => {
    // Unique ID for this specific request to prevent race conditions
    const requestId = Math.random().toString(36).substring(7);
    currentRequestRef.current = requestId;

    // Cancel any pending requests
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    const controller = new AbortController();
    abortControllerRef.current = controller;

    if (isInitial) {
      // Only show spinner if we're forced to or have no data
      // This prevents the flicker when re-opening or normalized path updates
      if (files.length === 0 || forceRefresh) {
        setLoading(true);
      }
      setError(null);
    } else {
      setLoadingMore(true);
    }

    try {
      const endpoint = mode === 'load-saves'
        ? '/saves/list'
        : '/backups/list';

      const params = new URLSearchParams();
      if (currentPath) params.append('path', currentPath);
      params.append('limit', ITEMS_PER_PAGE.toString());
      
      const offset = isInitial ? 0 : files.length;
      params.append('offset', offset.toString());
      params.append('_t', Date.now().toString());

      const data = await apiClient.get<FileListResponse>(
        `${endpoint}?${params.toString()}`,
        { signal: controller.signal }
      );

      // Only proceed if this is still the latest request
      if (currentRequestRef.current !== requestId) return;

      const newFiles = data.files || [];
      
      if (isInitial) {
        setFiles(newFiles);
        setTotalFiles(data.total_count);
        if (forceRefresh || files.length === 0) {
          listRef.current?.scrollTo(0);
        }
      } else {
        setFiles(prev => [...prev, ...newFiles]);
        setTotalFiles(data.total_count);
      }

      // Track that we've loaded this path
      lastPath.current = currentPath;

      if (isInitial && selectedFile && !newFiles.find(f => f.path === selectedFile.path)) {
        setSelectedFile(null);
      }

      // Manage path normalization from backend
      if (data.current_path && currentPath !== data.current_path) {
        // Update lastPath locally BEFORE informing parent to prevent normalization loop flickers
        lastPath.current = data.current_path;
        if (!currentPath) {
          onPathChange?.(data.current_path);
        }
      }
    } catch (err) {
      if (err instanceof Error && err.name === 'AbortError') return;
      if (currentRequestRef.current !== requestId) return;

      console.error('Failed to load files:', err);
      setError(err instanceof Error ? err.message : 'Failed to load files');
      if (isInitial) setFiles([]);
    } finally {
      if (currentRequestRef.current === requestId) {
        setLoading(false);
        setLoadingMore(false);
      }
    }
  }, [mode, currentPath, files.length, onPathChange, selectedFile]);

  useEffect(() => {
    if (isOpen) {
      const pathChanged = lastPath.current !== currentPath;
      const refreshRequested = previousRefreshKey.current !== refreshKey;
      
      if (pathChanged || refreshRequested || files.length === 0) {
        if (pathChanged && currentPath !== '') {
          setFiles([]);
          setTotalFiles(0);
        }
        
        previousRefreshKey.current = refreshKey;
        loadFiles(true, pathChanged || refreshRequested);
      } else {
        // Re-opening same path, just reset scroll for UX
        listRef.current?.scrollTo(0);
      }
    }

    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, currentPath, mode, refreshKey]);

  const isItemLoaded = (index: number) => index < files.length;
  const loadMoreItems = (startIndex: number) => {
    if (loadingMore || files.length >= totalFiles) return Promise.resolve();
    return loadFiles(false);
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(prev => prev === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  };

  const sortedFiles = useMemo(() => {
    // Filter out campaign_backups folder in backup mode
    const filteredFiles = mode === 'manage-backups'
      ? files.filter(f => f.name.toLowerCase() !== 'campaign_backups')
      : files;

    return [...filteredFiles].sort((a, b) => {
      let comparison = 0;

      if (a.is_directory !== b.is_directory) {
        return a.is_directory ? -1 : 1;
      }

      switch (sortField) {
        case 'name':
          comparison = a.name.localeCompare(b.name);
          break;
        case 'date':
          comparison = parseFloat(a.modified) - parseFloat(b.modified);
          break;
        case 'size':
          comparison = a.size - b.size;
          break;
        case 'character_name':
             comparison = (a.character_name || '').localeCompare(b.character_name || '');
             if (comparison === 0) {
                 return parseFloat(b.modified) - parseFloat(a.modified);
             }
             break;
      }

      return sortDirection === 'asc' ? comparison : -comparison;
    });
  }, [files, sortField, sortDirection, mode]);

  const formatDate = (dateString: string) => {
    const timestamp = parseFloat(dateString);
    if (isNaN(timestamp)) {
      return '-';
    }
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
  };

  const formatSize = (bytes: number) => {
    if (bytes === 0) return '-';
    const units = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
  };

  const handleFileClick = (file: FileInfo) => {
    if (file.is_directory) {
      setSelectedFile(file);
    } else {
      setSelectedFile(file);
    }
  };

  const handleConfirm = () => {
    if (selectedFile && onSelectFile) {
      if (mode === 'manage-backups') {
        // Show confirmation for restore
        setShowRestoreConfirm(true);
      } else {
        // Load save directly
        onSelectFile(selectedFile);
        onClose();
      }
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
        title: 'Select Save Location'
      });

      if (selected && typeof selected === 'string') {
        onPathChange?.(selected);
      }
    } catch (err) {
      console.error('Failed to select directory:', err);
    }
  };

  const renderSortHeader = (field: SortField, label: string) => (
    <button
      onClick={() => handleSort(field)}
      className="file-browser-sort-header"
    >
      <span>{label}</span>
      {sortField === field && (
        sortDirection === 'asc'
          ? <ChevronUp className="w-4 h-4" />
          : <ChevronDown className="w-4 h-4" />
      )}
    </button>
  );

  if (!isOpen) return null;

  const title = mode === 'load-saves' ? 'Load Save' : 'Manage Backups';
  const actionLabel = mode === 'load-saves' ? 'Load' : 'Restore';

  return (
    <div className="file-browser-overlay">
      <Card className="file-browser-container">
        <CardContent padding="p-0" className="flex flex-col h-full">
          {/* Header */}
          <div className="file-browser-header">
            <div className="file-browser-header-row">
              <h3 className="file-browser-title">{title}</h3>
              <Button
                onClick={onClose}
                variant="ghost"
                size="sm"
                className="file-browser-close-button"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>

            {/* Path Display */}
            <div className="file-browser-path-container">
              <div className="flex items-center gap-2">
                <FolderIcon className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
                <span className="text-sm text-[rgb(var(--color-text-muted))]">
                  {mode === 'load-saves' ? 'Save Location:' : 'Backup Location:'}
                </span>
              </div>
              <div className="flex items-center gap-2 mt-1">
                <span className="text-sm font-mono text-[rgb(var(--color-text-secondary))]">
                  {display(currentPath) || 'Default location'}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={handleChangeLocation}
                  className="text-xs"
                >
                  Change Location...
                </Button>
              </div>
            </div>
          </div>

          {/* Success Message - Reserve space to prevent layout shift */}
          <div className="mx-4 mt-3">
            {successMessage && (
              <div className="p-2 bg-green-900/20 border border-green-700 text-green-400 rounded text-sm">
                {successMessage}
              </div>
            )}
          </div>

          {/* Content */}
          <div className="file-browser-content">
            {/* Table Header - Always visible to prevent layout shift */}
            <div className="file-browser-table-header">
              <div className="flex-[2]">
                {renderSortHeader('name', 'Folder Name')}
              </div>
              <div className="flex-1">
                {renderSortHeader('character_name', 'Character')}
              </div>
              <div className="flex-1">
                <span className="text-xs font-semibold text-[rgb(var(--color-text-muted))] uppercase">Save Name</span>
              </div>
              <div className="w-48">
                {renderSortHeader('date', mode === 'manage-backups' ? 'Created' : 'Modified')}
              </div>
              <div className="w-24 text-left">
                {renderSortHeader('size', 'Size')}
              </div>
            </div>

            <div className="file-browser-list" ref={containerRef}>
              {loading && files.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full gap-4">
                  <div className="w-8 h-8 rounded-full border-2 border-[rgb(var(--color-primary)/0.2)] border-t-[rgb(var(--color-primary))] animate-spin" />
                  <span className="text-sm text-[rgb(var(--color-text-muted))]">Loading files...</span>
                </div>
              ) : error ? (
                <div className="flex items-center justify-center h-full">
                  <span className="text-red-400">{error}</span>
                </div>
              ) : sortedFiles.length === 0 ? (
                <div className="flex items-center justify-center h-32 text-[rgb(var(--color-text-muted))]">
                  No files found
                </div>
              ) : (
                <InfiniteLoader
                  key={currentPath} // Force reset when path changes
                  isItemLoaded={isItemLoaded}
                  itemCount={totalFiles}
                  loadMoreItems={loadMoreItems}
                  threshold={5}
                >
                  {({ onItemsRendered, ref }: { onItemsRendered: (props: ListOnItemsRenderedProps) => any, ref: (ref: any) => void }) => (
                    <List
                      key={`list-${currentPath}`} // Only reset on path change, not totalFiles change
                      ref={(node) => {
                        // @ts-ignore
                        listRef.current = node;
                        ref(node);
                      }}
                      height={containerHeight}
                      itemCount={totalFiles}
                      itemSize={48}
                      onItemsRendered={onItemsRendered}
                      width="100%"
                    >
                      {({ index, style }) => {
                        const file = sortedFiles[index];
                        if (!file) {
                          return (
                            <div style={style} className="file-browser-row loading">
                              <div className="flex-1 flex items-center gap-2">
                                <div className="w-4 h-4 rounded-full bg-[rgb(var(--color-surface-2))] animate-pulse" />
                                <div className="h-4 w-32 bg-[rgb(var(--color-surface-2))] rounded animate-pulse" />
                              </div>
                              <div className="flex-1 h-4 bg-[rgb(var(--color-surface-2))] rounded animate-pulse opacity-50 mx-4" />
                              <div className="w-48 h-4 bg-[rgb(var(--color-surface-2))] rounded animate-pulse opacity-30" />
                              <div className="w-24 h-4 bg-[rgb(var(--color-surface-2))] rounded animate-pulse opacity-20" />
                            </div>
                          );
                        }

                        return (
                          <div
                            style={style}
                            key={file.path}
                            className={`file-browser-row ${
                              selectedFile?.path === file.path ? 'selected' : ''
                            }`}
                            onClick={() => handleFileClick(file)}
                          >
                            <div className="flex-[2] flex items-center gap-2">
                              {file.is_directory && (
                                <FolderIcon className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
                              )}
                              <span className="text-sm text-[rgb(var(--color-text-primary))] font-medium">
                                {display(file.name)}
                              </span>
                            </div>
                              <div className="flex-1 text-sm text-[rgb(var(--color-text-secondary))]">
                                {display(file.character_name)}
                              </div>
                              <div className="flex-1 text-sm text-[rgb(var(--color-text-secondary))]">
                                {display(file.save_name)}
                              </div>
                            <div className="w-48 text-sm text-[rgb(var(--color-text-muted))]">
                              {formatDate(file.modified)}
                            </div>
                            <div className="w-24 text-sm text-[rgb(var(--color-text-muted))] text-left">
                              {formatSize(file.size)}
                            </div>
                          </div>
                        );
                      }}
                    </List>
                  )}
                </InfiniteLoader>
              )}

              {loadingMore && files.length > 0 && (
                <div className="absolute bottom-4 left-1/2 -translate-x-1/2 px-4 py-2 bg-[rgb(var(--color-surface-1))] rounded-full border border-[rgb(var(--color-surface-border))] shadow-lg flex items-center gap-2 z-10">
                  <div className="w-4 h-4 rounded-full border-2 border-[rgb(var(--color-primary)/0.2)] border-t-[rgb(var(--color-primary))] animate-spin" />
                  <span className="text-xs font-medium">Loading more...</span>
                </div>
              )}
            </div>
          </div>

          {/* Footer */}
          <div className="file-browser-footer">
            <div className="file-browser-footer-content">
              <span className="text-sm text-[rgb(var(--color-text-muted))]">
                {formatNumber(files.length)} of {formatNumber(totalFiles)} items
              </span>
              <div className="flex gap-2">
                {mode === 'manage-backups' && selectedFile && onDeleteBackup && (
                  <Button
                    variant="ghost"
                    onClick={async () => {
                      const fileName = selectedFile.name;
                      await onDeleteBackup(selectedFile);
                      setSuccessMessage(`Backup "${fileName}" deleted successfully`);
                      setTimeout(() => setSuccessMessage(null), 3000);
                    }}
                    className="text-red-400 hover:text-red-300"
                  >
                    Delete
                  </Button>
                )}
                <Button
                  variant="ghost"
                  onClick={onClose}
                >
                  Cancel
                </Button>
                {mode === 'manage-backups' && canRestore && (
                  <Button
                    onClick={handleConfirm}
                    disabled={!selectedFile}
                  >
                    {actionLabel}
                  </Button>
                )}
                {mode === 'load-saves' && (
                  <Button
                    onClick={handleConfirm}
                    disabled={!selectedFile}
                  >
                    {actionLabel}
                  </Button>
                )}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Restore Confirmation Dialog */}
      {showRestoreConfirm && selectedFile && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
          <Card className="max-w-md w-full mx-4">
            <CardContent className="p-6">
              <h3 className="text-lg font-semibold mb-4">
                Confirm Restore
              </h3>
              <p className="text-sm text-[rgb(var(--color-text-muted))] mb-6">
                This will restore the save <strong className="text-[rgb(var(--color-text))]">{selectedFile.save_name?.replace('Backup of ', '') || selectedFile.name}</strong> to its state before any modifications were made.
                <br /><br />
                <strong className="text-yellow-400">Warning:</strong> This will permanently replace your current save folder with this backup. Any progress or changes made after this backup was created will be lost and cannot be recovered.
              </p>
              <div className="flex gap-3 justify-end">
                <Button
                  variant="ghost"
                  onClick={() => setShowRestoreConfirm(false)}
                >
                  Cancel
                </Button>
                <Button
                  onClick={handleRestoreConfirmed}
                >
                  Restore Backup
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
