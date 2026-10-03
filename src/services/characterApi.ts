import { invoke } from '@tauri-apps/api/core';
import type { BackendFeatPrerequisites } from '@/components/Feats/types';
import type { SpellClassLevel } from '@/components/Spells/types';

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
  class_id?: number;
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

export interface SetDeityResponse {
  success: boolean;
  deity: string;
}

export interface SetBiographyResponse {
  success: boolean;
  biography_length: number;
}

export interface DamageResistance {
  type: string;
  amount: number;
}

export interface SaveCharacterClass {
  name: string;
  level: number;
}

export interface SaveCharacterOption {
  player_index: number;
  name: string | null;
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
  prerequisites: BackendFeatPrerequisites;
  has_feat?: boolean;
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
  auto_added_feats: AutoAddedFeat[];
  auto_modified_abilities: AbilityChange[];
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
  prerequisites: BackendFeatPrerequisites;
  has_feat?: boolean;
  effects?: Record<string, unknown>;
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
  class_levels: SpellClassLevel[];
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
  initiative?: number;
  // Ability modifiers from overview dashboard
  abilityModifiers?: { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };
  // Campaign stats
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
        background: string | null;
        domains: Array<{ id: number; name: string; description: string; has_domain: boolean }>;
        ability_scores: { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };
        ability_modifiers: { Str: number; Dex: number; Con: number; Int: number; Wis: number; Cha: number };
        melee_attack_bonus: number;
        ranged_attack_bonus: number;
        initiative: number;
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
        baseAttackBonus: overview.base_attack_bonus,
        meleeAttackBonus: overview.melee_attack_bonus,
        rangedAttackBonus: overview.ranged_attack_bonus,
        initiative: overview.initiative,
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
        classes: overview.classes.map(c => ({ class_id: c.class_id, name: c.name, level: c.level })),
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
      throw error;
    }
  }

  static async listSaveCharacters(savePath: string): Promise<SaveCharacterOption[]> {
    try {
      return await invoke<SaveCharacterOption[]>('list_save_characters', { filePath: savePath });
    } catch (error) {
      console.error('Error listing save characters:', error);
      throw error;
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
      throw error;
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
      throw error;
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
    return invoke<FeatActionResponse>('add_feat', { featId });
  }

  static async removeFeat(characterId: number, featId: number): Promise<void> {
    await invoke('remove_feat', { featId });
  }

  static async getFeatDetails(characterId: number, featId: number): Promise<FeatDetailsResponse> {
    return await invoke('get_feat_info', { featId: featId });
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
        class_levels: SpellClassLevel[];
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
      classId: options.class_id,
      spellLevel: options.levels ? parseInt(options.levels.split(',')[0], 10) : undefined,
      schoolIds: options.schools ? options.schools.split(',').map(name => SPELL_SCHOOL_NAME_TO_ID[name]).filter((id): id is number => id !== undefined) : undefined,
      search: options.search,
    });

    return {
      spells: result.spells.map(s => ({
        id: s.id,
        name: s.name,
        description: s.description,
        icon: s.icon,
        school_id: s.school_id,
        school_name: s.school_name,
        level: s.class_levels[0].level,
        available_classes: s.class_levels.map(cl => cl.class_name),
        class_levels: s.class_levels,
      })),
      pagination: result.pagination,
    };
  }

  static async addSpell(characterId: number, spellId: number, classIndex: number): Promise<void> {
    await invoke('add_known_spell', { classIndex, spellId });
  }

  static async removeSpell(characterId: number, spellId: number, classIndex: number, spellLevel: number): Promise<void> {
    await invoke('remove_known_spell', { classIndex, spellLevel, spellId });
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
          throw error;
      }
  }

  static async resetSkills(characterId: number): Promise<SkillsUpdateResponse> {
    await invoke('reset_all_skills');
    return { success: true, updated_skills: {}, available_points: 0 };
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

  static async setBiography(characterId: number, biography: string): Promise<SetBiographyResponse> {
    await invoke('set_biography', { description: biography });
    return { success: true, biography_length: biography.length };
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
}
