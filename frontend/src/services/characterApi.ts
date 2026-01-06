import DynamicAPI from '../lib/utils/dynamicApi';

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
  portrait?: string;
  // New fields
  background?: { name: string; id: number; icon?: string; description?: string };
  domains?: Array<{ name: string; id: number; icon?: string; description?: string }>;
}

export interface CharacterClass {
  name: string;
  level: number;
}

export interface Deity {
  id: number;
  name: string;
  description?: string;
  icon?: string;
}

export interface AvailableDeitiesResponse {
  deities: Deity[];
  total: number;
}

export interface DeityResponse {
  deity: string;
}

export interface SetDeityResponse {
  success: boolean;
  deity: string;
}

export interface BiographyResponse {
  biography: string;
}

export interface SetBiographyResponse {
  success: boolean;
  biography_length: number;
}

export interface DamageResistance {
  type: string;
  amount: number;
}

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
    background_feats?: FeatResponse[];
    domain_feats?: FeatResponse[];
  };
  all_feats: FeatResponse[];
  available_feats: FeatResponse[];
  legitimate_feats: FeatResponse[];
  recommended_feats: FeatResponse[];
}

export interface AvailableFeatsResponse {
  available_feats: FeatResponse[];
  total: number;
}

export interface LegitimateFeatsResponse {
  feats: FeatResponse[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    pages: number;
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

export interface SpellResponse {
  id: number;
  name: string;
  description?: string;
  icon?: string;
  school_id?: number;
  school_name?: string;
  level: number;
  cast_time?: string;
  range?: string;
  conjuration_time?: string;
  components?: string;
  target_type?: string;
  metamagic?: string;
  available_classes: string[];
}

export interface LegitimateSpellsResponse {
  spells: SpellResponse[];
  pagination: {
    page: number;
    limit: number;
    total: number;
    pages: number;
    has_next: boolean;
    has_previous: boolean;
  };
}

export interface SpellManageResponse {
  message: string;
  spell_summary: Record<string, unknown>;
  has_unsaved_changes: boolean;
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
  point_summary?: {
    total: number;
    spent: number;
    available: number;
    overdrawn: number;
  };
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
  age: number;
  alignment: string;
  deity: string;
  biography?: string;
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
  // New fields
  background?: { name: string; id: number; icon?: string; description?: string };
  domains?: Array<{ name: string; id: number; icon?: string; description?: string }>;
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
  languageLabel?: string;
  difficultyLabel?: string;
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
  classes?: CharacterClass[];
  first_name?: string;
  last_name?: string;
  skill_points_available?: number;
  has_unsaved_changes?: boolean;
}

export class CharacterAPI {
  static async getCharacterState(characterId: number): Promise<CharacterData> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/state`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character state: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  static async getCharacterDetails(characterId: number): Promise<CharacterData> {
    // Use summary endpoint instead of non-existent basic details endpoint
    const response = await DynamicAPI.fetch(`/characters/${characterId}/summary`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character details: ${response.statusText}`);
    }
    
