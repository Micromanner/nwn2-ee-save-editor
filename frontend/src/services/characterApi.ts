// Character API service for fetching character data from Django backend

export interface CharacterAbilities {
  strength: number;
  dexterity: number;
  constitution: number;
  intelligence: number;
  wisdom: number;
  charisma: number;
}

export interface CharacterSaves {
  fortitude: number;
  reflex: number;
  will: number;
}

export interface CharacterClass {
  name: string;
  level: number;
}

export interface DamageResistance {
  type: string;
  amount: number;
}

// API Response interfaces
export interface SaveResult {
  success: boolean;
  changes: Record<string, unknown>;
  backup_created: boolean;
}

export interface FeatResponse {
  id: number;
  feat_id?: number;
  label: string;
  name: string;
  type: number;
  protected: boolean;
  custom: boolean;
  icon?: string;
  description?: string;
  prerequisites?: Record<string, unknown>;
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
}

export interface FeatsStateResponse {
  summary: {
    total: number;
    protected: FeatResponse[];
    class_feats: FeatResponse[];
    general_feats: FeatResponse[];
    custom_feats: FeatResponse[];
  };
  all_feats: FeatResponse[];
  available_feats: FeatResponse[];
  legitimate_feats: FeatResponse[];
  feat_chains: Record<string, unknown>;
  recommended_feats: FeatResponse[];
}

export interface LegitimateFeatsResponse {
  feats: FeatResponse[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    has_next: boolean;
    has_previous: boolean;
  };
  search?: string;
  category?: string;
  subcategory?: string;
}

export interface FeatActionResponse {
  feat_id: number;
  success: boolean;
  message: string;
  character_feats?: FeatResponse[];
}

export interface FeatDetailsResponse {
  id: number;
  feat_id?: number;
  label: string;
  name: string;
  description: string;
  type: number;
  protected: boolean;
  custom: boolean;
  icon?: string;
  prerequisites?: Record<string, unknown>;
  effects?: Record<string, unknown>;
}

export interface FeatValidationResponse {
  feat_id: number;
  can_take: boolean;
  reason: string;
  has_feat: boolean;
  missing_requirements: string[];
}

export interface SkillEntry {
  skill_id: number;
  name: string;
  ranks: number;
  max_ranks: number;
  bonus: number;
  total: number;
  is_class_skill: boolean;
}

export interface SkillInfo {
  name: string;
  description?: string;
  key_ability: string;
}

export interface SkillsStateResponse {
  character_skills: Record<number, SkillEntry>;
  available_points: number;
  total_points: number;
  skill_info: Record<number, SkillInfo>;
  // Additional properties to match SkillsData structure
  skills: Array<{
    id: number;
    name: string;
    rank: number;
    max_rank: number;
    is_class_skill: boolean;
    total_bonus: number;
    ability_modifier: number;
    misc_modifier: number;
  }>;
  skill_points: {
    available: number;
    spent: number;
  };
}

export interface SkillsUpdateResponse {
  success: boolean;
  updated_skills: Record<number, number>;
  available_points: number;
  message?: string;
  skill_summary?: {
    total_spent?: number;
    available?: number;
  };
}

export interface AbilitiesStateResponse {
  abilities: {
    strength: number;
    dexterity: number;
    constitution: number;
    intelligence: number;
    wisdom: number;
    charisma: number;
  };
  modifiers: {
    strength: number;
    dexterity: number;
    constitution: number;
    intelligence: number;
    wisdom: number;
    charisma: number;
  };
  available_points?: number;
}

export interface AbilitiesUpdateResponse {
  success: boolean;
  updated_abilities: Record<string, number>;
  message?: string;
}

export interface AlignmentResponse {
  lawChaos: number;
  goodEvil: number;
  alignment_string?: string;
}

export interface AlignmentUpdateResponse {
  success: boolean;
  law_chaos: number;
  good_evil: number;
  lawChaos: number;
  goodEvil: number;
  alignment_string: string;
}

export interface RaceDataResponse {
  race_id: number;
  race_name: string;
  subrace: string;
  size: number;
  size_name: string;
  base_speed: number;
  ability_modifiers: Record<string, number>;
  racial_feats: number[];
  favored_class?: number;
}

