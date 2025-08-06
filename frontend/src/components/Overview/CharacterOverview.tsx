'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import Image from 'next/image';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { display, formatModifier, formatNumber } from '@/utils/dataHelpers';
import { CharacterAPI } from '@/services/characterApi';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import CampaignOverview from './CampaignOverview';

interface CharacterOverviewProps {
  onNavigate?: (tab: string) => void;
}

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
        <div className={`transition-all duration-300 ease-out ${isOpen ? 'max-h-[1000px] opacity-100' : 'max-h-0 opacity-0 overflow-hidden'}`}>
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

export default function CharacterOverview({ onNavigate }: CharacterOverviewProps) {
  const t = useTranslations();
  const { character, isLoading, error, refreshAll } = useCharacterContext();
  const combat = useSubsystem('combat');
  const skills = useSubsystem('skills');
  const feats = useSubsystem('feats');
  
  // Name editing state
  const [isEditingName, setIsEditingName] = useState(false);
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  
  // Initialize name fields when character changes
  useEffect(() => {
    if (character && character.name) {
      const parts = character.name.split(' ');
      setFirstName(parts[0] || '');
      setLastName(parts.slice(1).join(' ') || '');
    }
  }, [character]);
  
  // Handle name save
  const handleSaveName = async () => {
    if (!character?.id || isSaving) return;
    
    setIsSaving(true);
    try {
      await CharacterAPI.updateCharacter(character.id, {
        first_name: firstName.trim(),
        last_name: lastName.trim()
      });
      
      // Reload character to reflect changes
      if (refreshAll) {
        await refreshAll();
      }
      
      setIsEditingName(false);
    } catch (error) {
      console.error('Failed to save character name:', error);
      // TODO: Show error notification
    } finally {
      setIsSaving(false);
    }
  };
  
  // Handle cancel edit
  const handleCancelEdit = () => {
    if (character && character.name) {
      const parts = character.name.split(' ');
      setFirstName(parts[0] || '');
      setLastName(parts.slice(1).join(' ') || '');
    }
    setIsEditingName(false);
  };
  
  // Load subsystems only if character exists and data hasn't been loaded
  useEffect(() => {
    if (character) {
      if (!combat.data && !combat.isLoading) {
        combat.load();
      }
      if (!skills.data && !skills.isLoading) {
        skills.load();
      }
      if (!feats.data && !feats.isLoading) {
        feats.load();
      }
    }
  }, [character, combat.data, combat.isLoading, skills.data, skills.isLoading, feats.data, feats.isLoading]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-[rgb(var(--color-primary))]"></div>
      </div>
    );
  }

  if (error) {
    return (
      <Card variant="error">
        <p className="text-error">{error}</p>
      </Card>
    );
  }

  if (!character) {
    return (
      <Card variant="warning">
        <p className="text-muted">No character loaded. Please import a save file to begin.</p>
      </Card>
    );
  }


  return (
    <div className="space-y-6">
      {/* Main Character Card - Hero Section */}
      <div className="relative overflow-hidden rounded-xl bg-gradient-to-br from-[rgb(var(--color-surface-2))] to-[rgb(var(--color-surface-1))] border border-[rgb(var(--color-surface-border)/0.5)]">
        {/* Background Pattern */}
        <div className="absolute inset-0 opacity-5">
          <div className="absolute top-0 right-0 w-96 h-96 bg-gradient-to-bl from-[rgb(var(--color-primary))] to-transparent rounded-full blur-3xl" />
          <div className="absolute bottom-0 left-0 w-64 h-64 bg-gradient-to-tr from-[rgb(var(--color-secondary))] to-transparent rounded-full blur-3xl" />
        </div>
        
        <div className="relative p-8">
          <div className="flex items-start gap-8">
            {/* Enhanced Portrait Section */}
            <div className="flex-shrink-0">
              <div className="relative">
                <div className="w-40 h-40 rounded-2xl overflow-hidden ring-4 ring-[rgb(var(--color-surface-border)/0.3)] shadow-2xl">
                  {character.portrait ? (
                    <Image src={character.portrait} alt={character.name} width={160} height={160} className="w-full h-full object-cover" />
                  ) : (
                    <div className="w-full h-full bg-gradient-to-br from-[rgb(var(--color-surface-3))] to-[rgb(var(--color-surface-2))] flex items-center justify-center">
                      <svg className="w-20 h-20 text-[rgb(var(--color-text-muted)/0.3)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                      </svg>
                    </div>
                  )}
                </div>
                {/* Level Badge */}
                <div className="absolute -bottom-2 -right-2 w-12 h-12 bg-gradient-to-br from-[rgb(var(--color-primary))] to-[rgb(var(--color-primary-600))] rounded-full flex items-center justify-center shadow-lg ring-4 ring-[rgb(var(--color-background))]">
                  <span className="text-lg font-bold text-white">{display(character.level)}</span>
                </div>
              </div>
            </div>

            {/* Character Info - Enhanced Layout */}
            <div className="flex-1">
              <div className="mb-6">
                {isEditingName ? (
                  <div className="flex items-center gap-2 mb-2">
                    <input
                      type="text"
                      value={firstName}
                      onChange={(e) => setFirstName(e.target.value)}
                      placeholder="First Name"
                      className="px-3 py-2 text-2xl font-bold bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded-lg text-[rgb(var(--color-text-primary))] focus:outline-none focus:border-[rgb(var(--color-primary))]"
                      disabled={isSaving}
                    />
                    <input
                      type="text"
                      value={lastName}
                      onChange={(e) => setLastName(e.target.value)}
                      placeholder="Last Name"
                      className="px-3 py-2 text-2xl font-bold bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded-lg text-[rgb(var(--color-text-primary))] focus:outline-none focus:border-[rgb(var(--color-primary))]"
                      disabled={isSaving}
                    />
                    <Button
                      onClick={handleSaveName}
                      disabled={isSaving}
                      variant="primary"
                      size="sm"
                    >
                      {isSaving ? (
                        <svg className="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"/>
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"/>
                        </svg>
                      ) : (
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
                        </svg>
                      )}
                    </Button>
                    <Button
                      onClick={handleCancelEdit}
                      disabled={isSaving}
                      variant="outline"
                      size="sm"
                    >
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </Button>
                  </div>
                ) : (
                  <div className="flex items-center gap-2 mb-2">
                    <h1 className="text-4xl font-bold text-[rgb(var(--color-text-primary))]">{display(character.name, 'Unknown Character')}</h1>
                    <Button
                      onClick={() => setIsEditingName(true)}
                      variant="ghost"
                      size="icon"
                      title="Edit character name"
                    >
                      <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                      </svg>
                    </Button>
                  </div>
                )}
                <div className="flex flex-wrap items-center gap-2 mb-3">
                  {(character.classes || []).map((cls, idx) => (
                    <span key={idx} className="px-3 py-1.5 bg-gradient-to-r from-[rgb(var(--color-primary)/0.15)] to-[rgb(var(--color-primary)/0.1)] border border-[rgb(var(--color-primary)/0.2)] rounded-lg text-sm font-medium text-[rgb(var(--color-text-primary))]">
                      {cls.name} {cls.level}
                    </span>
                  ))}
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <span className="px-2.5 py-1 bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur border border-[rgb(var(--color-surface-border)/0.3)] rounded-md text-sm text-[rgb(var(--color-text-secondary))]">
                    {character.subrace ? `${character.subrace} ${character.race}` : display(character.race)}
                  </span>
                  <span className="px-2.5 py-1 bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur border border-[rgb(var(--color-surface-border)/0.3)] rounded-md text-sm text-[rgb(var(--color-text-secondary))]">
                    {display(character.gender)}
                  </span>
                  <span className="px-2.5 py-1 bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur border border-[rgb(var(--color-surface-border)/0.3)] rounded-md text-sm text-[rgb(var(--color-text-secondary))]">
                    {display(character.alignment)}
                  </span>
                  {character.deity && character.deity !== "None" && (
                    <span className="px-2.5 py-1 bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur border border-[rgb(var(--color-surface-border)/0.3)] rounded-md text-sm text-[rgb(var(--color-text-secondary))]">
                      {character.deity}
                    </span>
                  )}
                </div>
              </div>
              
              {/* Core Stats Grid - Redesigned */}
              <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
                {/* HP Card */}
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-4 border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="mb-2">
                    <span className="text-sm text-[rgb(var(--color-text-muted))]">Hit Points</span>
                  </div>
                  <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">
                    {display(character.hitPoints)}<span className="text-lg text-[rgb(var(--color-text-muted))]">/</span>{display(character.maxHitPoints)}
                  </div>
                  <div className="mt-2 h-2 bg-[rgb(var(--color-surface-3))] rounded-full overflow-hidden">
                    <div 
                      className="h-full bg-gradient-to-r from-[rgb(var(--color-error))] to-[rgb(var(--color-error-dark))] transition-all duration-500"
                      style={{ width: `${Math.min(100, ((character.hitPoints || 0) / (character.maxHitPoints || 1)) * 100)}%` }}
                    />
                  </div>
                </div>
                
                {/* AC Card */}
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-4 border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="mb-2">
                    <span className="text-sm text-[rgb(var(--color-text-muted))]">Armor Class</span>
                  </div>
                  <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">{display(character.armorClass)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))] mt-1">Defense Rating</div>
                </div>
                
                {/* XP Card */}
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-4 border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="mb-2">
                    <span className="text-sm text-[rgb(var(--color-text-muted))]">Experience</span>
                  </div>
                  <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">{formatNumber(character.experience)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))] mt-1">Total XP</div>
                </div>
                
                {/* Gold Card */}
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-4 border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="mb-2">
                    <span className="text-sm text-[rgb(var(--color-text-muted))]">Gold</span>
                  </div>
                  <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">{formatNumber(character.gold)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))] mt-1">Gold Pieces</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Progressive Disclosure Sections */}
      <div className="space-y-4">
        
        {/* Core Attributes - Always important for RPG */}
        <CollapsibleSection 
          title="Attributes" 
          defaultOpen={true}
          badge={character.abilities ? formatModifier(Object.values(character.abilities).reduce((sum, val) => sum + Math.floor((val - 10) / 2), 0)) : '-'}
        >
          <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
            {character.abilities && Object.entries(character.abilities).map(([key, value]) => {
              const modifier = Math.floor((value - 10) / 2);
              const modifierColor = modifier > 0 ? 'var(--color-success)' : modifier < 0 ? 'var(--color-error)' : 'var(--color-text-muted)';
              return (
                <div key={key} className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 border border-[rgb(var(--color-surface-border)/0.3)] hover:border-[rgb(var(--color-primary)/0.3)] transition-colors">
                  <div className="flex justify-between items-start mb-1">
                    <span className="text-xs text-[rgb(var(--color-text-muted))] uppercase tracking-wider">{t(`attributes.${key}`)}</span>
                    <span className={`text-xs font-medium text-[rgb(${modifierColor})]`}>
                      {modifier >= 0 ? '+' : ''}{modifier}
                    </span>
                  </div>
                  <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))]">{value}</div>
                </div>
              );
            })}
          </div>
        </CollapsibleSection>


        {/* Combat & Progression */}
        <CollapsibleSection 
          title="Combat & Progression" 
          defaultOpen={false}
          badge={(character.availableSkillPoints ?? 0) > 0 ? `${character.availableSkillPoints} skill points` : `AC ${character.armorClass}`}
        >
          <div className="space-y-6">
            {/* Combat Stats */}
            <div>
              <p className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-3">Combat Statistics</p>
              <div className="grid grid-cols-3 gap-3 mb-4">
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.baseAttackBonus)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.baseAttackBonus')}</div>
                </div>
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.meleeAttackBonus)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.meleeAttack')}</div>
                </div>
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.rangedAttackBonus)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.rangedAttack')}</div>
                </div>
              </div>
              
              <p className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-3">{t('attributes.savingThrows')}</p>
              <div className="grid grid-cols-3 gap-3">
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.saves?.fortitude)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('attributes.fortitude')}</div>
                </div>
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.saves?.reflex)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('attributes.reflex')}</div>
                </div>
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.saves?.will)}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('attributes.will')}</div>
                </div>
              </div>
            </div>

            {/* Character Development */}
            <div className="pt-4 border-t border-[rgb(var(--color-surface-border)/0.6)]">
              <p className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-3">Character Development</p>
              <div className="grid grid-cols-3 gap-3 mb-4">
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{character.totalSkillPoints}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">Total Skill Points</div>
                </div>
                <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{character.totalFeats}</div>
                  <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.totalFeats')}</div>
                </div>
                {character.knownSpells !== undefined && (
                  <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{character.knownSpells}</div>
                    <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.knownSpells')}</div>
                  </div>
                )}
              </div>

              {/* Movement & Physical Stats */}
              <div className="pt-4 border-t border-[rgb(var(--color-surface-border)/0.6)]">
                <p className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-3">Physical Attributes</p>
                <div className="grid grid-cols-3 gap-3">
                  <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(character.movementSpeed)} ft</div>
                    <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.speed')}</div>
                  </div>
                  <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(character.initiative)}</div>
                    <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.initiative')}</div>
                  </div>
                  <div className="bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded-lg p-3 text-center border border-[rgb(var(--color-surface-border)/0.3)]">
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(character.size)}</div>
                    <div className="text-xs text-[rgb(var(--color-text-muted))]">{t('character.size')}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </CollapsibleSection>


        {/* Advanced/Rare Data - Hidden by default */}
        {(character.damageResistances || character.damageImmunities || character.spellResistance) && (
          <CollapsibleSection 
            title="Special Defenses" 
            defaultOpen={false}
            badge={character.spellResistance ? `SR ${character.spellResistance}` : "Resistances"}
          >
            <div className="space-y-3">
              {character.spellResistance && (
                <div className="flex justify-between">
                  <span className="text-[rgb(var(--color-text-secondary))]">{t('character.spellResistance')}</span>
                  <span className="font-medium text-[rgb(var(--color-text-primary))]">{character.spellResistance}</span>
                </div>
              )}
              {character.damageResistances && character.damageResistances.length > 0 && (
                <div>
                  <p className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-2">{t('character.resistances')}</p>
                  <div className="space-y-1">
                    {character.damageResistances.map((res, idx) => (
                      <div key={idx} className="flex justify-between text-sm">
                        <span className="text-[rgb(var(--color-text-secondary))]">{res.type}</span>
                        <span className="font-medium text-[rgb(var(--color-text-primary))]">{res.amount}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {character.damageImmunities && character.damageImmunities.length > 0 && (
                <div>
                  <p className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-2">{t('character.immunities')}</p>
                  <div className="flex flex-wrap gap-1">
                    {character.damageImmunities.map((immunity, idx) => (
                      <span key={idx} className="px-2 py-1 bg-[rgb(var(--color-surface-1)/0.5)] backdrop-blur rounded text-xs text-[rgb(var(--color-text-secondary))]">
                        {immunity}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </CollapsibleSection>
        )}

        {/* Enhanced Campaign Overview */}
        <CampaignOverview character={character} />
      </div>
    </div>
  );
}