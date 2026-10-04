//! Lists every decoded wrapper script on a quest route whose remaining engine calls
//! include an alignment or feat call the editor did not translate.
//!
//!   QUEST_DECODE_SAVE=<save dir> [QUEST_DECODE_DOCS=<user Neverwinter Nights 2 folder>] cargo test --features integration-tests --test debugging dump_quest_decodes -- --ignored --nocapture

use app_lib::config::NWN2Paths;
use app_lib::services::campaign::content::extract_module_info;
use app_lib::services::save_graph::{self, BuildContext};
use app_lib::services::savegame_handler::SaveGameHandler;
use app_lib::services::toolset_bridge::BridgeClient;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

#[test]
#[ignore = "diagnostic; needs QUEST_DECODE_SAVE"]
fn dump_quest_decodes() {
    let Ok(save) = std::env::var("QUEST_DECODE_SAVE") else {
        eprintln!("skipping: QUEST_DECODE_SAVE not set");
        return;
    };
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    if common::skip_if_toolset_prereqs_missing(&bridge, &install) {
        return;
    }
    let handler = SaveGameHandler::new(std::path::Path::new(&save)).expect("handler");
    let mut paths = NWN2Paths::new();
    paths.set_game_folder(&install).expect("game folder");
    if let Ok(docs) = std::env::var("QUEST_DECODE_DOCS") {
        paths.set_documents_folder(docs).expect("documents folder");
    }
    let (module_info, module_vars) = extract_module_info(&handler, &paths).expect("module info");
    let cache_dir = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install, cache_dir.path().to_path_buf()).expect("bridge");
    let graph = save_graph::build(BuildContext {
        handler: &handler,
        paths: &paths,
        client: &client,
        player_index: 0,
        current_module: &module_info,
        current_module_vars: &module_vars,
        progress: None,
    })
    .expect("build");
    println!(
        "modules: {:?}",
        graph
            .modules
            .iter()
            .map(|m| (m.name.clone(), m.resolved_path.clone()))
            .collect::<Vec<_>>()
    );
    println!(
        "campaign: {:?}, quests: {}",
        graph.campaign.campaign_path,
        graph.quests.len()
    );
    let mut decoded_scripts = std::collections::BTreeSet::new();
    for quest in &graph.quests {
        for tr in &quest.transitions {
            for action in tr
                .actions
                .iter()
                .chain(tr.follow_up.iter().flat_map(|f| f.actions.iter()))
            {
                let Some(decoded) = &action.decoded else {
                    continue;
                };
                decoded_scripts.insert(action.script.clone());
                let mentions = |text: &str| {
                    let lower = text.to_ascii_lowercase();
                    lower.contains("alignment") || lower.contains("feat")
                };
                let interesting = decoded.calls.iter().any(|c| mentions(c))
                    || decoded.actions.iter().any(|a| mentions(&a.script));
                if !interesting {
                    continue;
                }
                println!(
                    "quest {} -> {} | {} {} node {} | {}({:?})\n  calls: {:?}\n  actions: {:?}\n  unresolved: {:?}",
                    quest.tag,
                    tr.new_state,
                    tr.module,
                    tr.dlg,
                    tr.node,
                    action.script,
                    action.params,
                    decoded.calls,
                    decoded.actions,
                    decoded.unresolved
                );
            }
        }
    }
    println!("decoded scripts: {decoded_scripts:?}");
}
