'use client';

import { useState, useEffect, useRef } from 'react';
import { FixedSizeList as List } from 'react-window';
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

type SortField = 'name' | 'date' | 'size';
type SortDirection = 'asc' | 'desc';

interface FileInfo {
  name: string;
  path: string;
  size: number;
  modified: string;
  is_directory: boolean;
  save_name?: string;
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
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);
  const [showRestoreConfirm, setShowRestoreConfirm] = useState(false);
  const [sortField, setSortField] = useState<SortField>('date');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  const [selectedFile, setSelectedFile] = useState<FileInfo | null>(null);
  const listRef = useRef<List>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const previousRefreshKey = useRef(refreshKey);

  useEffect(() => {
    if (isOpen) {
      loadFiles();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, currentPath, mode, refreshKey]);

  const loadFiles = async () => {
    // Only show loading spinner on initial load or path change
    // Don't show it on refresh (when only refreshKey changes)
    const isRefresh = previousRefreshKey.current !== refreshKey && files.length > 0;
    previousRefreshKey.current = refreshKey;

    if (!isRefresh) {
      setLoading(true);
    }
    setError(null);

    try {
      const endpoint = mode === 'load-saves'
        ? '/saves/list'
        : '/backups/list';

      const params = new URLSearchParams();
      if (currentPath) params.append('path', currentPath);
      // Add timestamp to prevent caching
      params.append('_t', Date.now().toString());

      const data = await apiClient.get<{ files: FileInfo[]; current_path?: string }>(
        `${endpoint}?${params.toString()}`
      );

      const newFiles = data.files || [];
      setFiles(newFiles);

      // Clear selected file if it no longer exists in the list
      if (selectedFile && !newFiles.find(f => f.path === selectedFile.path)) {
        setSelectedFile(null);
      }

      if (data.current_path && !currentPath) {
        onPathChange?.(data.current_path);
      }
    } catch (err) {
      console.error('Failed to load files:', err);
      setError(err instanceof Error ? err.message : 'Failed to load files');
      setFiles([]);
    } finally {
      setLoading(false);
    }
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(prev => prev === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortDirection('desc');
    }
  };

  const sortedFiles = [...files].sort((a, b) => {
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
    }

    return sortDirection === 'asc' ? comparison : -comparison;
  });

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
    // For load-saves mode, select directories (save folders)
    // For manage-backups mode, select directories (backup folders)
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
          <div className="mx-4 mt-3 h-10">
            {successMessage && (
              <div className="p-2 bg-green-900/20 border border-green-700 text-green-400 rounded text-sm">
                {successMessage}
              </div>
            )}
          </div>

          {/* Content */}
          <div className="file-browser-content">
            {loading ? (
              <div className="flex items-center justify-center h-full">
                <span className="text-[rgb(var(--color-text-muted))]">Loading...</span>
              </div>
            ) : error ? (
              <div className="flex items-center justify-center h-full">
                <span className="text-red-400">{error}</span>
              </div>
            ) : (
              <>
                {/* Table Header */}
                <div className="file-browser-table-header">
                  <div className="flex-1">
                    {renderSortHeader('name', 'Folder Name')}
                  </div>
                  <div className="flex-1">
                    <span className="text-xs font-semibold text-[rgb(var(--color-text-muted))] uppercase">Save Name</span>
                  </div>
                  <div className="w-48">
                    {renderSortHeader('date', mode === 'manage-backups' ? 'Created' : 'Modified')}
                  </div>
                  <div className="w-24 text-right">
                    {renderSortHeader('size', 'Size')}
                  </div>
                </div>

                {/* File List */}
                <div className="file-browser-list" ref={containerRef}>
                  {sortedFiles.length === 0 ? (
                    <div className="flex items-center justify-center h-32 text-[rgb(var(--color-text-muted))]">
                      No files found
                    </div>
                  ) : (
                    <List
                      ref={listRef}
                      height={containerRef.current?.clientHeight || 400}
                      itemCount={sortedFiles.length}
                      itemSize={48}
                      width="100%"
                    >
                      {({ index, style }) => {
                        const file = sortedFiles[index];
                        return (
                          <div
                            style={style}
                            key={file.path}
                            className={`file-browser-row ${
                              selectedFile?.path === file.path ? 'selected' : ''
                            }`}
                            onClick={() => handleFileClick(file)}
                          >
                            <div className="flex-1 flex items-center gap-2">
                              {file.is_directory && (
                                <FolderIcon className="w-4 h-4 text-[rgb(var(--color-text-muted))]" />
                              )}
                              <span className="text-sm text-[rgb(var(--color-text-primary))]">
                                {display(file.name)}
                              </span>
                            </div>
                            <div className="flex-1 text-sm text-[rgb(var(--color-text-secondary))]">
                              {display(file.save_name)}
                            </div>
                            <div className="w-48 text-sm text-[rgb(var(--color-text-muted))]">
                              {formatDate(file.modified)}
                            </div>
                            <div className="w-24 text-sm text-[rgb(var(--color-text-muted))] text-right">
                              {formatSize(file.size)}
                            </div>
                          </div>
                        );
                      }}
                    </List>
                  )}
                </div>
              </>
            )}
          </div>

          {/* Footer */}
          <div className="file-browser-footer">
            <div className="file-browser-footer-content">
              <span className="text-sm text-[rgb(var(--color-text-muted))]">
                {formatNumber(files.length)} {files.length === 1 ? 'file' : 'files'}
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
