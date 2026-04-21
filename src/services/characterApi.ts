import { invoke } from '@tauri-apps/api/core';

// Helper to format alignment string from numbers
const formatAlignment = (lawChaos: number, goodEvil: number): string => {
  const lc = lawChaos > 66 ? 'Chaotic' : lawChaos > 33 ? 'Neutral' : 'Lawful';
  const ge = goodEvil > 66 ? 'Evil' : goodEvil > 33 ? 'Neutral' : 'Good';
  if (lc === 'Neutral' && ge === 'Neutral') return 'True Neutral';
  return `${lc} ${ge}`;
};

const SPELL_SCHOOL_NAME_TO_ID: Record<string, number> = {
  General: 0, Abjuration: 1, Conjuration: 2, Divination: 3,
  Enchantment: 4, Evocation: 5, Illusion: 6, Necromancy: 7,
  Transmutation: 8, Universal: 0,
};

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
  aliases?: string;
  alignment?: string;
  portfolio?: string;
  favored_weapon?: string;
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
  first_name: string;
  last_name: string;
  full_name: string;
  age: number;
  description: string;
  background?: string;
  experience: number;
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

export interface SaveCharacterClass {
  name: string;
  level: number;
}

export interface SaveCharacterOption {
  player_index: number;
  name: string;
  race: string;
  total_level: number;
  classes: SaveCharacterClass[];
}

