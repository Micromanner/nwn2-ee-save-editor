//! Round-trip diagnostic for the hair/head tint write path (Nexus report:
//! hair colors "mix constantly and randomly").
//!
//! Loads a real game-written player.bic fixture, sets distinct sentinel
//! colors on every hair/head channel, serializes through the same
//! `merge_fields_into_gff` path the save pipeline uses, reparses, and checks
//! for channel scrambling, retyped fields, or drift across repeated cycles.

use app_lib::character::{Character, TintChannel, TintChannels};
use app_lib::parsers::gff::merge_fields_into_gff;
use app_lib::parsers::gff::parser::GffParser;
use app_lib::parsers::gff::types::GffValue;
use indexmap::IndexMap;
use std::fmt::Write;
use std::path::PathBuf;

fn fixture_bic() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("gff")
        .join("player.bic");
    std::fs::read(path).expect("fixture player.bic")
}

fn resolve_struct(val: &GffValue<'_>) -> Option<IndexMap<String, GffValue<'static>>> {
    match val {
        GffValue::StructOwned(s) => Some(
            s.as_ref()
                .clone()
                .into_iter()
                .map(|(k, v)| (k, v.force_owned()))
                .collect(),
        ),
        GffValue::Struct(lazy) => Some(
            lazy.force_load()
                .into_iter()
                .map(|(k, v)| (k, v.force_owned()))
                .collect(),
        ),
        _ => None,
    }
}

fn dump_tint_struct(out: &mut String, label: &str, val: Option<&GffValue<'_>>) {
    let Some(val) = val else {
        writeln!(out, "{label}: MISSING").unwrap();
        return;
    };
    let Some(outer) = resolve_struct(val) else {
        writeln!(
            out,
            "{label}: NOT A STRUCT ({:?})",
            std::mem::discriminant(val)
        )
        .unwrap();
        return;
    };
    // Accept either direct Tint (ArmorTint style) or Tintable->Tint nesting
    let tint = if let Some(tintable) = outer.get("Tintable").and_then(resolve_struct) {
        tintable.get("Tint").and_then(resolve_struct)
    } else if let Some(t) = outer.get("Tint").and_then(resolve_struct) {
        Some(t)
    } else {
        Some(outer.clone())
    };
    let Some(tint) = tint else {
        writeln!(
            out,
            "{label}: no Tint struct; keys={:?}",
            outer.keys().collect::<Vec<_>>()
        )
        .unwrap();
        return;
    };
    for ch_key in ["1", "2", "3"] {
        match tint.get(ch_key).and_then(resolve_struct) {
            Some(ch) => {
                let fmt_field = |k: &str| -> String {
                    match ch.get(k) {
                        Some(GffValue::Byte(v)) => format!("Byte({v})"),
                        Some(other) => format!("WRONG_TYPE({other:?})"),
                        None => "MISSING".to_string(),
                    }
                };
                writeln!(
                    out,
                    "{label}.{ch_key}: r={} g={} b={} a={}",
                    fmt_field("r"),
                    fmt_field("g"),
                    fmt_field("b"),
                    fmt_field("a")
                )
                .unwrap();
            }
            None => writeln!(out, "{label}.{ch_key}: MISSING").unwrap(),
        }
    }
}

fn ch(r: u8, g: u8, b: u8) -> TintChannel {
    TintChannel { r, g, b, a: 0 }
}

#[test]
fn diagnose_hair_tint_roundtrip() {
    let original = fixture_bic();
    let gff = GffParser::from_bytes(original.clone()).expect("parse fixture");
    let fields: IndexMap<String, GffValue<'static>> = gff
        .read_struct_fields(0)
        .expect("root")
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let mut out = String::new();

    writeln!(out, "=== BEFORE (game-written) ===").unwrap();
    dump_tint_struct(&mut out, "Tint_Hair", fields.get("Tint_Hair"));
    dump_tint_struct(&mut out, "Tint_Head", fields.get("Tint_Head"));
    dump_tint_struct(&mut out, "Tintable(root)", fields.get("Tintable"));
    dump_tint_struct(&mut out, "ArmorTint", fields.get("ArmorTint"));

    let mut character = Character::from_gff(fields);

    // Distinct sentinels: hair = pure R/G/B, head = recognizable triples
    let hair = TintChannels {
        channel1: ch(255, 0, 0),
        channel2: ch(0, 255, 0),
        channel3: ch(0, 0, 255),
    };
    let head = TintChannels {
        channel1: ch(10, 11, 12),
        channel2: ch(20, 21, 22),
        channel3: ch(30, 31, 32),
    };
    character.set_tint_hair(&hair);
    character.set_tint_head(&head);

    writeln!(out, "\n=== IN-MEMORY read-back after set ===").unwrap();
    let h = character.tint_hair();
    writeln!(
        out,
        "tint_hair(): ch1=({},{},{}) ch2=({},{},{}) ch3=({},{},{})",
        h.channel1.r,
        h.channel1.g,
        h.channel1.b,
        h.channel2.r,
        h.channel2.g,
        h.channel2.b,
        h.channel3.r,
        h.channel3.g,
        h.channel3.b
    )
    .unwrap();
    let hd = character.tint_head();
    writeln!(
        out,
        "tint_head(): ch1=({},{},{}) ch2=({},{},{}) ch3=({},{},{})",
        hd.channel1.r,
        hd.channel1.g,
        hd.channel1.b,
        hd.channel2.r,
        hd.channel2.g,
        hd.channel2.b,
        hd.channel3.r,
        hd.channel3.g,
        hd.channel3.b
    )
    .unwrap();

    // Serialize through the real save path, reparse
    let written = merge_fields_into_gff(Some(&original), &character.clone_gff(), "BIC ", false)
        .expect("merge write");
    let gff2 = GffParser::from_bytes(written.clone()).expect("reparse");
    let fields2: IndexMap<String, GffValue<'static>> = gff2
        .read_struct_fields(0)
        .expect("root2")
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    writeln!(out, "\n=== AFTER round-trip (raw GFF) ===").unwrap();
    dump_tint_struct(&mut out, "Tint_Hair", fields2.get("Tint_Hair"));
    dump_tint_struct(&mut out, "Tint_Head", fields2.get("Tint_Head"));
    dump_tint_struct(&mut out, "Tintable(root)", fields2.get("Tintable"));
    dump_tint_struct(&mut out, "ArmorTint", fields2.get("ArmorTint"));

    // Drift check: re-load written bytes, read + write back UNCHANGED 3x
    let mut current = written;
    for cycle in 1..=3 {
        let g = GffParser::from_bytes(current.clone()).expect("cycle parse");
        let f: IndexMap<String, GffValue<'static>> = g
            .read_struct_fields(0)
            .expect("cycle root")
            .into_iter()
            .map(|(k, v)| (k, v.force_owned()))
            .collect();
        let mut c = Character::from_gff(f);
        let hair_now = c.tint_hair();
        let head_now = c.tint_head();
        // Simulate the frontend confirm: write back what was read
        c.set_tint_hair(&hair_now);
        c.set_tint_head(&head_now);
        current = merge_fields_into_gff(Some(&current), &c.clone_gff(), "BIC ", false)
            .expect("cycle write");
        writeln!(
            out,
            "\n=== CYCLE {cycle}: hair ch1=({},{},{}) ch2=({},{},{}) ch3=({},{},{}) | head ch1=({},{},{}) ch2=({},{},{}) ch3=({},{},{})",
            hair_now.channel1.r, hair_now.channel1.g, hair_now.channel1.b,
            hair_now.channel2.r, hair_now.channel2.g, hair_now.channel2.b,
            hair_now.channel3.r, hair_now.channel3.g, hair_now.channel3.b,
            head_now.channel1.r, head_now.channel1.g, head_now.channel1.b,
            head_now.channel2.r, head_now.channel2.g, head_now.channel2.b,
            head_now.channel3.r, head_now.channel3.g, head_now.channel3.b,
        )
        .unwrap();
    }

    let g = GffParser::from_bytes(current).expect("final parse");
    let f: IndexMap<String, GffValue<'static>> = g
        .read_struct_fields(0)
        .expect("final root")
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    writeln!(
        out,
        "\n=== FINAL raw GFF after 3 unchanged apply cycles ==="
    )
    .unwrap();
    dump_tint_struct(&mut out, "Tint_Hair", f.get("Tint_Hair"));
    dump_tint_struct(&mut out, "Tint_Head", f.get("Tint_Head"));

    let out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target_test")
        .join("hair_tint_roundtrip.txt");
    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(&out_path, &out).unwrap();
    eprintln!("{out}");
}
