'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/Button';
import AboutDialog from '@/components/AboutDialog';

import { getCurrentWindow } from '@tauri-apps/api/window';
import { getName } from '@tauri-apps/api/app';

export default function CustomTitleBar() {
  const [appName, setAppName] = useState('NWN2:EE Save Editor');
  const [isMaximized, setIsMaximized] = useState(false);
  const [isAboutOpen, setIsAboutOpen] = useState(false);

  useEffect(() => {
    const getAppName = async () => {
      try {
        const name = await getName();
        setAppName(name);
      } catch {
        // Running in dev mode, using default app name
      }
    };
    getAppName();
  }, []);

  useEffect(() => {
    const checkMaximized = async () => {
      try {
        const appWindow = getCurrentWindow();
        const maximized = await appWindow.isMaximized();
        setIsMaximized(maximized);
      } catch {
        // Could not check maximized state
      }
    };

    checkMaximized();

    const unlisten = getCurrentWindow().onResized(() => {
      checkMaximized();
    });

    return () => {
      unlisten.then(fn => fn());
    };
  }, []);


  const handleMinimize = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.minimize();
    } catch {
      // Minimize not available in dev mode
    }
  };

  const handleMaximize = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.toggleMaximize();
      const maximized = await appWindow.isMaximized();
      setIsMaximized(maximized);
    } catch {
      // Maximize not available in dev mode
    }
  };

  const handleClose = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.close();
    } catch {
      // Close not available in dev mode
    }
  };

  return (
    <div data-tauri-drag-region className="h-8 bg-[rgb(var(--color-surface-2))] flex items-center justify-between px-3 border-b border-[rgb(var(--color-surface-border)/0.6)] select-none">
      <div className="flex items-center space-x-2 text-sm">
        <div className="w-4 h-4 bg-gradient-to-br from-[rgb(var(--color-primary))] to-[rgb(var(--color-primary-600))] rounded flex items-center justify-center">
          <span className="text-white font-bold text-xs">N</span>
        </div>
        <span className="text-[rgb(var(--color-text-primary))] font-medium">{appName}</span>
      </div>

      <div className="flex-1"></div>

      <div className="flex items-center space-x-1">
        <Button
          variant="ghost"
          className="p-1 hover:bg-[rgb(var(--color-surface-3))] rounded-md"
          onClick={() => setIsAboutOpen(true)}
          title="About"
        >
          <svg className="!w-4 !h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
            <path d="M12 17h.01" />
          </svg>
        </Button>
        <div className="w-px h-4 bg-[rgb(var(--color-surface-border))] mx-1" />
        <Button
          variant="ghost"
          size="sm"
          className="px-3 py-2 h-7 w-10 hover:bg-[rgb(var(--color-surface-3))]"
          onClick={handleMinimize}
          title="Minimize"
        >
          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 12 12">
            <rect x="2" y="5" width="8" height="2" />
          </svg>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="px-3 py-2 h-7 w-10 hover:bg-[rgb(var(--color-surface-3))]"
          onClick={handleMaximize}
          title={isMaximized ? "Restore" : "Maximize"}
        >
          {isMaximized ? (
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 12 12">
              <path d="M3.5 4.5h4v4h-4z M4.5 3.5v-1h4v4h-1" strokeWidth="1.2" />
            </svg>
          ) : (
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 12 12">
              <rect x="2" y="2" width="8" height="8" fill="none" stroke="currentColor" strokeWidth="1.2" />
            </svg>
          )}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="px-3 py-2 h-7 w-10 hover:bg-red-500 hover:text-white"
          onClick={handleClose}
          title="Close"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 12 12">
            <path d="M2.5 2.5L9.5 9.5M9.5 2.5L2.5 9.5" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </Button>
      </div>
      
      <AboutDialog isOpen={isAboutOpen} onClose={() => setIsAboutOpen(false)} />
    </div>
  );
}