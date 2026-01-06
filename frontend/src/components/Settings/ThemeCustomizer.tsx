'use client';

import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Label } from '@/components/ui/Label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { SwatchIcon, ArrowPathIcon, PlusIcon, TrashIcon } from '@heroicons/react/24/outline';

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

interface SavedTheme {
  name: string;
  colors: ThemeColors;
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

const PRESET_THEMES: SavedTheme[] = [
  { name: 'Default Dark', colors: DEFAULT_THEME },
  {
    name: 'Light',
    colors: {
      background: '#f5f5f5',
      surface1: '#ffffff',
      surface2: '#fafafa',
      surface3: '#f0f0f0',
      surface4: '#e8e8e8',
      surfaceBorder: '#d9d9d9',
      textPrimary: '#1a1a1a',
      textSecondary: '#525252',
      textMuted: '#8c8c8c',
      primary: '#6366f1',
      primary600: '#4f46e5',
      primary50: '#eef2ff',
      secondary: '#a855f7',
      secondary600: '#9333ea',
      success: '#22c55e',
      warning: '#f59e0b',
      error: '#ef4444',
      errorDark: '#dc2626',
    }
  }
];

const hexToRgb = (hex: string): string => {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return '0 0 0';
  return `${parseInt(result[1], 16)} ${parseInt(result[2], 16)} ${parseInt(result[3], 16)}`;
};

const rgbToHex = (rgb: string): string => {
  const parts = rgb.split(' ').map(n => parseInt(n));
  if (parts.length !== 3) return '#000000';
  return '#' + parts.map(n => n.toString(16).padStart(2, '0')).join('');
};

export default function ThemeCustomizer() {
  const [colors, setColors] = useState<ThemeColors>(DEFAULT_THEME);
  const [hasChanges, setHasChanges] = useState(false);
  const [customThemes, setCustomThemes] = useState<SavedTheme[]>([]);
  const [selectedThemeName, setSelectedThemeName] = useState<string>('Default Dark');
  const [showSaveDialog, setShowSaveDialog] = useState(false);
  const [newThemeName, setNewThemeName] = useState('');

  const applyColorsToDOM = useCallback((themeColors: ThemeColors) => {
    const root = document.documentElement;
    root.style.setProperty('--color-background', hexToRgb(themeColors.background));
    root.style.setProperty('--color-surface-1', hexToRgb(themeColors.surface1));
    root.style.setProperty('--color-surface-2', hexToRgb(themeColors.surface2));
    root.style.setProperty('--color-surface-3', hexToRgb(themeColors.surface3));
    root.style.setProperty('--color-surface-4', hexToRgb(themeColors.surface4));
    root.style.setProperty('--color-surface-border', hexToRgb(themeColors.surfaceBorder));
    root.style.setProperty('--color-text-primary', hexToRgb(themeColors.textPrimary));
    root.style.setProperty('--color-text-secondary', hexToRgb(themeColors.textSecondary));
    root.style.setProperty('--color-text-muted', hexToRgb(themeColors.textMuted));
    root.style.setProperty('--color-primary', hexToRgb(themeColors.primary));
    root.style.setProperty('--color-primary-600', hexToRgb(themeColors.primary600));
    root.style.setProperty('--color-primary-50', hexToRgb(themeColors.primary50));
    root.style.setProperty('--color-secondary', hexToRgb(themeColors.secondary));
    root.style.setProperty('--color-secondary-600', hexToRgb(themeColors.secondary600));
    root.style.setProperty('--color-success', hexToRgb(themeColors.success));
    root.style.setProperty('--color-warning', hexToRgb(themeColors.warning));
    root.style.setProperty('--color-error', hexToRgb(themeColors.error));
    root.style.setProperty('--color-error-dark', hexToRgb(themeColors.errorDark));
  }, []);

  useEffect(() => {
    const savedCustomThemes = localStorage.getItem('nwn2ee-custom-themes');
    if (savedCustomThemes) {
      try {
        setCustomThemes(JSON.parse(savedCustomThemes));
      } catch {
        // Invalid JSON in localStorage, use empty array
      }
    }

    const savedActiveTheme = localStorage.getItem('nwn2ee-active-theme');
    if (savedActiveTheme) {
      try {
        const { name, colors: savedColors } = JSON.parse(savedActiveTheme);
        setSelectedThemeName(name);
        setColors(savedColors);
        applyColorsToDOM(savedColors);
      } catch {
        loadThemeFromCSS();
      }
    } else {
      loadThemeFromCSS();
    }
  }, [applyColorsToDOM]);

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
    setSelectedThemeName('Custom');
  };

