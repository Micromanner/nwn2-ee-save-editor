//! Prepares and reads the in-game alignment test slots (quest editor `ga_alignment`).

use app_lib::config::NWN2Paths;
use app_lib::state::session_state::SessionState;

#[path = "../common/mod.rs"]
mod common;

const BASE: &str = "000067 - 12-09-2026-20-47";
const SLOTS: &[&str] = &["000912 - CLAUDE Align Game", "000913 - CLAUDE Align Editor"];

fn print_alignments(dir: &std::path::Path) {
    let mut session = SessionState::new();
    common::load_save(&mut session, dir);
    let pc = session.character().expect("player").alignment();
    println!(
        "{}\n  PC: law_chaos {} good_evil {}",
        dir.display(),
        pc.law_chaos,
        pc.good_evil
    );
    for member in session.list_roster().expect("roster") {
        let a = session
            .companion_snapshot(&member.ros_name)
            .expect("companion")
            .alignment();
        println!(
            "  {} (party {}): law_chaos {} good_evil {}",
            member.ros_name, member.in_party, a.law_chaos, a.good_evil
        );
    }
}

#[tokio::test]
#[ignore = "writes into the user's save folder"]
async fn prepare_alignment_saves() {
    let ctx = common::create_test_context().await;
    let saves = NWN2Paths::new().saves().expect("saves");
    for slot in SLOTS {
        let dst = saves.join(slot);
        assert!(!dst.exists(), "{slot} already exists");
        common::copy_dir_recursive(&saves.join(BASE), &dst).expect("copy");
        std::fs::write(dst.join("savename.txt"), slot.split(" - ").nth(1).unwrap()).unwrap();
        let mut session = SessionState::new();
        common::load_save(&mut session, &dst);
        session
            .character_mut()
            .expect("player")
            .set_alignment(None, Some(69))
            .expect("alignment");
        session
            .save_character(&ctx.game_data, &NWN2Paths::new(), 0)
            .expect("save");
        print_alignments(&dst);
    }
}

/// `ALIGN_SAVE=<save dir>`: PC and roster alignments of any save.
#[test]
#[ignore = "reads a save named by ALIGN_SAVE"]
fn read_alignments() {
    print_alignments(std::path::Path::new(&std::env::var("ALIGN_SAVE").unwrap()));
}