    const data = await response.json();
    return this.mapBackendToFrontend(data);
  }

  static async listCharacters(): Promise<CharacterData[]> {
    return [];
  }

  static async importCharacter(savePath: string): Promise<{id: number; name: string}> {
    const response = await DynamicAPI.fetch(`/savegames/import`, {
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

  static async updateCharacter(characterId: number, updates: Partial<{ first_name: string; last_name: string; [key: string]: unknown }>): Promise<CharacterData> {
    await this.saveCharacter(characterId, updates);
    return this.getCharacterState(characterId);
  }

  static async saveCharacter(characterId: number, updates: Record<string, unknown> = {}): Promise<SaveResult> {
    const response = await DynamicAPI.fetch(`/${characterId}/update`, {
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

  static async getCharacterFeats(characterId: number, featType?: number): Promise<FeatsStateResponse> {
    const typeParam = featType !== undefined ? `?type=${featType}` : '';
    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/state${typeParam}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch character feats: ${response.statusText}`);
    }
    return response.json();
  }

  static async getAvailableFeats(characterId: number, featType?: number): Promise<AvailableFeatsResponse> {
    const typeParam = featType !== undefined ? `?feat_type=${featType}` : '';
    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/available${typeParam}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch available feats: ${response.statusText}`);
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
    if (options.featType !== undefined) params.append('feat_type', options.featType.toString());
    if (options.category) params.append('category', options.category);
    if (options.subcategory) params.append('subcategory', options.subcategory);
    if (options.page !== undefined) params.append('page', options.page.toString());
    if (options.limit !== undefined) params.append('limit', options.limit.toString());
    if (options.search) params.append('search', options.search);

    const queryString = params.toString();

    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/legitimate${queryString ? `?${queryString}` : ''}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch legitimate feats: ${response.statusText}`);
    }

    return await response.json();
  }

  static async addFeat(characterId: number, featId: number): Promise<FeatActionResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/add`, {
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
    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/remove`, {
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
    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/${featId}/details`);
    if (!response.ok) {
      throw new Error(`Failed to fetch feat details: ${response.statusText}`);
    }
    return response.json();
  }

  static async validateFeat(characterId: number, featId: number): Promise<FeatValidationResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/feats/${featId}/validate`);
    if (!response.ok) {
      throw new Error(`Failed to validate feat: ${response.statusText}`);
    }
    return response.json();
  }



  static async getLegitimateSpells(
    characterId: number,
    options: {
      levels?: string;
      schools?: string;
      page?: number;
      limit?: number;
      search?: string;
      class_id?: number;
    } = {}
  ): Promise<LegitimateSpellsResponse> {
    const params = new URLSearchParams();
    if (options.levels) params.append('levels', options.levels);
    if (options.schools) params.append('schools', options.schools);
    if (options.page !== undefined) params.append('page', options.page.toString());
    if (options.limit !== undefined) params.append('limit', options.limit.toString());
    if (options.search) params.append('search', options.search);
    if (options.class_id !== undefined) params.append('class_id', options.class_id.toString());

    const queryString = params.toString();

    const response = await DynamicAPI.fetch(`/characters/${characterId}/spells/legitimate${queryString ? `?${queryString}` : ''}`);
    if (!response.ok) {
      throw new Error(`Failed to fetch legitimate spells: ${response.statusText}`);
    }

    return await response.json();
  }

  static async manageSpell(
    characterId: number,
    action: 'add' | 'remove',
    spellId: number,
    classIndex: number,
    spellLevel?: number
  ): Promise<SpellManageResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/spells/manage`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        action,
        spell_id: spellId,
        class_index: classIndex,
        spell_level: spellLevel,
      }),
    });

    if (!response.ok) {
      const errorData = await response.json();
      throw new Error(errorData.detail || `Failed to ${action} spell: ${response.statusText}`);
    }

    return response.json();
  }

  static async getSkillsState(characterId: number): Promise<SkillsStateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/skills/state`);
    if (!response.ok) {
      throw new Error(`Failed to fetch skills state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateSkills(characterId: number, skills: Record<number, number>): Promise<SkillsUpdateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/skills/update`, {
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
    const response = await DynamicAPI.fetch(`/characters/${characterId}/skills/reset`, {
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

  static async getAttributesState(characterId: number): Promise<AbilitiesStateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/abilities`);
    if (!response.ok) {
      throw new Error(`Failed to fetch abilities state: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAttributes(characterId: number, attributes: Record<string, number>): Promise<AbilitiesUpdateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/abilities/update`, {
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

  static async setAttribute(characterId: number, attribute: string, value: number): Promise<{ success: boolean; attribute: string; value: number }> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/abilities/${attribute}/set`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ attribute, value, should_validate: true }),
    });

    if (!response.ok) {
        throw new Error(`Failed to set attribute: ${response.statusText}`);
    }
    return response.json();
  }

  static async getAlignment(characterId: number): Promise<AlignmentResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/alignment`);
    if (!response.ok) {
      throw new Error(`Failed to fetch alignment: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateAlignment(characterId: number, alignment: { lawChaos: number; goodEvil: number }): Promise<AlignmentUpdateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/alignment`, {
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

  static async getBiography(characterId: number): Promise<string> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/biography`);
    if (!response.ok) {
        throw new Error(`Failed to fetch biography: ${response.statusText}`);
    }
    const data: BiographyResponse = await response.json();
    return data.biography;
  }

  static async setBiography(characterId: number, biography: string): Promise<SetBiographyResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/biography`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ biography }),
    });

    if (!response.ok) {
        throw new Error(`Failed to set biography: ${response.statusText}`);
    }
    return response.json();
  }

  static async getAvailableDeities(characterId: number): Promise<Deity[]> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/available-deities`);
    if (!response.ok) {
        throw new Error(`Failed to fetch available deities: ${response.statusText}`);
    }
    const data: AvailableDeitiesResponse = await response.json();
    return data.deities;
  }

  static async getDeity(characterId: number): Promise<string> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/deity`);
    if (!response.ok) {
        throw new Error(`Failed to fetch deity: ${response.statusText}`);
    }
    const data: DeityResponse = await response.json();
    return data.deity;
  }

  static async setDeity(characterId: number, deityName: string): Promise<SetDeityResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/deity`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ deity: deityName }),
    });

    if (!response.ok) {
        throw new Error(`Failed to set deity: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateHitPoints(characterId: number, currentHp?: number, maxHp?: number): Promise<{ success: boolean; current_hp?: number; max_hp?: number }> {
    const payload: { current_hp?: number; max_hp?: number } = {};
    if (currentHp !== undefined) payload.current_hp = currentHp;
    if (maxHp !== undefined) payload.max_hp = maxHp;

    const response = await DynamicAPI.fetch(`/characters/${characterId}/combat/update-hp`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(payload),
    });

    if (!response.ok) {
        throw new Error(`Failed to update hit points: ${response.statusText}`);
    }
    return response.json();
  }

  static async updateArmorClass(characterId: number, naturalAC: number): Promise<CombatUpdateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/combat/update-ac`, {
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

  static async updateInitiativeBonus(characterId: number, initiativeBonus: number): Promise<CombatUpdateResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/combat/update-initiative`, {
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

  static async updateSavingThrows(characterId: number, saveUpdates: Record<string, number>): Promise<{ success: boolean; updated: string[] }> {
    // Use misc-bonus endpoint for each save type (backend doesn't have bulk update)
    const promises = Object.entries(saveUpdates).map(([saveType, value]) =>
      DynamicAPI.fetch(`/characters/${characterId}/saves/misc-bonus`, {
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

  static async getRaceData(characterId: number): Promise<RaceDataResponse> {
    const response = await DynamicAPI.fetch(`/characters/${characterId}/race/current`);
    if (!response.ok) {
      throw new Error(`Failed to fetch race data: ${response.statusText}`);
    }
    return response.json();
  }

  private static mapBackendToFrontend(backendData: Record<string, unknown>): CharacterData {
    if (!backendData) {
      throw new Error('No character data received from backend');
    }

    const summary = (backendData.summary as Record<string, unknown>) || backendData;
    const info = (backendData.info as Record<string, unknown>) || {};
    const abilities = (backendData.abilities as Record<string, unknown>) || summary.abilities || {};
    const classesData = (backendData.classes as Record<string, unknown>) || summary.classes || {};
    const alignmentData = (backendData.alignment as Record<string, unknown>) || summary.alignment || {};
    const combatStats = (backendData.combat_stats as Record<string, unknown>) || {};

    const characterId = info.id || summary.id || backendData.id;
    const name = summary.name || info.full_name ||
                 `${info.first_name || ''} ${info.last_name || ''}`.trim() ||
                 'Unknown Character';
    const classesArray = ((classesData as { classes: Array<Record<string, unknown>> })?.classes) || [];
    const alignmentString = (alignmentData as { alignment_string?: string }).alignment_string || 'True Neutral';
    
    return {
      id: typeof characterId === 'string' ? parseInt(characterId) : (characterId as number) || undefined,
      name: String(name),
      race: String(summary.race || info.race_name || 'Unknown'),
      subrace: summary.subrace ? String(summary.subrace) : undefined,
      gender: info.gender === 0 ? 'Male' : info.gender === 1 ? 'Female' : 'Other',
      age: Number(summary.age || 0),
      classes: classesArray.map((cls) => ({
        name: String(cls.name || 'Unknown Class'),
        level: Number(cls.level || 1)
      })),
      alignment: String(alignmentString),
      deity: String(summary.deity || ''),
      biography: String(summary.biography || ''),
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
      background: (summary.background as CharacterData['background']) || undefined,
      domains: (summary.domains as CharacterData['domains']) || [],
      gold: Number(summary.gold || 0),
      location: String(summary.area_name || backendData.area_name || ''),
      portrait: String(summary.portrait || info.portrait || ''),
      customPortrait: summary.custom_portrait ? String(summary.custom_portrait) : undefined,
      baseAttackBonus: Number(summary.base_attack_bonus || 0),
      totalSkillPoints: summary.skill_points_total ? Number(summary.skill_points_total) : undefined,
      availableSkillPoints: summary.skill_points_available ? Number(summary.skill_points_available) : undefined,
      totalFeats: Number(summary.total_feats || 0),
      movementSpeed: 30,
      size: 'Medium',
      initiative: Number((combatStats.initiative as { total?: number })?.total || 0),
      campaignName: String(backendData.campaign_name || ''),
      moduleName: String(backendData.module_name || ''),
      completedQuests: Number((backendData.quest_details as CharacterData['questDetails'])?.summary?.completed_quests || 0),
      currentQuests: Number((backendData.quest_details as CharacterData['questDetails'])?.summary?.active_quests || 0),
      questDetails: backendData.quest_details as CharacterData['questDetails'],
      difficultyLabel: String(summary.difficulty_label || 'Normal'),
      languageLabel: String(summary.language_label || 'English')
    };
  }
}