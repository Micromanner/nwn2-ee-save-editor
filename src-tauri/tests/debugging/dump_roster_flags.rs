//! Dump every `RosMembers` entry's flags and the `.ros` files of a save.
//! Run: `ROSTER_DUMP_SAVE=<save dir> cargo test --features integration-tests --test debugging dump_roster_flags -- --ignored --nocapture`

use app_lib::parsers::gff::{GffParser, GffValue};
use app_lib::services::savegame_handler::SaveGameHandler;

#[test]
#[ignore = "diagnostic; needs ROSTER_DUMP_SAVE"]
fn dump_roster_flags() {
    let save = std::env::var("ROSTER_DUMP_SAVE").expect("ROSTER_DUMP_SAVE not set");
    let handler = SaveGameHandler::new(&save, false, false).expect("handler");
    let files = handler.list_companions().unwrap_or_default();
    println!("== {save}\n.ros files: {files:?}");
    let Ok(Some((_, bytes))) = handler.extract_roster() else {
        println!("no roster file");
        return;
    };
    let gff = GffParser::from_bytes(bytes).expect("parse roster");
    let root = gff.read_struct_fields(0).expect("root");
    let Some(GffValue::ListOwned(members)) =
        root.get("RosMembers").map(|v| v.clone().force_owned())
    else {
        println!("no RosMembers");
        return;
    };
    for member in &members {
        let mut line = String::new();
        for (key, value) in member {
            if matches!(
                value,
                GffValue::List(_) | GffValue::ListOwned(_) | GffValue::StructOwned(_)
            ) || key.starts_with("RosClass")
                || key.starts_with("RosLevel")
            {
                continue;
            }
            use std::fmt::Write as _;
            let _ = write!(line, "{key}={value:?} ");
        }
        println!("{line}");
    }
}
