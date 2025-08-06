'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Label } from '@/components/ui/Label';
import { SwatchIcon, ArrowPathIcon } from '@heroicons/react/24/outline';

interface ThemeColors {
  // Background and surfaces
  background: string;
  surface1: string;
  surface2: string;
  surface3: string;
  surface4: string;
  surfaceBorder: string;
  
  // Text colors
  textPrimary: string;
  textSecondary: string;
  textMuted: string;
  
  // Accent colors
  primary: string;
  primary600: string;
  primary50: string;
  secondary: string;
  secondary600: string;
  
  // Semantic colors
  success: string;
  warning: string;
  error: string;
  errorDark: string;
}

const DEFAULT_THEME: ThemeColors = {
  background: '#0f0f11',
  surface1: '#161619',
  surface2: '#1c1c20',
  surface3: '#232328',
  surface4: '#2a2a30',
  surfaceBorder: '#34343c',
  textPrimary: '#f8f8fc',
  textSecondary: '#cdcdd7',
  textMuted: '#91919b',
  primary: '#6366f1',
  primary600: '#4f46e5',
  primary50: '#eef2ff',
  secondary: '#a855f7',
  secondary600: '#9333ea',
  success: '#22c55e',
  warning: '#fbbf24',
  error: '#ef4444',
  errorDark: '#dc2626',
};

// Convert hex to RGB values
const hexToRgb = (hex: string): string => {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return '0 0 0';
  return `${parseInt(result[1], 16)} ${parseInt(result[2], 16)} ${parseInt(result[3], 16)}`;
};

// Convert RGB string to hex
const rgbToHex = (rgb: string): string => {
  const parts = rgb.split(' ').map(n => parseInt(n));
  if (parts.length !== 3) return '#000000';
  return '#' + parts.map(n => n.toString(16).padStart(2, '0')).join('');
};

