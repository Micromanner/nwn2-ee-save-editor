'use client';

import { useState } from 'react';
import { useTauri } from '@/providers/TauriProvider';
import { Button } from '@/components/ui/Button';

interface GameLaunchDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onLaunch: (closeEditor: boolean) => void;
  saveName?: string;
  gamePathDetected?: boolean;
}

export function GameLaunchDialog({ isOpen, onClose, onLaunch, saveName, gamePathDetected = true }: GameLaunchDialogProps) {
  const { api } = useTauri();
  const [closeEditor, setCloseEditor] = useState(false);
  const [isLaunching, setIsLaunching] = useState(false);

  const handleLaunch = async () => {
    if (!api) return;
    
    setIsLaunching(true);
    try {
      await onLaunch(closeEditor);
    } finally {
      setIsLaunching(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      {/* Backdrop with blur */}
      <div 
        className="absolute inset-0 bg-black/60 backdrop-blur-sm animate-in fade-in duration-200"
        onClick={onClose}
      />
      
      {/* Modal Content */}
      <div className="relative bg-[rgb(var(--color-surface-1))] rounded-xl border border-[rgb(var(--color-surface-border))] shadow-2xl p-6 max-w-md w-full animate-in zoom-in-95 duration-200 slide-in-from-bottom-2">
        
        {/* Header */}
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-3">
             <div>
               <h2 className="text-xl font-bold text-[rgb(var(--color-text-primary))]">Save Complete</h2>
             </div>
          </div>
          <button
            onClick={onClose}
            disabled={isLaunching}
            className="text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))] transition-colors p-1 rounded hover:bg-[rgb(var(--color-surface-2))] disabled:opacity-50"
          >
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" /></svg>
          </button>
        </div>
        
        <div className="mb-6 space-y-4">
          <p className="text-[rgb(var(--color-text-secondary))] leading-relaxed text-base">
            <span className="font-semibold text-[rgb(var(--color-text-primary))]">{saveName || "Character"}</span> has been saved successfully.
          </p>
          <div className="bg-[rgb(var(--color-surface-2))] p-4 rounded-lg border border-[rgb(var(--color-surface-border))]">
            <p className="text-[rgb(var(--color-text-primary))] font-medium mb-1">
              Ready to verify in-game?
            </p>
            <p className="text-sm text-[rgb(var(--color-text-secondary))]">
              Launch NWN2:EE now to test your changes immediately.
            </p>
          </div>
          
          {!gamePathDetected && (
            <div className="flex items-start gap-3 p-3 bg-yellow-500/10 border border-yellow-500/20 rounded-md text-yellow-500 text-sm">
               <svg className="w-5 h-5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" /></svg>
               <span>Game installation not automatically detected. We'll try to find it when you launch.</span>
            </div>
          )}
        </div>

        {/* Action Area */}
        <div className="space-y-4">
          <label className="flex items-center space-x-3 cursor-pointer group bg-[rgb(var(--color-surface-2))] p-3 rounded-md border border-[rgb(var(--color-surface-border))] hover:border-[rgb(var(--color-primary)/0.5)] transition-colors">
            <div className="relative flex items-center">
              <input
                type="checkbox"
                checked={closeEditor}
                onChange={(e) => setCloseEditor(e.target.checked)}
                className="peer h-5 w-5 cursor-pointer appearance-none rounded-md border border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-1))] checked:border-[rgb(var(--color-primary))] checked:bg-[rgb(var(--color-primary))] focus:ring-2 focus:ring-[rgb(var(--color-primary))] focus:ring-offset-1 focus:ring-offset-[rgb(var(--color-surface-1))]"
                disabled={isLaunching}
              />
              <svg 
                className="pointer-events-none absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 w-3.5 h-3.5 text-white opacity-0 peer-checked:opacity-100 transition-opacity" 
                fill="none" 
                stroke="currentColor" 
                viewBox="0 0 24 24"
              >
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
              </svg>
            </div>
            <span className="text-[rgb(var(--color-text-secondary))] text-sm group-hover:text-[rgb(var(--color-text-primary))] transition-colors">
              Close editor after launching game
            </span>
          </label>

          <div className="flex gap-3">
            <Button
              onClick={handleLaunch}
              variant="primary"
              loading={isLaunching}
              loadingText="Launching..."
              className="flex-1 h-11 text-base shadow-lg shadow-[rgb(var(--color-primary)/0.2)]"
            >
              Launch Game
            </Button>
            <Button
              onClick={onClose}
              variant="outline"
              disabled={isLaunching}
              className="flex-1 h-11 text-base bg-transparent border-[rgb(var(--color-surface-border))]"
            >
              Stay in Editor
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}