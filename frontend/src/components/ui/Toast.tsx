'use client';

import { useEffect } from 'react';
import { X, CheckCircle, AlertCircle, Info } from 'lucide-react';
import { cn } from '@/lib/utils';

export type ToastType = 'success' | 'error' | 'warning' | 'info';

export interface ToastProps {
  id: string;
  message: string;
  type?: ToastType;
  duration?: number;
  onClose: (id: string) => void;
}

export function Toast({ id, message, type = 'info', duration = 5000, onClose }: ToastProps) {
  useEffect(() => {
    const timer = setTimeout(() => {
      onClose(id);
    }, duration);

    return () => clearTimeout(timer);
  }, [id, duration, onClose]);

  const icons = {
    success: <CheckCircle className="w-5 h-5 text-[rgb(var(--color-success))]" />,
    error: <AlertCircle className="w-5 h-5 text-[rgb(var(--color-error))]" />,
    warning: <AlertCircle className="w-5 h-5 text-[rgb(var(--color-warning))]" />,
    info: <Info className="w-5 h-5 text-[rgb(var(--color-primary))]" />,
  };

  const styles = {
    success: 'bg-[rgb(var(--color-success)/0.1)] border-[rgb(var(--color-success)/0.3)]',
    error: 'bg-[rgb(var(--color-error)/0.1)] border-[rgb(var(--color-error)/0.3)]',
    warning: 'bg-[rgb(var(--color-warning)/0.1)] border-[rgb(var(--color-warning)/0.3)]',
    info: 'bg-[rgb(var(--color-primary)/0.1)] border-[rgb(var(--color-primary)/0.3)]',
  };

  return (
    <div
      className={cn(
        'flex items-center gap-3 p-4 rounded-lg border shadow-lg min-w-[320px] max-w-[500px]',
        'animate-in slide-in-from-top-5 duration-300',
        styles[type]
      )}
    >
      {icons[type]}
      <p className="flex-1 text-sm text-[rgb(var(--color-text))]">{message}</p>
      <button
        onClick={() => onClose(id)}
        className="text-[rgb(var(--color-text-muted))] hover:text-[rgb(var(--color-text))] transition-colors"
      >
        <X className="w-4 h-4" />
      </button>
    </div>
  );
}

export function ToastContainer({ children }: { children: React.ReactNode }) {
  return (
    <div className="fixed top-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
      <div className="pointer-events-auto">{children}</div>
    </div>
  );
}
