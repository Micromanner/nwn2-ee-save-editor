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
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-surface-2 rounded-lg p-6 max-w-md w-full mx-4 border border-primary">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-text-primary">Save Complete!</h2>
          <Button
            onClick={onClose}
            variant="ghost"
            size="icon"
            disabled={isLaunching}
          >
            ×
          </Button>
        </div>
        
        <div className="mb-6">
          <p className="text-text-secondary mb-2">
            {saveName ? `"${saveName}" has been saved successfully.` : 'Your character has been saved successfully.'}
          </p>
          <p className="text-text-primary font-medium">
            Would you like to launch NWN2:EE to test your changes?
          </p>
          {!gamePathDetected && (
            <p className="text-warning text-sm mt-2">
              ⚠️ NWN2 installation not detected automatically. The editor will attempt to find the game when launching.
            </p>
          )}
        </div>

        <div className="mb-6">
          <label className="flex items-center space-x-2 cursor-pointer">
            <input
              type="checkbox"
              checked={closeEditor}
              onChange={(e) => setCloseEditor(e.target.checked)}
              className="rounded border-surface-3 bg-surface-1 text-primary focus:ring-primary focus:ring-offset-0"
              disabled={isLaunching}
            />
            <span className="text-text-secondary text-sm">
              Close editor when game starts
            </span>
          </label>
        </div>

        <div className="flex gap-3">
          <Button
            onClick={handleLaunch}
            variant="primary"
            loading={isLaunching}
            loadingText="Launching..."
            className="flex-1"
          >
            Launch Game
          </Button>
          <Button
            onClick={onClose}
            variant="secondary"
            disabled={isLaunching}
            className="flex-1"
          >
            Stay in Editor
          </Button>
        </div>
      </div>
    </div>
  );
}