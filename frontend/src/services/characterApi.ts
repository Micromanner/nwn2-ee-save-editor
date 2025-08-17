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
}

// Base API URL - FastAPI server running on localhost:8000
const API_BASE_URL = 'http://localhost:8000/api';

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
  static async saveCharacter(characterId: number, updates: Record<string, unknown> = {}): Promise<{ success: boolean; changes: any; backup_created: boolean }> {
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
  static async getCharacterFeats(characterId: number, featType?: number): Promise<any> {
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
  ): Promise<{ feats: any[]; pagination: any; search?: string; category?: string; subcategory?: string }> {
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
      pagination: data.pagination || {},
      search: data.search,
      category: data.category,
      subcategory: data.subcategory
    };
  }

  static async addFeat(characterId: number, featId: number): Promise<any> {
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

  static async removeFeat(characterId: number, featId: number): Promise<any> {
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

  static async getFeatDetails(characterId: number, featId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/${featId}/details`);
    if (!response.ok) {
      throw new Error(`Failed to fetch feat details: ${response.statusText}`);
    }
    return response.json();
  }

  static async validateFeat(characterId: number, featId: number): Promise<{
    feat_id: number;
    can_take: boolean;
    reason: string;
    has_feat: boolean;
    missing_requirements: string[];
  }> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/feats/${featId}/validate`);
    if (!response.ok) {
      throw new Error(`Failed to validate feat: ${response.statusText}`);
    }
    return response.json();
  }

  // Skills API methods
  static async getSkillsState(characterId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/skills/state`);
    if (!response.ok) {
      throw new Error(`Failed to fetch skills state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateSkills(characterId: number, skills: Record<number, number>): Promise<any> {
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

  static async resetSkills(characterId: number): Promise<any> {
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
  static async getAttributesState(characterId: number): Promise<any> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/abilities`);
    if (!response.ok) {
      throw new Error(`Failed to fetch abilities state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAttributes(characterId: number, attributes: Record<string, number>): Promise<any> {
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
  static async getAlignment(characterId: number): Promise<{ lawChaos: number; goodEvil: number }> {
    const response = await fetch(`${API_BASE_URL}/characters/${characterId}/alignment`);
    if (!response.ok) {
      throw new Error(`Failed to fetch alignment: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAlignment(characterId: number, alignment: { lawChaos: number; goodEvil: number }): Promise<any> {
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
  static async updateArmorClass(characterId: number, naturalAC: number): Promise<any> {
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
  static async updateInitiativeBonus(characterId: number, initiativeBonus: number): Promise<any> {
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
  static async updateSavingThrows(characterId: number, saveUpdates: Record<string, number>): Promise<any> {
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
    const classesArray = (classesData.classes as Array<Record<string, unknown>>) || [];
    
    // Map alignment from law_chaos and good_evil
    const alignmentMap: { [key: string]: string } = {
      '0_0': 'True Neutral', '0_1': 'Neutral Good', '0_2': 'Neutral Evil',
      '1_0': 'Lawful Neutral', '1_1': 'Lawful Good', '1_2': 'Lawful Evil', 
      '2_0': 'Chaotic Neutral', '2_1': 'Chaotic Good', '2_2': 'Chaotic Evil',
    };
    
    const lawChaos = alignmentData.law_chaos || 0;
    const goodEvil = alignmentData.good_evil || 0;
    const alignmentKey = `${Math.floor(lawChaos / 50)}_${Math.floor(goodEvil / 50)}`;
    const alignmentString = alignmentData.alignment_string || alignmentMap[alignmentKey] || 'True Neutral';
    
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
      level: Number(summary.level || classesData.total_level || info.level || 1),
      experience: Number(summary.experience || info.experience || 0),
      hitPoints: Number(summary.current_hit_points || summary.hit_points || 10),
      maxHitPoints: Number(summary.max_hit_points || 10),
      abilities: {
        strength: Number(abilities.strength || 10),
        dexterity: Number(abilities.dexterity || 10),
        constitution: Number(abilities.constitution || 10),
        intelligence: Number(abilities.intelligence || 10),
        wisdom: Number(abilities.wisdom || 10),
        charisma: Number(abilities.charisma || 10)
      },
      saves: {
        fortitude: Number((backendData.saves as any)?.fortitude || 0),
        reflex: Number((backendData.saves as any)?.reflex || 0),
        will: Number((backendData.saves as any)?.will || 0)
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
      initiative: Math.floor(((abilities.dexterity as number || 10) - 10) / 2),
      // Campaign info
      campaignName: String(backendData.campaign_name || ''),
      moduleName: String(backendData.module_name || ''),
      // Quest data
      completedQuests: Number((backendData.quest_details as any)?.summary?.completed_quests || 0),
      currentQuests: Number((backendData.quest_details as any)?.summary?.active_quests || 0),
      questDetails: backendData.quest_details as any
    };
  }
}