import React from 'react';
import { Button } from '@/components/ui/Button';

interface EditorHeaderProps {
  characterName: string;
  saveName?: string;
  onBack: () => void;
  onImport: () => void;
  onExport: () => void;
  onSave: () => void;
  isModified?: boolean;
}

export default function EditorHeader({
  characterName,
  saveName: _saveName,
  onBack,
  onImport: _onImport,
  onExport,
  onSave,
  isModified = false,
}: EditorHeaderProps) {
  return (
    <div className="h-14 bg-[rgb(var(--color-surface-2))] border-b border-[rgb(var(--color-surface-border))] flex items-center justify-between px-4 shadow-sm z-10 select-none">
      <div className="flex items-center gap-4">
        <Button 
          variant="ghost" 
          size="sm" 
          onClick={onBack}
          className="flex items-center gap-2 text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))] px-2"
          title="Back to Dashboard"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
          <span className="font-medium text-sm">Back</span>
        </Button>
        
        <div className="h-6 w-px bg-[rgb(var(--color-surface-border))] mx-2" />

        <div className="flex items-center gap-2">
          <span className="text-[rgb(var(--color-text-muted))] text-sm font-medium uppercase tracking-wider">Editing:</span>
          <h1 className="text-base font-semibold text-[rgb(var(--color-text-primary))] truncate max-w-[400px]">{characterName}</h1>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={onExport} className="h-9 px-4 text-sm font-medium">
          Export
        </Button>
        <div className="w-px h-6 bg-[rgb(var(--color-surface-border))] mx-2"></div>
        <Button 
            variant={isModified ? "primary" : "outline"} 
            size="sm" 
            onClick={onSave} 
            className={`h-9 px-6 font-semibold shadow-sm text-sm ${isModified ? 'animate-pulse' : 'text-muted-foreground'}`}
            title={isModified ? "You have unsaved changes" : "No changes to save"}
        >
          Save Changes
        </Button>
      </div>
    </div>
  );
}