export interface FeatResponse {
  id: number;
  feat_id?: number;
  label: string;
  name: string;
  type: number;
  category?: string;
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

export interface AutoAddedFeat {
  feat_id: number;
  label: string;
}

export interface AbilityChange {
  ability: string;
  old_value: number;
  new_value: number;
}

export interface FeatActionResponse {
  feat_id: number;
  success: boolean;
  message: string;
  auto_added_feats?: AutoAddedFeat[];
  auto_modified_abilities?: AbilityChange[];
  character_feats?: FeatResponse[];
}

export interface FeatDetailsResponse {
  id: number;
  feat_id?: number;
  label: string;
  name: string;
  description: string;
  type: number;
  category?: string;
  protected: boolean;
  custom: boolean;
  icon?: string;
  prerequisites?: Record<string, unknown>;
  can_take?: boolean;
  missing_requirements?: string[];
  has_feat?: boolean;
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
  // Primary data from Rust get_skills_state command
  class_skills: Array<{
    id: number;
    name: string;
    current_ranks: number;
    max_ranks: number;
    total_modifier: number;
    is_class_skill: boolean;
    untrained: boolean;
  }>;
  cross_class_skills: Array<{
    id: number;
    name: string;
    current_ranks: number;
    max_ranks: number;
    total_modifier: number;
    is_class_skill: boolean;
    untrained: boolean;
  }>;
  total_available: number;
  spent_points: number;
  // Legacy compatibility fields
  character_skills: Record<number, SkillEntry>;
  available_points: number;
  total_points: number;
  skill_info: Record<number, SkillInfo>;
  skills: Array<{
    id: number;
    name: string;
    current_ranks: number;
    max_ranks: number;
    total_modifier: number;
    is_class_skill: boolean;
    untrained: boolean;
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
  law_chaos: number;
  good_evil: number;
  alignment_string?: string;
}

export interface AlignmentUpdateResponse {
  success: boolean;
  law_chaos: number;
  good_evil: number;
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
  gender_id: number;
  age: number;
  alignment: string;
  alignmentValues?: { law_chaos: number; good_evil: number };
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
  // Ability modifiers from overview dashboard
  abilityModifiers?: { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };
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
  gameTime?: { year: number | null; month: number | null; day: number | null; hour: number | null };
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
  race_id?: number;
}

export class CharacterAPI {
  static async getCharacterState(characterId: number): Promise<CharacterData> {
    try {
      // Use the new aggregated state command - single call instead of 17 parallel calls
      const overview = await invoke<{
        first_name: string;
        last_name: string;
        full_name: string;
        race_id: number;
        race_name: string;
        subrace: string | null;
        gender: string;
        gender_id: number;
        age: number;
        deity: string;
        alignment: { law_chaos: number; good_evil: number };
        alignment_string: string;
        description: string;
        total_level: number;
        experience: number;
        xp_progress: { current_xp: number; next_level_xp: number; progress_percent: number };
        classes: Array<{ class_id: number; name: string; level: number; hit_die: number }>;
        hit_points: { current: number; max: number; temp: number };
        armor_class: number;
        base_attack_bonus: number;
        saving_throws: { fortitude: number; reflex: number; will: number };
        gold: number;
        skill_points_available: number;
        size_name: string;
        background: string | null;
        domains: Array<{ id: number; name: string; description: string; has_domain: boolean }>;
        ability_scores: { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };
        ability_modifiers: { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };
        melee_attack_bonus: number;
        ranged_attack_bonus: number;
        initiative: number;
        movement_speed: number;
        total_feats: number;
        known_spells_count: number;
        spent_skill_points: number;
        campaign_info?: {
          campaign_name: string | null;
          module_name: string | null;
          area_name: string | null;
          game_year: number | null;
          game_month: number | null;
          game_day: number | null;
          game_hour: number | null;
          game_act: string | null;
          last_saved: string | null;
          difficulty: string | null;
        };
      }>('get_overview_state');

      return {
        id: characterId,
        name: overview.full_name || `${overview.first_name} ${overview.last_name}`.trim(),
        first_name: overview.first_name,
        last_name: overview.last_name,
        race: overview.race_name,
        race_id: overview.race_id,
        subrace: overview.subrace || undefined,
        gender: overview.gender,
        gender_id: overview.gender_id,
        age: overview.age,
        alignment: overview.alignment_string,
        alignmentValues: { law_chaos: overview.alignment.law_chaos, good_evil: overview.alignment.good_evil },
        deity: overview.deity,
        biography: overview.description,
        level: overview.total_level,
        experience: overview.experience,
        hitPoints: overview.hit_points.current,
        maxHitPoints: overview.hit_points.max,
        abilities: {
          strength: overview.ability_scores.Str,
          dexterity: overview.ability_scores.Dex,
          constitution: overview.ability_scores.Con,
          intelligence: overview.ability_scores.Int,
          wisdom: overview.ability_scores.Wis,
          charisma: overview.ability_scores.Cha,
        },
        saves: {
          fortitude: overview.saving_throws.fortitude,
          reflex: overview.saving_throws.reflex,
          will: overview.saving_throws.will,
        },
        armorClass: overview.armor_class,
        gold: overview.gold,
        background: overview.background ? { name: overview.background, id: 0 } : undefined,
        domains: overview.domains,
        size: overview.size_name,
        baseAttackBonus: overview.base_attack_bonus,
        meleeAttackBonus: overview.melee_attack_bonus,
        rangedAttackBonus: overview.ranged_attack_bonus,
        initiative: overview.initiative,
        movementSpeed: overview.movement_speed,
        totalFeats: overview.total_feats,
        knownSpells: overview.known_spells_count,
        totalSkillPoints: overview.spent_skill_points,
        skill_points_available: overview.skill_points_available,
        abilityModifiers: {
          Str: overview.ability_modifiers.Str,
          Dex: overview.ability_modifiers.Dex,
          Con: overview.ability_modifiers.Con,
          Int: overview.ability_modifiers.Int,
          Wis: overview.ability_modifiers.Wis,
          Cha: overview.ability_modifiers.Cha,
        },
        campaignName: overview.campaign_info?.campaign_name ?? undefined,
        moduleName: overview.campaign_info?.module_name ?? undefined,
        location: overview.campaign_info?.area_name ?? undefined,
        gameAct: overview.campaign_info?.game_act != null ? Number(overview.campaign_info.game_act) || undefined : undefined,
        difficultyLabel: overview.campaign_info?.difficulty ?? undefined,
        lastSaved: overview.campaign_info?.last_saved ?? undefined,
        gameTime: overview.campaign_info ? {
          year: overview.campaign_info.game_year,
          month: overview.campaign_info.game_month,
          day: overview.campaign_info.game_day,
          hour: overview.campaign_info.game_hour,
        } : undefined,
        classes: overview.classes.map(c => ({ name: c.name, level: c.level })),
        derived_stats: {
          armor_class: overview.armor_class,
          current_hit_points: overview.hit_points.current,
          max_hit_points: overview.hit_points.max,
          base_attack_bonus: overview.base_attack_bonus,
          fortitude: overview.saving_throws.fortitude,
          reflex: overview.saving_throws.reflex,
          will: overview.saving_throws.will,
          skill_points_available: overview.skill_points_available,
        },
      } as CharacterData;

    } catch (error) {
      console.error("Error fetching character state:", error);
      throw new Error(String(error));
    }
  }