  const applyTheme = () => {
    applyColorsToDOM(colors);
    localStorage.setItem('nwn2ee-active-theme', JSON.stringify({ name: selectedThemeName, colors }));
    setHasChanges(false);
  };

  const resetToDefault = () => {
    setColors(DEFAULT_THEME);
    applyColorsToDOM(DEFAULT_THEME);
    setSelectedThemeName('Default Dark');
    localStorage.setItem('nwn2ee-active-theme', JSON.stringify({ name: 'Default Dark', colors: DEFAULT_THEME }));
    setHasChanges(false);
  };

  const selectTheme = (themeName: string) => {
    const allThemes = [...PRESET_THEMES, ...customThemes];
    const theme = allThemes.find(t => t.name === themeName);
    if (theme) {
      setColors(theme.colors);
      setSelectedThemeName(themeName);
      applyColorsToDOM(theme.colors);
      localStorage.setItem('nwn2ee-active-theme', JSON.stringify({ name: themeName, colors: theme.colors }));
      setHasChanges(false);
    }
  };

  const saveAsCustomTheme = () => {
    if (!newThemeName.trim()) return;
    
    const newTheme: SavedTheme = {
      name: newThemeName.trim(),
      colors: { ...colors }
    };
    
    const existingIndex = customThemes.findIndex(t => t.name === newTheme.name);
    let updated: SavedTheme[];
    if (existingIndex >= 0) {
      updated = [...customThemes];
      updated[existingIndex] = newTheme;
    } else {
      updated = [...customThemes, newTheme];
    }
    
    setCustomThemes(updated);
    setSelectedThemeName(newTheme.name);
    localStorage.setItem('nwn2ee-custom-themes', JSON.stringify(updated));
    localStorage.setItem('nwn2ee-active-theme', JSON.stringify({ name: newTheme.name, colors }));
    setShowSaveDialog(false);
    setNewThemeName('');
    setHasChanges(false);
  };

  const deleteCustomTheme = (themeName: string) => {
    const updated = customThemes.filter(t => t.name !== themeName);
    setCustomThemes(updated);
    localStorage.setItem('nwn2ee-custom-themes', JSON.stringify(updated));
    
    if (selectedThemeName === themeName) {
      resetToDefault();
    }
  };

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
        <div className="space-y-2">
          <Label>Theme Preset</Label>
          <div className="flex gap-2">
            <Select value={selectedThemeName} onValueChange={selectTheme}>
              <SelectTrigger className="flex-1">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <div className="px-2 py-1 text-xs text-[rgb(var(--color-text-muted))] font-medium">Presets</div>
                {PRESET_THEMES.map(theme => (
                  <SelectItem key={theme.name} value={theme.name}>{theme.name}</SelectItem>
                ))}
                {customThemes.length > 0 && (
                  <>
                    <div className="px-2 py-1 text-xs text-[rgb(var(--color-text-muted))] font-medium border-t border-[rgb(var(--color-surface-border))] mt-1 pt-2">Custom Themes</div>
                    {customThemes.map(theme => (
                      <div key={theme.name} className="flex items-center justify-between pr-2">
                        <SelectItem value={theme.name}>{theme.name}</SelectItem>
                        <button
                          onClick={(e) => {
                            e.stopPropagation();
                            deleteCustomTheme(theme.name);
                          }}
                          className="text-[rgb(var(--color-error))] hover:text-[rgb(var(--color-error-dark))] p-1"
                        >
                          <TrashIcon className="w-3 h-3" />
                        </button>
                      </div>
                    ))}
                  </>
                )}
                {selectedThemeName === 'Custom' && (
                  <SelectItem value="Custom">Custom (unsaved)</SelectItem>
                )}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="space-y-4">
          <h3 className="theme-section-title">Background & Surfaces</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput label="Background" value={colors.background} onChange={(v) => updateColor('background', v)} description="Main app background" />
            <ColorInput label="Surface 1" value={colors.surface1} onChange={(v) => updateColor('surface1', v)} description="Cards and panels" />
            <ColorInput label="Surface 2" value={colors.surface2} onChange={(v) => updateColor('surface2', v)} description="Input fields" />
            <ColorInput label="Surface 3" value={colors.surface3} onChange={(v) => updateColor('surface3', v)} description="Hover states" />
            <ColorInput label="Surface 4" value={colors.surface4} onChange={(v) => updateColor('surface4', v)} description="Active states" />
            <ColorInput label="Border" value={colors.surfaceBorder} onChange={(v) => updateColor('surfaceBorder', v)} description="Borders and dividers" />
          </div>
        </div>

