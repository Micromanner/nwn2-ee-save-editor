'use client';

import React, { useState, useEffect, useMemo } from 'react';
import { Card, CardContent } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/Tabs';
import { FixedSizeList as List } from 'react-window';
import { ItemTemplate, ITEM_CATEGORIES } from '@/services/inventoryApi';
import { useTranslations } from '@/hooks/useTranslations';
import { display } from '@/utils/dataHelpers';

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

interface AddItemModalProps {
  isOpen: boolean;
  onClose: () => void;
  onAddBaseItem: (baseItemId: number) => Promise<number | null>;
  onAddTemplate: (resref: string) => void;
  baseItems: { id: number; name: string; category?: string }[];
  templates: ItemTemplate[];
  isLoadingTemplates: boolean;
}

const useContainerDimensions = (myRef: React.RefObject<HTMLDivElement | null>) => {
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const getDimensions = () => ({
      width: myRef.current?.offsetWidth || 0,
      height: myRef.current?.offsetHeight || 0
    });

    const handleResize = () => {
      setDimensions(getDimensions());
    };

    if (myRef.current) {
      setDimensions(getDimensions());
    }

    window.addEventListener("resize", handleResize);

    const observer = new ResizeObserver(() => {
      setDimensions(getDimensions());
    });
    if (myRef.current) {
      observer.observe(myRef.current);
    }

    return () => {
      window.removeEventListener("resize", handleResize);
      observer.disconnect();
    };
  }, [myRef]);

  return dimensions;
};

const InnerList = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(({ style, ...rest }, ref) => (
  <div
    ref={ref}
    style={{
      ...style,
      height: `${(typeof style?.height === 'number' ? style.height : parseFloat(style?.height as string || '0')) + 16}px`,
    }}
    {...rest}
  />
));
InnerList.displayName = "InnerList";

