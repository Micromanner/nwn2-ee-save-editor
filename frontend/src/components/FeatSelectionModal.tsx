
'use client';

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/Button';
import { useCharacterContext } from '@/contexts/CharacterContext';
import { CharacterAPI } from '@/services/characterApi';
import { useToast } from '@/contexts/ToastContext';
import type { FeatInfo } from './Feats/types';

interface FeatSelectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  featType: number; // Bitmask to filter by
  title: string;
}

export function FeatSelectionModal({ isOpen, onClose, featType, title }: FeatSelectionModalProps) {
  const { character, invalidateSubsystems } = useCharacterContext();
  const { showToast } = useToast();
  
  const [feats, setFeats] = useState<FeatInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  
  // Load feats when modal opens or filter changes
  useEffect(() => {
    if (!isOpen || !character?.id) return;
    
    const loadFeats = async () => {
      setIsLoading(true);
      setError(null);
      try {
        if (!character?.id) return;
        const response = await CharacterAPI.getLegitimateFeats(character.id, {
            featType: featType,
            limit: 1000, 
            search: searchTerm || undefined
        });
        setFeats(response.feats);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load feats');
      } finally {
        setIsLoading(false);
      }
    };
    
    loadFeats();
  }, [isOpen, character?.id, featType, searchTerm]);

  const handleSelect = async (featId: number) => {
      if (!character?.id) return;
      try {
          await CharacterAPI.addFeat(character.id, featId);
          await invalidateSubsystems(['feats', 'combat', 'abilityScores']);
          showToast('Feature added successfully', 'success');
          onClose();
      } catch (err) {
          showToast(err instanceof Error ? err.message : 'Failed to add feature', 'error');
      }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <div className="bg-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border))] rounded-xl shadow-2xl w-full max-w-2xl max-h-[80vh] flex flex-col overflow-hidden">
        <div className="p-4 border-b border-[rgb(var(--color-surface-border))] flex justify-between items-center bg-[rgb(var(--color-surface-2))]">

          <h2 className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{title}</h2>
          <Button variant="ghost" size="sm" onClick={onClose}>
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </Button>
        </div>
        
        <div className="p-4 border-b border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-1))]">

            <input 
                type="text" 
                placeholder="Search..." 
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                className="w-full px-3 py-2 bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded-md focus:outline-none focus:border-[rgb(var(--color-primary))]"
            />
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-2">

          {isLoading ? (
             <div className="flex justify-center p-8">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
             </div>
          ) : error ? (
            <div className="text-error p-4 text-center">{error}</div>
          ) : feats.length === 0 ? (
            <div className="text-muted p-4 text-center">No options available.</div>
          ) : (
            feats.map(feat => (
              <div key={feat.id} className="flex justify-between items-center p-3 rounded-lg bg-[rgb(var(--color-surface-2))] hover:bg-[rgb(var(--color-surface-3))] transition-colors border border-[rgb(var(--color-surface-border)/0.5)]">
                <div>
                   <div className="font-bold text-[rgb(var(--color-text-primary))]">{feat.name}</div>
                   <div className="text-sm text-[rgb(var(--color-text-secondary))] line-clamp-1">{feat.description || 'No description'}</div>
                </div>
                <Button size="sm" onClick={() => handleSelect(feat.id)}>Select</Button>
              </div>
            ))
          )}
        </div>
        
        <div className="p-4 border-t border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-2))] flex justify-end">

          <Button variant="outline" onClick={onClose}>Cancel</Button>
        </div>
      </div>
    </div>
  );
}
