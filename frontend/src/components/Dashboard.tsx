import React, { useState, useEffect } from 'react';
import { Button } from '@/components/ui/Button';
import { SaveFileSelector } from '@/components/Saves/SaveFileSelector';
import { useTranslations } from '@/hooks/useTranslations';
import { getName } from '@tauri-apps/api/app';

interface DashboardProps {
  onOpenBackups: () => void;
  onSettings: () => void;
  onOpenFolder: () => void;
  onImportCharacter: () => void;
  activeCharacter?: { name: string };
  onContinueEditing?: () => void;
  onCloseSession?: () => void;
}

export default function Dashboard({ 
  onOpenBackups, 
  onSettings, 
  onOpenFolder, 
  onImportCharacter,
  activeCharacter,
  onContinueEditing,
  onCloseSession
}: DashboardProps) {
  useTranslations();
  const [appName, setAppName] = useState('NWN2:EE Save Editor');

  useEffect(() => {
    getName().then(setAppName).catch(() => {});
  }, []);

  return (
    <div className="flex flex-col h-full bg-[rgb(var(--color-background))] text-[rgb(var(--color-text-primary))] p-8 overflow-y-auto w-full">
      <div className="mb-8 text-center">
        <h1 className="text-4xl font-bold bg-gradient-to-r from-[rgb(var(--color-primary))] to-[rgb(var(--color-primary-600))] bg-clip-text text-transparent mb-2">
          {appName}
        </h1>
        <p className="text-[rgb(var(--color-text-secondary))]">
          Manage, edit, and backup your Neverwinter Nights 2 saves
        </p>
      </div>

      <div className="max-w-6xl mx-auto w-full flex flex-col gap-8">
        {activeCharacter && (
          <div className="w-full animate-in fade-in slide-in-from-top-4 duration-300">
            <div className="bg-[rgb(var(--color-primary)/0.1)] rounded-lg p-6 border border-[rgb(var(--color-primary)/0.3)] shadow-elevation-2 flex flex-col sm:flex-row items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 rounded-full bg-[rgb(var(--color-primary)/0.2)] flex items-center justify-center text-[rgb(var(--color-primary))]">
                  <span className="text-xl">✎</span>
                </div>
                <div>
                  <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                    Session Active: <span className="text-[rgb(var(--color-primary))]">{activeCharacter.name}</span>
                  </h3>
                  <p className="text-sm text-[rgb(var(--color-text-secondary))]">
                    You have a loaded save file.
                  </p>
                </div>
              </div>
              <div className="flex items-center gap-3 w-full sm:w-auto">
                 <Button 
                   variant="ghost" 
                   onClick={onCloseSession}
                   className="text-red-400 hover:text-red-300 hover:bg-red-900/20"
                 >
                   Close Session
                 </Button>
                 <Button 
                   variant="primary" 
                   onClick={onContinueEditing}
                   className="bg-[rgb(var(--color-primary))] hover:bg-[rgb(var(--color-primary-600))] text-white shadow-lg"
                 >
                   Continue Editing
                 </Button>
              </div>
            </div>
          </div>
        )}

        <div className="w-full">
          <div className="bg-[rgb(var(--color-surface-1))] rounded-lg p-6 shadow-elevation-1 border border-[rgb(var(--color-surface-border))]">
            <h2 className="text-xl font-semibold mb-4 text-[rgb(var(--color-text-primary))]">Quick Actions</h2>
            <div className="flex flex-wrap gap-4">
              <Button 
                variant="outline" 
                className="flex-1 min-w-[200px] h-12"
                onClick={onImportCharacter}
              >
                <span>Import Character</span>
              </Button>
              <Button 
                variant="outline" 
                className="flex-1 min-w-[200px] h-12"
                onClick={onOpenFolder}
              >
                <span>Open Save Folder</span>
              </Button>
              <Button 
                variant="outline" 
                className="flex-1 min-w-[200px] h-12"
                onClick={onOpenBackups}
              >
                <span>Manage Backups</span>
              </Button>
              <Button 
                variant="outline" 
                className="flex-1 min-w-[200px] h-12"
                onClick={onSettings}
              >
                <span>Settings</span>
              </Button>
            </div>
          </div>
        </div>

        <div className="w-full space-y-6">
          <div className="bg-[rgb(var(--color-surface-1))] rounded-lg p-6 shadow-elevation-1 border border-[rgb(var(--color-surface-border))] h-full min-h-[660px] flex flex-col">
            <h2 className="text-xl font-semibold mb-4 text-[rgb(var(--color-text-primary))]">Select a Save Game</h2>
            <div className="flex-1 overflow-hidden">
               <SaveFileSelector />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