  static async getCharacterDetails(characterId: number): Promise<CharacterData> {
    // In Rust mode, details and state are effectively the same aggregation
    return this.getCharacterState(characterId);
  }

  static async listCharacters(): Promise<CharacterData[]> {
    return [];
  }

  static async listSaveCharacters(savePath: string): Promise<SaveCharacterOption[]> {
    try {
      return await invoke<SaveCharacterOption[]>('list_save_characters', { filePath: savePath });
    } catch (error) {
      console.error('Error listing save characters:', error);
      throw new Error(String(error));
    }
  }

  static async importCharacter(savePath: string, playerIndex?: number): Promise<{id: number; name: string}> {
    try {
      await invoke('load_character', { filePath: savePath, playerIndex });
      const name = await invoke<string>('get_character_name');
      return {
        id: Date.now(), // Unique session ID to trigger state updates
        name: name || 'Unknown Character'
      };
    } catch (error) {
      console.error("Error importing character:", error);
      throw new Error(String(error));
    }
  }

  static async getAvailableDeities(): Promise<AvailableDeitiesResponse> {
      try {
          const deities = await invoke<Deity[]>('get_available_deities');
          return { deities, total: deities.length };
      } catch (error) {
          console.error("Error fetching available deities:", error);
          return { deities: [], total: 0 };
      }
  }

  static async updateCharacter(characterId: number, updates: Partial<{ first_name: string; last_name: string; age: number; gender: number; deity: string; description: string; alignment: [number, number]; experience: number; [key: string]: unknown }>): Promise<CharacterData> {
    try {
      // Use the batch update command
      const updatePayload: Record<string, unknown> = {};

      if (updates.first_name !== undefined) updatePayload.first_name = updates.first_name;
      if (updates.last_name !== undefined) updatePayload.last_name = updates.last_name;
      if (updates.age !== undefined) updatePayload.age = updates.age;
      if (updates.gender !== undefined) updatePayload.gender = updates.gender;
      if (updates.deity !== undefined) updatePayload.deity = updates.deity;
      if (updates.description !== undefined) updatePayload.description = updates.description;
      if (updates.alignment !== undefined) updatePayload.alignment = updates.alignment;
      if (updates.experience !== undefined) updatePayload.experience = updates.experience;

      // Use the batch update_character command
      await invoke('update_character', { updates: updatePayload });
      return this.getCharacterState(characterId);
    } catch (error) {
      console.error("Error updating character:", error);
      throw new Error(String(error));
    }
  }

  static async saveCharacter(characterId: number, updates: Record<string, unknown> = {}): Promise<SaveResult> {
    try {
       // 1. Apply updates
      if (Object.keys(updates).length > 0) {
        // Cast to partial for updateCharacter - simplistic but works for now logic
        await this.updateCharacter(characterId, updates as any);
      }

      // 2. Save
      await invoke('save_character', { filePath: null }); // Null = overwrite current

      return {
        success: true,
        changes: updates,
        backup_created: false // Rust side handles backup logic internally usually
      };
    } catch (error) {
      console.error("Error saving character:", error);
      throw new Error(String(error));
    }
  }

