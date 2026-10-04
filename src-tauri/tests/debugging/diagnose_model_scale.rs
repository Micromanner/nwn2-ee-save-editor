//! Diagnostic: dump ModelScale (x, y, z) from the min/max height fixtures
//! to determine the real in-game height range (vs the current slider cap of 1.05).
//!
//! Run with:
//!   cargo test --test debugging diagnose_model_scale -- --ignored --nocapture

use app_lib::parsers::gff::{GffParser, GffValue};
use indexmap::IndexMap;
use std::fs;
use std::path::PathBuf;

fn fixture(variant: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/gff/maxandminsize")
        .join(variant)
        .join("player.bic")
}

fn read_model_scale(label: &str, path: &std::path::Path) {
    let bytes = fs::read(path).expect("read bic");
    let parser = GffParser::from_bytes(bytes).expect("parse gff");
    let root = parser.read_struct_fields(0).expect("root struct");

    let owned: IndexMap<String, GffValue<'static>> = root
        .into_iter()
        .map(|(k, v)| (k, v.force_owned().expect("force owned")))
        .collect();

    let fields: &IndexMap<String, GffValue<'static>> = match owned.get("ModelScale") {
        Some(GffValue::StructOwned(map)) => map.as_ref(),
        Some(other) => panic!("ModelScale is not an owned struct: {other:?}"),
        None => panic!("ModelScale field missing"),
    };

    let get = |k: &str| -> f32 {
        match fields.get(k) {
            Some(GffValue::Float(v)) => *v,
            _ => f32::NAN,
        }
    };

    println!(
        "[{label}] ModelScale  x={:.6}  y={:.6}  z={:.6}  (height=z, girth=x)",
        get("x"),
        get("y"),
        get("z"),
    );
}

#[test]
#[ignore = "diagnostic: prints raw model scale values; run manually with --ignored"]
fn diagnose_model_scale() {
    println!();
    read_model_scale("min", &fixture("min"));
    read_model_scale("max", &fixture("max"));
    println!();
}
