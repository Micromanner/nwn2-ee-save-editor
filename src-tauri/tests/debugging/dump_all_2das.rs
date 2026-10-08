//! Dumps every vanilla 2DA as parsed, for before/after tokenizer diffs.
//! `TDA_DUMP_OUT=<file> cargo test --features integration-tests --test debugging dump_all_2das -- --ignored`

use app_lib::parsers::TDAParser;
use std::io::{Read, Write};

#[test]
#[ignore = "needs the NWN2 EE install"]
fn dump_all_2das() {
    let out_path = std::env::var("TDA_DUMP_OUT").expect("TDA_DUMP_OUT");
    let data = std::path::Path::new(
        r"C:\Program Files (x86)\Steam\steamapps\common\NWN2 Enhanced Edition\data",
    );
    let mut out = std::io::BufWriter::new(std::fs::File::create(out_path).unwrap());
    for zip_name in ["2da.zip", "2da_x1.zip", "2da_x2.zip"] {
        let mut archive =
            zip::ZipArchive::new(std::fs::File::open(data.join(zip_name)).unwrap()).unwrap();
        let mut names: Vec<String> = archive.file_names().map(str::to_string).collect();
        names.sort();
        for name in names.iter().filter(|n| n.to_lowercase().ends_with(".2da")) {
            let mut bytes = Vec::new();
            archive
                .by_name(name)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            let mut parser = TDAParser::new();
            if let Err(e) = parser.parse_from_bytes(&bytes) {
                writeln!(out, "## {zip_name}/{name} ERROR {e}").unwrap();
                continue;
            }
            writeln!(out, "## {zip_name}/{name}").unwrap();
            writeln!(out, "COLUMNS {}", parser.column_names().join(" | ")).unwrap();
            for row in 0..parser.row_count() {
                let cells: Vec<String> = (0..parser.column_count())
                    .map(|c| match parser.get_cell(row, c).unwrap() {
                        Some(v) => format!("{v:?}"),
                        None => "-".into(),
                    })
                    .collect();
                writeln!(out, "{row}: {}", cells.join(" | ")).unwrap();
            }
        }
    }
}
