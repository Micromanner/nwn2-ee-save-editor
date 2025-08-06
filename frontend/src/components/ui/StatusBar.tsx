'use client';

import { useTranslations } from '@/hooks/useTranslations';

export default function StatusBar() {
  const t = useTranslations();
  return (
    <div className="h-8 bg-[rgb(var(--color-surface-1))] flex items-center justify-between px-4 text-xs text-[rgb(var(--color-text-muted))] border-t border-[rgb(var(--color-surface-border)/0.6)]">
      <div className="flex items-center space-x-4">
        <span className="flex items-center space-x-2">
          {/* Animated dots loader */}
          <div className="flex space-x-1">
            <div className="w-1.5 h-1.5 bg-[rgb(var(--color-text-muted))] rounded-full animate-bounce" style={{ animationDelay: '0ms' }}></div>
            <div className="w-1.5 h-1.5 bg-[rgb(var(--color-text-muted))] rounded-full animate-bounce" style={{ animationDelay: '150ms' }}></div>
            <div className="w-1.5 h-1.5 bg-[rgb(var(--color-text-muted))] rounded-full animate-bounce" style={{ animationDelay: '300ms' }}></div>
          </div>
          <span className="text-[rgb(var(--color-text-secondary))]">{t('status.noFileLoaded')}</span>
        </span>
      </div>
      <div className="flex items-center space-x-4">
        <span className="text-[rgb(var(--color-text-secondary))]">{t('status.version')} 1.0.0</span>
      </div>
    </div>
  );
}