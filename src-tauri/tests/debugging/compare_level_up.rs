//! Compares an editor level-up against the game's own level-up of the same character.
//!
//! Level up once in-game from save A and save to slot B, then run:
//!   LEVELUP_BEFORE=<save A dir> LEVELUP_AFTER=<save B dir> \
//!     cargo test --test debugging --features integration-tests \
//!     compare_level_up_with_game -- --ignored --nocapture
//!
//! The editor levels up the class the game added. Player choices (picked feats, skill
//! ranks, ability increase) are reported, not compared.

use app_lib::character::gff_helpers::gff_value_to_i32;
use app_lib::character::{Character, ClassId};
use app_lib::config::nwn2_paths::NWN2Paths;
use app_lib::parsers::gff::GffParser;
use app_lib::parsers::gff::helpers::variant_name;
use app_lib::parsers::gff::types::GffValue;
use app_lib::services::item_property_decoder::ItemPropertyDecoder;
use app_lib::services::resource_manager::ResourceManager;
use app_lib::services::savegame_handler::SaveGameHandler;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[path = "../common/mod.rs"]
mod common;

type Fields = IndexMap<String, GffValue<'static>>;

fn load_player(env_var: &str) -> Character {
    let dir = PathBuf::from(std::env::var(env_var).unwrap_or_else(|_| panic!("{env_var} not set")));
    let handler = SaveGameHandler::new(&dir, false).expect("open save");
    let bic = handler
        .extract_player_bic()
        .expect("extract player.bic")
        .expect("no player.bic in save");
    let parser = GffParser::from_bytes(bic).expect("parse player.bic");
    Character::from_gff(parser.read_struct_fields(0).expect("read root"))
}

fn int(fields: &Fields, key: &str) -> Option<i32> {
    fields.get(key).and_then(gff_value_to_i32)
}

fn list<'a>(fields: &'a Fields, key: &str) -> &'a [Fields] {
    match fields.get(key) {
        Some(GffValue::ListOwned(items)) => items,
        _ => &[],
    }
}

fn shape(fields: &Fields) -> BTreeMap<String, &'static str> {
    fields
        .iter()
        .map(|(k, v)| (k.clone(), variant_name(v)))
        .collect()
}

fn feat_ids(fields: &Fields) -> BTreeSet<i32> {
    list(fields, "FeatList")
        .iter()
        .filter_map(|f| int(f, "Feat"))
        .collect()
}

async fn create_decoder() -> ItemPropertyDecoder {
    let paths = Arc::new(RwLock::new(NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(paths)));
    let _ = rm.write().await.initialize().await;
    let mut decoder = ItemPropertyDecoder::new(rm.clone());
    decoder.initialize_with_rm(&*rm.read().await);
    decoder
}

