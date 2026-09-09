import { Button, Dialog, DialogBody, DialogFooter } from '@blueprintjs/core';
import type { ReactNode } from 'react';
import { T } from '../theme';
import { useTranslations } from '@/hooks/useTranslations';

interface ParchmentDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onOpened?: () => void;
  onClosed?: () => void;
  title: string;
  width?: number;
  minHeight?: number;
  height?: number | string;
  children: ReactNode;
  footerActions: ReactNode;
  footerLeft?: ReactNode;
}

export function ParchmentDialog({
  isOpen, onClose, onOpened, onClosed, title, width = 480, minHeight, height,
  children, footerActions, footerLeft,
}: ParchmentDialogProps) {
  const t = useTranslations();
  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      onOpened={onOpened}
      onClosed={onClosed}
      title={title}
      // Explicit flex column so a minHeight/height stretches the body and keeps the footer at the bottom.
      // maxHeight clamps both fixed-height and content-sized dialogs to the viewport
      // (bp5-dialog has 30px vertical margins); the body scrolls internally instead.
      // minHeight must carry its own cap because CSS min-height overrides max-height.
      style={{
        width,
        minHeight: typeof minHeight === 'number' ? `min(${minHeight}px, calc(100vh - 60px))` : minHeight,
        height,
        maxHeight: 'calc(100vh - 60px)',
        paddingBottom: 0, background: T.surface, display: 'flex', flexDirection: 'column',
      }}
      canOutsideClickClose
    >
      <div style={{ background: T.surface, display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }}>
        {/* useOverflowScrollContainer adds max-height:70vh, which caps the body short of the
            dialog's fixed height in small windows and strands the footer mid-dialog. */}
        <DialogBody useOverflowScrollContainer={false} style={{ padding: 16, margin: 0, background: T.surface, flex: 1, minHeight: 0, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
          {children}
        </DialogBody>
        <DialogFooter
          style={{ background: T.surfaceAlt, borderTop: `1px solid ${T.borderLight}` }}
          actions={
            <>
              <Button text={t('common.cancel')} onClick={onClose} />
              {footerActions}
            </>
          }
        >
          {footerLeft}
        </DialogFooter>
      </div>
    </Dialog>
  );
}
