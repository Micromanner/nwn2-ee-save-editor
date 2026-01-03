'use client';

import { useState, useEffect } from 'react';
import { useTranslations } from '@/hooks/useTranslations';
import { useCharacterContext, useSubsystem } from '@/contexts/CharacterContext';
import { display, formatModifier, formatNumber } from '@/utils/dataHelpers';
import { CharacterAPI } from '@/services/characterApi';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import CampaignOverview from './CampaignOverview';
import DeitySelectionModal, { Deity } from './DeitySelectionModal';

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
          <div className={`px-6 md:px-8 pb-6 md:pb-8 ${isOpen ? 'border-t border-[rgb(var(--color-surface-border)/0.3)]' : 'border-t-0 border-transparent'}`}>
            <div className="pt-6 md:pt-8 grid-flow-row">
              {children}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default function CharacterOverview({ onNavigate: _onNavigate }: CharacterOverviewProps) { // eslint-disable-line @typescript-eslint/no-unused-vars
  const t = useTranslations();
  const { character, isLoading, error, refreshAll, updateCharacterPartial } = useCharacterContext();
  const combat = useSubsystem('combat');
  const skills = useSubsystem('skills');
  const feats = useSubsystem('feats');
  const saves = useSubsystem('saves');
  const abilities = useSubsystem('abilityScores');
  const classes = useSubsystem('classes');
  
  console.log('CharacterOverview component rendered/mounted');
  console.log('Character HP data:', character?.hitPoints, character?.maxHitPoints, character?.current_hit_points, character?.max_hit_points);
  console.log('Character BAB data:', character?.baseAttackBonus);
  console.log('Abilities subsystem data:', abilities.data);
  if (abilities.data) {
    console.log('Abilities data keys:', Object.keys(abilities.data));
    console.log('Looking for HP in abilities data...', abilities.data.derived_stats?.hit_points);
  }
  console.log('Combat subsystem data:', combat.data);
  if (combat.data) {
    console.log('Combat data keys:', Object.keys(combat.data));
    console.log('Combat BAB data:', combat.data.base_attack_bonus);
    console.log('Combat BAB type:', typeof combat.data.base_attack_bonus);
    if (combat.data.base_attack_bonus && typeof combat.data.base_attack_bonus === 'object') {
      console.log('Combat BAB object keys:', Object.keys(combat.data.base_attack_bonus));
      console.log('Combat BAB base_attack_bonus:', combat.data.base_attack_bonus.base_attack_bonus);
    }
  }
  console.log('Skills subsystem data:', skills.data);
  console.log('Feats subsystem data:', feats.data);
  console.log('Saves subsystem data:', saves.data);

  // Debug: Manual refresh function to test data fetching
  const _handleManualRefresh = () => { // eslint-disable-line @typescript-eslint/no-unused-vars
    console.log('Manual refresh triggered');
    refreshAll();
  };
  
  // Debug: Log character data to see what we're getting
  useEffect(() => {
    if (character) {
      console.log('CharacterOverview - Successfully loaded character:', character.name, 'Level', character.level);
    }
  }, [character]);
  
  // Name editing state
  const [isEditingName, setIsEditingName] = useState(false);
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  
  // Deity editing state
  const [isDeityModalOpen, setIsDeityModalOpen] = useState(false);
  const [deity, setDeity] = useState('');
  const [availableDeities, setAvailableDeities] = useState<Deity[]>([]);
  
  // Biography editing state
  const [isEditingBio, setIsEditingBio] = useState(false);
  const [biography, setBiography] = useState('');
  
  // Fetch available deities on mount
  useEffect(() => {
    const fetchDeities = async () => {
      if (!character?.id) return;
      try {
        const deities = await CharacterAPI.getAvailableDeities(character.id);
        // Map API Deity to local Deity interface if needed, or trust structural compatibility
        setAvailableDeities(deities);
      } catch (error) {
        console.error('Failed to fetch deities:', error);
      }
    };
    fetchDeities();
  }, [character?.id]);
  
  // Initialize name fields when character changes
  useEffect(() => {
    if (character && character.name) {
      const parts = character.name.split(' ');
      setFirstName(parts[0] || '');
      setLastName(parts.slice(1).join(' ') || '');
    }
  }, [character]);
  
  // Initialize deity when character changes
  useEffect(() => {
    if (character) {
      setDeity(character.deity || '');
    }
  }, [character]);
  
  // Initialize biography when character changes
  useEffect(() => {
    if (character) {
      setBiography(character.biography || '');
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
      
      // Update local state smoothly
      if (updateCharacterPartial) {
        // Construct the new name properly based on input
        const newFullName = lastName.trim() ? `${firstName.trim()} ${lastName.trim()}` : firstName.trim();
        updateCharacterPartial({ 
            name: newFullName,
            first_name: firstName.trim(),
            last_name: lastName.trim()
        });
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
  
  const handleSelectDeity = async (selectedDeityName: string) => {
    if (!character?.id || isSaving) return;
    
    setIsSaving(true);
    try {
      await CharacterAPI.setDeity(character.id, selectedDeityName.trim());
      
      // Update local state immediately
      setDeity(selectedDeityName.trim());
      
      // Update context smoothly
      if (updateCharacterPartial) {
        updateCharacterPartial({ deity: selectedDeityName.trim() });
      }
      setIsDeityModalOpen(false);
    } catch (error) {
      console.error('Failed to save deity:', error);
    } finally {
      setIsSaving(false);
    }
  };
  
  const handleSaveBiography = async () => {
    if (!character?.id || isSaving) return;
    
    setIsSaving(true);
    try {
      await CharacterAPI.setBiography(character.id, biography.trim());
      
      // Update context smoothly
      if (updateCharacterPartial) {
        updateCharacterPartial({ biography: biography.trim() });
      }
      setIsEditingBio(false);
    } catch (error) {
      console.error('Failed to save biography:', error);
    } finally {
      setIsSaving(false);
    }
  };
  
  // Load subsystems data only if missing - don't force refresh on every tab switch
  useEffect(() => {
    console.log('CharacterOverview - Character changed or component mounted, checking data status');
    console.log('CharacterOverview - Character data:', character);
    if (character) {
      console.log('CharacterOverview - Checking subsystem data status for character:', character.name || character.id);
      
      // Only load data if missing, don't force refresh
      if (!abilities.data && !abilities.isLoading) {
        console.log('CharacterOverview - Loading abilities data (missing)');
        abilities.load().catch(err => console.warn('Failed to load abilities data:', err));
      }
      if (!combat.data && !combat.isLoading) {
        console.log('CharacterOverview - Loading combat data (missing)');
        combat.load().catch(err => console.warn('Failed to load combat data:', err));
      }
      if (!skills.data && !skills.isLoading) {
        console.log('CharacterOverview - Loading skills data (missing)');
        skills.load().catch(err => console.warn('Failed to load skills data:', err));
      }
      if (!feats.data && !feats.isLoading) {
        console.log('CharacterOverview - Loading feats data (missing)');
        feats.load().catch(err => console.warn('Failed to load feats data:', err));
      }
      if (!saves.data && !saves.isLoading) {
        console.log('CharacterOverview - Loading saves data (missing)');
        saves.load().catch(err => console.warn('Failed to load saves data:', err));
      }
      if (!classes.data && !classes.isLoading) {
        console.log('CharacterOverview - Loading classes data (missing)');
        classes.load().catch(err => console.warn('Failed to load classes data:', err));
      }
    } else {
      console.log('CharacterOverview - No character loaded, skipping data loading');
    }
  }, [character?.id, abilities, combat, skills, feats, saves, classes]); // eslint-disable-line react-hooks/exhaustive-deps

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
        
        <div className="relative p-6 md:p-8">
            {/* Top Section: Identity & Bio */}
            <div className="space-y-6">
                {/* Header Row: Name & Edit */}
                {isEditingName ? (
                  <div className="flex items-center gap-2 mb-4">
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
                      type="button"
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
                      type="button"
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
                  <div className="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-6">
                    <div className="flex items-center gap-2">
                        <h1 className="text-4xl font-bold text-[rgb(var(--color-text-primary))]">{display(character.name, 'Unknown Character')}</h1>
                        <Button
                        type="button"
                        onClick={() => setIsEditingName(true)}
                        variant="ghost"
                        size="icon"
                        className="text-[rgb(var(--color-text-muted))] hover:text-[rgb(var(--color-primary))] transition-colors"
                        title="Edit character name"
                        >
                        <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                        </svg>
                        </Button>
                    </div>
                    {/* Classes Pills */}
                     <div className="flex flex-wrap items-center gap-2">
                      {(character.classes || []).map((cls, idx) => (
                        <span key={idx} className="px-3 py-1.5 bg-gradient-to-r from-[rgb(var(--color-primary)/0.2)] to-[rgb(var(--color-primary)/0.1)] border border-[rgb(var(--color-primary)/0.3)] shadow-[0_0_10px_rgba(var(--color-primary),0.1)] rounded-lg text-sm font-bold text-[rgb(var(--color-primary-light))]">
                          {cls.name} {cls.level}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
                
                {/* Structured Bio Grid */}
                <div className="grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-4 pt-4 border-t border-[rgb(var(--color-surface-border)/0.4)]">
                    
                    {/* Basic Info Row */}
                    <div className="col-span-1 md:col-span-2 grid grid-cols-2 md:grid-cols-4 gap-4 pb-4 border-b border-[rgb(var(--color-surface-border)/0.2)]">
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Race & Origin</div>
                            <div className="font-medium text-[rgb(var(--color-text-primary))] truncate" title={character.subrace ? `${character.race} (${character.subrace})` : character.race}>
                                {character.race}{character.subrace ? <span className="text-[rgb(var(--color-text-secondary))] text-sm ml-1">({character.subrace})</span> : null}
                            </div>
                         </div>
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Gender & Age</div>
                            <div className="font-medium text-[rgb(var(--color-text-primary))]">
                                {display(character.gender)} <span className="text-[rgb(var(--color-text-muted))] text-sm">/ {character.age || '?'} yrs</span>
                            </div>
                         </div>
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Alignment</div>
                             <div className="font-medium text-[rgb(var(--color-text-primary))]">{display(character.alignment)}</div>
                         </div>
                         <div>
                             <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Deity</div>
                               <div className="flex items-center gap-2">
                                 <span className="font-medium text-[rgb(var(--color-text-primary))] truncate max-w-[120px]" title={character.deity || 'None'}>{character.deity || '-'}</span>
                                 <Button 
                                    type="button"
                                    onClick={() => setIsDeityModalOpen(true)} 
                                    variant="ghost" 
                                    size="sm" 
                                    className="h-6 w-6 text-[rgb(var(--color-text-muted))] hover:text-[rgb(var(--color-primary))] p-0 transition-colors"
                                    disabled={isSaving}
                                    title="Change Deity"
                                 >
                                   <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                     <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                                   </svg>
                                 </Button>
                               </div>
                         </div>

                         {/* Row 2: Core Stats (HP, AC, XP, Gold) */}
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Hit Points</div>
                            <div className="font-medium text-[rgb(var(--color-text-primary))]">
                                {abilities.isLoading ? <span className="text-xs">...</span> : (
                                    <>
                                        {display(abilities.data?.derived_stats?.hit_points?.current || character.hitPoints || 0)}
                                        <span className="text-[rgb(var(--color-text-muted))]">/{display(abilities.data?.derived_stats?.hit_points?.maximum || character.maxHitPoints || 0)}</span>
                                    </>
                                )}
                            </div>
                         </div>
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Armor Class</div>
                            <div className="font-medium text-[rgb(var(--color-text-primary))]">
                                {combat.isLoading ? <span className="text-xs">...</span> : display(combat.data?.armor_class?.total || character.armorClass)}
                            </div>
                         </div>
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Experience</div>
                            <div className="font-medium text-[rgb(var(--color-text-primary))]">{formatNumber(character.experience)}</div>
                         </div>
                         <div>
                            <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Gold</div>
                            <div className="font-medium text-[rgb(var(--color-warning))]">{formatNumber(character.gold || 0)}</div>
                         </div>
                    </div>


                    {/* Background */}
                    <div className="flex flex-col gap-1">
                        <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase font-semibold mb-1">
                             Background
                        </div>
                        <div className="flex items-center">
                            {character.background ? (
                                <span className="inline-flex items-center px-3 py-1.5 rounded-md bg-[rgb(var(--color-surface-3))] border border-[rgb(var(--color-surface-border)/0.5)] text-[rgb(var(--color-text-primary))] text-sm font-medium">
                                    {character.background.name}
                                </span>
                            ) : (
                                <span className="text-[rgb(var(--color-text-muted))] text-sm italic">None</span>
                            )}
                        </div>
                    </div>

                    {/* Domains */}
                     <div className="flex flex-col gap-1">
                        <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase font-semibold mb-1">
                            Domains
                        </div>
                        <div className="flex flex-wrap gap-2">
                            {character.domains && character.domains.length > 0 ? (
                                character.domains.map((domain, idx) => (
                                    <span key={idx} className="inline-flex items-center px-3 py-1.5 rounded-md bg-gradient-to-br from-[rgb(var(--color-secondary)/0.15)] to-[rgb(var(--color-secondary)/0.05)] border border-[rgb(var(--color-secondary)/0.2)] text-[rgb(var(--color-text-primary))] text-sm font-medium">
                                        {domain.name}
                                    </span>
                                ))
                            ) : (
                                <span className="text-[rgb(var(--color-text-muted))] text-sm italic">None</span>
                            )}
                        </div>
                    </div>
                    
                    {/* Biography */}
                    <div className="col-span-1 md:col-span-2 flex flex-col gap-1">
                        <div className="flex items-center justify-between">
                          <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase font-semibold">Biography</div>
                          {!isEditingBio && (

                            <Button 
                                type="button"
                                onClick={() => setIsEditingBio(true)} 
                                variant="ghost" 
                                size="sm" 
                                className="h-6 w-6 text-[rgb(var(--color-text-muted))] hover:text-[rgb(var(--color-primary))] p-0 transition-colors"
                                title="Edit Biography"
                            >
                              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
                              </svg>
                            </Button>
                          )}
                        </div>
                        {isEditingBio ? (
                          <div className="space-y-2">
                            <textarea
                              value={biography}
                              onChange={(e) => setBiography(e.target.value)}
                              placeholder="Enter character biography..."
                              className="w-full px-3 py-2 text-sm bg-[rgb(var(--color-surface-2))] border border-[rgb(var(--color-surface-border))] rounded-lg text-[rgb(var(--color-text-primary))] focus:outline-none focus:border-[rgb(var(--color-primary))] min-h-[100px] resize-y"
                              disabled={isSaving}
                            />
                            <div className="flex gap-2">
                              <Button type="button" onClick={handleSaveBiography} disabled={isSaving} variant="primary" size="sm">
                                {isSaving ? 'Saving...' : 'Save'}
                              </Button>
                              <Button type="button" onClick={() => { setBiography(character.biography || ''); setIsEditingBio(false); }} disabled={isSaving} variant="outline" size="sm">
                                Cancel
                              </Button>
                            </div>
                          </div>
                        ) : (
                          <div className="text-sm text-[rgb(var(--color-text-secondary))]">
                            {character.biography ? (
                              <p className="whitespace-pre-wrap">{character.biography}</p>
                            ) : (
                              <span className="text-[rgb(var(--color-text-muted))] italic">No biography written</span>
                            )}
                          </div>
                        )}
                    </div>
                </div>
              </div>
      </div>
      </div>

      {/* Progressive Disclosure Sections */}
      <div className="space-y-4">
        
        {/* Core Abilities - Always important for RPG */}
        <CollapsibleSection 
          title={t('navigation.abilityScores')} 
          defaultOpen={true}
          badge={(abilities.data?.effective_attributes ? formatModifier(Object.values(abilities.data.effective_attributes).reduce((sum, val) => sum + Math.floor((val - 10) / 2), 0)) : character.abilities ? formatModifier(Object.values(character.abilities).reduce((sum, val) => sum + Math.floor((val - 10) / 2), 0)) : '-')}
        >
          <div className="grid grid-cols-3 md:grid-cols-6 gap-y-6 gap-x-4">
            {(abilities.data?.effective_attributes ? Object.entries(abilities.data.effective_attributes) : character.abilities ? Object.entries(character.abilities) : []).map(([key, value]) => {
              const modifier = Math.floor((value - 10) / 2);
              const modifierColor = modifier > 0 ? 'var(--color-success)' : modifier < 0 ? 'var(--color-error)' : 'var(--color-text-muted)';
              return (
                <div key={key} className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase tracking-wider mb-1">{
                    key === 'Str' ? t('abilityScores.strength') :
                    key === 'Dex' ? t('abilityScores.dexterity') :
                    key === 'Con' ? t('abilityScores.constitution') :
                    key === 'Int' ? t('abilityScores.intelligence') :
                    key === 'Wis' ? t('abilityScores.wisdom') :
                    key === 'Cha' ? t('abilityScores.charisma') :
                    key
                  }</div>
                  <div className="text-2xl font-bold text-[rgb(var(--color-text-primary))] leading-none mb-1">{value}</div>
                  <div className={`text-sm font-medium text-[rgb(${modifierColor})]`}>
                    {modifier >= 0 ? '+' : ''}{modifier}
                  </div>
                </div>
              );
            })}
          </div>
        </CollapsibleSection>


        {/* Combat & Progression */}
        <CollapsibleSection 
          title="Combat & Progression" 
          defaultOpen={false}
          badge={(skills.data?.skill_points_available ?? character.availableSkillPoints ?? 0) > 0 ? `${skills.data?.skill_points_available || character.availableSkillPoints} skill points` : `AC ${combat.data?.armor_class?.total || character.armorClass}`}
        >
          <div className="space-y-6">
            {/* Combat Stats */}
            <div>
              <div className="grid grid-cols-3 gap-4 mb-4">
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.baseAttackBonus')}</div>
                  <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">
                    {combat.isLoading ? (
                      <span className="text-sm">...</span>
                    ) : (
                      formatModifier(
                        (typeof combat.data?.base_attack_bonus === 'object' && combat.data?.base_attack_bonus?.total_bab) || 
                        combat.data?.summary?.base_attack_bonus || 
                        character.baseAttackBonus || 
                        0
                      )
                    )}
                  </div>
                </div>
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.meleeAttack')}</div>
                  <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(combat.data?.attack_bonuses?.melee_attack_bonus || character.meleeAttackBonus || 0)}</div>
                </div>
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.rangedAttack')}</div>
                  <div className="text-xl font-bold text-[rgb(var(--color-text-primary))]">{formatModifier(combat.data?.attack_bonuses?.ranged_attack_bonus || character.rangedAttackBonus || 0)}</div>
                </div>
              </div>
              <div className="grid grid-cols-3 gap-4">
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('abilityScores.fortitude')}</div>
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                    {saves.isLoading ? (
                      <span className="text-sm">...</span>
                    ) : (
                      formatModifier(
                        (typeof saves.data?.fortitude === 'object' && saves.data.fortitude !== null ? (saves.data.fortitude as { total?: number })?.total : undefined) || 
                        (typeof saves.data?.fortitude === 'number' ? saves.data.fortitude : 0) || 
                        (typeof character.saves?.fortitude === 'number' ? character.saves.fortitude : 0)
                      )
                    )}
                  </div>
                </div>
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('abilityScores.reflex')}</div>
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                    {saves.isLoading ? (
                      <span className="text-sm">...</span>
                    ) : (
                      formatModifier(
                        (typeof saves.data?.reflex === 'object' && saves.data.reflex !== null ? (saves.data.reflex as { total?: number })?.total : undefined) || 
                        (typeof saves.data?.reflex === 'number' ? saves.data.reflex : 0) || 
                        (typeof character.saves?.reflex === 'number' ? character.saves.reflex : 0)
                      )
                    )}
                  </div>
                </div>
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('abilityScores.will')}</div>
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                    {saves.isLoading ? (
                      <span className="text-sm">...</span>
                    ) : (
                      formatModifier(
                        (typeof saves.data?.will === 'object' && saves.data.will !== null ? (saves.data.will as { total?: number })?.total : undefined) || 
                        (typeof saves.data?.will === 'number' ? saves.data.will : 0) || 
                        (typeof character.saves?.will === 'number' ? character.saves.will : 0)
                      )
                    )}
                  </div>
                </div>
              </div>
            </div>

            {/* Character Development */}
            <div className="pt-4 border-t border-[rgb(var(--color-surface-border)/0.6)]">
              <div className="grid grid-cols-3 gap-4 mb-4">
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">Total Skill Points</div>
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(skills.data?.spent_points || character.totalSkillPoints || 0)}</div>
                </div>
                <div className="">
                  <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.totalFeats')}</div>
                  <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(feats.data?.summary?.total || character.totalFeats || 0)}</div>
                </div>
                {character.knownSpells !== undefined && (
                  <div className="">
                    <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.knownSpells')}</div>
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(character.knownSpells || 0)}</div>
                  </div>
                )}
              </div>

              {/* Movement & Physical Stats */}
              <div className="pt-4 border-t border-[rgb(var(--color-surface-border)/0.6)]">
                <div className="grid grid-cols-3 gap-4">
                  <div className="">
                    <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.speed')}</div>
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(character.movementSpeed)} ft</div>
                  </div>
                  <div className="">
                    <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.initiative')}</div>
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">
                      {formatModifier(
                        (typeof combat.data?.initiative === 'object' && combat.data?.initiative?.total) || 
                        (typeof combat.data?.initiative === 'number' ? combat.data?.initiative : 0) || 
                        character.initiative || 
                        0
                      )}
                    </div>
                  </div>
                  <div className="">
                    <div className="text-xs text-[rgb(var(--color-text-muted))] uppercase mb-1">{t('character.size')}</div>
                    <div className="text-lg font-bold text-[rgb(var(--color-text-primary))]">{display(character.size)}</div>
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

      {/* Deity Selection Modal */}
      <DeitySelectionModal
        isOpen={isDeityModalOpen}
        onClose={() => setIsDeityModalOpen(false)}
        onSelectDeity={handleSelectDeity}
        availableDeities={availableDeities}
        currentDeity={character.deity}
      />
    </div>
  );
}