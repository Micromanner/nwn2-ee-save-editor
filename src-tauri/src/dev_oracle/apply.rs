use indexmap::IndexMap;
use serde::Deserialize;

use crate::character::classes::EPIC_LEVEL_THRESHOLD;
use crate::character::{AbilityIndex, Character};
use crate::loaders::GameData;
use crate::parsers::gff::GffValue;
use crate::utils::parsing::{Row, row_bool, row_int};

/// Struct ids the game writes for these lists in a player `.bic`.
const CLASS_STRUCT: u32 = 2;
const KNOWN_SPELL_STRUCT: u32 = 3;
const FEAT_STRUCT: u32 = 1;
const SPELLS_PER_DAY_STRUCT: u32 = 17767;
const NO_DOMAIN: i32 = 255;

#[derive(Debug, Deserialize)]
pub struct EngineBuild {
    pub race: i32,
    pub subrace: i32,
    pub package: i32,
    pub abilities: Vec<i32>,
    pub good_evil: i32,
    pub lawful_chaotic: i32,
    pub level: i32,
    pub bab: i32,
    pub save_totals: Vec<i32>,
    pub hp_max: i32,
    pub hp_hit_dice: i32,
    pub classes: Vec<EngineClass>,
    pub feats: Vec<i32>,
    pub skill_ranks: Vec<i32>,
    pub levels: Vec<EngineLevel>,
}

