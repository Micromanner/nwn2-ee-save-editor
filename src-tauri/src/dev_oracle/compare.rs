use serde::{Deserialize, Serialize};

use crate::character::classes::LevelUpResult;
use crate::character::{
    AbilitiesState, AbilityIndex, CombatSummary, SaveSummary, SkillSummaryEntry,
};

#[derive(Debug, Deserialize)]
pub struct EngineCheck {
    pub abilities: Vec<EngineAbility>,
    pub bab: i32,
    pub saves: Vec<EngineSave>,
    pub hp_max: i32,
    pub hp_hit_dice: i32,
    pub ac_stat: i32,
    pub ac_dodge_feats: i32,
    pub skills: Vec<EngineSkill>,
    pub casters: Vec<EngineCaster>,
}

#[derive(Debug, Deserialize)]
pub struct EngineAbility {
    pub base: i32,
    pub effective: i32,
    pub modifier: i32,
}

#[derive(Debug, Deserialize)]
pub struct EngineSave {
    pub class: i32,
    pub feat: i32,
    pub ability: i32,
}

#[derive(Debug, Deserialize)]
pub struct EngineSkill {
    pub id: i32,
    pub max_ranks: i32,
    pub class_skill: bool,
}

#[derive(Debug, Deserialize)]
pub struct EngineCaster {
    pub class: i32,
    pub level: i32,
    pub prestige_bonus: i32,
    pub spells_per_day: Vec<i32>,
}

/// The editor's view of the same character, items stripped like the engine side.
pub struct EditorSide {
    pub abilities: AbilitiesState,
    pub saves: SaveSummary,
    pub combat: CombatSummary,
    pub skills: Vec<SkillSummaryEntry>,
    pub casters: Vec<EditorCaster>,
    pub max_hp: i32,
    pub base_hp: i32,
}

pub struct EditorCaster {
    pub class: i32,
    pub caster_level: i32,
    pub slots: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct Row {
    pub field: String,
    pub editor: i32,
    pub engine: i32,
}

pub fn rows(editor: &EditorSide, engine: &EngineCheck) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut push = |field: String, editor: i32, engine: i32| {
        rows.push(Row {
            field,
            editor,
            engine,
        });
    };

    let a = &editor.abilities;
    for (ability, e) in AbilityIndex::all().into_iter().zip(&engine.abilities) {
        let name = ability.gff_field();
        push(
            format!("ability.{name}.base"),
            a.base_scores.get(ability),
            e.base,
        );
        push(
            format!("ability.{name}.effective"),
            a.effective_scores.get(ability),
            e.effective,
        );
        push(
            format!("ability.{name}.modifier"),
            a.modifiers.get(ability),
            e.modifier,
        );
    }

    push("bab".into(), editor.combat.bab, engine.bab);

    let s = &editor.saves.saves;
    for ((name, ed), en) in [
        ("fortitude", &s.fortitude),
        ("reflex", &s.reflex),
        ("will", &s.will),
    ]
    .into_iter()
    .zip(&engine.saves)
    {
        push(format!("save.{name}.class"), ed.base, en.class);
        push(format!("save.{name}.ability"), ed.ability, en.ability);
        // The engine counts racial feats, Divine Grace and similar as one feat bonus.
        push(
            format!("save.{name}.feat_racial_class"),
            ed.feat + ed.racial + ed.class_bonus,
            en.feat,
        );
    }

    push("hp.max".into(), editor.max_hp, engine.hp_max);
    push("hp.hit_dice".into(), editor.base_hp, engine.hp_hit_dice);
    push(
        "ac.ability".into(),
        editor.combat.armor_class.breakdown.dex,
        engine.ac_stat,
    );
    push(
        "ac.dodge_feats".into(),
        editor.combat.armor_class.breakdown.dodge,
        engine.ac_dodge_feats,
    );

    for skill in &editor.skills {
        let Some(e) = engine.skills.iter().find(|e| e.id == skill.skill_id.0) else {
            continue;
        };
        let id = skill.skill_id.0;
        push(
            format!("skill.{id}.{}.max_ranks", skill.name),
            skill.max_ranks,
            e.max_ranks,
        );
        push(
            format!("skill.{id}.{}.class_skill", skill.name),
            i32::from(skill.is_class_skill),
            i32::from(e.class_skill),
        );
    }

    for e in &engine.casters {
        let ed = editor.casters.iter().find(|c| c.class == e.class);
        if ed.is_none() && e.spells_per_day.iter().all(|n| *n == 0) {
            continue;
        }
        push(
            format!("caster.{}.caster_level", e.class),
            ed.map_or(0, |c| c.caster_level),
            e.level + e.prestige_bonus,
        );
        for (level, engine_slots) in e.spells_per_day.iter().enumerate() {
            let editor_slots = ed.and_then(|c| c.slots.get(level)).copied().unwrap_or(0);
            if editor_slots != 0 || *engine_slots != 0 {
                push(
                    format!("caster.{}.slots.{level}", e.class),
                    editor_slots,
                    *engine_slots,
                );
            }
        }
    }

    rows
}

#[derive(Debug, Deserialize)]
pub struct EngineLevelUp {
    pub package: i32,
    pub steps: Vec<EngineLevelStep>,
}

#[derive(Debug, Deserialize)]
pub struct EngineLevelStep {
    pub level: i32,
    pub skill_points_gained: i32,
    pub hp_after: i32,
    pub ability_increase: i32,
    pub bab: i32,
    pub saves: Vec<i32>,
    pub feats_added: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct LevelComparison {
    pub level: i32,
    pub mismatches: Vec<Row>,
    pub engine_feats_added: Vec<i32>,
    pub editor_feats_granted: Vec<i32>,
    /// Automatic feats the editor granted that the engine did not add.
    pub editor_only_feats: Vec<i32>,
    pub engine_ability_increase: i32,
}

pub fn level_step(
    engine: &EngineLevelStep,
    editor: &LevelUpResult,
    editor_hp_max: i32,
    editor_bab: i32,
    editor_saves: [i32; 3],
) -> LevelComparison {
    let mut mismatches = Vec::new();
    let mut push = |field: &str, editor: i32, engine: i32| {
        if editor != engine {
            mismatches.push(Row {
                field: field.to_string(),
                editor,
                engine,
            });
        }
    };
    push(
        "skill_points_gained",
        editor.skill_points_gained,
        engine.skill_points_gained,
    );
    push("hp_max_after", editor_hp_max, engine.hp_after);
    push("bab", editor_bab, engine.bab);
    for ((name, ed), en) in ["fortitude", "reflex", "will"]
        .into_iter()
        .zip(editor_saves)
        .zip(&engine.saves)
    {
        push(&format!("save.{name}.class"), ed, *en);
    }
    let granted: Vec<i32> = editor.granted_feats.iter().map(|f| f.0).collect();
    let editor_only: Vec<i32> = granted
        .iter()
        .copied()
        .filter(|f| !engine.feats_added.contains(f))
        .collect();
    push(
        "automatic_feats_missing_in_engine",
        editor_only.len() as i32,
        0,
    );
    LevelComparison {
        level: engine.level,
        mismatches,
        engine_feats_added: engine.feats_added.clone(),
        editor_feats_granted: granted,
        editor_only_feats: editor_only,
        engine_ability_increase: engine.ability_increase,
    }
}
