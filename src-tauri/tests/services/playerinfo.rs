use std::path::PathBuf;

use app_lib::character::Character;
use app_lib::parsers::gff::{GffValue, LocalizedString, LocalizedSubstring};
use app_lib::services::playerinfo::PlayerInfo;
use app_lib::state::session_state::SessionState;
use indexmap::IndexMap;

fn saves_dir() -> PathBuf {
    crate::common::fixtures_path().join("saves")
}

fn empty_game_data() -> app_lib::loaders::GameData {
    app_lib::loaders::GameData::new(std::sync::Arc::new(std::sync::RwLock::new(
        app_lib::parsers::tlk::TLKParser::default(),
    )))
}

/// Without TLK and 2DAs: names, background, deity and the alignment axes come
/// from the GFF, the alignment text falls back to English, modifiers follow the
/// scores.
#[test]
fn test_playerinfo_from_character_fields() {
    let name = |text: &str| {
        GffValue::LocString(LocalizedString {
            string_ref: -1,
            substrings: vec![LocalizedSubstring {
                language: 0,
                gender: 0,
                string: text.to_string().into(),
            }],
        })
    };
    let mut fields = IndexMap::new();
    fields.insert("FirstName".to_string(), name("Hero"));
    fields.insert("LastName".to_string(), name("Last"));
    fields.insert("Deity".to_string(), GffValue::String("Tyr".into()));
    fields.insert("CharBackground".to_string(), GffValue::Dword(7));
    fields.insert("LawfulChaotic".to_string(), GffValue::Byte(10));
    fields.insert("GoodEvil".to_string(), GffValue::Byte(90));
    for (label, score) in [
        ("Str", 18),
        ("Dex", 14),
        ("Con", 13),
        ("Int", 10),
        ("Wis", 8),
        ("Cha", 20),
    ] {
        fields.insert(label.to_string(), GffValue::Byte(score));
    }
    let character = Character::from_gff(fields).expect("character");

    let info = PlayerInfo::from_character(&character, &empty_game_data());

    assert_eq!(info.data.full_name(), "Hero Last");
    assert_eq!(info.data.deity, "Tyr");
    assert_eq!(info.data.alignment, "Chaotic Good");
    assert_eq!(info.data.alignment_vertical, 4, "Good");
    assert_eq!(info.data.alignment_horizontal, 3, "Chaotic");
    assert_eq!(info.data.background_id, 7);
    assert_eq!(info.data.scores, [18, 14, 13, 10, 8, 20]);
    assert_eq!(info.data.modifiers, [4, 2, 1, 0, -1, 5]);
}

/// Parse known fixtures and confirm the decoded fields match Arbos' spec:
/// <https://gist.github.com/Arbos/225c724f91309d3f515e0f110524feee>
#[test]
fn test_playerinfo_fixtures_decode_alignment_and_background() {
    // (relative path, alignment text, vertical, horizontal, background_id)
    let cases = [
        (
            "Classic_Campaign/playerinfo.bin",
            "Chaotic Neutral",
            1,
            3,
            3,
        ),
        ("STORM_Campaign/playerinfo.bin", "Chaotic Evil", 5, 3, 7),
        ("MOTB/playerinfo.bin", "Neutral Good", 4, 1, 9),
        ("Westgate_Campaign/playerinfo.bin", "Neutral Evil", 5, 1, 10),
    ];

    for (rel, alignment, vert, horiz, bg) in cases {
        let info =
            PlayerInfo::load(saves_dir().join(rel)).unwrap_or_else(|e| panic!("load {rel}: {e}"));

        assert_eq!(info.data.alignment, alignment, "{rel}: alignment text");
        assert_eq!(info.data.alignment_vertical, vert, "{rel}: vertical");
        assert_eq!(info.data.alignment_horizontal, horiz, "{rel}: horizontal");
        assert_eq!(info.data.background_id, bg, "{rel}: background_id");
    }
}

/// Re-writing a parsed fixture must produce byte-identical output.
#[test]
fn test_playerinfo_round_trip_preserves_bytes() {
    let fixtures = [
        "000055 - SAVE GAME COMPLETE/playerinfo.bin",
        "Classic_Campaign/playerinfo.bin",
        "MOTB/playerinfo.bin",
        "Russian_Client/playerinfo.bin",
        "STORM_Campaign/playerinfo.bin",
        "Westgate_Campaign/playerinfo.bin",
        "user_save_linux/playerinfo.bin",
        "cheatdebug/000061 - 16-04-2026-23-01/playerinfo.bin",
        "classdebug/000058 - 15-04-2026-16-56/playerinfo.bin",
    ];

    for rel in fixtures {
        let src = saves_dir().join(rel);
        let original = std::fs::read(&src).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        let info = PlayerInfo::load(&src).unwrap_or_else(|e| panic!("parse {rel}: {e}"));

        let mut rewritten = Vec::new();
        info.write(&mut rewritten).unwrap();

        assert_eq!(
            original, rewritten,
            "{rel}: parse+write must preserve bytes exactly"
        );
    }
}

/// A one-letter last name ("a a") used to be misread as padding, which broke
/// every later field.
#[test]
fn test_playerinfo_one_letter_last_name() {
    let path = saves_dir().join("000055 - SAVE GAME COMPLETE/playerinfo.bin");
    let info = PlayerInfo::load(&path).expect("load 000055");
    assert_eq!(info.data.last_name, "a");
    assert_eq!(info.data.subrace, "Yuan-ti Pureblood ");
    assert_eq!(PlayerInfo::get_player_name(&path).unwrap(), "a a");
}

/// A player save must reproduce the game-written `playerinfo.bin` byte for
/// byte, even over an unreadable file: STORM Half-Orc stores
/// 20/16/14/10/10/10, the game writes stored + racial 22/16/14/8/10/8.
#[tokio::test]
async fn test_player_save_rebuilds_game_written_playerinfo() {
    let ctx = crate::common::create_test_context().await;
    let game_data = ctx.loader.game_data().expect("game data");
    let (_tmp, save_path) = crate::common::temp_save_copy("STORM_Campaign");
    let path = save_path.join("playerinfo.bin");
    let original = std::fs::read(&path).unwrap();
    std::fs::write(&path, [0xff; 12]).unwrap();

    let mut session = SessionState::new();
    crate::common::load_save(&mut session, &save_path);
    let character = session.character_mut().expect("player");
    let xp = character.experience();
    character.set_experience(xp + 1).expect("set xp");
    session
        .save_character(game_data, &app_lib::config::NWN2Paths::new(), 3)
        .expect("save");

    let info = PlayerInfo::load(&path).expect("rebuilt playerinfo.bin");
    assert_eq!(info.data.scores, [22, 16, 14, 8, 10, 8]);
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
