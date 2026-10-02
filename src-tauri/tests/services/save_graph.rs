//! Integration tests for the save-graph aggregator.
//!
//! Same self-skip pattern as `toolset_bridge.rs`: these tests depend on a real
//! NWN2:EE install and a committed `toolset-bridge/dist/toolset-bridge.exe`.
//! Without both, they log a skip reason and return success.

use app_lib::config::NWN2Paths;
use app_lib::services::campaign::content::extract_module_info;
use app_lib::services::save_graph::{self, BuildContext};
use app_lib::services::savegame_handler::SaveGameHandler;
use app_lib::services::toolset_bridge::BridgeClient;
use serde_json::json;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

#[test]
fn save_graph_builds_on_classic_campaign() {
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    if common::skip_if_toolset_prereqs_missing(&bridge, &install) {
        return;
    }

    let fixture = common::fixtures_path().join("saves/Classic_Campaign");
    if !fixture.exists() {
        eprintln!("skipping: Classic_Campaign fixture not present");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let save_path = tmp.path().join("Classic_Campaign");
    common::copy_dir_recursive(&fixture, &save_path).expect("copy fixture");

    let handler = SaveGameHandler::new(&save_path, false).expect("handler");

    let mut paths = NWN2Paths::new();
    paths
        .set_game_folder(&install)
        .expect("set_game_folder (install must exist for this test)");

    let (module_info, module_vars) =
        extract_module_info(&handler, &paths).expect("extract_module_info");

    let cache_dir = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install, cache_dir.path().to_path_buf()).expect("BridgeClient");

    let graph = save_graph::build(BuildContext {
        handler: &handler,
        paths: &paths,
        client: &client,
        player_index: 0,
        current_module: &module_info,
        current_module_vars: &module_vars,
        progress: None,
    })
    .expect("save_graph::build");

    // The OC's campaign_id must resolve; otherwise aggregation falls back to a
    // single-module view and we want to know about that during testing.
    assert!(
        graph.campaign.campaign_path.is_some(),
        "expected classic campaign to resolve; orphans = {:?}",
        graph.orphans
    );
    assert!(
        !graph.modules.is_empty(),
        "campaign.cam should list modules for the OC"
    );
    assert!(
        !graph.quests.is_empty(),
        "OC tutorial save should expose at least one quest (live + bridge-declared)"
    );

    // Classic_Campaign fixture's VarTable has real NW_JOURNAL_ENTRY* entries.
    let quests_with_live_state = graph
        .quests
        .iter()
        .filter(|q| q.live_state.is_some())
        .count();
    assert!(
        quests_with_live_state > 0,
        "expected at least one quest with a live VarTable entry"
    );

    // The campaign journal declares the OC quests (no module journal does).
    let reward = graph
        .quests
        .iter()
        .find(|q| q.tag == "MarshalReward")
        .expect("MarshalReward from the campaign module.jrl");
    assert_eq!(reward.category.source, "campaign");
    assert!(
        reward
            .category
            .entries
            .iter()
            .any(|e| e.id == 11 && e.is_final)
    );

    // Docks dialogue drives NPC state through wrapper scripts; the aggregator
    // decodes them from the shipped .nss and synthesizes the global writes.
    let blacklake = graph
        .quests
        .iter()
        .find(|q| q.tag == "choosefaction")
        .expect("choosefaction quest");
    let caleb = blacklake
        .transitions
        .iter()
        .find(|tr| tr.dlg == "10_caleb" && tr.new_state == 20)
        .expect("Caleb line that sets choosefaction 20");
    let wrapper = caleb
        .actions
        .iter()
        .find(|a| a.script == "10a_caleb_state")
        .expect("10a_caleb_state action");
    let decoded = wrapper.decoded.as_ref().expect("wrapper decoded from .nss");
    assert!(decoded.calls.is_empty(), "{decoded:?}");
    let synthesized: Vec<_> = caleb
        .actions
        .iter()
        .filter(|a| a.derived_from.as_deref() == Some("10a_caleb_state(1)"))
        .map(|a| (a.script.as_str(), a.params.clone()))
        .collect();
    assert_eq!(
        synthesized,
        vec![
            ("ga_global_int", vec![json!("CalebState"), json!("1")]),
            ("ga_global_int", vec![json!("gbWatch"), json!("0")]),
            ("ga_lock", vec![json!("10_hagen_door_ext"), json!(0)]),
        ]
    );
    assert!(
        caleb
            .co_authored_globals
            .iter()
            .any(|a| a.params.first() == Some(&json!("CalebState")))
    );

    // Cormick's "Sign me up" line carries no flags itself; the Watch flag sits two
    // lines later on Cormick's reply, reachable through an empty continue reply.
    let cormick = blacklake
        .transitions
        .iter()
        .find(|tr| tr.dlg == "10_cormick" && tr.new_state == 30)
        .expect("Cormick line that sets choosefaction 30");
    assert!(cormick.is_player);
    assert_eq!(cormick.children.len(), 2);
    let watch_line = cormick
        .follow_up
        .iter()
        .find(|n| {
            n.actions
                .iter()
                .any(|a| a.params.first() == Some(&json!("00_bSand_Watch")))
        })
        .expect("follow-up line setting 00_bSand_Watch");
    assert!(!watch_line.is_player);
    let continue_line = cormick
        .follow_up
        .iter()
        .find(|n| n.children.contains(&watch_line.node))
        .expect("continue reply leading to the Watch line");
    assert!(continue_line.is_player && !continue_line.has_text);
}

