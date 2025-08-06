import React from 'react';
import { Flame, Shield, Sparkles, BookOpen, Star, Zap, Eye, Skull, Search, Info, Grid, List } from 'lucide-react';

export interface SpellSection {
  id: string;
  title: string;
  component: 'filters' | 'header' | 'spellList' | 'spellSlots';
  props?: {
    showResetButton?: boolean;
    defaultExpandedLevels?: number[];
  };
  className?: string;
}

export interface FilterConfig {
  id: string;
  type: 'search' | 'checkbox' | 'select';
  label: string;
  placeholder?: string;
  options?: Array<{value: string; label: string}>;
  icon?: React.ReactNode;
}

export const spellFilterConfigs: FilterConfig[] = [
  {
    id: 'search',
    type: 'search',
    label: 'Search',
    placeholder: 'Search spells...',
    icon: <Search className="w-4 h-4" />
  },
  {
    id: 'onlyLearned',
    type: 'checkbox',
    label: 'Learned Only'
  },
  {
    id: 'onlyFavorites',
    type: 'checkbox',
    label: 'Favorites Only'
  },
  {
    id: 'level',
    type: 'select',
    label: 'Spell Level',
    options: [
      { value: 'all', label: 'All Levels' },
      { value: '0', label: 'Cantrips' },
      { value: '1', label: 'Level 1' },
      { value: '2', label: 'Level 2' },
      { value: '3', label: 'Level 3' },
      { value: '4', label: 'Level 4' },
      { value: '5', label: 'Level 5' },
      { value: '6', label: 'Level 6' },
      { value: '7', label: 'Level 7' },
      { value: '8', label: 'Level 8' },
      { value: '9', label: 'Level 9' }
    ]
  },
  {
    id: 'school',
    type: 'select',
    label: 'Schools',
    options: [] // Will be populated dynamically
  }
];

export const spellSections: SpellSection[] = [
  {
    id: 'filters',
    title: 'Spell Filters',
    component: 'filters',
    className: 'w-80',
    props: {
      showResetButton: true
    }
  },
  {
    id: 'header',
    title: 'Spell Header',
    component: 'header',
    className: 'flex-1'
  },
  {
    id: 'spellList',
    title: 'Spell List',
    component: 'spellList',
    className: 'flex-1',
    props: {
      defaultExpandedLevels: [0, 1]
    }
  }
];

export const spellSlotConfig = {
  title: 'Spell Slots',
  icon: <Info className="w-4 h-4 text-muted" />,
  showProgress: true
};

export const viewModeConfig = {
  modes: [
    { id: 'grid', icon: <Grid className="w-4 h-4" />, label: 'Grid View' },
    { id: 'list', icon: <List className="w-4 h-4" />, label: 'List View' }
  ],
  defaultMode: 'list'
};

export const schoolIcons: Record<string, React.ReactNode> = {
  'Evocation': <Flame className="w-5 h-5" />,
  'Abjuration': <Shield className="w-5 h-5" />,
  'Conjuration': <Sparkles className="w-5 h-5" />,
  'Divination': <BookOpen className="w-5 h-5" />,
  'Enchantment': <Star className="w-5 h-5" />,
  'Transmutation': <Zap className="w-5 h-5" />,
  'Illusion': <Eye className="w-5 h-5" />,
  'Necromancy': <Skull className="w-5 h-5" />,
  'Universal': <Sparkles className="w-5 h-5" />
};

export function getSchoolIcon(school: string, size: 'sm' | 'md' | 'lg' = 'md'): React.ReactNode {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-5 h-5',
    lg: 'w-6 h-6'
  };
  
  const Icon = schoolIcons[school];
  if (!Icon) return <Sparkles className={sizeClasses[size]} />;
  
  // Clone the icon with the appropriate size class
  if (React.isValidElement(Icon)) {
    return React.cloneElement(Icon as React.ReactElement<{className?: string}>, {
      className: sizeClasses[size]
    });
  }
  
  return Icon;
}