export interface CombatUpdateResponse {
  success: boolean;
  updated_value: number;
  message?: string;
}

export interface SavesData {
  fortitude: number;
  reflex: number;
  will: number;
}

export interface CharacterData {
  id?: number;
  name: string;
  race: string;
  subrace?: string;
  gender: string;
  classes: CharacterClass[];
  alignment: string;
  deity: string;
  level: number;
  experience: number;
  hitPoints: number;
  maxHitPoints: number;
  abilities: CharacterAbilities;
  saves: CharacterSaves;
  armorClass: number;
  armor_class?: number;
  current_hit_points?: number;
  max_hit_points?: number;
  gold: number;
  location?: string;
  playTime?: string;
  lastSaved?: string;
  portrait?: string;
  customPortrait?: string;
  // Appearance properties
  appearance?: number;
  soundset?: number;
  bodyType?: number;
  hairStyle?: number;
  hairColor?: number;
  skinColor?: number;
  headVariation?: number;
  tattooColor1?: number;
  tattooColor2?: number;
  // Combat stats
  baseAttackBonus: number;
  meleeAttackBonus?: number;
  rangedAttackBonus?: number;
  mainHandDamage?: string;
  offHandDamage?: string;
  // Character progress
  totalSkillPoints?: number;
  availableSkillPoints?: number;
  totalFeats?: number;
  knownSpells?: number;
  // Defenses
  damageResistances?: DamageResistance[];
  damageImmunities?: string[];
  spellResistance?: number;
  // Physical stats
  movementSpeed?: number;
  size?: string;
  initiative?: number;
  // Campaign stats
  completedQuests?: number;
  currentQuests?: number;
  journalEntries?: number;
  companionsRecruited?: number;
  deaths?: number;
  killCount?: number;
  // Campaign info
  campaignName?: string;
  moduleName?: string;
  campaignModules?: string[];
  // Enhanced campaign data
  gameAct?: number;
  difficultyLevel?: number;
  lastSavedTimestamp?: number;
  companionStatus?: Record<string, {name: string, influence: number, status: string, influence_found: boolean}>;
  hiddenStatistics?: Record<string, number>;
  storyMilestones?: Record<string, {name: string, milestones: Array<{description: string, completed: boolean, variable: string}>}>;
  questDetails?: {
    summary: {
      completed_quests: number;
      active_quests: number;
      total_quest_variables: number;
      completed_quest_list: string[];
      active_quest_list: string[];
    };
    categories: Record<string, {
      name: string;
      completed: string[];
      active: string[];
    }>;
    progress_stats: {
      total_completion_rate: number;
      main_story_progress: number;
      companion_progress: number;
      exploration_progress: number;
    };
  };
  // Session & locale data
  detectedLanguage?: string;
  languageId?: number;
  localizationStatus?: string;
  createdAt?: string;
  updatedAt?: string;
  // Additional properties for overview
  derived_stats?: {
    armor_class?: number;
    current_hit_points?: number;
    max_hit_points?: number;
    base_attack_bonus?: number;
    fortitude?: number;
    reflex?: number;
    will?: number;
    effective_attributes?: Record<string, number>;
    skill_points_available?: number;
    attack_bonuses?: {
      melee?: number;
      ranged?: number;
    };
    initiative?: number;
  };
  base_attack_bonus?: number;
  summary?: {
    spent_points?: number;
    total_feats?: number;
  };
  skill_points_available?: number;
}

// Base API URL - FastAPI server (from environment variable)
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL;

export class CharacterAPI {
  // Get character state (comprehensive data)
  static async getCharacterState(characterId: number): Promise<CharacterData> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/state`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character state: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  // Get character details (basic data)
  static async getCharacterDetails(characterId: number): Promise<CharacterData> {
    // Use summary endpoint instead of non-existent basic details endpoint
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/summary`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character details: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  // List all characters
  static async listCharacters(): Promise<CharacterData[]> {
    // Backend doesn't have a list characters endpoint - this would need to be implemented
    // For now, return empty array
    console.warn('listCharacters() called but no backend endpoint exists');
    return [];
  }

