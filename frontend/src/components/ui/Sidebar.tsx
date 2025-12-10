'use client';

import { useState, useEffect } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { useTranslations } from '@/hooks/useTranslations';
import CharacterPortrait from '@/components/ui/CharacterPortrait';
import NWN2Icon from '@/components/ui/NWN2Icon';

type NavItem = {
  id: string;
  labelKey: string;
  icon?: string;
};

// Base navigation items for all characters
const baseNavItems: NavItem[] = [
  {
    id: 'overview',
    labelKey: 'navigation.overview',
    icon: 'b_character'
  },
  {
    id: 'abilityScores',
    labelKey: 'navigation.abilityScores',
    icon: 'ia_appear'
  },
  {
    id: 'classes',
    labelKey: 'navigation.classes',
    icon: 'ife_constructac'
  },
  {
    id: 'skills',
    labelKey: 'navigation.skills',
    icon: 'ia_contempabilities'
  },
  {
    id: 'feats',
    labelKey: 'navigation.feats',
    icon: 'ife_expertise'
  },
  {
    id: 'spells',
    labelKey: 'navigation.spells',
    icon: 'ia_spells'
  },
  {
    id: 'inventory',
    labelKey: 'navigation.inventory',
    icon: 'b_inventory'
  },
  {
    id: 'gameState',
    labelKey: 'navigation.gameState',
    icon: 'b_journal'
  },
];

// Development items (shown at bottom)
const developmentNavItems: NavItem[] = [
  {
    id: 'appearance',
    labelKey: 'navigation.appearance',
    icon: 'ia_appear'
  },
  {
    id: 'companions',
    labelKey: 'navigation.companions',
    icon: 'ia_partycommands'
  },
];

// Additional navigation items only for companions
const companionOnlyNavItems: NavItem[] = [
  {
    id: 'influence',
    labelKey: 'navigation.influence',
    icon: 'is_influence'
  },
  {
    id: 'ai-settings',
    labelKey: 'navigation.aiSettings',
    icon: 'iit_misc_007'
  },
];

// Settings item (always shown at bottom)
const settingsItem: NavItem = {
  id: 'settings',
  labelKey: 'navigation.settings',
  icon: 'b_options'
};

interface SidebarProps {
  activeTab: string;
  onTabChange: (tabId: string) => void;
  isCollapsed?: boolean;
  onCollapsedChange?: (collapsed: boolean) => void;
  currentCharacter?: {
    name: string;
    portrait?: string;
    customPortrait?: string;
    isCompanion?: boolean;
  } | null;
  onBackToMain?: () => void;
  isLoading?: boolean;
}

export default function Sidebar({ activeTab, onTabChange, currentCharacter, onBackToMain, isLoading }: SidebarProps) {
  const t = useTranslations();
  const [showBackToMainDialog, setShowBackToMainDialog] = useState(false);
  
  // TODO: Get this from character provider when available
  const hasUnsavedChanges = true; // temporarily true for testing
  
  // Determine which nav items to show based on character type
  const navItems = currentCharacter?.isCompanion 
    ? [...baseNavItems, ...companionOnlyNavItems]
    : baseNavItems;
    
  const handleBackToMainClick = () => {
    if (hasUnsavedChanges) {
      setShowBackToMainDialog(true);
    } else {
      onBackToMain?.();
    }
  };
  
  const confirmBackToMain = () => {
    setShowBackToMainDialog(false);
    onBackToMain?.();
  };
  
  return (
    <div className="w-56 bg-[rgb(var(--color-surface-2))] h-full flex flex-col border-r border-[rgb(var(--color-surface-border)/0.6)] shadow-elevation-2">
      
      {/* Navigation */}
      <nav className="flex-1 py-2 relative mt-4 overflow-y-auto">
        {navItems.map((item) => (
          <Button
            key={item.id}
            onClick={() => onTabChange(item.id)}
            variant="ghost"
            className={`w-full px-4 py-2.5 text-left h-auto justify-start transition-all ${
              activeTab === item.id
                ? 'bg-[rgb(var(--color-primary)/0.1)] border-l-4 border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] font-medium'
                : 'hover:bg-[rgb(var(--color-surface-1))] text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))] border-l-4 border-transparent'
            } relative group`}
          >
            <div className="flex items-center gap-3">
              {item.icon && (
                <NWN2Icon 
                  icon={item.icon} 
                  size="sm" 
                  className="flex-shrink-0"
                />
              )}
              <span className="text-sm font-medium">{t(item.labelKey)}</span>
            </div>
          </Button>
        ))}
        
        {/* Development items */}
        <div className="mt-4 pt-2 border-t border-[rgb(var(--color-surface-border)/0.6)]">
          {developmentNavItems.map((item) => (
            <Button
              key={item.id}
              onClick={() => onTabChange(item.id)}
              variant="ghost"
              className={`w-full px-4 py-2.5 text-left h-auto justify-start transition-all ${
                activeTab === item.id
                  ? 'bg-[rgb(var(--color-primary)/0.1)] border-l-4 border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] font-medium'
                  : 'hover:bg-[rgb(var(--color-surface-1))] text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))] border-l-4 border-transparent'
              } relative group`}
              title={`${t(item.labelKey)} (IN DEV)`}
            >
              <div className="flex items-center gap-3">
                {item.icon && (
                  <NWN2Icon 
                    icon={item.icon} 
                    size="sm" 
                    className="flex-shrink-0"
                  />
                )}
                <span className="text-sm font-medium">
                  {t(item.labelKey)} <span className="text-xs text-amber-600">(IN DEV)</span>
                </span>
              </div>
            </Button>
          ))}
        </div>
      </nav>
      
      {/* Footer (Settings) */}
      <div className="p-2 border-t border-[rgb(var(--color-surface-border)/0.6)]">
        <Button
          key={settingsItem.id}
          onClick={() => onTabChange(settingsItem.id)}
          variant="ghost"
          className={`w-full px-4 py-2.5 text-left h-auto justify-start transition-all ${
            activeTab === settingsItem.id
              ? 'bg-[rgb(var(--color-primary)/0.1)] border-l-4 border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] font-medium'
              : 'hover:bg-[rgb(var(--color-surface-1))] text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))] border-l-4 border-transparent'
          } relative group`}
        >
          <div className="flex items-center gap-3">
            {settingsItem.icon && (
              <NWN2Icon 
                icon={settingsItem.icon} 
                size="sm" 
                className="flex-shrink-0"
              />
            )}
            <span className="text-sm font-medium">{t(settingsItem.labelKey)}</span>
          </div>
        </Button>
      </div>
      
      {/* Confirmation Dialog for Back to Main */}
      {showBackToMainDialog && (
        <div className="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
          <Card className="max-w-md">
            <CardHeader>
              <CardTitle>Unsaved Changes</CardTitle>
              <CardDescription>
                You have unsaved changes to {currentCharacter?.name}. Going back to the main character will discard these changes. Do you want to continue?
              </CardDescription>
            </CardHeader>
            <CardContent className="flex gap-2 justify-end">
              <Button variant="outline" onClick={() => setShowBackToMainDialog(false)}>Cancel</Button>
              <Button onClick={confirmBackToMain}>Back to Main</Button>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}