#[test]
#[ignore]
fn compare_level_up_with_game() {
    let before = load_player("LEVELUP_BEFORE");
    let game = load_player("LEVELUP_AFTER");
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let ctx = runtime.block_on(common::create_test_context());
    let decoder = runtime.block_on(create_decoder());
    let game_data = ctx.loader.game_data().expect("game data");

    assert_eq!(
        game.total_level(),
        before.total_level() + 1,
        "AFTER must be BEFORE plus exactly one in-game level-up"
    );
    let game_entry = list(game.gff(), "LvlStatList")
        .last()
        .expect("game level entry");
    let class_id = ClassId(int(game_entry, "LvlStatClass").expect("LvlStatClass"));

    let mut editor = load_player("LEVELUP_BEFORE");
    editor
        .level_up(class_id, game_data)
        .expect("editor level up");
    let editor_entry = list(editor.gff(), "LvlStatList").last().unwrap();

    let mut mismatches = Vec::new();
    let mut check = |what: String, editor_value: String, game_value: String| {
        let marker = if editor_value == game_value {
            "  "
        } else {
            "!!"
        };
        println!("{marker} {what:<28} editor {editor_value:<24} game {game_value}");
        if editor_value != game_value {
            mismatches.push(what);
        }
    };

    println!(
        "\n=== Level-up class {} ({} -> {}) ===",
        class_id.0,
        before.total_level(),
        game.total_level()
    );
    for key in [
        "BaseAttackBonus",
        "fortbonus",
        "refbonus",
        "willbonus",
        "HitPoints",
        "MaxHitPoints",
        "MClassLevUpIn",
    ] {
        check(
            key.to_string(),
            format!(
                "{:?}",
                editor
                    .gff()
                    .get(key)
                    .map(|v| (variant_name(v), gff_value_to_i32(v)))
            ),
            format!(
                "{:?}",
                game.gff()
                    .get(key)
                    .map(|v| (variant_name(v), gff_value_to_i32(v)))
            ),
        );
    }

    let class_levels = |c: &Character| {
        c.class_entries()
            .iter()
            .map(|e| (e.class_id.0, e.level))
            .collect::<Vec<_>>()
    };
    check(
        "ClassList".into(),
        format!("{:?}", class_levels(&editor)),
        format!("{:?}", class_levels(&game)),
    );

    for key in ["LvlStatClass", "LvlStatHitDie", "EpicLevel"] {
        check(
            format!("level entry {key}"),
            format!("{:?}", int(editor_entry, key)),
            format!("{:?}", int(game_entry, key)),
        );
    }
    check(
        "level entry SkillList len".into(),
        list(editor_entry, "SkillList").len().to_string(),
        list(game_entry, "SkillList").len().to_string(),
    );

    let player_choice_keys =
        |k: &String| k.starts_with("KnownList") || k == "LvlStatAbility" || k == "__struct_id__";
    let entry_shape = |entry: &Fields| {
        shape(entry)
            .into_iter()
            .filter(|(k, _)| !player_choice_keys(k))
            .collect::<BTreeMap<_, _>>()
    };
    check(
        "level entry fields".into(),
        format!("{:?}", entry_shape(editor_entry)),
        format!("{:?}", entry_shape(game_entry)),
    );

    let before_root: BTreeSet<_> = before.gff().keys().cloned().collect();
    let invented: Vec<_> = editor
        .gff()
        .keys()
        .filter(|k| !before_root.contains(*k) && !game.gff().contains_key(*k))
        .cloned()
        .collect();
    check(
        "root fields the game lacks".into(),
        format!("{invented:?}"),
        "[]".into(),
    );

    let before_feats = feat_ids(before.gff());
    let game_added: BTreeSet<_> = feat_ids(game.gff())
        .difference(&before_feats)
        .copied()
        .collect();
    let editor_added: BTreeSet<_> = feat_ids(editor.gff())
        .difference(&before_feats)
        .copied()
        .collect();
    let editor_only: Vec<_> = editor_added.difference(&game_added).copied().collect();
    let game_only: Vec<_> = game_added.difference(&editor_added).copied().collect();
    check(
        "feats granted, game lacks".into(),
        format!("{editor_only:?}"),
        "[]".into(),
    );
    println!(
        "   feats the game added beyond the editor (player picks or missed grants): {game_only:?}"
    );
    println!(
        "   LvlStatAbility: editor {:?}, game {:?} (player choice)",
        int(editor_entry, "LvlStatAbility"),
        int(game_entry, "LvlStatAbility")
    );

    // Stored *SaveThrow values include whatever effects were active when the game saved,
    // so they are reported, not compared.
    for (label, character) in [("before", &before), ("game", &game)] {
        let totals = character.get_save_summary(game_data, &decoder);
        println!(
            "   {label}: editor saves {}/{}/{}, stored {:?}/{:?}/{:?}",
            totals.fortitude,
            totals.reflex,
            totals.will,
            int(character.gff(), "FortSaveThrow"),
            int(character.gff(), "RefSaveThrow"),
            int(character.gff(), "WillSaveThrow"),
        );
        println!("     fort {:?}", totals.saves.fortitude);
        println!("     ref  {:?}", totals.saves.reflex);
        println!("     will {:?}", totals.saves.will);
    }

    assert!(
        mismatches.is_empty(),
        "editor differs from the game: {mismatches:?}"
    );
}