  // Import character from save game
  static async importCharacter(savePath: string): Promise<{id: number; name: string}> {
    const response = await fetch(`${API_BASE_URL}/savegames/import`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ save_path: savePath }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      const errorMessage = errorData.detail || `Failed to import character: ${response.statusText}`;
      throw new Error(errorMessage);
    }
    
    const data = await response.json();
    // FastAPI returns character_id as string, convert to number and return proper format
    const characterId = parseInt(data.character_id);
    if (!characterId) {
      throw new Error('Import successful but no character ID returned');
    }
    
    return {
      id: characterId,
      name: data.character_name || 'Unknown Character'
    };
  }

  // Update character data
  static async updateCharacter(characterId: number, updates: Partial<{ first_name: string; last_name: string; [key: string]: unknown }>): Promise<CharacterData> {
    // Backend doesn't have a generic update character endpoint
    // Use the savegame update endpoint with sync_current_state for now
    console.warn('updateCharacter() called but no backend endpoint exists, using save instead');
    await this.saveCharacter(characterId, updates);
    
    // Return updated character state
    return this.getCharacterState(characterId);
  }

  // Save character changes to save game
  static async saveCharacter(characterId: number, updates: Record<string, unknown> = {}): Promise<SaveResult> {
    const response = await fetch(`${API_BASE_URL}/${characterId}/update`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ 
        sync_current_state: true,
        create_backup: true,
        updates 
      }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      const errorMessage = errorData.detail || `Failed to save character: ${response.statusText}`;
      throw new Error(errorMessage);
    }
    
    return response.json();
  }

  // Feat management methods
  static async getCharacterFeats(characterId: number, featType?: number): Promise<FeatsStateResponse> {
    const typeParam = featType !== undefined ? `?type=${featType}` : '';
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/state${typeParam}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character feats: ${response.statusText}`);
    }
    return response.json();
  }

  static async getLegitimateFeats(
    characterId: number, 
    options: { 
      featType?: number; 
      category?: string;
      subcategory?: string;
      page?: number; 
      limit?: number; 
      search?: string; 
    } = {}
  ): Promise<LegitimateFeatsResponse> {
    const params = new URLSearchParams();
    if (options.featType !== undefined) params.append('type', options.featType.toString());
    if (options.category) params.append('category', options.category);
    if (options.subcategory) params.append('subcategory', options.subcategory);
    if (options.page !== undefined) params.append('page', options.page.toString());
    if (options.limit !== undefined) params.append('limit', options.limit.toString());
    if (options.search) params.append('search', options.search);
    
    const queryString = params.toString();
    const url = `${API_BASE_URL}/characters/${characterId}/feats/legitimate${queryString ? `?${queryString}` : ''}`;
    
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch legitimate feats: ${response.statusText}`);
    }
    const data = await response.json();
    return {
      feats: data.legitimate_feats || [],
      pagination: data.pagination || {
        page: 1,
        limit: 50,
        total: 0,
        has_next: false,
        has_previous: false
      },
      search: data.search,
      category: data.category,
      subcategory: data.subcategory
    };
  }

  static async addFeat(characterId: number, featId: number): Promise<FeatActionResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/add`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ feat_id: featId }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.detail || `Failed to add feat: ${response.statusText}`);
    }
    
    return response.json();
  }

  static async removeFeat(characterId: number, featId: number): Promise<FeatActionResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/remove`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ feat_id: featId }),
    });
    
    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.detail || `Failed to remove feat: ${response.statusText}`);
    }
    
    return response.json();
  }

  static async getFeatDetails(characterId: number, featId: number): Promise<FeatDetailsResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/${featId}/details`);
    if (!response.ok) {
      throw new Error(`Failed to fetch feat details: ${response.statusText}`);
    }
    return response.json();
  }

  static async validateFeat(characterId: number, featId: number): Promise<FeatValidationResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/${featId}/validate`);
    if (!response.ok) {
      throw new Error(`Failed to validate feat: ${response.statusText}`);
    }
    return response.json();
  }

  // Skills API methods
  static async getSkillsState(characterId: number): Promise<SkillsStateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/state`);
    if (!response.ok) {
      throw new Error(`Failed to fetch skills state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateSkills(characterId: number, skills: Record<number, number>): Promise<SkillsUpdateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/update`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ skills }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update skills: ${response.statusText}`);
    }
    return response.json();
  }

  static async resetSkills(characterId: number): Promise<SkillsUpdateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/reset`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        preserve_class_skills: false,
        refund_percentage: 100
      }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to reset skills: ${response.statusText}`);
    }
    return response.json();
  }

  // Attributes API methods
  static async getAttributesState(characterId: number): Promise<AbilitiesStateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/abilities`);
    if (!response.ok) {
      throw new Error(`Failed to fetch abilities state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAttributes(characterId: number, attributes: Record<string, number>): Promise<AbilitiesUpdateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/abilities/update`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ attributes: attributes }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update abilities: ${response.statusText}`);
    }
    return response.json();
  }

  // Alignment API methods
  static async getAlignment(characterId: number): Promise<AlignmentResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/alignment`);
    if (!response.ok) {
      throw new Error(`Failed to fetch alignment: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAlignment(characterId: number, alignment: { lawChaos: number; goodEvil: number }): Promise<AlignmentUpdateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/alignment`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(alignment),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update alignment: ${response.statusText}`);
    }
    return response.json();
  }

  // Combat stats API methods
  static async updateArmorClass(characterId: number, naturalAC: number): Promise<CombatUpdateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/combat/update-ac`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ natural_ac: naturalAC }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update armor class: ${response.statusText}`);
    }
    return response.json();
  }

  // Initiative API methods
  static async updateInitiativeBonus(characterId: number, initiativeBonus: number): Promise<CombatUpdateResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/combat/update-initiative`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ initiative_bonus: initiativeBonus }),
    });
    
    if (!response.ok) {
      throw new Error(`Failed to update initiative bonus: ${response.statusText}`);
    }
    return response.json();
  }

  // Saving throws API methods
  static async updateSavingThrows(characterId: number, saveUpdates: Record<string, number>): Promise<{ success: boolean; updated: string[] }> {
    // Use misc-bonus endpoint for each save type (backend doesn't have bulk update)
    const promises = Object.entries(saveUpdates).map(([saveType, value]) =>
      fetch(`${API_BASE_URL}/characters/${characterId}/saves/misc-bonus`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ save_type: saveType, value }),
      })
    );
    
    const responses = await Promise.all(promises);
    const failedResponses = responses.filter(r => !r.ok);
    
    if (failedResponses.length > 0) {
      throw new Error(`Failed to update saving throws: ${failedResponses[0].statusText}`);
    }
    
    return { success: true, updated: Object.keys(saveUpdates) };
  }

  // Race manager API methods
  static async getRaceData(characterId: number): Promise<RaceDataResponse> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/race/current`);
    if (!response.ok) {
      throw new Error(`Failed to fetch race data: ${response.statusText}`);
    }
    return response.json();
  }

  // Map backend data structure to frontend interface
  private static mapBackendToFrontend(backendData: Record<string, unknown>): CharacterData {
    // Handle null or undefined input
    if (!backendData) {
      throw new Error('No character data received from backend');
    }
    
    console.log('Mapping backend data:', backendData);
    
    // Handle FastAPI response format - extract from nested structure
    const summary = (backendData.summary as Record<string, unknown>) || backendData;
    const info = (backendData.info as Record<string, unknown>) || {};
    const abilities = (backendData.abilities as Record<string, unknown>) || summary.abilities || {};
    const classesData = (backendData.classes as Record<string, unknown>) || summary.classes || {};
    const alignmentData = (backendData.alignment as Record<string, unknown>) || summary.alignment || {};
    
    // Extract character ID - prefer from info, then summary, then root
    const characterId = info.id || summary.id || backendData.id;
    
    // Extract name - prefer from summary, then info, then construct from parts
    const name = summary.name || info.full_name || 
                 `${info.first_name || ''} ${info.last_name || ''}`.trim() || 
                 'Unknown Character';
    
    // Extract classes array
    const classesArray = ((classesData as { classes: Array<Record<string, unknown>> })?.classes) || [];
    
    // Map alignment from law_chaos and good_evil
    const alignmentMap: { [key: string]: string } = {
      '0_0': 'True Neutral', '0_1': 'Neutral Good', '0_2': 'Neutral Evil',
      '1_0': 'Lawful Neutral', '1_1': 'Lawful Good', '1_2': 'Lawful Evil', 
      '2_0': 'Chaotic Neutral', '2_1': 'Chaotic Good', '2_2': 'Chaotic Evil',
    };
    
    const lawChaos = (alignmentData as { law_chaos?: number }).law_chaos || 0;
    const goodEvil = (alignmentData as { good_evil?: number }).good_evil || 0;
    const alignmentKey = `${Math.floor(lawChaos / 50)}_${Math.floor(goodEvil / 50)}`;
    const alignmentString = (alignmentData as { alignment_string?: string }).alignment_string || alignmentMap[alignmentKey] || 'True Neutral';
    
    return {
      id: typeof characterId === 'string' ? parseInt(characterId) : (characterId as number) || undefined,
      name: String(name),
      race: String(summary.race || info.race_name || 'Unknown'),
      subrace: summary.subrace ? String(summary.subrace) : undefined,
      gender: info.gender === 0 || info.gender === 'Male' ? 'Male' : 'Female',
      classes: classesArray.map((cls) => ({
        name: String(cls.name || 'Unknown Class'),
        level: Number(cls.level || 1)
      })),
      alignment: String(alignmentString),
      deity: String(summary.deity || 'None'),
      level: Number(summary.level || (classesData as { total_level?: number }).total_level || info.level || 1),
      experience: Number(summary.experience || info.experience || 0),
      hitPoints: Number(summary.current_hit_points || summary.hit_points || 10),
      maxHitPoints: Number(summary.max_hit_points || 10),
      abilities: {
        strength: Number((abilities as { strength?: number }).strength || 10),
        dexterity: Number((abilities as { dexterity?: number }).dexterity || 10),
        constitution: Number((abilities as { constitution?: number }).constitution || 10),
        intelligence: Number((abilities as { intelligence?: number }).intelligence || 10),
        wisdom: Number((abilities as { wisdom?: number }).wisdom || 10),
        charisma: Number((abilities as { charisma?: number }).charisma || 10)
      },
      saves: {
        fortitude: Number((backendData.saves as SavesData)?.fortitude || 0),
        reflex: Number((backendData.saves as SavesData)?.reflex || 0),
        will: Number((backendData.saves as SavesData)?.will || 0)
      },
      armorClass: Number(summary.armor_class || 10),
      gold: Number(summary.gold || 0),
      location: String(summary.area_name || backendData.area_name || ''),
      portrait: summary.portrait ? String(summary.portrait) : undefined,
      customPortrait: summary.custom_portrait ? String(summary.custom_portrait) : undefined,
      // Combat stats
      baseAttackBonus: Number(summary.base_attack_bonus || 0),
      // Character progress
      totalSkillPoints: summary.skill_points_total ? Number(summary.skill_points_total) : undefined,
      availableSkillPoints: summary.skill_points_available ? Number(summary.skill_points_available) : undefined,
      totalFeats: Number(summary.total_feats || 0),
      // Physical stats
      movementSpeed: 30,
      size: 'Medium',
      initiative: Math.floor(((Number((abilities as { dexterity?: number }).dexterity) || 10) - 10) / 2),
      // Campaign info
      campaignName: String(backendData.campaign_name || ''),
      moduleName: String(backendData.module_name || ''),
      // Quest data
      completedQuests: Number((backendData.quest_details as CharacterData['questDetails'])?.summary?.completed_quests || 0),
      currentQuests: Number((backendData.quest_details as CharacterData['questDetails'])?.summary?.active_quests || 0),
      questDetails: backendData.quest_details as CharacterData['questDetails']
    };
  }
}