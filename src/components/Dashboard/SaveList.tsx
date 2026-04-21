import { useState } from 'react';
import { Button } from '@blueprintjs/core';
import { GiMagnifyingGlass, GiOpenFolder } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { GameIcon } from '../shared/GameIcon';
import { SectionBar } from '../shared';
import { T } from '../theme';
import { SaveEntry } from './SaveEntry';
import type { SaveEntryData } from './SaveEntry';
import { FileBrowserDialog } from './FileBrowserDialog';
import type { FileInfo } from './FileBrowserDialog';

interface SaveListProps {
  saves: SaveEntryData[];
  selectedIndex: number | null;
  onSelect: (index: number) => void;
  onDoubleClick?: (index: number) => void;
  onBrowseFile?: (file: FileInfo) => void;
  defaultBrowsePath?: string;
}

export function SaveList({
  saves,
  selectedIndex,
  onSelect,
  onDoubleClick,
  onBrowseFile,
  defaultBrowsePath,
}: SaveListProps) {
  const t = useTranslations();
  const [showBrowser, setShowBrowser] = useState(false);
  const [browsePath, setBrowsePath] = useState('');

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <SectionBar title={t('dashboard.saveGames')} />

      <div style={{ flex: 1, overflowY: 'auto' }}>
        {saves.length > 0 ? (
          saves.map((save, i) => (
            <SaveEntry
              key={save.folderName}
              save={save}
              isSelected={selectedIndex === i}
              onClick={() => onSelect(i)}
              onDoubleClick={() => onDoubleClick?.(i)}
            />
          ))
        ) : (
          <div style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '48px 16px',
            gap: 12,
          }}>
            <GameIcon icon={GiMagnifyingGlass} size={32} color={T.border} />
            <span className="t-md" style={{ color: T.textMuted }}>
              {t('dashboard.noSaves')}
            </span>
          </div>
        )}

        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '14px 16px',
            borderBottom: `1px solid ${T.borderLight}`,
          }}
        >
          <Button
            minimal
            icon={<GameIcon icon={GiOpenFolder} size={16} />}
            intent="primary"
            onClick={() => {
              setBrowsePath(defaultBrowsePath ?? '');
              setShowBrowser(true);
            }}
          >
            {t('dashboard.browse')}
          </Button>
        </div>
      </div>

      <FileBrowserDialog
        isOpen={showBrowser}
        onClose={() => setShowBrowser(false)}
        mode="load-saves"
        currentPath={browsePath}
        onPathChange={setBrowsePath}
        onSelectFile={(file) => {
          setShowBrowser(false);
          onBrowseFile?.(file);
        }}
      />
    </div>
  );
}
