//! Reduce ranks on real multiclass fixtures and print how the level history changed.
//! Run: `cargo test --features integration-tests --test debugging verify_skill_take_back -- --ignored --nocapture`

use app_lib::character::Character;
use app_lib::character::types::SkillId;
use app_lib::parsers::gff::GffParser;
use std::path::PathBuf;

fn load(rel: &str) -> Character {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gff")
        .join(rel);
    let bytes = std::fs::read(path).unwrap();
    let gff = GffParser::from_bytes(bytes).unwrap();
    Character::from_gff(gff.read_struct_fields(0).unwrap()).expect("character from gff")
}

fn history_of(c: &Character, skill: SkillId) -> Vec<(i32, i32)> {
    c.level_history()
        .iter()
        .filter_map(|l| {
            l.skills_gained
                .iter()
                .find(|s| s.skill_id == skill)
                .map(|s| (l.character_level, s.ranks))
        })
        .collect()
}

fn raw_byte(c: &Character, level_idx: usize, skill: SkillId) -> Option<i32> {
    let levels = c.get_list("LvlStatList")?;
    let entry = levels.get(level_idx)?;
    let list = app_lib::character::gff_helpers::extract_list_from_map(entry, "SkillList")?;
    list.get(skill.0 as usize)
        .and_then(|s| s.get("Rank"))
        .and_then(app_lib::character::gff_helpers::gff_value_to_i32)
}

#[test]
#[ignore = "diagnostic"]
fn verify_skill_take_back() {
    // Occidio 4: Heal(4)=33, L1-20 Paladin, L21-30 Divine Champion (1 each).
    let heal = SkillId(4);
    let mut c = load("occidiooctavon/occidiooctavon4.bic");
    println!(
        "Occidio Heal before: root={} hist tail={:?}",
        c.skill_rank(heal),
        &history_of(&c, heal)[25..]
    );
    c.set_skill_rank(heal, 31).unwrap();
    println!(
        "Occidio Heal after 31: root={} hist tail={:?} rawL30={:?} rawL29={:?} rawL28={:?}",
        c.skill_rank(heal),
        &history_of(&c, heal)[25..],
        raw_byte(&c, 29, heal),
        raw_byte(&c, 28, heal),
        raw_byte(&c, 27, heal)
    );
    let sum: i32 = history_of(&c, heal).iter().map(|(_, r)| r).sum();
    println!(
        "Occidio history sum={sum} (expect 31), any 255? {}",
        (0..30).any(|i| raw_byte(&c, i, heal) == Some(255))
    );

    // Qara 2: Heal(4)=2 total but no level recorded it (level 1 creation ranks).
    let mut q = load("qaraofblacklake/qaraofblacklake2.bic");
    println!(
        "Qara Heal before: root={} hist={:?} plan={:?}",
        q.skill_rank(heal),
        history_of(&q, heal),
        q.skill_rank_take_back_plan(heal, 1)
    );
    q.set_skill_rank(heal, 1).unwrap();
    println!(
        "Qara Heal after 1: root={} hist={:?} any 255? {}",
        q.skill_rank(heal),
        history_of(&q, heal),
        (0..10).any(|i| raw_byte(&q, i, heal) == Some(255))
    );

    // Round trip: add one back lands on the last level.
    q.set_skill_rank(heal, 2).unwrap();
    println!(
        "Qara Heal re-added: root={} hist={:?}",
        q.skill_rank(heal),
        history_of(&q, heal)
    );
}
