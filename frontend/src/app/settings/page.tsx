'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { Label } from '@/components/ui/Label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { pathService, PathConfig } from '@/lib/api/paths';
import { open } from '@tauri-apps/plugin-dialog';
import { FolderIcon, CheckCircleIcon, XCircleIcon, PlusIcon, TrashIcon, CogIcon, SwatchIcon, LanguageIcon, PaintBrushIcon } from '@heroicons/react/24/outline';
import ThemeCustomizer from '@/components/Settings/ThemeCustomizer';
import { useLocale } from '@/providers/LocaleProvider';

interface AppSettings {
  theme: 'light' | 'dark';
  fontSize: 'small' | 'medium' | 'large';
}

export default function SettingsPage() {
  const { locale, setLocale } = useLocale();
  const [paths, setPaths] = useState<PathConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings>({
    theme: 'dark',
    fontSize: 'medium'
  });

  useEffect(() => {
    loadPaths();
    loadAppSettings();
  }, []);

  const loadAppSettings = () => {
    try {
      const saved = localStorage.getItem('nwn2ee-app-settings');
      if (saved) {
        setAppSettings(JSON.parse(saved));
      }
    } catch (err) {
      console.error('Error loading app settings:', err);
    }
  };

  const saveAppSettings = (newSettings: Partial<AppSettings>) => {
    const updated = { ...appSettings, ...newSettings };
    setAppSettings(updated);
    localStorage.setItem('nwn2ee-app-settings', JSON.stringify(updated));
    
    // Apply theme changes
    if (newSettings.theme) {
      applyTheme(newSettings.theme);
    }
    
    // Apply font size changes
    if (newSettings.fontSize) {
      applyFontSize(newSettings.fontSize);
    }
  };

  const applyTheme = (theme: 'light' | 'dark') => {
    const root = document.documentElement;
    if (theme === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  };

  const applyFontSize = (fontSize: 'small' | 'medium' | 'large') => {
    const root = document.documentElement;
    root.classList.remove('text-sm', 'text-base', 'text-lg');
    
    switch (fontSize) {
      case 'small':
        root.classList.add('text-sm');
        break;
      case 'large':
        root.classList.add('text-lg');
        break;
      default:
        root.classList.add('text-base');
    }
  };

  const loadPaths = async () => {
    try {
      setLoading(true);
      const response = await pathService.getConfig();
      setPaths(response.paths);
    } catch (err) {
      setError('Failed to load path configuration');
      console.error('Error loading paths:', err);
    } finally {
      setLoading(false);
    }
  };

  const selectFolder = async (title: string): Promise<string | null> => {
    const selected = await open({
      directory: true,
      multiple: false,
      title
    });
    return selected as string | null;
  };

  const updatePath = async (
    type: 'game' | 'documents' | 'workshop'
  ) => {
    const title = type === 'game' ? 'Select NWN2 Game Folder' :
                  type === 'documents' ? 'Select NWN2 Documents Folder' :
                  'Select Steam Workshop Folder';
    
    const selected = await selectFolder(title);
    if (!selected) return;

    try {
      setSaving(true);
      let response;
      switch (type) {
        case 'game':
          response = await pathService.setGameFolder(selected);
          break;
        case 'documents':
          response = await pathService.setDocumentsFolder(selected);
          break;
        case 'workshop':
          response = await pathService.setSteamWorkshopFolder(selected);
          break;
      }
      setPaths(response.paths);
    } catch (err) {
      setError(`Failed to update ${type} folder`);
      console.error('Error updating path:', err);
    } finally {
      setSaving(false);
    }
  };

  const resetPath = async (
    type: 'game' | 'documents' | 'workshop'
  ) => {
    try {
      setSaving(true);
      let response;
      switch (type) {
        case 'game':
          response = await pathService.resetGameFolder();
          break;
        case 'documents':
          response = await pathService.resetDocumentsFolder();
          break;
        case 'workshop':
          response = await pathService.resetSteamWorkshopFolder();
          break;
      }
      setPaths(response.paths);
    } catch (err) {
      setError(`Failed to reset ${type} folder`);
      console.error('Error resetting path:', err);
    } finally {
      setSaving(false);
    }
  };

  const addCustomFolder = async (type: 'override' | 'module' | 'hak') => {
    const title = `Select Custom ${type.charAt(0).toUpperCase() + type.slice(1)} Folder`;
    const selected = await selectFolder(title);
    if (!selected) return;

    try {
      setSaving(true);
      let response;
      switch (type) {
        case 'override':
          response = await pathService.addOverrideFolder(selected);
          break;
        case 'module':
          response = await pathService.addModuleFolder(selected);
          break;
        case 'hak':
          response = await pathService.addHakFolder(selected);
          break;
      }
      setPaths(response.paths);
    } catch (err) {
      setError(`Failed to add custom ${type} folder`);
      console.error('Error adding custom folder:', err);
    } finally {
      setSaving(false);
    }
  };

  const removeCustomFolder = async (type: 'override' | 'module' | 'hak', path: string) => {
    try {
      setSaving(true);
      let response;
      switch (type) {
        case 'override':
          response = await pathService.removeOverrideFolder(path);
          break;
        case 'module':
          response = await pathService.removeModuleFolder(path);
          break;
        case 'hak':
          response = await pathService.removeHakFolder(path);
          break;
      }
      setPaths(response.paths);
    } catch (err) {
      setError(`Failed to remove custom ${type} folder`);
      console.error('Error removing custom folder:', err);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto"></div>
          <p className="mt-4 text-sm text-muted-foreground">Loading path configuration...</p>
        </div>
      </div>
    );
  }

  if (!paths) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center text-destructive">
          <p>Failed to load path configuration</p>
          <Button onClick={loadPaths} className="mt-4">
            Retry
          </Button>
        </div>
      </div>
    );
  }

  const PathRow = ({ 
    label, 
    path, 
    exists, 
    autoDetected,
    onEdit,
    onReset
  }: { 
    label: string;
    path: string | null;
    exists: boolean;
    autoDetected: boolean;
    onEdit: () => void;
    onReset: () => void;
  }) => (
    <div className="flex items-center justify-between py-3 border-b last:border-0">
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <FolderIcon className="w-5 h-5 text-muted-foreground" />
          <span className="font-medium">{label}</span>
          {autoDetected ? (
            <span className="text-xs bg-blue-100 text-blue-800 px-2 py-0.5 rounded">Auto-detected</span>
          ) : (
            <span className="text-xs bg-amber-100 text-amber-800 px-2 py-0.5 rounded">Manually Set</span>
          )}
        </div>
        <div className="mt-1 flex items-center gap-2">
          <span className="text-sm text-muted-foreground font-mono">
            {path || '(Not configured)'}
          </span>
          {path && (
            exists ? 
              <CheckCircleIcon className="w-4 h-4 text-green-600" /> :
              <XCircleIcon className="w-4 h-4 text-red-600" />
          )}
        </div>
      </div>
      <div className="flex items-center gap-2">
        {!autoDetected && (
          <Button 
            onClick={onReset} 
            variant="ghost" 
            size="sm"
            disabled={saving}
            className="text-muted-foreground hover:text-destructive transition-colors"
          >
            Reset
          </Button>
        )}
        <Button 
          onClick={onEdit} 
          variant="outline" 
          size="sm"
          disabled={saving}
        >
          {path ? 'Change' : 'Set'}
        </Button>
      </div>
    </div>
  );

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-semibold text-[rgb(var(--color-text-primary))]">Settings</h2>
      
      <Tabs defaultValue="general" className="w-full">
        <TabsList className="w-full flex bg-transparent p-0 gap-2 mb-6">
          <TabsTrigger 
            value="general" 
            className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10] flex items-center justify-center gap-2"
          >
            <CogIcon className="w-4 h-4" />
            General
          </TabsTrigger>
          <TabsTrigger 
            value="theme" 
            className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10] flex items-center justify-center gap-2"
          >
            <PaintBrushIcon className="w-4 h-4" />
            Theme
          </TabsTrigger>
          <TabsTrigger 
            value="paths" 
            className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10] flex items-center justify-center gap-2"
          >
            <FolderIcon className="w-4 h-4" />
            Game Paths
          </TabsTrigger>
        </TabsList>

        <TabsContent value="general" className="space-y-6">

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <LanguageIcon className="w-5 h-5" />
                Language & Region
              </CardTitle>
              <CardDescription>Language and localization settings</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="language">Language</Label>
                <Select 
                  value={locale} 
                  onValueChange={(value: string) => setLocale(value)}
                >
                  <SelectTrigger id="language">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="en">English</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <CogIcon className="w-5 h-5" />
                Display
              </CardTitle>
              <CardDescription>Adjust display settings</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="fontSize">Font Size</Label>
                <Select 
                  value={appSettings.fontSize} 
                  onValueChange={(value: 'small' | 'medium' | 'large') => 
                    saveAppSettings({ fontSize: value })
                  }
                >
                  <SelectTrigger id="fontSize">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="small">Small</SelectItem>
                    <SelectItem value="medium">Medium</SelectItem>
                    <SelectItem value="large">Large</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>


        </TabsContent>

        <TabsContent value="theme" className="space-y-6">
          <ThemeCustomizer />
        </TabsContent>

        <TabsContent value="paths" className="space-y-6">
          <h2 className="text-xl font-semibold">Game Paths Configuration</h2>

      {error && (
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <p className="text-sm text-destructive">{error}</p>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Main Paths</CardTitle>
          <CardDescription>Configure the main NWN2 installation and user directories</CardDescription>
        </CardHeader>
        <CardContent>
          <PathRow
            label="Game Installation Folder"
            path={paths.game_folder.path}
            exists={paths.game_folder.exists}
            autoDetected={paths.game_folder.auto_detected}
            onEdit={() => updatePath('game')}
            onReset={() => resetPath('game')}
          />
          <PathRow
            label="Documents Folder"
            path={paths.documents_folder.path}
            exists={paths.documents_folder.exists}
            autoDetected={paths.documents_folder.auto_detected}
            onEdit={() => updatePath('documents')}
            onReset={() => resetPath('documents')}
          />
          <PathRow
            label="Steam Workshop Folder"
            path={paths.steam_workshop_folder.path}
            exists={paths.steam_workshop_folder.exists}
            autoDetected={paths.steam_workshop_folder.auto_detected}
            onEdit={() => updatePath('workshop')}
            onReset={() => resetPath('workshop')}
          />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Custom Override Folders</CardTitle>
          <CardDescription>Additional directories to search for override files</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {paths.custom_override_folders.map((folder, i) => (
              <div key={i} className="flex items-center justify-between p-3 bg-muted rounded-md">
                <div className="flex items-center gap-2">
                  <FolderIcon className="w-5 h-5 text-muted-foreground" />
                  <span className="text-sm font-mono">{folder.path}</span>
                  {folder.exists ? 
                    <CheckCircleIcon className="w-4 h-4 text-green-600" /> :
                    <XCircleIcon className="w-4 h-4 text-red-600" />
                  }
                </div>
                <Button
                  onClick={() => removeCustomFolder('override', folder.path)}
                  variant="ghost"
                  size="sm"
                  disabled={saving}
                >
                  <TrashIcon className="w-4 h-4" />
                </Button>
              </div>
            ))}
            <Button
              onClick={() => addCustomFolder('override')}
              variant="outline"
              size="sm"
              className="w-full"
              disabled={saving}
            >
              <PlusIcon className="w-4 h-4 mr-2" />
              Add Override Folder
            </Button>
          </div>
        </CardContent>
      </Card>


      <Card>
        <CardHeader>
          <CardTitle>Custom HAK Folders</CardTitle>
          <CardDescription>Additional directories to search for HAK files</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {paths.custom_hak_folders.map((folder, i) => (
              <div key={i} className="flex items-center justify-between p-3 bg-muted rounded-md">
                <div className="flex items-center gap-2">
                  <FolderIcon className="w-5 h-5 text-muted-foreground" />
                  <span className="text-sm font-mono">{folder.path}</span>
                  {folder.exists ? 
                    <CheckCircleIcon className="w-4 h-4 text-green-600" /> :
                    <XCircleIcon className="w-4 h-4 text-red-600" />
                  }
                </div>
                <Button
                  onClick={() => removeCustomFolder('hak', folder.path)}
                  variant="ghost"
                  size="sm"
                  disabled={saving}
                >
                  <TrashIcon className="w-4 h-4" />
                </Button>
              </div>
            ))}
            <Button
              onClick={() => addCustomFolder('hak')}
              variant="outline"
              size="sm"
              className="w-full"
              disabled={saving}
            >
              <PlusIcon className="w-4 h-4 mr-2" />
              Add HAK Folder
            </Button>
          </div>
        </CardContent>
      </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}