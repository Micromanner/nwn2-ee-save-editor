'use client';

import { useState, useEffect, useMemo } from 'react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { inventoryAPI, BaseItem } from '@/services/inventoryApi';
import { useTranslations } from '@/hooks/useTranslations';

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

interface ItemTemplateSelectorProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (baseItemId: number) => Promise<void>;
  characterId: number | undefined;
}

export default function ItemTemplateSelector({
  isOpen,
  onClose,
  onSelect,
  characterId
}: ItemTemplateSelectorProps) {
  const t = useTranslations();
  const [baseItems, setBaseItems] = useState<BaseItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('All');
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (isOpen && characterId) {
      loadBaseItems();
    }
  }, [isOpen, characterId]);

  const loadBaseItems = async () => {
    if (!characterId) return;
    setIsLoading(true);
    try {
      const data = await inventoryAPI.getAllBaseItems(characterId);
      setBaseItems(data.base_items);
    } catch (error) {
      console.error('Failed to load base items:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const categories = useMemo(() => {
    const cats = new Set<string>(['All']);
    baseItems.forEach(item => {
      if (item.category) cats.add(item.category);
    });
    return Array.from(cats);
  }, [baseItems]);

  const filteredItems = useMemo(() => {
    return baseItems.filter(item => {
      const matchesSearch = item.name.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesCategory = selectedCategory === 'All' || item.category === selectedCategory;
      return matchesSearch && matchesCategory;
    });
  }, [baseItems, searchQuery, selectedCategory]);

  const handleSelect = async (baseItemId: number) => {
    setIsSubmitting(true);
    try {
      await onSelect(baseItemId);
      onClose();
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="class-modal-overlay">
      <Card className="class-modal-container max-w-2xl h-[80vh]">
        <CardContent padding="p-0" className="flex flex-col h-full">
          {/* Header */}
          <div className="class-modal-header">
            <div className="class-modal-header-row">
              <h3 className="class-modal-title">Select Item Template</h3>
              <Button onClick={onClose} variant="ghost" size="sm" className="p-1">
                <X className="w-5 h-5" />
              </Button>
            </div>
            
            <div className="flex gap-4 mt-4">
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[rgb(var(--color-text-muted))]" />
                <Input
                  placeholder="Search templates..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10"
                />
              </div>
              <select
                value={selectedCategory}
                onChange={(e) => setSelectedCategory(e.target.value)}
                className="bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded px-3 py-1 outline-none text-sm"
              >
                {categories.map(cat => (
                  <option key={cat} value={cat}>{cat}</option>
                ))}
              </select>
            </div>
          </div>

          {/* Content */}
          <div className="flex-1 overflow-hidden p-4">
            {isLoading ? (
              <div className="flex items-center justify-center h-full">
                <span className="text-[rgb(var(--color-text-muted))]">Loading templates...</span>
              </div>
            ) : (
              <ScrollArea className="h-full pr-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                  {filteredItems.map(item => (
                    <button
                      key={item.id}
                      onClick={() => handleSelect(item.id)}
                      disabled={isSubmitting}
                      className="flex items-center gap-3 p-3 rounded-lg border border-[rgb(var(--color-surface-border))] bg-[rgb(var(--color-surface-2))] hover:bg-[rgb(var(--color-primary)/0.1)] hover:border-[rgb(var(--color-primary))] transition-all text-left disabled:opacity-50"
                    >
                      <div className="w-10 h-10 rounded bg-[rgb(var(--color-surface-3))] flex items-center justify-center text-xs font-bold text-[rgb(var(--color-text-muted))]">
                        {item.name.charAt(0)}
                      </div>
                      <div>
                        <div className="font-medium text-sm text-[rgb(var(--color-text-primary))]">{item.name}</div>
                        <div className="text-xs text-[rgb(var(--color-text-muted))]">{item.category}</div>
                      </div>
                    </button>
                  ))}
                  {filteredItems.length === 0 && (
                    <div className="col-span-full py-10 text-center text-[rgb(var(--color-text-muted))]">
                      No matching templates found.
                    </div>
                  )}
                </div>
              </ScrollArea>
            )}
          </div>

          {/* Footer */}
          <div className="p-4 border-t border-[rgb(var(--color-surface-border))] text-xs text-[rgb(var(--color-text-muted))] flex justify-between">
            <span>{filteredItems.length} templates available</span>
            <span>Select a template to create a new item</span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
