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
];

// Development items (shown at bottom)
const developmentNavItems: NavItem[] = [
  {
    id: 'character-builder',
    labelKey: 'navigation.characterBuilder',
    icon: 'it_smithhammer_adamantine'
  },
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

export default function Sidebar({ activeTab, onTabChange, isCollapsed: controlledCollapsed, onCollapsedChange, currentCharacter, onBackToMain, isLoading }: SidebarProps) {
  const t = useTranslations();
  const [internalCollapsed, setInternalCollapsed] = useState(false);
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
  
  // Use controlled state if provided, otherwise use internal state
  const isCollapsed = controlledCollapsed !== undefined ? controlledCollapsed : internalCollapsed;
  
  // Load collapsed state from localStorage on mount (only if not controlled)
  useEffect(() => {
    if (controlledCollapsed === undefined) {
      const savedState = localStorage.getItem('sidebar-collapsed');
      if (savedState === 'true') {
        setInternalCollapsed(true);
      }
    }
  }, [controlledCollapsed]);
  
  // Save collapsed state to localStorage when it changes
  useEffect(() => {
    localStorage.setItem('sidebar-collapsed', isCollapsed.toString());
  }, [isCollapsed]);
  
  const toggleSidebar = () => {
    if (onCollapsedChange) {
      onCollapsedChange(!isCollapsed);
    } else {
      setInternalCollapsed(!isCollapsed);
    }
  };
  
  return (
    <div className={`${isCollapsed ? 'w-16' : 'w-60'} bg-[rgb(var(--color-surface-2))] h-full flex flex-col border-r border-[rgb(var(--color-surface-border)/0.6)] shadow-elevation-2 transition-all duration-300`}>
      
      {/* Character Portrait */}
      <div className="p-4 bg-[rgb(var(--color-surface-1))] border-b border-[rgb(var(--color-surface-border)/0.6)]">
        <div className={`mx-auto ${isCollapsed ? '' : 'mb-3'} transition-all duration-300`}>
          <CharacterPortrait
            portrait={currentCharacter?.portrait}
            customPortrait={currentCharacter?.customPortrait}
            size={isCollapsed ? 'sm' : 'lg'}
            className="mx-auto shadow-elevation-1"
          />
        </div>
        {!isCollapsed && (
          <div className="text-center">
            <div className="text-[rgb(var(--color-text-primary))] font-medium">
              {isLoading ? (
                <span className="flex items-center justify-center gap-2">
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-[rgb(var(--color-primary))]"></div>
                  Loading...
                </span>
              ) : (
                currentCharacter?.name || t('character.noCharacter')
              )}
            </div>
            <div className="text-[rgb(var(--color-text-muted))] text-sm">
              {isLoading ? (
                'Please wait...'
              ) : currentCharacter ? (
                currentCharacter.isCompanion ? (
                  <Button 
                    onClick={handleBackToMainClick}
                    variant="ghost"
                    size="sm"
                    className="text-[rgb(var(--color-primary))] hover:underline flex items-center gap-1 mx-auto h-auto p-0"
                  >
                    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10 19l-7-7m0 0l7-7m-7 7h18" />
                    </svg>
                    Back to Main
                  </Button>
                ) : (
                  'Main Character'
                )
              ) : (
                t('character.loadSaveFile')
              )}
            </div>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 py-2 relative">
        {/* Toggle Button */}
        <Button
          onClick={toggleSidebar}
          variant="ghost"
          size="icon"
          className="absolute -right-3 top-[15%] -translate-y-1/2 w-6 h-6 bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border)/0.6)] rounded-full hover:bg-[rgb(var(--color-surface-1))] z-10 shadow-elevation-2"
        >
          <svg className="w-3 h-3 text-[rgb(var(--color-text-secondary))]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d={isCollapsed ? "M9 5l7 7-7 7" : "M15 19l-7-7 7-7"} />
          </svg>
        </Button>
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
            title={isCollapsed ? t(item.labelKey) : ''}
          >
            <div className="flex items-center gap-3">
              {item.icon && (
                <NWN2Icon 
                  icon={item.icon} 
                  size="sm" 
                  className="flex-shrink-0"
                />
              )}
              {!isCollapsed && (
                <span className="text-sm font-medium">{t(item.labelKey)}</span>
              )}
            </div>
            {isCollapsed && (
              <div className="absolute left-full ml-2 px-2 py-1 bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-primary))] text-sm rounded-md opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity whitespace-nowrap z-50 shadow-elevation-3">
                {t(item.labelKey)}
              </div>
            )}
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
              title={isCollapsed ? `${t(item.labelKey)} (IN DEV)` : ''}
            >
              <div className="flex items-center gap-3">
                {item.icon && (
                  <NWN2Icon 
                    icon={item.icon} 
                    size="sm" 
                    className="flex-shrink-0"
                  />
                )}
                {!isCollapsed && (
                  <span className="text-sm font-medium">
                    {t(item.labelKey)} <span className="text-xs text-amber-600">(IN DEV)</span>
                  </span>
                )}
              </div>
              {isCollapsed && (
                <div className="absolute left-full ml-2 px-2 py-1 bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-primary))] text-sm rounded-md opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity whitespace-nowrap z-50 shadow-elevation-3">
                  {t(item.labelKey)} <span className="text-amber-400">(IN DEV)</span>
                </div>
              )}
            </Button>
          ))}
        </div>
      </nav>
      
      {/* Settings Button (Always at bottom) */}
      <div className="px-2 py-2 border-t border-[rgb(var(--color-surface-border)/0.6)]">
        <Button
          key={settingsItem.id}
          onClick={() => onTabChange(settingsItem.id)}
          variant="ghost"
          className={`w-full px-4 py-2.5 text-left h-auto justify-start transition-all ${
            activeTab === settingsItem.id
              ? 'bg-[rgb(var(--color-primary)/0.1)] border-l-4 border-[rgb(var(--color-primary))] text-[rgb(var(--color-primary))] font-medium'
              : 'hover:bg-[rgb(var(--color-surface-1))] text-[rgb(var(--color-text-secondary))] hover:text-[rgb(var(--color-text-primary))] border-l-4 border-transparent'
          } relative group`}
          title={isCollapsed ? t(settingsItem.labelKey) : ''}
        >
          <div className="flex items-center gap-3">
            {settingsItem.icon && (
              <NWN2Icon 
                icon={settingsItem.icon} 
                size="sm" 
                className="flex-shrink-0"
              />
            )}
            {!isCollapsed && (
              <span className="text-sm font-medium">{t(settingsItem.labelKey)}</span>
            )}
          </div>
          {isCollapsed && (
            <div className="absolute left-full ml-2 px-2 py-1 bg-[rgb(var(--color-surface-3))] text-[rgb(var(--color-text-primary))] text-sm rounded-md opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity whitespace-nowrap z-50 shadow-elevation-3">
              {t(settingsItem.labelKey)}
            </div>
          )}
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