export default function AddItemModal({
  isOpen,
  onClose,
  onAddBaseItem,
  onAddTemplate,
  baseItems,
  templates,
  isLoadingTemplates
}: AddItemModalProps) {
  const t = useTranslations();
  const [activeTab, setActiveTab] = useState<'custom' | 'template'>('custom');
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedBaseId, setSelectedBaseId] = useState<number | null>(null);
  const [selectedTemplateResref, setSelectedTemplateResref] = useState<string | null>(null);
  const [isAdding, setIsAdding] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState<number | null>(null);
  const [selectedBaseCategory, setSelectedBaseCategory] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen) {
      setSearchQuery('');
      setSelectedBaseId(null);
      setSelectedTemplateResref(null);
      setIsAdding(false);
      setSelectedCategory(null);
      setSelectedBaseCategory(null);
    }
  }, [isOpen]);

  const baseCategories = useMemo(() => {
    const BASE_CATEGORY_ORDER = ['Armor & Clothing', 'Weapons', 'Magic Items', 'Accessories', 'Miscellaneous'];
    const presentCategories = new Set<string>();
    baseItems.forEach(item => {
      if (item.category) presentCategories.add(item.category);
    });
    return BASE_CATEGORY_ORDER.filter(cat => presentCategories.has(cat));
  }, [baseItems]);

  const filteredBaseItems = useMemo(() => {
    let filtered = baseItems;

    if (selectedBaseCategory !== null) {
      filtered = filtered.filter(item => item.category === selectedBaseCategory);
    }

    if (searchQuery) {
      const lowerQuery = searchQuery.toLowerCase();
      filtered = filtered.filter(item =>
        item.name.toLowerCase().includes(lowerQuery) ||
        item.category?.toLowerCase().includes(lowerQuery)
      );
    }

    return filtered;
  }, [baseItems, searchQuery, selectedBaseCategory]);

  const filteredTemplates = useMemo(() => {
    let filtered = templates;

    if (selectedCategory !== null) {
      filtered = filtered.filter(tmpl => tmpl.category === selectedCategory);
    }

    if (searchQuery) {
      const lowerQuery = searchQuery.toLowerCase();
      filtered = filtered.filter(tmpl =>
        tmpl.name.toLowerCase().includes(lowerQuery) ||
        tmpl.resref.toLowerCase().includes(lowerQuery)
      );
    }

    return filtered;
  }, [templates, searchQuery, selectedCategory]);

  const listContainerRef = React.useRef<HTMLDivElement>(null);
  const { height: listHeight, width: listWidth } = useContainerDimensions(listContainerRef);

  const handleAdd = async () => {
    if (isAdding) return;

    if (activeTab === 'custom' && selectedBaseId !== null) {
      setIsAdding(true);
      try {
        await onAddBaseItem(selectedBaseId);
        onClose();
      } finally {
        setIsAdding(false);
      }
    } else if (activeTab === 'template' && selectedTemplateResref) {
      onAddTemplate(selectedTemplateResref);
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div className="add-item-modal-overlay">
      <Card className="add-item-modal-container">
        <CardContent padding="p-0" className="flex flex-col h-full">
          <div className="add-item-modal-header">
            <div className="add-item-modal-header-row">
              <h3 className="add-item-modal-title">
                {t('inventory.addItem')}
              </h3>
              <Button
                onClick={onClose}
                variant="ghost"
                size="sm"
                className="add-item-modal-close-button"
              >
                <X className="w-4 h-4" />
              </Button>
            </div>
          </div>

          <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as 'custom' | 'template')} className="flex-1 flex flex-col overflow-hidden">
            <div className="add-item-modal-tabs">
              <TabsList className="w-full flex bg-transparent p-0 gap-2">
                <TabsTrigger
                  value="custom"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  {t('inventory.createNewBaseType')}
                </TabsTrigger>
                <TabsTrigger
                  value="template"
                  className="flex-1 h-10 rounded-md border border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] bg-transparent data-[state=active]:!bg-[rgb(var(--color-primary))] data-[state=active]:!text-white transition-colors hover:bg-[rgb(var(--color-primary))/10]"
                >
                  {t('inventory.addExistingTemplate')}
                </TabsTrigger>
              </TabsList>
            </div>

            <div className="add-item-modal-search">
              <Search className="add-item-modal-search-icon" />
              <Input
                placeholder={activeTab === 'custom' ? t('inventory.searchBaseItems') : t('inventory.searchTemplates')}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="add-item-modal-search-input"
                autoFocus
              />
            </div>

            <div className="add-item-modal-content">
              <TabsContent value="custom" className="add-item-modal-tab-content">
                {isLoadingTemplates ? (
                  <div className="add-item-modal-loading">
                    <div className="w-8 h-8 rounded-full border-2 border-[rgb(var(--color-primary)/0.2)] border-t-[rgb(var(--color-primary))] animate-spin" />
                    <span>{t('inventory.loadingBaseItems')}</span>
                  </div>
                ) : (
                  <>
                    <div className="add-item-modal-category-filter">
                      <Button
                        variant={selectedBaseCategory === null ? 'primary' : 'outline'}
                        size="sm"
                        onClick={() => setSelectedBaseCategory(null)}
                      >
                        {t('common.all')} ({baseItems.length})
                      </Button>
                      {baseCategories.map((category) => {
                        const count = baseItems.filter(item => item.category === category).length;
                        return (
                          <Button
                            key={category}
                            variant={selectedBaseCategory === category ? 'primary' : 'outline'}
                            size="sm"
                            onClick={() => setSelectedBaseCategory(category)}
                          >
                            {category} ({count})
                          </Button>
                        );
                      })}
                    </div>

                    <div className="add-item-modal-list">
                      {filteredBaseItems.map((item) => (
                        <div
                          key={item.id}
                          onClick={() => setSelectedBaseId(item.id)}
                          className={`add-item-modal-list-item ${selectedBaseId === item.id ? 'selected' : ''}`}
                        >
                          <div className="add-item-modal-item-info">
                            <span className="add-item-modal-item-name">{display(item.name)}</span>
                            <span className="add-item-modal-item-category">{display(item.category)}</span>
                          </div>
                        </div>
                      ))}
                      {filteredBaseItems.length === 0 && (
                        <div className="add-item-modal-empty">
                          {t('inventory.noBaseItemsFound')}
                        </div>
                      )}
                    </div>
                  </>
                )}
              </TabsContent>

              <TabsContent value="template" className="add-item-modal-tab-content">
                {isLoadingTemplates ? (
                  <div className="add-item-modal-loading">
                    <div className="w-8 h-8 rounded-full border-2 border-[rgb(var(--color-primary)/0.2)] border-t-[rgb(var(--color-primary))] animate-spin" />
                    <span>{t('inventory.loadingTemplates')}</span>
                  </div>
                ) : (
                  <>
                    <div className="add-item-modal-category-filter">
                      <Button
                        variant={selectedCategory === null ? 'primary' : 'outline'}
                        size="sm"
                        onClick={() => setSelectedCategory(null)}
                      >
                        {t('common.all')} ({templates.length})
                      </Button>
                      {Object.entries(ITEM_CATEGORIES).map(([catId, catName]) => {
                        const count = templates.filter(t => t.category === Number(catId)).length;
                        return (
                          <Button
                            key={catId}
                            variant={selectedCategory === Number(catId) ? 'primary' : 'outline'}
                            size="sm"
                            onClick={() => setSelectedCategory(Number(catId))}
                          >
                            {catName} ({count})
                          </Button>
                        );
                      })}
                    </div>

                    <div className="add-item-modal-virtualized-list" ref={listContainerRef}>
                      {filteredTemplates.length > 0 ? (
                        <List
                          height={listHeight || 400}
                          itemCount={filteredTemplates.length}
                          itemSize={72}
                          width={listWidth || '100%'}
                          innerElementType={InnerList}
                        >
                          {({ index, style }) => {
                            const item = filteredTemplates[index];
                            return (
                              <div
                                style={{
                                  ...style,
                                  top: (parseFloat(String(style.top)) || 0) + 8,
                                  left: (parseFloat(String(style.left)) || 0) + 8,
                                  width: typeof style.width === 'number'
                                    ? style.width - 16
                                    : 'calc(100% - 16px)',
                                  height: (parseFloat(String(style.height)) || 72) - 4
                                }}
                                onClick={() => setSelectedTemplateResref(item.resref)}
                                className={`add-item-modal-list-item ${selectedTemplateResref === item.resref ? 'selected' : ''}`}
                              >
                                <div className="add-item-modal-item-info">
                                  <span className="add-item-modal-item-name" title={item.resref}>
                                    {display(item.name)}
                                  </span>
                                  <span className="add-item-modal-item-category">
                                    {ITEM_CATEGORIES[item.category as keyof typeof ITEM_CATEGORIES] || 'Unknown'} - {item.source}
                                  </span>
                                </div>
                              </div>
                            );
                          }}
                        </List>
                      ) : (
                        <div className="add-item-modal-empty">
                          {t('inventory.noTemplatesFound')} {searchQuery && `"${searchQuery}"`}
                        </div>
                      )}
                    </div>
                  </>
                )}
              </TabsContent>
            </div>
          </Tabs>

          <div className="add-item-modal-footer">
            <div className="add-item-modal-footer-info">
              {activeTab === 'custom'
                ? `${t('inventory.selectedId')}: ${selectedBaseId !== null ? selectedBaseId : t('common.none')}`
                : `${t('inventory.selectedResref')}: ${selectedTemplateResref || t('common.none')}`
              }
            </div>
            <div className="add-item-modal-footer-actions">
              <Button variant="outline" onClick={onClose} disabled={isAdding}>
                {t('actions.cancel')}
              </Button>
              <Button
                onClick={handleAdd}
                disabled={isAdding || (activeTab === 'custom' ? selectedBaseId === null : !selectedTemplateResref)}
                loading={isAdding}
              >
                {t('inventory.addItem')}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
