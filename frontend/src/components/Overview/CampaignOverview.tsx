import React, { useState } from 'react';
import { CharacterData } from '@/services/characterApi';
import { display, formatNumber } from '@/utils/dataHelpers';
import { Button } from '@/components/ui/Button';

interface CollapsibleSectionProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  badge?: string | number;
}

function CollapsibleSection({ title, children, defaultOpen = false, badge }: CollapsibleSectionProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen);
  
  return (
    <div className="group">
      <div className={`bg-gradient-to-r ${isOpen ? 'from-[rgb(var(--color-surface-2))] to-[rgb(var(--color-surface-1))]' : 'from-[rgb(var(--color-surface-1))] to-[rgb(var(--color-surface-1))]'} rounded-lg border border-[rgb(var(--color-surface-border)/0.5)] overflow-hidden transition-all duration-300 hover:border-[rgb(var(--color-primary)/0.3)]`}>
        <Button
          onClick={() => setIsOpen(!isOpen)}
          variant="ghost"
          className="w-full p-4 flex items-center justify-between h-auto"
        >
          <div className="flex items-center space-x-3">
            <h3 className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">{title}</h3>
            {badge && (
              <span className="px-2.5 py-1 bg-gradient-to-r from-[rgb(var(--color-primary)/0.15)] to-[rgb(var(--color-primary)/0.1)] text-[rgb(var(--color-primary))] text-xs font-medium rounded-full">
                {badge}
              </span>
            )}
          </div>
          <svg 
            className={`w-5 h-5 text-[rgb(var(--color-text-muted))] transition-all duration-300 ${isOpen ? 'rotate-180' : ''}`}
            fill="none" 
            stroke="currentColor" 
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </Button>
        <div className={`transition-all duration-300 ease-out ${isOpen ? 'max-h-none opacity-100' : 'max-h-0 opacity-0 overflow-hidden'}`}>
          <div className="px-4 pb-4 border-t border-[rgb(var(--color-surface-border)/0.3)]">
            <div className="pt-4">
              {children}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

interface CampaignOverviewProps {
  character: CharacterData;
}

const CampaignOverview: React.FC<CampaignOverviewProps> = ({ character }) => {
  // Helper function to format timestamp
  const formatTimestamp = (timestamp?: number): string => {
    if (!timestamp) return 'Unknown';
    try {
      return new Date(timestamp * 1000).toLocaleString();
    } catch {
      return 'Invalid Date';
    }
  };

  // Helper function to get difficulty name
  const getDifficultyName = (level?: number): string => {
    const difficulties = {
      0: 'Very Easy',
      1: 'Easy', 
      2: 'Normal',
      3: 'Hard',
      4: 'Very Hard'
    };
    return difficulties[level as keyof typeof difficulties] || `Level ${level || 1}`;
  };


  // Helper function to get language name from ID
  const getLanguageName = (languageId?: number): string => {
    const languages = {
      0: 'English',
      1: 'French', 
      2: 'German',
      3: 'Italian',
      4: 'Spanish',
      5: 'Polish',
      6: 'Korean',
      7: 'Chinese Traditional',
      8: 'Chinese Simplified',
      9: 'Japanese'
    };
    return languages[languageId as keyof typeof languages] || character.detectedLanguage || 'English (Default)';
  };

  return (
    <CollapsibleSection 
      title="Campaign Overview" 
      defaultOpen={true}
      badge={character.gameAct ? `Act ${character.gameAct}` : "Campaign"}
    >
      <div className="space-y-6">
        
        {/* General Information */}
        <div>
          <h4 className="font-semibold text-[rgb(var(--color-text-primary))] mb-3 border-b border-[rgb(var(--color-surface-border)/0.6)] pb-1">
            General Information
          </h4>
          <div className="grid grid-cols-2 gap-3">
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Game Act</div>
              <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                {character.gameAct ? `Act ${character.gameAct}` : 'Unknown'}
              </div>
            </div>
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Difficulty</div>
              <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                {getDifficultyName(character.difficultyLevel)}
              </div>
            </div>
            {character.lastSavedTimestamp ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)] col-span-2">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Last Saved</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {formatTimestamp(character.lastSavedTimestamp)}
                </div>
              </div>
            ) : null}
          </div>
        </div>

        {/* Session Information */}
        <div>
          <h4 className="font-semibold text-[rgb(var(--color-text-primary))] mb-3 border-b border-[rgb(var(--color-surface-border)/0.6)] pb-1">
            Session Information
          </h4>
          <div className="grid grid-cols-2 gap-3">
            {character.campaignName ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Campaign</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {display(character.campaignName)}
                </div>
              </div>
            ) : null}
            {character.moduleName ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Module</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {display(character.moduleName)}
                </div>
              </div>
            ) : null}
            {character.location ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Location</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {display(character.location)}
                </div>
              </div>
            ) : null}
            {character.playTime ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Play Time</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {display(character.playTime)}
                </div>
              </div>
            ) : null}
          </div>
        </div>

        {/* Locale & Language */}
        <div>
          <h4 className="font-semibold text-[rgb(var(--color-text-primary))] mb-3 border-b border-[rgb(var(--color-surface-border)/0.6)] pb-1">
            Locale & Language
          </h4>
          <div className="grid grid-cols-2 gap-3">
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Language</div>
              <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                {getLanguageName(character.languageId)}
              </div>
            </div>
            {character.lastSavedTimestamp ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Timezone</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {Intl.DateTimeFormat().resolvedOptions().timeZone}
                </div>
              </div>
            ) : null}
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Localization</div>
              <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                {character.localizationStatus || (character.name ? 'Active' : 'Default')}
              </div>
            </div>
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Region Format</div>
              <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                {Intl.DateTimeFormat().resolvedOptions().locale || 'en-US'}
              </div>
            </div>
            {character.createdAt ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Imported</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {new Date(character.createdAt).toLocaleDateString()}
                </div>
              </div>
            ) : null}
            {character.updatedAt ? (
              <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)]">
                <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Last Modified</div>
                <div className="text-lg font-semibold text-[rgb(var(--color-text-primary))]">
                  {new Date(character.updatedAt).toLocaleDateString()}
                </div>
              </div>
            ) : null}
          </div>
        </div>


        {/* Quest Progress */}
        <div>
          <h4 className="font-semibold text-[rgb(var(--color-text-primary))] mb-3 border-b border-[rgb(var(--color-surface-border)/0.6)] pb-1">
            Quest Progress
          </h4>
          
          {/* Quest Summary Stats */}
          <div className="grid grid-cols-3 gap-3">
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)] text-center">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Completed</div>
              <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
                {formatNumber(character.questDetails?.summary?.completed_quests || 0)}
              </div>
            </div>
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)] text-center">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Active</div>
              <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
                {formatNumber(character.questDetails?.summary?.active_quests || 0)}
              </div>
            </div>
            <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)] text-center">
              <div className="text-xs text-[rgb(var(--color-text-muted))] mb-1">Complete</div>
              <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
                {character.questDetails?.progress_stats?.total_completion_rate || 0}%
              </div>
            </div>
          </div>

        </div>


      </div>
    </CollapsibleSection>
  );
};

export default CampaignOverview;