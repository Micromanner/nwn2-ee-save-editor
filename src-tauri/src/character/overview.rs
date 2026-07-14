use serde::{Deserialize, Serialize};

use crate::loaders::GameData;

use super::Character;
use super::classes::{ClassSummaryEntry, XpProgress};
use super::feats::DomainInfo;
use super::identity::Alignment;
use super::types::{AbilityModifiers, AbilityScores, HitPoints, SaveBonuses};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OverviewState {
    pub first_name: String,
    pub last_name: String,
    pub full_name: String,
    pub race_id: i32,
    pub race_name: String,
    pub subrace: Option<String>,
    pub gender: String,
    pub gender_id: i32,
    pub age: i32,
    pub deity: String,
    pub alignment: Alignment,
    pub alignment_string: String,
    pub description: String,

    pub total_level: i32,
    pub experience: i32,
    pub xp_progress: XpProgress,
    pub classes: Vec<ClassSummaryEntry>,

    pub hit_points: HitPoints,
    pub armor_class: i32,
    pub base_attack_bonus: i32,
    pub saving_throws: SaveBonuses,

    pub gold: u32,
    pub skill_points_available: i32,

    pub size_name: String,

    pub background: Option<String>,
    pub domains: Vec<DomainInfo>,

    pub ability_scores: AbilityScores,
    pub ability_modifiers: AbilityModifiers,
    pub melee_attack_bonus: i32,
    pub ranged_attack_bonus: i32,
    pub initiative: i32,
    pub movement_speed: i32,
    pub total_feats: i32,
    pub known_spells_count: i32,
    pub spent_skill_points: i32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_info: Option<CampaignOverviewInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignOverviewInfo {
    pub campaign_name: Option<String>,
    pub module_name: Option<String>,
    pub area_name: Option<String>,
    pub game_year: Option<u32>,
    pub game_month: Option<u8>,
    pub game_day: Option<u8>,
    pub game_hour: Option<u8>,
    pub game_act: Option<String>,
    pub last_saved: Option<String>,
    pub difficulty: Option<String>,
}

impl Character {
    pub fn get_overview_state(
        &self,
        game_data: &GameData,
        decoder: &crate::services::item_property_decoder::ItemPropertyDecoder,
    ) -> OverviewState {
        let alignment = self.alignment();
        let gender_id = self.gender();
        let gender_str = match gender_id {
            0 => "Male",
            1 => "Female",
            _ => "Unknown",
        };

        let domains = self
            .get_available_domains(game_data)
            .into_iter()
            .filter(|d| d.has_domain)
            .collect();

        let hit_points = HitPoints {
            current: self.current_hp(),
            max: self.max_hp(),
            temp: self.temp_hp(),
        };

        let spellcraft = self.skill_rank(crate::character::types::SkillId(16));
        let spellcraft_save_bonus = spellcraft / 5;

        let base_saves = self.calculate_base_saves(game_data);
        let saving_throws = SaveBonuses {
            fortitude: base_saves.fortitude + spellcraft_save_bonus,
            reflex: base_saves.reflex + spellcraft_save_bonus,
            will: base_saves.will + spellcraft_save_bonus,
        };

        let bab = self.calculate_bab(game_data);
        let armor_class = self.get_armor_class(game_data, decoder).total;
        let attack_bonuses = self.get_attack_bonuses(game_data, decoder);
        let initiative = self.get_initiative_breakdown(game_data, decoder);
        let movement = self.get_movement_speed(game_data);

        let total_scores = self.get_total_abilities(game_data, decoder);
        let ability_modifiers = AbilityModifiers::from_scores(&total_scores);

        let first_name = super::identity::resolve_localized_name(
            self.first_name(),
            self.first_name_strref(),
            game_data,
        );
        let last_name = super::identity::resolve_localized_name(
            self.last_name(),
            self.last_name_strref(),
            game_data,
        );
        let full_name = format!("{first_name} {last_name}").trim().to_string();

        OverviewState {
            first_name,
            last_name,
            full_name,
            race_id: self.race_id().0,
            race_name: self.race_name(game_data),
            subrace: self.subrace(),
            gender: gender_str.to_string(),
            gender_id,
            age: self.age(),
            deity: self.deity(),
            alignment,
            alignment_string: alignment.alignment_string(),
            description: self.description(),

            total_level: self.total_level(),
            experience: self.experience(),
            xp_progress: self.get_xp_progress(game_data),
            classes: self.get_class_summary(game_data),

            hit_points,
            armor_class,
            base_attack_bonus: bab,
            saving_throws,

            gold: self.gold(),
            skill_points_available: self.get_available_skill_points(),

            size_name: self.get_size_name_from_2da(self.creature_size(), game_data),

            background: self.background(game_data),
            domains,

            ability_scores: total_scores,
            ability_modifiers,
            melee_attack_bonus: attack_bonuses.melee,
            ranged_attack_bonus: attack_bonuses.ranged,
            initiative: initiative.total,
            movement_speed: movement.current,
            total_feats: self.feat_count() as i32,
            known_spells_count: self.count_unique_spells(),
            spent_skill_points: self.calculate_total_spent_with_costs(game_data),
            campaign_info: None,
        }
    }

    pub fn count_unique_spells(&self) -> i32 {
        use crate::parsers::gff::GffValue;
        use std::collections::HashSet;

        let Some(class_list) = self.get_list("ClassList") else {
            return 0;
        };

        let mut unique: HashSet<i32> = HashSet::new();

        for class_entry in class_list {
            for level in 0..=9 {
                let known_key = format!("KnownList{level}");
                if let Some(GffValue::ListOwned(list)) = class_entry.get(&known_key) {
                    for entry in list {
                        if let Some(id) = entry
                            .get("Spell")
                            .and_then(crate::character::gff_helpers::gff_value_to_i32)
                        {
                            unique.insert(id);
                        }
                    }
                }

                let mem_key = format!("MemorizedList{level}");
                if let Some(GffValue::ListOwned(list)) = class_entry.get(&mem_key) {
                    for entry in list {
                        if let Some(id) = entry
                            .get("Spell")
                            .and_then(crate::character::gff_helpers::gff_value_to_i32)
                        {
                            unique.insert(id);
                        }
                    }
                }
            }
        }

        unique.len() as i32
    }
}