#[test]
fn save_graph_degrades_without_campaign() {
    // Purely structural test: feed a handler that has no Campaign_ID resolution and
    // assert the aggregator returns a structured `orphans` record instead of erroring.
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    if common::skip_if_toolset_prereqs_missing(&bridge, &install) {
        return;
    }

    let fixture = common::fixtures_path().join("saves/Classic_Campaign");
    if !fixture.exists() {
        eprintln!("skipping: Classic_Campaign fixture not present");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let save_path = tmp.path().join("Classic_Campaign_orphan");
    common::copy_dir_recursive(&fixture, &save_path).expect("copy fixture");

    let handler = SaveGameHandler::new(&save_path, false).expect("handler");

    // Intentionally leave NWN2Paths game_folder unset so the OnceLock-cached
    // campaign_index has nowhere to look.
    let paths = NWN2Paths::new();

    let (mut module_info, module_vars) =
        extract_module_info(&handler, &paths).expect("extract_module_info");
    // Force a never-registered campaign_id so find_campaign_path returns None even
    // when the user's real install is reachable.
    module_info.campaign_id = "deadbeef-never-registered".to_string();

    let cache_dir = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install, cache_dir.path().to_path_buf()).expect("BridgeClient");

    let graph = save_graph::build(BuildContext {
        handler: &handler,
        paths: &paths,
        client: &client,
        player_index: 0,
        current_module: &module_info,
        current_module_vars: &module_vars,
        progress: None,
    })
    .expect("save_graph::build should degrade, not error");

    assert!(graph.campaign.campaign_path.is_none());
    assert!(graph.modules.is_empty());
    assert!(
        graph.orphans.iter().any(|o| matches!(
            o.kind,
            app_lib::services::save_graph::OrphanKind::UnresolvedCampaign
        )),
        "expected an UnresolvedCampaign orphan note; got {:?}",
        graph.orphans
    );
}

#[test]
fn save_graph_transitions_carry_text_strref() {
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    if common::skip_if_toolset_prereqs_missing(&bridge, &install) {
        return;
    }

    let fixture = common::fixtures_path().join("saves/Classic_Campaign");
    if !fixture.exists() {
        eprintln!("skipping: Classic_Campaign fixture not present");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let save_path = tmp.path().join("Classic_Campaign_textstrref");
    common::copy_dir_recursive(&fixture, &save_path).expect("copy fixture");

    let handler = SaveGameHandler::new(&save_path, false).expect("handler");

    let mut paths = NWN2Paths::new();
    paths.set_game_folder(&install).expect("set_game_folder");

    let (module_info, module_vars) =
        extract_module_info(&handler, &paths).expect("extract_module_info");

    let cache_dir = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install, cache_dir.path().to_path_buf()).expect("BridgeClient");

    let graph = save_graph::build(BuildContext {
        handler: &handler,
        paths: &paths,
        client: &client,
        player_index: 0,
        current_module: &module_info,
        current_module_vars: &module_vars,
        progress: None,
    })
    .expect("save_graph::build");

    // At least one transition on the OC should carry a resolvable strref — module
    // authors overwhelmingly use real localized lines on journal-advancing nodes.
    let transitions_with_strref = graph
        .quests
        .iter()
        .flat_map(|q| q.transitions.iter())
        .filter(|t| t.text_strref.is_some())
        .count();
    assert!(
        transitions_with_strref > 0,
        "expected at least one TransitionNode with a populated text_strref"
    );
}
