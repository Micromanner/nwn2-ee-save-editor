//! Writes a sentinel .bic with Tint_Head and Tint_Hair raw GFF channels set to
//! red(ch1)/green(ch2)/blue(ch3), for in-game calibration of the engine's
//! channel -> region mapping. Run:
//!   cargo test --features integration-tests --test debugging make_tint_sentinel_save -- --ignored --nocapture

use app_lib::character::{Character, TintChannel, TintChannels};
use app_lib::parsers::gff::merge_fields_into_gff;
use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use indexmap::IndexMap;
use std::path::PathBuf;

fn primaries() -> TintChannels {
    TintChannels {
        channel1: TintChannel {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        },
        channel2: TintChannel {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        },
        channel3: TintChannel {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        },
    }
}

#[test]
#[ignore]
fn make_tint_sentinel_save() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gff/player.bic");
    let original = std::fs::read(&fixture).expect("fixture player.bic");

    let gff = GffParser::from_bytes(original.clone()).expect("parse fixture");
    let fields: IndexMap<String, GffValue<'static>> = gff
        .read_struct_fields(0)
        .expect("root")
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    let mut character = Character::from_gff(fields);

    // Write RAW GFF channel order (no UI 2/3 swap) so the in-game result maps
    // directly to GFF Tint_*.channel{1,2,3}. set_tint_hair already writes raw
    // order; write_raw_tint_head writes Tint_Head without the head 2/3 swap.
    character.set_tint_hair(&primaries());
    character.write_raw_tint_head(&primaries());

    let written = merge_fields_into_gff(Some(&original), &character.clone_gff(), "BIC ", false)
        .expect("merge write");

    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target_test/tint_sentinel.bic");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, written).unwrap();
    eprintln!("Sentinel save written to: {}", out.display());
}
