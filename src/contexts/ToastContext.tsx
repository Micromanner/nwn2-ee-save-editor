
import { createContext, useContext, useCallback, useRef, useEffect, ReactNode } from 'react';
import { OverlayToaster, Toaster, Intent, IconName } from '@blueprintjs/core';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

interface ToastContextType {
  showToast: (message: string, type?: ToastType, duration?: number, description?: string) => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

const intentMap: Record<ToastType, Intent> = {
  success: Intent.SUCCESS,
  error: Intent.DANGER,
  warning: Intent.WARNING,
  info: Intent.PRIMARY,
};

const iconMap: Record<ToastType, IconName> = {
  success: 'tick-circle',
  error: 'error',
  warning: 'warning-sign',
  info: 'info-sign',
};

export function ToastProvider({ children }: { children: ReactNode }) {
  const toasterRef = useRef<Toaster | null>(null);

  useEffect(() => {
    OverlayToaster.createAsync({ position: 'bottom-right', className: 'bp-app' }).then(t => {
      toasterRef.current = t;
    });
  }, []);

  const showToast = useCallback((message: string, type: ToastType = 'info', duration = 2500, description?: string) => {
    toasterRef.current?.show({
      message: description ? (
        <div>
          <strong>{message}</strong>
          <div style={{ fontSize: 'var(--font-sm)', opacity: 0.85, marginTop: 2 }}>{description}</div>
        </div>
      ) : message,
      intent: intentMap[type],
      icon: iconMap[type],
      timeout: duration,
    });
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within ToastProvider');
  }
  return context;
}