#[derive(Debug, Deserialize)]
pub struct EngineClass {
    pub class: i32,
    pub level: i32,
    pub domain1: i32,
    pub domain2: i32,
    pub school: i32,
    pub spells_per_day: Vec<i32>,
    pub known: Vec<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct EngineLevel {
    pub level: i32,
    pub class: i32,
    pub hit_die: i32,
    pub ability_increase: i32,
    pub skill_points_remaining: i32,
    pub feats_added: Vec<i32>,
    pub skill_ranks_added: Vec<i32>,
    pub spells_added: Vec<Vec<i32>>,
}

type Fields = IndexMap<String, GffValue<'static>>;

/// Replaces the build of `character` with the engine-built one. Identity, appearance,
/// items, tag and everything else the template carries stay untouched.
pub fn apply(character: &mut Character, build: &EngineBuild, game_data: &GameData, name: &str) {
    character.set_byte("Race", build.race as u8);
    character.set_byte("Subrace", build.subrace as u8);
    for (ability, value) in AbilityIndex::all().into_iter().zip(&build.abilities) {
        character.set_byte(ability.gff_field(), *value as u8);
    }
    character.set_byte("GoodEvil", build.good_evil as u8);
    character.set_byte("LawfulChaotic", build.lawful_chaotic as u8);
    character.set_byte("StartingPackage", build.package as u8);
    character.set_u32("CharBackground", 0);

    let classes = game_data.get_table("classes");
    character.set_list(
        "ClassList",
        build
            .classes
            .iter()
            .map(|c| class_entry(c, classes.and_then(|t| t.get_by_id(c.class)).as_ref()))
            .collect(),
    );
    character.set_list(
        "FeatList",
        build
            .feats
            .iter()
            .map(|&f| feat(f, Some(FEAT_STRUCT)))
            .collect(),
    );
    character.set_list(
        "SkillList",
        build.skill_ranks.iter().map(|&r| rank(r)).collect(),
    );
    character.set_list(
        "LvlStatList",
        build.levels.iter().map(level_entry).collect(),
    );

    character.set_byte("BaseAttackBonus", build.bab as u8);
    for (field, total) in ["FortSaveThrow", "RefSaveThrow", "WillSaveThrow"]
        .iter()
        .zip(&build.save_totals)
    {
        character.gff_mut().insert(
            (*field).to_string(),
            GffValue::Char(*total as i8 as u8 as char),
        );
    }
    for field in ["fortbonus", "refbonus", "willbonus"] {
        character.set_i16(field, 0);
    }

    let skill_points = build.levels.last().map_or(0, |l| l.skill_points_remaining);
    character.set_u16("SkillPoints", skill_points as u16);
    character.set_i16("HitPoints", build.hp_hit_dice as i16);
    for field in ["MaxHitPoints", "CurrentHitPoints", "PregameCurrent"] {
        character.set_i16(field, build.hp_max as i16);
    }
    let ecl = game_data
        .get_table("racialsubtypes")
        .and_then(|t| t.get_by_id(build.subrace))
        .map_or(0, |row| row_int(&row, "ecl", 0));
    let xp = character.calculate_xp_for_level(build.level + ecl, game_data);
    character.set_u32("Experience", xp as u32);

    if let Some(appearance) = game_data
        .get_table("racialtypes")
        .and_then(|t| t.get_by_id(build.race))
        .map(|row| row_int(&row, "appearance", -1))
        .filter(|a| *a >= 0)
    {
        character.set_u16("Appearance_Type", appearance as u16);
    }

    character.set_first_name(name.to_string());
    character.set_last_name(String::new());
    character.mark_modified();
}

fn class_entry(c: &EngineClass, row: Option<&Row>) -> Fields {
    let mut entry = gff_struct(
        Some(CLASS_STRUCT),
        [
            ("Class", GffValue::Int(c.class)),
            ("ClassLevel", GffValue::Short(c.level as i16)),
        ],
    );
    let has = |column: &str| row.is_some_and(|r| row_bool(r, column, false));
    if has("hasdomains") {
        for (field, domain) in [("Domain1", c.domain1), ("Domain2", c.domain2)] {
            if domain != NO_DOMAIN {
                entry.insert(field.into(), GffValue::Byte(domain as u8));
            }
        }
    }
    if has("hasschool") {
        entry.insert("School".into(), GffValue::Byte(c.school as u8));
    }
    for (level, spells) in c.known.iter().enumerate() {
        if !spells.is_empty() {
            entry.insert(
                format!("KnownList{level}"),
                known_list(spells, Some(KNOWN_SPELL_STRUCT)),
            );
        }
    }
    let spontaneous = has("spellcaster") && !has("memorizesspells");
    if spontaneous {
        let per_day = c
            .spells_per_day
            .iter()
            .map(|&n| {
                gff_struct(
                    Some(SPELLS_PER_DAY_STRUCT),
                    [("NumSpellsLeft", GffValue::Byte(n as u8))],
                )
            })
            .collect();
        entry.insert("SpellsPerDayList".into(), GffValue::ListOwned(per_day));
    }
    entry
}

fn level_entry(l: &EngineLevel) -> Fields {
    let mut entry = Fields::new();
    entry.insert(
        "FeatList".into(),
        GffValue::ListOwned(l.feats_added.iter().map(|&f| feat(f, None)).collect()),
    );
    entry.insert(
        "EpicLevel".into(),
        GffValue::Byte(u8::from(l.level > EPIC_LEVEL_THRESHOLD)),
    );
    entry.insert(
        "SkillPoints".into(),
        GffValue::Word(l.skill_points_remaining as u16),
    );
    if l.ability_increase >= 0 {
        entry.insert(
            "LvlStatAbility".into(),
            GffValue::Byte(l.ability_increase as u8),
        );
    }
    for (level, spells) in l.spells_added.iter().enumerate() {
        if !spells.is_empty() {
            entry.insert(format!("KnownList{level}"), known_list(spells, None));
        }
    }
    entry.insert("LvlStatClass".into(), GffValue::Byte(l.class as u8));
    entry.insert(
        "SkillList".into(),
        GffValue::ListOwned(l.skill_ranks_added.iter().map(|&r| rank(r)).collect()),
    );
    entry.insert("LvlStatHitDie".into(), GffValue::Byte(l.hit_die as u8));
    entry
}

fn known_list(spells: &[i32], struct_id: Option<u32>) -> GffValue<'static> {
    GffValue::ListOwned(
        spells
            .iter()
            .map(|&spell| gff_struct(struct_id, [("Spell", GffValue::Word(spell as u16))]))
            .collect(),
    )
}

fn feat(id: i32, struct_id: Option<u32>) -> Fields {
    gff_struct(struct_id, [("Feat", GffValue::Word(id as u16))])
}

fn rank(value: i32) -> Fields {
    gff_struct(None, [("Rank", GffValue::Byte(value as u8))])
}

fn gff_struct<const N: usize>(
    struct_id: Option<u32>,
    fields: [(&str, GffValue<'static>); N],
) -> Fields {
    struct_id
        .map(|id| ("__struct_id__".to_string(), GffValue::Dword(id)))
        .into_iter()
        .chain(fields.map(|(k, v)| (k.to_string(), v)))
        .collect()
}
