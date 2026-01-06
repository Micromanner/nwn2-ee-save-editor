'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { CharacterAPI } from '@/services/characterApi';

const X = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
  </svg>
);

const Search = ({ className }: { className?: string }) => (
  <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
  </svg>
);

export interface Deity {
  id: number;
  name: string;
  description?: string;
  aliases?: string;
  alignment?: string;
  portfolio?: string;
  favored_weapon?: string;
  icon?: string;
}

interface DeitySelectionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSelectDeity: (deityName: string) => void;
  characterId: number;
  currentDeity?: string;
}

export default function DeitySelectionModal({
  isOpen,
  onClose,
  onSelectDeity,
  characterId,
  currentDeity
}: DeitySelectionModalProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedDeity, setSelectedDeity] = useState<Deity | null>(null);
  const [, setIsLoading] = useState(false);
  const [deities, setDeities] = useState<Deity[]>([]);

  useEffect(() => {
    const fetchDeities = async () => {
      try {
        setIsLoading(true);
        const deitiesData = await CharacterAPI.getAvailableDeities(characterId);
        setDeities(deitiesData);
      } catch (error) {
        console.error('Failed to load deities:', error);
      } finally {
        setIsLoading(false);
      }
    };

    if (isOpen) {
      fetchDeities();
    }
  }, [isOpen, characterId]);

  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      if (currentDeity) {
        const match = deities.find(d => d.name === currentDeity);
        setSelectedDeity(match || null);
      } else {
        setSelectedDeity(null);
      }
    }
  }, [isOpen, currentDeity, deities]);

  const filteredDeities = useMemo(() => {
    let filtered = deities;
    if (searchQuery) {
      const lowerQuery = searchQuery.toLowerCase();
      filtered = filtered.filter(d => 
        d.name.toLowerCase().includes(lowerQuery) ||
        (d.aliases && d.aliases.toLowerCase().includes(lowerQuery))
      );
    }
    return filtered;
  }, [deities, searchQuery]);

  const handleSelect = () => {
    if (selectedDeity) {
      onSelectDeity(selectedDeity.name);
      onClose();
    }
  };

  const handleSelectNone = () => {
    onSelectDeity('');
    onClose();
  };

  const renderSection = (title: string, content: string | undefined, splitByComma: boolean = true) => {
    if (!content) return null;

    let items: string[] = [];
    
    if (splitByComma) {
        items = content.split(/,|;/).map(s => s.trim()).filter(s => s.length > 0);
    } else {
        items = [content.trim()];
    }

    if (items.length === 0) return null;

    return (
        <div className="mt-6 mb-2 first:mt-0">
            <h4 className="text-sm uppercase tracking-wider font-bold text-[rgb(var(--color-primary-light))] border-b border-[rgb(var(--color-primary))/0.3] pb-1 w-full mb-2">
                {title}
            </h4>
            <ul className="list-disc list-inside space-y-1 pl-1">
                {items.map((item, index) => (
                    <li key={`${title}-${index}`} className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed">
                        {item}
                    </li>
                ))}
            </ul>
        </div>
    );
  };

  const renderDeityDetails = (deity: Deity) => {
    let cleanDescription = deity.description || '';
    
    const nameRegex = new RegExp(`^\\s*(${deity.name}|None)[:\\s]*`, 'i');
    if (nameRegex.test(cleanDescription)) {
        cleanDescription = cleanDescription.replace(nameRegex, '').trim();
    }

    return (
        <div className="space-y-1">
            {renderSection('Aliases', deity.aliases, true)}
            {renderSection('Alignment', deity.alignment, false)}
            {renderSection('Portfolio', deity.portfolio, true)}
            {renderSection('Favored Weapon', deity.favored_weapon, false)}
            
            {cleanDescription && (
                <div className="mt-6 mb-2 first:mt-0">
                    <h4 className="text-sm uppercase tracking-wider font-bold text-[rgb(var(--color-primary-light))] border-b border-[rgb(var(--color-primary))/0.3] pb-1 w-full mb-2">
                        Notes
                    </h4>
                    <div className="text-sm text-[rgb(var(--color-text-secondary))] leading-relaxed whitespace-pre-wrap">
                        {cleanDescription}
                    </div>
                </div>
            )}
        </div>
    );
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
      <Card className="w-[90vw] max-w-5xl h-[80vh] flex flex-col bg-[rgb(var(--color-surface-1))] border-[rgb(var(--color-surface-border))] shadow-2xl relative overflow-hidden">
        
        <div className="absolute top-4 right-4 z-20">
            <Button onClick={onClose} variant="ghost" size="sm" className="h-8 w-8 p-0 bg-black/20 hover:bg-black/40 text-white rounded-full transition-all">
              <X className="w-5 h-5" />
            </Button>
        </div>

        <CardContent padding="p-0" className="flex flex-col h-full">
          
          <div className="flex-1 flex overflow-hidden">
            
            <div className="w-1/3 min-w-[300px] flex flex-col border-r border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-2))/30]">
              <div className="p-4 border-b border-[rgb(var(--color-surface-border))] space-y-3 pt-12">
                 <div className="relative">
                    <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
                    <Input 
                      placeholder="Search deities..." 
                      value={searchQuery} 
                      onChange={e => setSearchQuery(e.target.value)} 
                      className="pl-9 bg-[rgb(var(--color-surface-1))]"
                      autoFocus
                    />
                 </div>
                 <div className="text-xs text-[rgb(var(--color-text-muted))] text-right">
                    {filteredDeities.length} found
                 </div>
              </div>
              
              <div className="flex-1 overflow-y-auto p-2 space-y-1">
                <Button
                    variant={!selectedDeity ? "secondary" : "ghost"}
                    className={`w-full justify-start text-left font-medium transition-all duration-200 ${
                        !selectedDeity 
                        ? 'bg-[rgb(var(--color-primary))/10] text-[rgb(var(--color-primary-light))] border-l-2 border-[rgb(var(--color-primary))]' 
                        : 'border-l-2 border-transparent text-[rgb(var(--color-text-muted))] hover:bg-[rgb(var(--color-surface-2))] hover:text-[rgb(var(--color-text-primary))]'
                    }`}
                    onClick={() => setSelectedDeity(null)}
                >
                    None (No Deity)
                </Button>
                
                {filteredDeities.map(deity => (
                  <Button
                    key={deity.id}
                    variant={selectedDeity?.id === deity.id ? "secondary" : "ghost"}
                    className={`w-full justify-start text-left h-auto py-3 px-4 transition-all duration-200 ${
                        selectedDeity?.id === deity.id 
                        ? 'bg-[rgb(var(--color-primary))/15] border-l-2 border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary-light))] shadow-sm' 
                        : 'border-l-2 border-transparent text-[rgb(var(--color-text-secondary))] hover:bg-[rgb(var(--color-surface-2))] hover:border-[rgb(var(--color-surface-border))] hover:text-[rgb(var(--color-text-primary))]'
                    }`}
                    onClick={() => setSelectedDeity(deity)}
                  >
                    <div className="truncate">
                        <span className="font-semibold block truncate">{deity.name}</span>
                        {deity.alignment && <span className="text-xs text-[rgb(var(--color-text-muted))] block">{deity.alignment}</span>}
                    </div>
                  </Button>
                ))}
                
                {filteredDeities.length === 0 && (
                    <div className="p-4 text-center text-[rgb(var(--color-text-muted))]">
                        No deities match your search.
                    </div>
                )}
              </div>
            </div>

            <div className="flex-1 flex flex-col overflow-hidden bg-[rgb(var(--color-surface-1))]">
                {selectedDeity ? (
                    <div className="flex-1 flex flex-col overflow-y-auto p-8">
                        <div className="mb-6 flex items-start gap-4">
                            <div>
                                <h1 className="text-3xl font-bold text-[rgb(var(--color-primary-light))] uppercase tracking-wide mb-2">
                                    {selectedDeity.name}
                                </h1>
                                <div className="h-1 w-20 bg-[rgb(var(--color-primary))] rounded-full"></div>
                            </div>
                        </div>
                        
                        <div className="prose prose-invert max-w-none text-[rgb(var(--color-text-primary))]">
                             <div className="bg-[rgb(var(--color-surface-2))/20] p-6 rounded-xl border border-[rgb(var(--color-surface-border))]">
                                {renderDeityDetails(selectedDeity)}
                             </div>
                        </div>
                    </div>
                ) : (
                    <div className="flex-1 flex items-center justify-center p-8 text-center bg-[rgb(var(--color-surface-2))/20]">
                        <div className="max-w-md">
                            <h3 className="text-xl font-medium text-[rgb(var(--color-text-muted))] mb-2">No Deity Selected</h3>
                            <p className="text-[rgb(var(--color-text-muted))/60]">Select a deity from the list on the left to view their details.</p>
                            <Button className="mt-6" onClick={handleSelectNone}>Select "None"</Button>
                        </div>
                    </div>
                )}
            </div>
            
          </div>

          <div className="flex justify-end gap-3 p-4 border-t border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-2))/50]">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button 
                type="button"
                onClick={selectedDeity ? handleSelect : handleSelectNone}
                disabled={!selectedDeity}
                className="px-8 font-semibold"
            >
              Confirm Selection
            </Button>
          </div>

        </CardContent>
      </Card>
    </div>
  );
}