        <div className="space-y-4">
          <h3 className="theme-section-title">Text Colors</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput label="Primary Text" value={colors.textPrimary} onChange={(v) => updateColor('textPrimary', v)} description="Main text color" />
            <ColorInput label="Secondary Text" value={colors.textSecondary} onChange={(v) => updateColor('textSecondary', v)} description="Subtitles and labels" />
            <ColorInput label="Muted Text" value={colors.textMuted} onChange={(v) => updateColor('textMuted', v)} description="Disabled and hints" />
          </div>
        </div>

        <div className="space-y-4">
          <h3 className="theme-section-title">Accent Colors</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput label="Primary" value={colors.primary} onChange={(v) => updateColor('primary', v)} description="Main brand color" />
            <ColorInput label="Primary Dark" value={colors.primary600} onChange={(v) => updateColor('primary600', v)} description="Hover state" />
            <ColorInput label="Primary Light" value={colors.primary50} onChange={(v) => updateColor('primary50', v)} description="Light backgrounds" />
            <ColorInput label="Secondary" value={colors.secondary} onChange={(v) => updateColor('secondary', v)} description="Secondary accent" />
          </div>
        </div>

        <div className="space-y-4">
          <h3 className="theme-section-title">Semantic Colors</h3>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <ColorInput label="Success" value={colors.success} onChange={(v) => updateColor('success', v)} description="Success states" />
            <ColorInput label="Warning" value={colors.warning} onChange={(v) => updateColor('warning', v)} description="Warning states" />
            <ColorInput label="Error" value={colors.error} onChange={(v) => updateColor('error', v)} description="Error states" />
            <ColorInput label="Error Dark" value={colors.errorDark} onChange={(v) => updateColor('errorDark', v)} description="Error hover" />
          </div>
        </div>

        {showSaveDialog && (
          <div className="p-4 bg-[rgb(var(--color-surface-2))] rounded-lg border border-[rgb(var(--color-surface-border))] space-y-3">
            <Label>Theme Name</Label>
            <input
              type="text"
              value={newThemeName}
              onChange={(e) => setNewThemeName(e.target.value)}
              placeholder="My Custom Theme"
              className="w-full px-3 py-2 bg-[rgb(var(--color-surface-3))] border border-[rgb(var(--color-surface-border))] rounded-md text-[rgb(var(--color-text-primary))]"
            />
            <div className="flex gap-2 justify-end">
              <Button variant="outline" size="sm" onClick={() => setShowSaveDialog(false)}>
                Cancel
              </Button>
              <Button variant="primary" size="sm" onClick={saveAsCustomTheme} disabled={!newThemeName.trim()}>
                Save Theme
              </Button>
            </div>
          </div>
        )}

        <div className="flex gap-3 justify-center theme-actions-border">
          <Button onClick={applyTheme} disabled={!hasChanges} variant="primary">
            Apply Changes
          </Button>
          <Button onClick={() => setShowSaveDialog(true)} variant="outline">
            <PlusIcon className="w-4 h-4" />
            Save as Custom Theme
          </Button>
          <Button onClick={resetToDefault} variant="outline">
            <ArrowPathIcon className="w-4 h-4" />
            Reset to Default
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}