//! Class categorization service for NWN2 classes.
//!
//! Provides categorization of classes by type (base, prestige, NPC) and
//! focus (combat, arcane, divine, skill, hybrid, stealth).

use std::collections::HashMap;

use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::character::classes::PrestigeRequirements;
use crate::loaders::GameData;
use crate::utils::parsing::{row_bool, row_int, row_str};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ClassType {
    Base,
    Prestige,
    Npc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ClassFocus {
    Combat,
    ArcaneCaster,
    DivineCaster,
    SkillSpecialist,
    Hybrid,
    StealthInfiltration,
}

impl ClassFocus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClassFocus::Combat => "combat",
            ClassFocus::ArcaneCaster => "arcane_caster",
            ClassFocus::DivineCaster => "divine_caster",
            ClassFocus::SkillSpecialist => "skill_specialist",
            ClassFocus::Hybrid => "hybrid",
            ClassFocus::StealthInfiltration => "stealth_infiltration",
        }
    }
}

impl ClassType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClassType::Base => "base",
            ClassType::Prestige => "prestige",
            ClassType::Npc => "npc",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ClassInfo {
    pub id: i32,
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub class_type: String,
    pub focus: String,
    pub max_level: i32,
    pub hit_die: i32,
    pub skill_points: i32,
    pub is_spellcaster: bool,
    pub has_arcane: bool,
    pub has_divine: bool,
    pub primary_ability: String,
    pub bab_progression: String,
    pub alignment_restricted: bool,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub prerequisites: Option<PrestigeRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct FocusInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Categories {
    pub base: HashMap<String, Vec<ClassInfo>>,
    pub prestige: HashMap<String, Vec<ClassInfo>>,
    pub npc: HashMap<String, Vec<ClassInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct CategorizedClasses {
    pub categories: Categories,
    pub focus_info: HashMap<String, FocusInfo>,
    pub total_classes: i32,
}

fn determine_class_type(row: &AHashMap<String, Option<String>>) -> ClassType {
    let player_class = row_int(row, "playerclass", 1);
    let max_level = row_int(row, "maxlevel", 0);

    if player_class == 0 && max_level == 0 {
        return ClassType::Npc;
    }

    if max_level > 0 {
        ClassType::Prestige
    } else {
        ClassType::Base
    }
}

fn determine_class_focus(row: &AHashMap<String, Option<String>>) -> ClassFocus {
    let has_arcane = row_bool(row, "hasarcane", false);
    let has_divine = row_bool(row, "hasdivine", false);
    let skill_points = row_int(row, "skillpointbase", 2);
    let hit_die = row_int(row, "hitdie", 8);
    let bab_table = row_str(row, "attackbonustable").unwrap_or_default();
    let is_full_bab = bab_table.eq_ignore_ascii_case("CLS_ATK_1");

    // Full-BAB classes are primarily martial even when they have half-caster
    // spell access (e.g. Paladin, Ranger have hasdivine=1 but are warriors,
    // not divine casters). Classify them by their non-spell profile.
    if is_full_bab {
        return if skill_points >= 6 {
            ClassFocus::SkillSpecialist
        } else {
            ClassFocus::Combat
        };
    }

    if has_arcane {
        ClassFocus::ArcaneCaster
    } else if has_divine {
        ClassFocus::DivineCaster
    } else if skill_points >= 6 {
        ClassFocus::SkillSpecialist
    } else if hit_die >= 10 {
        ClassFocus::Combat
    } else {
        ClassFocus::Hybrid
    }
}

fn is_placeholder_class(row: &AHashMap<String, Option<String>>) -> bool {
    let name = row_str(row, "name").unwrap_or_default().to_lowercase();
    let label = row_str(row, "label").unwrap_or_default().to_lowercase();

    name == "padding"
        || name == "****"
        || name.is_empty()
        || name == "none"
        || label == "padding"
        || label == "****"
        || label.is_empty()
        || label == "none"
}

fn get_focus_display_info() -> HashMap<String, FocusInfo> {
    let mut info = HashMap::new();

    info.insert(
        "combat".to_string(),
        FocusInfo {
            id: "combat".to_string(),
            name: "Combat".to_string(),
            description: "Warriors and martial specialists".to_string(),
            icon: String::new(),
        },
    );

    info.insert(
        "arcane_caster".to_string(),
        FocusInfo {
            id: "arcane_caster".to_string(),
            name: "Arcane Caster".to_string(),
            description: "Wizards, sorcerers, and arcane magic users".to_string(),
            icon: String::new(),
        },
    );

    info.insert(
        "divine_caster".to_string(),
        FocusInfo {
            id: "divine_caster".to_string(),
            name: "Divine Caster".to_string(),
            description: "Clerics, druids, and divine magic users".to_string(),
            icon: String::new(),
        },
    );

    info.insert(
        "skill_specialist".to_string(),
        FocusInfo {
            id: "skill_specialist".to_string(),
            name: "Skill Specialist".to_string(),
            description: "Rogues, bards, and skill-focused classes".to_string(),
            icon: String::new(),
        },
    );

    info.insert(
        "hybrid".to_string(),
        FocusInfo {
            id: "hybrid".to_string(),
            name: "Hybrid".to_string(),
            description: "Multi-role classes and unique specialists".to_string(),
            icon: String::new(),
        },
    );

    info.insert(
        "stealth_infiltration".to_string(),
        FocusInfo {
            id: "stealth_infiltration".to_string(),
            name: "Stealth & Infiltration".to_string(),
            description: "Assassins, spies, and shadow specialists".to_string(),
            icon: String::new(),
        },
    );

    info
}

pub fn get_categorized_classes(game_data: &GameData) -> CategorizedClasses {
    let mut categories = Categories {
        base: HashMap::new(),
        prestige: HashMap::new(),
        npc: HashMap::new(),
    };

    for focus in [
        ClassFocus::Combat,
        ClassFocus::ArcaneCaster,
        ClassFocus::DivineCaster,
        ClassFocus::SkillSpecialist,
        ClassFocus::Hybrid,
        ClassFocus::StealthInfiltration,
    ] {
        let focus_str = focus.as_str().to_string();
        categories.base.insert(focus_str.clone(), Vec::new());
        categories.prestige.insert(focus_str.clone(), Vec::new());
        categories.npc.insert(focus_str, Vec::new());
    }

    let Some(classes_table) = game_data.get_table("classes") else {
        tracing::warn!("Could not load classes table");
        return CategorizedClasses {
            categories,
            focus_info: get_focus_display_info(),
            total_classes: 0,
        };
    };

    let mut total_classes = 0;

    for row_idx in 0..classes_table.row_count() {
        let Ok(row) = classes_table.get_row(row_idx) else {
            continue;
        };

        if is_placeholder_class(&row) {
            continue;
        }

        let class_type = determine_class_type(&row);
        let class_focus = determine_class_focus(&row);

        let name_strref = row_int(&row, "name", -1);
        let name = if name_strref >= 0 {
            game_data.get_string(name_strref).filter(|n| !n.is_empty())
        } else {
            None
        }
        .or_else(|| row_str(&row, "label"))
        .unwrap_or_else(|| format!("Class{row_idx}"));

        let label = row_str(&row, "label").unwrap_or_else(|| format!("Class{row_idx}"));

        let has_arcane = row_bool(&row, "hasarcane", false);
        let has_divine = row_bool(&row, "hasdivine", false);

        let align_restrict = row_int(&row, "alignrestrict", 0);

        let description_strref = row_int(&row, "description", -1);
        let description = if description_strref >= 0 {
            game_data
                .get_string(description_strref)
                .filter(|d| !d.is_empty())
        } else {
            None
        };

        let prerequisites = if align_restrict > 0 {
            Some(PrestigeRequirements {
                alignment: crate::character::classes::AlignmentRestriction(align_restrict)
                    .decode_to_string(),
                ..Default::default()
            })
        } else {
            None
        };

        let icon = row_str(&row, "icon");

        let class_info = ClassInfo {
            id: row_idx as i32,
            name,
            label,
            class_type: class_type.as_str().to_string(),
            focus: class_focus.as_str().to_string(),
            max_level: row_int(&row, "maxlevel", 0),
            hit_die: row_int(&row, "hitdie", 8),
            skill_points: row_int(&row, "skillpointbase", 2),
            is_spellcaster: has_arcane || has_divine,
            has_arcane,
            has_divine,
            primary_ability: row_str(&row, "primaryabil").unwrap_or_else(|| "STR".to_string()),
            bab_progression: row_str(&row, "attackbonustable")
                .unwrap_or_else(|| "CLS_ATK_2".to_string()),
            alignment_restricted: align_restrict > 0,
            icon,
            description,
            prerequisites,
        };

        let focus_key = class_focus.as_str().to_string();

        match class_type {
            ClassType::Base => {
                if let Some(list) = categories.base.get_mut(&focus_key) {
                    list.push(class_info);
                }
            }
            ClassType::Prestige => {
                if let Some(list) = categories.prestige.get_mut(&focus_key) {
                    list.push(class_info);
                }
            }
            ClassType::Npc => {
                if let Some(list) = categories.npc.get_mut(&focus_key) {
                    list.push(class_info);
                }
            }
        }

        total_classes += 1;
    }

    for type_map in [
        &mut categories.base,
        &mut categories.prestige,
        &mut categories.npc,
    ] {
        for list in type_map.values_mut() {
            list.sort_by_key(|a| a.name.to_lowercase());
        }
    }

    tracing::info!("Categorized {} classes", total_classes);

    CategorizedClasses {
        categories,
        focus_info: get_focus_display_info(),
        total_classes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(fields: &[(&str, &str)]) -> AHashMap<String, Option<String>> {
        fields
            .iter()
            .map(|(k, v)| ((*k).to_string(), Some((*v).to_string())))
            .collect()
    }

    #[test]
    fn full_bab_half_caster_is_not_divine() {
        // Ranger: full BAB, 6 skill points, hasdivine=1.
        let ranger = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "1"),
            ("skillpointbase", "6"),
            ("hitdie", "8"),
            ("attackbonustable", "CLS_ATK_1"),
        ]);
        assert_eq!(determine_class_focus(&ranger), ClassFocus::SkillSpecialist);

        // Paladin: full BAB, 2 skill points, hasdivine=1.
        let paladin = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "1"),
            ("skillpointbase", "2"),
            ("hitdie", "10"),
            ("attackbonustable", "CLS_ATK_1"),
        ]);
        assert_eq!(determine_class_focus(&paladin), ClassFocus::Combat);
    }

    #[test]
    fn full_casters_keep_caster_focus() {
        let cleric = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "1"),
            ("skillpointbase", "2"),
            ("hitdie", "8"),
            ("attackbonustable", "CLS_ATK_2"),
        ]);
        assert_eq!(determine_class_focus(&cleric), ClassFocus::DivineCaster);

        let druid = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "1"),
            ("skillpointbase", "4"),
            ("hitdie", "8"),
            ("attackbonustable", "CLS_ATK_2"),
        ]);
        assert_eq!(determine_class_focus(&druid), ClassFocus::DivineCaster);

        let wizard = row(&[
            ("hasarcane", "1"),
            ("hasdivine", "0"),
            ("skillpointbase", "2"),
            ("hitdie", "4"),
            ("attackbonustable", "CLS_ATK_3"),
        ]);
        assert_eq!(determine_class_focus(&wizard), ClassFocus::ArcaneCaster);

        let bard = row(&[
            ("hasarcane", "1"),
            ("hasdivine", "0"),
            ("skillpointbase", "6"),
            ("hitdie", "6"),
            ("attackbonustable", "CLS_ATK_2"),
        ]);
        assert_eq!(determine_class_focus(&bard), ClassFocus::ArcaneCaster);
    }

    #[test]
    fn martial_non_casters_unaffected() {
        let fighter = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "0"),
            ("skillpointbase", "2"),
            ("hitdie", "10"),
            ("attackbonustable", "CLS_ATK_1"),
        ]);
        assert_eq!(determine_class_focus(&fighter), ClassFocus::Combat);

        let barbarian = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "0"),
            ("skillpointbase", "4"),
            ("hitdie", "12"),
            ("attackbonustable", "CLS_ATK_1"),
        ]);
        assert_eq!(determine_class_focus(&barbarian), ClassFocus::Combat);

        let rogue = row(&[
            ("hasarcane", "0"),
            ("hasdivine", "0"),
            ("skillpointbase", "8"),
            ("hitdie", "6"),
            ("attackbonustable", "CLS_ATK_2"),
        ]);
        assert_eq!(determine_class_focus(&rogue), ClassFocus::SkillSpecialist);
    }
}
