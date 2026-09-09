import { Tag } from '@blueprintjs/core';
import { GiScrollQuill } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { T } from '../theme';
import { GameIcon } from '../shared/GameIcon';

export interface SaveEntryData {
  characterName: string;
  folderName: string;
  date: string;
  thumbnail: string | null;
  isActive: boolean;
}

interface SaveEntryProps {
  save: SaveEntryData;
  isSelected: boolean;
  onClick: () => void;
  onDoubleClick?: () => void;
}

export function SaveEntry({ save, isSelected, onClick, onDoubleClick }: SaveEntryProps) {
  const t = useTranslations();

  return (
    <div
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: '10px 16px',
        cursor: 'pointer',
        borderBottom: `1px solid ${T.borderLight}`,
        borderLeft: isSelected ? `3px solid ${T.accent}` : '3px solid transparent',
        background: isSelected ? 'rgba(160, 82, 45, 0.06)' : 'transparent',
        transition: 'background 0.15s',
      }}
      onMouseEnter={(e) => {
        if (!isSelected) e.currentTarget.style.background = 'rgba(160, 82, 45, 0.03)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.background = isSelected ? 'rgba(160, 82, 45, 0.06)' : 'transparent';
      }}
    >
      {save.thumbnail ? (
        <img
          src={`data:image/webp;base64,${save.thumbnail}`}
          alt={save.characterName}
          style={{
            width: 128,
            height: 96,
            borderRadius: 4,
            objectFit: 'cover',
            border: `1px solid ${T.borderLight}`,
            flexShrink: 0,
          }}
        />
      ) : (
        <div style={{
          width: 128,
          height: 96,
          borderRadius: 4,
          background: T.surfaceAlt,
          border: `1px solid ${T.borderLight}`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
        }}>
          <GameIcon icon={GiScrollQuill} size={32} color={T.border} />
        </div>
      )}

      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <span className="t-semibold" style={{
            color: isSelected ? T.accent : T.text,
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}>
            {save.characterName}
          </span>
          {save.isActive && (
            <Tag minimal round className="t-xs" style={{
              background: 'rgba(160, 82, 45, 0.12)',
              color: T.accent,
            }}>
              {t('dashboard.active')}
            </Tag>
          )}
        </div>
        <div style={{ color: T.textMuted, marginTop: 2 }}>
          {save.folderName}
        </div>
        <div className="t-sm" style={{ color: T.textMuted, marginTop: 2 }}>
          {save.date}
        </div>
      </div>
    </div>
  );
}
