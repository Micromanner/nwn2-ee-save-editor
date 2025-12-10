'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/Select';
import { Label } from '@/components/ui/Label';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/Tabs';
import { pathService, PathConfig } from '@/lib/api/paths';
import { open } from '@tauri-apps/plugin-dialog';
import { FolderIcon, CheckCircleIcon, XCircleIcon, PlusIcon, TrashIcon, ArrowPathIcon, CogIcon, SwatchIcon, LanguageIcon, PaintBrushIcon } from '@heroicons/react/24/outline';
import ThemeCustomizer from '@/components/Settings/ThemeCustomizer';

interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  language: string;
  fontSize: 'small' | 'medium' | 'large';
  autoSave: boolean;
}

export default function SettingsPage() {
  const [paths, setPaths] = useState<PathConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [autoDetecting, setAutoDetecting] = useState(false);
  const [detectedPaths, setDetectedPaths] = useState<string[]>([]);
  const [detectedDocsFolder, setDetectedDocsFolder] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [appSettings, setAppSettings] = useState<AppSettings>({
    theme: 'system',
    language: 'en',
    fontSize: 'medium',
    autoSave: true
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

  const applyTheme = (theme: 'light' | 'dark' | 'system') => {
    const root = document.documentElement;
    
    if (theme === 'system') {
      const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      root.classList.toggle('dark', isDark);
    } else if (theme === 'dark') {
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

  const autoDetectPaths = async () => {
    try {
      setAutoDetecting(true);
      const response = await pathService.autoDetect();
      setDetectedPaths(response.game_installations);
      setDetectedDocsFolder(response.documents_folder);
      setPaths(response.current_paths);
      
      // If documents folder was detected but not set, offer to set it
      if (response.documents_folder && !response.current_paths.documents_folder.path) {
        const confirmSet = window.confirm('Documents folder detected. Would you like to set it automatically?');
        if (confirmSet) {
          try {
            const updateResponse = await pathService.setDocumentsFolder(response.documents_folder);
            setPaths(updateResponse.paths);
          } catch (err) {
            console.error('Error setting documents folder:', err);
          }
        }
      }
    } catch (err) {
      setError('Failed to auto-detect paths');
      console.error('Error auto-detecting paths:', err);
    } finally {
      setAutoDetecting(false);
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
    onEdit 
  }: { 
    label: string;
    path: string | null;
    exists: boolean;
    autoDetected: boolean;
    onEdit: () => void;
  }) => (
    <div className="flex items-center justify-between py-3 border-b last:border-0">
      <div className="flex-1">
        <div className="flex items-center gap-2">
          <FolderIcon className="w-5 h-5 text-muted-foreground" />
          <span className="font-medium">{label}</span>
          {autoDetected && (
            <span className="text-xs bg-blue-100 text-blue-800 px-2 py-0.5 rounded">Auto-detected</span>
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
      <Button 
        onClick={onEdit} 
        variant="outline" 
        size="sm"
        disabled={saving}
      >
        {path ? 'Change' : 'Set'}
      </Button>
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
                <SwatchIcon className="w-5 h-5" />
                Appearance
              </CardTitle>
              <CardDescription>Customize the look and feel of the application</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="theme">Theme</Label>
                <Select 
                  value={appSettings.theme} 
                  onValueChange={(value: 'light' | 'dark' | 'system') => 
                    saveAppSettings({ theme: value })
                  }
                >
                  <SelectTrigger id="theme">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="light">Light</SelectItem>
                    <SelectItem value="dark">Dark</SelectItem>
                    <SelectItem value="system">System</SelectItem>
                  </SelectContent>
                </Select>
              </div>

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
                  value={appSettings.language} 
                  onValueChange={(value: string) => 
                    saveAppSettings({ language: value })
                  }
                >
                  <SelectTrigger id="language">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="en">English</SelectItem>
                    <SelectItem value="es">Español</SelectItem>
                    <SelectItem value="fr">Français</SelectItem>
                    <SelectItem value="de">Deutsch</SelectItem>
                    <SelectItem value="it">Italiano</SelectItem>
                    <SelectItem value="pt">Português</SelectItem>
                    <SelectItem value="ru">Русский</SelectItem>
                    <SelectItem value="zh">中文</SelectItem>
                    <SelectItem value="ja">日本語</SelectItem>
                    <SelectItem value="ko">한국어</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Editor Settings</CardTitle>
              <CardDescription>Configure save editor behavior</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <Label htmlFor="autoSave">Auto-save changes</Label>
                  <p className="text-sm text-muted-foreground">Automatically save changes as you edit</p>
                </div>
                <Button
                  variant={appSettings.autoSave ? "primary" : "outline"}
                  size="sm"
                  onClick={() => saveAppSettings({ autoSave: !appSettings.autoSave })}
                >
                  {appSettings.autoSave ? "Enabled" : "Disabled"}
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="theme" className="space-y-6">
          <ThemeCustomizer />
        </TabsContent>

        <TabsContent value="paths" className="space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-semibold">Game Paths Configuration</h2>
            <Button 
              onClick={autoDetectPaths}
              disabled={autoDetecting}
              variant="outline"
            >
              <ArrowPathIcon className={`w-4 h-4 mr-2 ${autoDetecting ? 'animate-spin' : ''}`} />
              Auto-detect Paths
            </Button>
          </div>

      {error && (
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <p className="text-sm text-destructive">{error}</p>
          </CardContent>
        </Card>
      )}

      {(detectedPaths.length > 0 || detectedDocsFolder) && (
        <Card>
          <CardHeader>
            <CardTitle>Detected Paths</CardTitle>
            <CardDescription>Auto-detected NWN2 installation paths</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            {detectedPaths.length > 0 && (
              <div>
                <h4 className="font-medium mb-2">Game Installations</h4>
                <div className="space-y-2">
                  {detectedPaths.map((path, i) => (
                    <Button
                      key={i}
                      onClick={async () => {
                        try {
                          setSaving(true);
                          const response = await pathService.setGameFolder(path);
                          setPaths(response.paths);
                          setDetectedPaths([]);
                        } catch {
                          setError('Failed to set game folder');
                        } finally {
                          setSaving(false);
                        }
                      }}
                      variant="ghost"
                      className="w-full text-left p-3 justify-start h-auto"
                    >
                      <span className="text-sm font-mono">{path}</span>
                    </Button>
                  ))}
                </div>
              </div>
            )}
            
            {detectedDocsFolder && !paths?.documents_folder.path && (
              <div>
                <h4 className="font-medium mb-2">Documents Folder</h4>
                <Button
                  onClick={async () => {
                    try {
                      setSaving(true);
                      const response = await pathService.setDocumentsFolder(detectedDocsFolder);
                      setPaths(response.paths);
                      setDetectedDocsFolder(null);
                    } catch {
                      setError('Failed to set documents folder');
                    } finally {
                      setSaving(false);
                    }
                  }}
                  variant="ghost"
                  className="w-full text-left p-3 justify-start h-auto"
                >
                  <span className="text-sm font-mono">{detectedDocsFolder}</span>
                  <span className="text-xs text-muted-foreground block mt-1">Click to use this folder</span>
                </Button>
              </div>
            )}
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
          />
          <PathRow
            label="Documents Folder"
            path={paths.documents_folder.path}
            exists={paths.documents_folder.exists}
            autoDetected={paths.documents_folder.auto_detected}
            onEdit={() => updatePath('documents')}
          />
          <PathRow
            label="Steam Workshop Folder"
            path={paths.steam_workshop_folder.path}
            exists={paths.steam_workshop_folder.exists}
            autoDetected={paths.steam_workshop_folder.auto_detected}
            onEdit={() => updatePath('workshop')}
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
          <CardTitle>Custom Module Folders</CardTitle>
          <CardDescription>Additional directories to search for modules</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            {paths.custom_module_folders.map((folder, i) => (
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
                  onClick={() => removeCustomFolder('module', folder.path)}
                  variant="ghost"
                  size="sm"
                  disabled={saving}
                >
                  <TrashIcon className="w-4 h-4" />
                </Button>
              </div>
            ))}
            <Button
              onClick={() => addCustomFolder('module')}
              variant="outline"
              size="sm"
              className="w-full"
              disabled={saving}
            >
              <PlusIcon className="w-4 h-4 mr-2" />
              Add Module Folder
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