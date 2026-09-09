import { useEffect, useState } from 'react';
import { Button, Callout, Checkbox, Dialog, DialogBody, DialogFooter } from '@blueprintjs/core';
import { GiHazardSign, GiPlayButton } from 'react-icons/gi';
import { invoke } from '@tauri-apps/api/core';
import { GameIcon } from './GameIcon';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';

interface GameLaunchDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onLaunch: (closeEditor: boolean) => Promise<void>;
  saveName?: string;
  gamePathDetected?: boolean;
}

export function GameLaunchDialog({ isOpen, onClose, onLaunch, saveName, gamePathDetected = true }: GameLaunchDialogProps) {
  const t = useTranslations();
  const [closeEditor, setCloseEditor] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    invoke<{ auto_close_on_launch: boolean }>('get_app_config')
      .then(config => setCloseEditor(config.auto_close_on_launch))
      .catch(() => {});
  }, [isOpen]);

  const handleCloseEditorChange = (next: boolean) => {
    setCloseEditor(next);
    invoke('update_app_config', { updates: { auto_close_on_launch: next } }).catch(() => {});
  };

  const handleLaunch = async () => {
    setIsLaunching(true);
    try {
      await onLaunch(closeEditor);
      onClose();
    } finally {
      setIsLaunching(false);
    }
  };

  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title={t('gameLaunch.saveComplete')}
      style={{ width: 440, paddingBottom: 0, background: T.surface }}
      canOutsideClickClose={!isLaunching}
      isCloseButtonShown={!isLaunching}
    >
      <DialogBody style={{ padding: 16, margin: 0, background: T.surface }}>
        <p style={{ color: T.textMuted, marginBottom: 12 }}>
          <strong style={{ color: T.text }}>{saveName || 'Character'}</strong>{' '}
          {t('gameLaunch.savedSuccessfully', { name: '' }).replace(/ $/, '')}
        </p>

        <div style={{
          background: T.sectionBg, border: `1px solid ${T.sectionBorder}`,
          borderRadius: 4, padding: 12, marginBottom: 12,
        }}>
          <div className="t-medium" style={{ color: T.text }}>{t('gameLaunch.readyToVerify')}</div>
          <div style={{ color: T.textMuted, marginTop: 4 }}>{t('gameLaunch.readyToVerifyDesc')}</div>
        </div>

        {!gamePathDetected && (
          <Callout intent="warning" icon={<GameIcon icon={GiHazardSign} size={16} />} style={{ marginBottom: 12 }}>
            {t('gameLaunch.gamePathWarning')}
          </Callout>
        )}

        <Checkbox
          checked={closeEditor}
          onChange={e => handleCloseEditorChange((e.target as HTMLInputElement).checked)}
          label={t('actions.closeEditorAfterLaunch')}
          disabled={isLaunching}
          style={{ color: T.textMuted }}
        />
      </DialogBody>
      <DialogFooter
        style={{ background: T.surfaceAlt, borderTop: `1px solid ${T.borderLight}` }}
        actions={
          <>
            <Button text={t('actions.stayInEditor')} onClick={onClose} disabled={isLaunching} />
            <Button
              text={isLaunching ? t('actions.launching') : t('actions.launchGame')}
              intent="primary"
              loading={isLaunching}
              onClick={handleLaunch}
              icon={<GameIcon icon={GiPlayButton} size={14} />}
            />
          </>
        }
      />
    </Dialog>
  );
}
