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
      className="bp-app"
      // Explicit flex column so a minHeight/height stretches the body and keeps the footer at the bottom.
      style={{ width, minHeight, height, paddingBottom: 0, background: T.surface, display: 'flex', flexDirection: 'column' }}
      canOutsideClickClose
    >
      <div style={{ background: T.surface, display: 'flex', flexDirection: 'column', flex: 1, minHeight: 0 }}>
        <DialogBody style={{ padding: 16, margin: 0, background: T.surface, flex: 1, minHeight: 0, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
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
