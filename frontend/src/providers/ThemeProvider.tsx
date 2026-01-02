'use client';

import { useEffect } from 'react';

interface ThemeColors {
  background: string;
  surface1: string;
  surface2: string;
  surface3: string;
  surface4: string;
  surfaceBorder: string;
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  primary: string;
  primary600: string;
  primary50: string;
  secondary: string;
  secondary600: string;
  success: string;
  warning: string;
  error: string;
  errorDark: string;
}

const hexToRgb = (hex: string): string => {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return '0 0 0';
  return `${parseInt(result[1], 16)} ${parseInt(result[2], 16)} ${parseInt(result[3], 16)}`;
};

const applyThemeColors = (colors: ThemeColors) => {
  const root = document.documentElement;
  root.style.setProperty('--color-background', hexToRgb(colors.background));
  root.style.setProperty('--color-surface-1', hexToRgb(colors.surface1));
  root.style.setProperty('--color-surface-2', hexToRgb(colors.surface2));
  root.style.setProperty('--color-surface-3', hexToRgb(colors.surface3));
  root.style.setProperty('--color-surface-4', hexToRgb(colors.surface4));
  root.style.setProperty('--color-surface-border', hexToRgb(colors.surfaceBorder));
  root.style.setProperty('--color-text-primary', hexToRgb(colors.textPrimary));
  root.style.setProperty('--color-text-secondary', hexToRgb(colors.textSecondary));
  root.style.setProperty('--color-text-muted', hexToRgb(colors.textMuted));
  root.style.setProperty('--color-primary', hexToRgb(colors.primary));
  root.style.setProperty('--color-primary-600', hexToRgb(colors.primary600));
  root.style.setProperty('--color-primary-50', hexToRgb(colors.primary50));
  root.style.setProperty('--color-secondary', hexToRgb(colors.secondary));
  root.style.setProperty('--color-secondary-600', hexToRgb(colors.secondary600));
  root.style.setProperty('--color-success', hexToRgb(colors.success));
  root.style.setProperty('--color-warning', hexToRgb(colors.warning));
  root.style.setProperty('--color-error', hexToRgb(colors.error));
  root.style.setProperty('--color-error-dark', hexToRgb(colors.errorDark));
};

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  useEffect(() => {
    const savedActiveTheme = localStorage.getItem('nwn2ee-active-theme');
    if (savedActiveTheme) {
      try {
        const { colors } = JSON.parse(savedActiveTheme);
        if (colors) {
          applyThemeColors(colors);
        }
      } catch (err) {
        console.error('Error loading saved theme:', err);
      }
    }
  }, []);

  return <>{children}</>;
}