  static async getCharacterFeats(characterId: number, featType?: number): Promise<FeatsStateResponse> {
    try {
        // Rust returns summary and list separately usually, implies we need to aggregate or correct the command
        // For now, assuming get_feat_summary gives us the big object if mapped, OR we construct it.
        // Frontend expects: summary, all_feats, available_feats, etc.
        // We'll construct what we can.
        
        const summary = await invoke<any>('get_feat_summary');
        const feats = await invoke<FeatResponse[]>('get_feat_list');
        
        return {
            summary: summary, // Verify if shape matches
            all_feats: feats,
            available_feats: [], // TODO: call get_available_feats
            legitimate_feats: [], // TODO
            recommended_feats: []
        };
    } catch (error) {
        throw new Error(String(error));
    }
  }

  static async getAvailableFeats(characterId: number, featType?: number): Promise<AvailableFeatsResponse> {
      try {
          const feats = await invoke<FeatResponse[]>('get_all_feats');
          const filtered = featType
              ? feats.filter(f => (f.type & featType) !== 0)
              : feats;
          return { available_feats: filtered, total: filtered.length };
      } catch (error) {
          console.error("Error fetching available feats:", error);
          return { available_feats: [], total: 0 };
      }
  }

  static async getLegitimateFeats(
    _characterId: number,
    options: { page?: number; limit?: number; featType?: number; search?: string; category?: string; subcategory?: string } = {}
  ): Promise<LegitimateFeatsResponse> {
      try {
          const page = options.page || 1;
          const limit = options.limit || 50;

          const response = await invoke<{
              feats: FeatResponse[];
              total: number;
              page: number;
              limit: number;
              pages: number;
              has_next: boolean;
              has_previous: boolean;
          }>('get_filtered_feats', {
              page,
              limit,
              featType: options.featType || null,
              search: options.search || null,
          });

          return {
              feats: response.feats,
              pagination: {
                  page: response.page,
                  limit: response.limit,
                  total: response.total,
                  pages: response.pages,
                  has_next: response.has_next,
                  has_previous: response.has_previous,
              }
          };
      } catch (error) {
          console.error("Error fetching legitimate feats:", error);
          return { feats: [], pagination: { page: 1, limit: 10, total: 0, pages: 0, has_next: false, has_previous: false } };
      }
  }

  static async addFeat(characterId: number, featId: number): Promise<FeatActionResponse> {
    try {
        const result = await invoke<FeatActionResponse>('add_feat', { featId: featId });
        return {
            feat_id: featId,
            success: result.success,
            message: result.message || "Feat added",
            auto_added_feats: result.auto_added_feats || [],
            auto_modified_abilities: result.auto_modified_abilities || [],
        };
    } catch (error) {
        throw new Error(String(error));
    }
  }

  static async removeFeat(characterId: number, featId: number): Promise<FeatActionResponse> {
      try {
        const result = await invoke<any>('remove_feat', { featId: featId });
        return { 
            feat_id: featId, 
            success: result.success,
            message: result.message || "Feat removed"
        };
    } catch (error) {
        throw new Error(String(error));
    }
  }

  static async getFeatDetails(characterId: number, featId: number): Promise<FeatDetailsResponse> {
    return await invoke('get_feat_info', { featId: featId });
  }

  static async validateFeat(characterId: number, featId: number): Promise<FeatValidationResponse> {
    return await invoke('validate_feat_prerequisites', { featId: featId });
  }