export default function ThemeCustomizer() {
  const [colors, setColors] = useState<ThemeColors>(DEFAULT_THEME);
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    loadThemeFromCSS();
  }, []);

  const loadThemeFromCSS = () => {
    const root = document.documentElement;
    const getColor = (varName: string): string => {
      const rgb = getComputedStyle(root).getPropertyValue(varName).trim();
      return rgbToHex(rgb);
    };

    setColors({
      background: getColor('--color-background'),
      surface1: getColor('--color-surface-1'),
      surface2: getColor('--color-surface-2'),
      surface3: getColor('--color-surface-3'),
      surface4: getColor('--color-surface-4'),
      surfaceBorder: getColor('--color-surface-border'),
      textPrimary: getColor('--color-text-primary'),
      textSecondary: getColor('--color-text-secondary'),
      textMuted: getColor('--color-text-muted'),
      primary: getColor('--color-primary'),
      primary600: getColor('--color-primary-600'),
      primary50: getColor('--color-primary-50'),
      secondary: getColor('--color-secondary'),
      secondary600: getColor('--color-secondary-600'),
      success: getColor('--color-success'),
      warning: getColor('--color-warning'),
      error: getColor('--color-error'),
      errorDark: getColor('--color-error-dark'),
    });
  };

  const updateColor = (key: keyof ThemeColors, value: string) => {
    setColors(prev => ({ ...prev, [key]: value }));
    setHasChanges(true);
  };

  const applyTheme = () => {
    const root = document.documentElement;
    
    // Apply all color changes
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
    
    // Save to localStorage
    localStorage.setItem('nwn2ee-theme-colors', JSON.stringify(colors));
    setHasChanges(false);
  };

  const resetTheme = () => {
    setColors(DEFAULT_THEME);
    applyTheme();
    localStorage.removeItem('nwn2ee-theme-colors');
  };

  const loadSavedTheme = () => {
    const saved = localStorage.getItem('nwn2ee-theme-colors');
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        setColors(parsed);
        // Apply the saved theme
        const root = document.documentElement;
        Object.entries(parsed).forEach(([key, value]) => {
          const cssVarName = key
            .replace(/([A-Z])/g, '-$1')
            .toLowerCase()
            .replace(/^-/, '')
            .replace('600', '-600')
            .replace('50', '-50')
            .replace('dark', '-dark');
          root.style.setProperty(`--color-${cssVarName}`, hexToRgb(value as string));
        });
      } catch (err) {
        console.error('Error loading saved theme:', err);
      }
    }
  };

  useEffect(() => {
    loadSavedTheme();
  }, []);

  const ColorInput = ({ label, value, onChange, description }: {
    label: string;
    value: string;
    onChange: (value: string) => void;
    description?: string;
  }) => (
    <div className="space-y-1">
      <Label htmlFor={label} className="text-sm font-medium">{label}</Label>
      {description && (
        <p className="theme-color-description">{description}</p>
      )}
      <div className="flex gap-2">
        <input
          type="color"
          id={label}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="theme-color-input"
        />
        <input
          type="text"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className="theme-text-input"
          placeholder="#000000"
        />
      </div>
    </div>
  );

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <SwatchIcon className="w-5 h-5" />
          Theme Colors
        </CardTitle>
        <CardDescription>Customize the application color scheme</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Background & Surfaces */}
        <div className="space-y-4">
          <h3 className="theme-section-title">Background & Surfaces</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput
              label="Background"
              value={colors.background}
              onChange={(v) => updateColor('background', v)}
              description="Main app background"
            />
            <ColorInput
              label="Surface 1"
              value={colors.surface1}
              onChange={(v) => updateColor('surface1', v)}
              description="Cards and panels"
            />
            <ColorInput
              label="Surface 2"
              value={colors.surface2}
              onChange={(v) => updateColor('surface2', v)}
              description="Input fields"
            />
            <ColorInput
              label="Surface 3"
              value={colors.surface3}
              onChange={(v) => updateColor('surface3', v)}
              description="Hover states"
            />
            <ColorInput
              label="Surface 4"
              value={colors.surface4}
              onChange={(v) => updateColor('surface4', v)}
              description="Active states"
            />
            <ColorInput
              label="Border"
              value={colors.surfaceBorder}
              onChange={(v) => updateColor('surfaceBorder', v)}
              description="Borders and dividers"
            />
          </div>
        </div>

        {/* Text Colors */}
        <div className="space-y-4">
          <h3 className="theme-section-title">Text Colors</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput
              label="Primary Text"
              value={colors.textPrimary}
              onChange={(v) => updateColor('textPrimary', v)}
              description="Main text color"
            />
            <ColorInput
              label="Secondary Text"
              value={colors.textSecondary}
              onChange={(v) => updateColor('textSecondary', v)}
              description="Subtitles and labels"
            />
            <ColorInput
              label="Muted Text"
              value={colors.textMuted}
              onChange={(v) => updateColor('textMuted', v)}
              description="Disabled and hints"
            />
          </div>
        </div>

        {/* Accent Colors */}
        <div className="space-y-4">
          <h3 className="theme-section-title">Accent Colors</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput
              label="Primary"
              value={colors.primary}
              onChange={(v) => updateColor('primary', v)}
              description="Main brand color"
            />
            <ColorInput
              label="Primary Dark"
              value={colors.primary600}
              onChange={(v) => updateColor('primary600', v)}
              description="Hover state"
            />
            <ColorInput
              label="Primary Light"
              value={colors.primary50}
              onChange={(v) => updateColor('primary50', v)}
              description="Light backgrounds"
            />
            <ColorInput
              label="Secondary"
              value={colors.secondary}
              onChange={(v) => updateColor('secondary', v)}
              description="Secondary accent"
            />
          </div>
        </div>

        {/* Semantic Colors */}
        <div className="space-y-4">
          <h3 className="theme-section-title">Semantic Colors</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput
              label="Success"
              value={colors.success}
              onChange={(v) => updateColor('success', v)}
              description="Success states"
            />
            <ColorInput
              label="Warning"
              value={colors.warning}
              onChange={(v) => updateColor('warning', v)}
              description="Warning states"
            />
            <ColorInput
              label="Error"
              value={colors.error}
              onChange={(v) => updateColor('error', v)}
              description="Error states"
            />
            <ColorInput
              label="Error Dark"
              value={colors.errorDark}
              onChange={(v) => updateColor('errorDark', v)}
              description="Error hover"
            />
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex gap-3 justify-center theme-actions-border">
          <Button
            onClick={applyTheme}
            disabled={!hasChanges}
            variant="primary"
          >
            Apply Changes
          </Button>
          <Button
            onClick={resetTheme}
            variant="outline"
          >
            <ArrowPathIcon className="w-4 h-4" />
            Reset to Default
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}