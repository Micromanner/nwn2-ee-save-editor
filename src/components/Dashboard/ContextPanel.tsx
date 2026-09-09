import { Button, Divider } from '@blueprintjs/core';
import { GiVisoredHelm, GiScrollUnfurled, GiOpenFolder, GiBackwardTime, GiCog, GiQuillInk } from 'react-icons/gi';
import { useTranslations } from '@/hooks/useTranslations';
import { T } from '../theme';
import { GameIcon } from '../shared/GameIcon';
import type { SaveEntryData } from './SaveEntry';

interface ContextPanelProps {
  selectedSave: SaveEntryData | null;
}

function DefaultContent() {
  const t = useTranslations();

  return (
    <>
      <div style={{ marginBottom: 4 }}>
        <span className="t-3xl t-bold" style={{ color: T.accent }}>
          {t('dashboard.title')}
        </span>
      </div>
      <div style={{ color: T.textMuted, marginBottom: 16 }}>
        {t('dashboard.tagline')}
      </div>

      <Divider style={{ borderColor: T.borderLight, margin: '0 0 16px 0' }} />

      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        <Button minimal icon={<GameIcon icon={GiScrollUnfurled} size={16} color={T.text} />} alignText="left" style={{ color: T.text, justifyContent: 'flex-start' }}>
          {t('actions.importCharacter')}
        </Button>
        <Button minimal icon={<GameIcon icon={GiOpenFolder} size={16} color={T.text} />} alignText="left" style={{ color: T.text, justifyContent: 'flex-start' }}>
          {t('actions.openDocumentsFolder')}
        </Button>
        <Button minimal icon={<GameIcon icon={GiBackwardTime} size={16} color={T.text} />} alignText="left" style={{ color: T.text, justifyContent: 'flex-start' }}>
          {t('actions.manageBackups')}
        </Button>
        <Button minimal icon={<GameIcon icon={GiCog} size={16} color={T.text} />} alignText="left" style={{ color: T.text, justifyContent: 'flex-start' }}>
          {t('navigation.settings')}
        </Button>
      </div>
    </>
  );
}

function SelectedContent({ save }: { save: SaveEntryData }) {
  const t = useTranslations();

  return (
    <>
      <div style={{
        width: 120,
        height: 120,
        borderRadius: 6,
        background: T.surfaceAlt,
        border: `1px solid ${T.borderLight}`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        marginBottom: 16,
      }}>
        <GameIcon icon={GiVisoredHelm} size={40} color={T.border} />
      </div>

      <div className="t-2xl t-bold" style={{ color: T.accent, marginBottom: 4 }}>
        {save.characterName}
      </div>
      <div style={{ color: T.textMuted, marginBottom: 4 }}>
        {save.folderName}
      </div>
      <div className="t-sm" style={{ color: T.textMuted, marginBottom: 20 }}>
        {save.date}
      </div>

      {save.isActive ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          <Button intent="primary" icon={<GameIcon icon={GiQuillInk} size={16} />} fill>
            {t('dashboard.continueEditing')}
          </Button>
          <Button minimal intent="danger" icon="cross">
            {t('dashboard.closeSession')}
          </Button>
        </div>
      ) : (
        <Button intent="primary" icon={<GameIcon icon={GiScrollUnfurled} size={16} />} fill>
          {t('dashboard.loadSave')}
        </Button>
      )}
    </>
  );
}

export function ContextPanel({ selectedSave }: ContextPanelProps) {
  return (
    <div style={{
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      height: '100%',
      borderLeft: `1px solid ${T.borderLight}`,
      padding: '24px 32px',
    }}>
      <div style={{ textAlign: 'center' }}>
        {selectedSave ? <SelectedContent save={selectedSave} /> : <DefaultContent />}
      </div>
    </div>
  );
}