  static async getLegitimateSpells(
    _characterId: number,
    options: {
      page?: number;
      limit?: number;
      class_id?: number;
      levels?: string;
      schools?: string;
      search?: string;
      show_all?: boolean;
    } = {}
  ): Promise<LegitimateSpellsResponse> {
    const result = await invoke<{
      spells: Array<{
        id: number;
        name: string;
        description?: string;
        icon: string;
        school_id?: number;
        school_name?: string;
        level: number;
        available_classes: string[];
      }>;
      pagination: {
        page: number;
        limit: number;
        total: number;
        pages: number;
        has_next: boolean;
        has_previous: boolean;
      };
    }>('get_character_available_spells', {
      page: options.page,
      limit: options.limit,
      classId: options.show_all ? undefined : options.class_id,
      spellLevel: options.levels ? parseInt(options.levels.split(',')[0], 10) : undefined,
      schoolIds: options.schools ? options.schools.split(',').map(name => SPELL_SCHOOL_NAME_TO_ID[name]).filter((id): id is number => id !== undefined) : undefined,
      search: options.search,
      showAll: options.show_all || undefined,
    });

    return {
      spells: result.spells.map(s => ({
        id: s.id,
        name: s.name,
        description: s.description,
        icon: s.icon,
        school_id: s.school_id,
        school_name: s.school_name,
        level: s.level,
        available_classes: s.available_classes,
      })),
      pagination: result.pagination,
    };
  }

  static async getAbilitySpells(): Promise<Array<{
    spell_id: number;
    name: string;
    icon: string;
    description?: string;
    school_name?: string;
    innate_level: number;
  }>> {
    return await invoke('get_character_ability_spells');
  }

  static async manageSpell(
    characterId: number,
    action: 'add' | 'remove',
    spellId: number,
    classIndex: number,
    spellLevel?: number
  ): Promise<SpellManageResponse> {
      try {
        const result = action === 'add'
          ? await invoke<{ success: boolean; message: string }>('add_known_spell', { classIndex, spellLevel: spellLevel ?? 0, spellId })
          : await invoke<{ success: boolean; message: string }>('remove_known_spell', { classIndex, spellLevel: spellLevel ?? 0, spellId });

        if (!result.success) {
          throw new Error(result.message);
        }

        return {
            message: result.message,
            spell_summary: {},
            has_unsaved_changes: true
        };
      } catch (error) {
          throw new Error(String(error));
      }
  }

  static async getSkillsState(characterId: number): Promise<SkillsStateResponse> {
    try {
        // Use the new get_skills_state command that returns properly structured data
        const state = await invoke<{
            class_skills: Array<{id: number; name: string; current_ranks: number; max_ranks: number; total_modifier: number; is_class_skill: boolean; untrained: boolean}>;
            cross_class_skills: Array<{id: number; name: string; current_ranks: number; max_ranks: number; total_modifier: number; is_class_skill: boolean; untrained: boolean}>;
            total_available: number;
            spent_points: number;
        }>('get_skills_state');

        return {
            class_skills: state.class_skills,
            cross_class_skills: state.cross_class_skills,
            total_available: state.total_available,
            spent_points: state.spent_points,
            character_skills: {},
            available_points: state.total_available - state.spent_points,
            total_points: state.total_available,
            skill_info: {},
            skills: [...state.class_skills, ...state.cross_class_skills],
            skill_points: { available: state.total_available - state.spent_points, spent: state.spent_points }
        };
    } catch (error) {
        throw new Error(String(error));
    }
  }

  static async updateSkills(characterId: number, skills: Record<number, number>): Promise<SkillsUpdateResponse> {
      try {
          for (const [skillId, rank] of Object.entries(skills)) {
              await invoke('set_skill_rank', { skillId: Number(skillId), ranks: rank });
          }
           return {
               success: true,
               updated_skills: skills,
               available_points: 0
           };
      } catch (error) {
          throw new Error(String(error));
      }
  }

  static async resetSkills(characterId: number): Promise<SkillsUpdateResponse> {
    await invoke('reset_all_skills');
    return { success: true, updated_skills: {}, available_points: 0 };
  }

