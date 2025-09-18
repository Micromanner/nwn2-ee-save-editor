import React, { useState, ReactNode } from 'react';
import { ChevronRight } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { ScrollArea } from '@/components/ui/ScrollArea';
import { Skeleton } from '@/components/ui/skeleton';
import { display } from '@/utils/dataHelpers';

export interface GameDataItem {
  id: number;
  name: string;
  icon?: string;
  icon_url?: string;
  description?: string;
  level?: number;
  isActive?: boolean; // For learned spells, taken feats, etc.
}

export interface GameDataMeta {
  label: string;
  value: string;
}

export interface GameDataTag {
  label: string;
  variant?: 'primary' | 'secondary';
}

export interface GameDataColumn {
  type: 'main' | 'details' | 'tags' | 'action';
  render: (item: GameDataItem) => ReactNode;
}

export interface GameDataListProps<T extends GameDataItem> {
  items: T[];
  isLoading?: boolean;
  groupBy?: (item: T) => string | number;
  groupLabels?: Record<string | number, string>;
  defaultExpandedGroups?: (string | number)[];
  onItemAction?: (item: T) => void;
  actionLabel?: (item: T) => string;
  actionVariant?: (item: T) => 'primary' | 'outline' | 'secondary';
  
  // Table header configuration
  showHeader?: boolean;
  headerLabels?: {
    main?: string;
    level?: string;
    range?: string;
    duration?: string;
    save?: string;
    tags?: string;
    action?: string;
  };
  
  // Column customization
  renderMain?: (item: T) => ReactNode;
  renderDetails?: (item: T) => ReactNode;
  renderTags?: (item: T) => ReactNode;
  renderAction?: (item: T) => ReactNode;
  
  // Additional column renderers for table layout
  renderLevel?: (item: T) => ReactNode;
  renderRange?: (item: T) => ReactNode;
  renderDuration?: (item: T) => ReactNode;
  renderSave?: (item: T) => ReactNode;
  
  // Additional customization
  getItemClassName?: (item: T) => string;
  emptyMessage?: string;
}