  static async getAttributesState(characterId: number): Promise<AbilitiesStateResponse> {
    try {
        // Rust now serializes ability scores with Str, Dex, Con, Int, Wis, Cha keys directly
        type AbilityScores = { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };

        const [scores, modifiers, hp, saves, combat, kp_summary, base_scores] = await Promise.all([
             invoke<AbilityScores>('get_ability_scores'),
             invoke<AbilityScores>('get_ability_modifiers'),
             invoke<{current: number, max: number}>('get_hit_points'),
             invoke<{fortitude: {total: number}, reflex: {total: number}, will: {total: number}}>('get_save_summary'),
             invoke<{
                base_attack_bonus: number,
                armor_class: {total: number, breakdown: {natural: number, dex: number, armor: number, shield: number}},
                initiative: {total: number, dex: number, feat: number, misc: number}
             }>('get_combat_summary'),
             invoke<{total_points: number, available_points: number}>('get_ability_points_summary').catch(() => ({ total_points: 0, available_points: 0 })),
             invoke<AbilityScores>('get_base_ability_scores')
        ]);

        return {
            abilities: {
                strength: scores.Str,
                dexterity: scores.Dex,
                constitution: scores.Con,
                intelligence: scores.Int,
                wisdom: scores.Wis,
                charisma: scores.Cha
            },
            modifiers: {
                strength: modifiers.Str,
                dexterity: modifiers.Dex,
                constitution: modifiers.Con,
                intelligence: modifiers.Int,
                wisdom: modifiers.Wis,
                charisma: modifiers.Cha
            },

            // Fields for useAbilityScores hook - now direct from Rust
            base_attributes: base_scores,
            effective_attributes: scores,
            attribute_modifiers: modifiers,

            derived_stats: {
                hit_points: {
                    current: hp.current,
                    maximum: hp.max
                }
            },
            combat_stats: {
                armor_class: {
                    total: combat.armor_class.total,
                    components: {
                        natural: combat.armor_class.breakdown.natural,
                        dex: combat.armor_class.breakdown.dex,
                        armor: combat.armor_class.breakdown.armor,
                        shield: combat.armor_class.breakdown.shield
                    }
                },
                initiative: {
                    total: combat.initiative.total,
                    dex_modifier: combat.initiative.dex,
                    misc_bonus: combat.initiative.misc,
                    improved_initiative: combat.initiative.feat
                }
            },
            saving_throws: {
                fortitude: saves.fortitude,
                reflex: saves.reflex,
                will: saves.will
            },
            point_summary: {
                 total: kp_summary.total_points || 0,
                 spent: 0,
                 available: kp_summary.available_points || 0,
                 overdrawn: 0
            }
        } as any;
    } catch (error) {
        console.error("Error fetching attributes state:", error);
        throw error;
    }
  }

  static async updateAttributes(characterId: number, attributes: Record<string, number>): Promise<AbilitiesUpdateResponse> {
       // Rust now uses Str, Dex, Con, Int, Wis, Cha keys (matching frontend)
       type AbilityScores = { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };

       // Start with current base values to ensure we send a complete set
       const current = await invoke<AbilityScores>('get_base_ability_scores');
       const merged = { ...current, ...attributes };

       await invoke('set_all_ability_scores', { scores: merged });
       return { success: true, updated_abilities: attributes };
  }

  static async setAttribute(characterId: number, attribute: string, value: number): Promise<{ success: boolean; attribute: string; value: number }> {
     // Map string "Str"/"strength" to AbilityIndex
     // AbilityIndex in Rust: 0=Str, 1=Dex...
     const map: Record<string, number> = {
         'Str': 0, 'strength': 0, 'Strength': 0,
         'Dex': 1, 'dexterity': 1, 'Dexterity': 1,
         'Con': 2, 'constitution': 2, 'Constitution': 2,
         'Int': 3, 'intelligence': 3, 'Intelligence': 3,
         'Wis': 4, 'wisdom': 4, 'Wisdom': 4,
         'Cha': 5, 'charisma': 5, 'Charisma': 5
     };
     
     const index = map[attribute];
     if (index === undefined) throw new Error(`Unknown attribute ${attribute}`);
     
     await invoke('set_attribute', { ability: index, value: value });
     return { success: true, attribute, value };
  }

  static async getAlignment(characterId: number): Promise<AlignmentResponse> {
     const align = await invoke<{law_chaos: number, good_evil: number}>('get_alignment');
     return {
         law_chaos: align.law_chaos,
         good_evil: align.good_evil,
         alignment_string: formatAlignment(align.law_chaos, align.good_evil)
     };
  }

  static async updateAlignment(characterId: number, alignment: { law_chaos: number; good_evil: number }): Promise<AlignmentUpdateResponse> {
     const res = await invoke<{law_chaos: number, good_evil: number}>('set_alignment', {
         lawChaos: alignment.law_chaos,
         goodEvil: alignment.good_evil
     });
     return {
         success: true,
         law_chaos: res.law_chaos,
         good_evil: res.good_evil,
         alignment_string: formatAlignment(res.law_chaos, res.good_evil)
     };
  }

  static async getBiography(characterId: number): Promise<string> {
    const bio = await invoke<{biography: string}>('get_biography');
    return bio.biography;
  }

  static async setBiography(characterId: number, biography: string): Promise<SetBiographyResponse> {
    await invoke('set_biography', { description: biography });
    return { success: true, biography_length: biography.length };
  }

  static async getDeity(characterId: number): Promise<string> {
    return await invoke<string>('get_deity');
  }

  static async setDeity(characterId: number, deityName: string): Promise<SetDeityResponse> {
    await invoke('set_deity', { deity: deityName });
    return { success: true, deity: deityName };
  }

  static async updateHitPoints(characterId: number, currentHp?: number, maxHp?: number): Promise<{ success: boolean; current_hp?: number; max_hp?: number }> {
    const res = await invoke<{current: number, max: number}>('update_hit_points', { current: currentHp, max: maxHp });
    return { success: true, current_hp: res.current, max_hp: res.max };
  }

  static async updateArmorClass(characterId: number, naturalAC: number): Promise<CombatUpdateResponse> {
    try {
        await invoke('update_natural_armor', { value: naturalAC });
        return { success: true, updated_value: naturalAC, message: "AC Updated" };
    } catch (e) {
        throw new Error(String(e));
    }
  }

  static async updateInitiativeBonus(characterId: number, initiativeBonus: number): Promise<CombatUpdateResponse> {
     try {
        await invoke('update_initiative_bonus', { value: initiativeBonus });
        return { success: true, updated_value: initiativeBonus };
    } catch (e) {
        throw new Error(String(e));
    }
  }

  static async updateSavingThrows(characterId: number, saveUpdates: Record<string, number>): Promise<{ success: boolean; updated: string[] }> {
    // saveUpdates is map of saveType string -> value
    const promises: Promise<unknown>[] = [];
    
    for (const [saveType, value] of Object.entries(saveUpdates)) {
        // Map string saveType to int for Rust
        let id = -1;
        if (saveType.toLowerCase() === 'fortitude') id = 1;
        else if (saveType.toLowerCase() === 'reflex') id = 2;
        else if (saveType.toLowerCase() === 'will') id = 3;
        
        if (id !== -1) {
            promises.push(invoke('set_misc_save_bonus', { saveType: id, value: value }));
        }
    }
    
    await Promise.all(promises);
    return { success: true, updated: Object.keys(saveUpdates) };
  }

  static async getRaceData(characterId: number): Promise<RaceDataResponse> {
    // Construct race response from commands
     const id = await invoke<number>('get_race_id');
     const name = await invoke<string>('get_race_name');
     const sub = await invoke<string | null>('get_subrace');
     
     return {
         race_id: id,
         race_name: name,
         subrace: sub || '',
         size: 0,
         size_name: 'Medium',
         base_speed: 30,
         ability_modifiers: {},
         racial_feats: []
     };
  }

  static async getClassesState(characterId: number): Promise<any> {
      return await invoke('get_class_summary');
  }

  static async getSpellsState(characterId: number): Promise<any> {
      return await invoke('get_spell_summary');
  }

  static async getInventoryState(characterId: number): Promise<any> {
      return await invoke('get_inventory_summary');
  }

  static async getCombatState(characterId: number): Promise<any> {
      return await invoke('get_combat_summary');
  }
  
  static async getSaveSummary(characterId: number): Promise<any> {
      return await invoke('get_save_summary');
  }
}