export default function GameDataList<T extends GameDataItem>({
  items,
  isLoading = false,
  groupBy,
  groupLabels = {},
  defaultExpandedGroups = [],
  onItemAction,
  actionLabel = (item) => item.isActive ? 'Remove' : 'Add',
  actionVariant = (item) => item.isActive ? 'primary' : 'outline',
  showHeader = false,
  headerLabels = {},
  renderMain,
  renderDetails: _renderDetails, // eslint-disable-line @typescript-eslint/no-unused-vars
  renderTags,
  renderAction,
  renderLevel,
  renderRange,
  renderDuration,
  renderSave,
  getItemClassName,
  emptyMessage = 'No items found'
}: GameDataListProps<T>) {
  
  const [expandedGroups, setExpandedGroups] = useState<Set<string | number>>(
    new Set(defaultExpandedGroups)
  );

  const toggleGroup = (group: string | number) => {
    const newExpanded = new Set(expandedGroups);
    if (newExpanded.has(group)) {
      newExpanded.delete(group);
    } else {
      newExpanded.add(group);
    }
    setExpandedGroups(newExpanded);
  };

  // Default renderers
  const defaultRenderMain = (item: T) => (
    <div className="game-data-col-main">
      {/* <div className="game-data-icon-wrapper">
        <NWN2Icon icon={item.icon || ''} iconUrl={item.icon_url} size="sm" />
      </div> */}
      <div className="game-data-content">
        <h4 className="game-data-title">
          {item.name}
        </h4>
        {item.level !== undefined && (
          <div className="game-data-subtitle">
            <span>Level {item.level === 0 ? 'Cantrip' : item.level}</span>
          </div>
        )}
      </div>
    </div>
  );

  const _defaultRenderDetails = (item: T) => ( // eslint-disable-line @typescript-eslint/no-unused-vars
    <div className="game-data-col-details">
      {item.description && (
        <p className="game-data-description">
          {display(item.description)}
        </p>
      )}
    </div>
  );

  const defaultRenderAction = (item: T) => (
    <div className="game-data-col-action">
      <Button
        variant={actionVariant(item)}
        size="sm"
        onClick={() => onItemAction?.(item)}
      >
        {actionLabel(item)}
      </Button>
    </div>
  );

  const renderHeader = () => {
    if (!showHeader) return null;
    
    return (
      <div className="game-data-header">
        {/* Main Column Header */}
        <div className="game-data-header-cell">
          {headerLabels.main || 'Name'}
        </div>
        
        {/* Level Column Header */}
        {renderLevel && (
          <div className="game-data-header-cell game-data-col-level">
            {headerLabels.level || 'Level'}
          </div>
        )}
        
        {/* Range Column Header */}
        {renderRange && (
          <div className="game-data-header-cell game-data-col-range">
            {headerLabels.range || 'Range'}
          </div>
        )}
        
        {/* Duration Column Header */}
        {renderDuration && (
          <div className="game-data-header-cell game-data-col-duration">
            {headerLabels.duration || 'Duration'}
          </div>
        )}
        
        {/* Save Column Header */}
        {renderSave && (
          <div className="game-data-header-cell game-data-col-save">
            {headerLabels.save || 'Save'}
          </div>
        )}
        
        {/* Tags Column Header */}
        {renderTags && (
          <div className="game-data-header-cell">
            {headerLabels.tags || 'Tags'}
          </div>
        )}
        
        {/* Action Column Header */}
        {onItemAction && (
          <div className="game-data-header-cell">
            {headerLabels.action || 'Action'}
          </div>
        )}
      </div>
    );
  };

  const renderItem = (item: T, index: number) => {
    const itemClassName = getItemClassName ? getItemClassName(item) : '';
    const baseClassName = `game-data-list-item ${item.isActive ? 'active' : ''} ${itemClassName}`.trim();
    
    // Create a more unique key using multiple properties
    const uniqueKey = `${item.id}-${item.name?.replace(/\s+/g, '-')}-${index}`;

    return (
      <div key={uniqueKey} className={baseClassName}>
        <div className="game-data-grid">
          {/* Main Column */}
          {renderMain ? renderMain(item) : defaultRenderMain(item)}
          
          {/* Level Column */}
          {renderLevel && renderLevel(item)}
          
          {/* Range Column */}
          {renderRange && renderRange(item)}
          
          {/* Duration Column */}
          {renderDuration && renderDuration(item)}
          
          {/* Save Column */}
          {renderSave && renderSave(item)}
          
          {/* Tags/Classes Column */}
          {renderTags && renderTags(item)}
          
          {/* Action Column */}
          {onItemAction && (renderAction ? renderAction(item) : defaultRenderAction(item))}
        </div>
      </div>
    );
  };

  if (isLoading) {
    return (
      <Card className="flex-1" padding="p-0">
        <ScrollArea className="h-full p-4">
          <div className="space-y-4">
            {[...Array(5)].map((_, i) => (
              <Skeleton key={i} className="h-16 w-full" />
            ))}
          </div>
        </ScrollArea>
      </Card>
    );
  }

  if (items.length === 0) {
    return (
      <Card className="flex-1" padding="p-0">
        <ScrollArea className="h-full p-4">
          <div className="text-center py-8 text-muted">
            {emptyMessage}
          </div>
        </ScrollArea>
      </Card>
    );
  }

  // Group items if groupBy function is provided
  const groupedItems = groupBy 
    ? items.reduce((acc, item) => {
        const group = groupBy(item);
        if (!acc[group]) {
          acc[group] = [];
        }
        acc[group].push(item);
        return acc;
      }, {} as Record<string | number, T[]>)
    : { 'all': items };

  return (
    <Card className="flex-1" padding="p-0">
      <ScrollArea className="h-full">
        {/* Table Header */}
        {renderHeader()}
        
        <div className="p-4">
          <div className="space-y-0">
            {Object.entries(groupedItems)
            .sort(([a], [b]) => {
              const numA = Number(a);
              const numB = Number(b);
              // Handle special cases (like -1 for unavailable)
              if (numA === -1 && numB !== -1) return 1;
              if (numB === -1 && numA !== -1) return -1;
              // Numeric sort
              if (!isNaN(numA) && !isNaN(numB)) return numA - numB;
              // String sort
              return String(a).localeCompare(String(b));
            })
            .map(([group, groupItems]) => (
              <div key={group} className="game-data-section">
                {groupBy && (
                  <div 
                    className="game-data-level-header"
                    onClick={() => toggleGroup(group)}
                  >
                    <div className="game-data-level-title">
                      <div className={`game-data-level-chevron ${expandedGroups.has(group) ? 'expanded' : ''}`}>
                        <ChevronRight className="w-4 h-4" />
                      </div>
                      {groupLabels[group] || `Level ${group}`}
                      <Badge variant="secondary" className="game-data-level-count">
                        {groupItems.filter(item => item.isActive).length}/{groupItems.length}
                      </Badge>
                    </div>
                  </div>
                )}
                
                {(!groupBy || expandedGroups.has(group)) && (
                  <div>
                    {groupItems.map((item, index) => {
                      // Calculate a global index across all groups for truly unique keys
                      const globalIndex = Object.entries(groupedItems)
                        .slice(0, Object.keys(groupedItems).indexOf(group))
                        .reduce((acc, [, items]) => acc + items.length, 0) + index;
                      return renderItem(item, globalIndex);
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      </ScrollArea>
    </Card>
  );
}