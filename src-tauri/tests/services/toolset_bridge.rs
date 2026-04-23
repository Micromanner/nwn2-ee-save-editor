//! Integration tests for the `toolset-bridge` client.
//!
//! These tests depend on the user's real NWN2:EE install and the committed
//! `toolset-bridge/dist/toolset-bridge.exe`. If either is missing the tests
//! self-skip so they don't explode on CI machines that lack the game.
//!
//! Env var `NWN2_INSTALL` overrides the default Steam path.

use std::path::Path;

use app_lib::services::toolset_bridge::{BridgeClient, ResolutionKind};
use tempfile::TempDir;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

#[test]
fn list_modules_parses_oc_campaign() {
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    if common::skip_if_toolset_prereqs_missing(&bridge, &install) {
        return;
    }

    let tmp = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install.clone(), tmp.path().to_path_buf()).expect("client");

    let campaign = install.join("campaigns/neverwinter nights 2 campaign");
    let result = client.list_modules(&campaign).expect("list_modules");

    assert_eq!(result.start_module, "0_Tutorial");
    assert!(result.journal_synch);
    // OC ships with more than a handful of modules — exact count is 18 at the time of writing,
    // but we assert a lower bound to tolerate patch changes.
    assert!(
        result.modules.len() >= 10,
        "expected at least 10 modules, got {}",
        result.modules.len()
    );

    let tutorial = result
        .modules
        .iter()
        .find(|m| m.name == "0_Tutorial")
        .expect("OC must list 0_Tutorial");
    assert_eq!(tutorial.resolution_kind, ResolutionKind::Install);
    assert!(Path::new(&tutorial.resolved_path).is_file());
}

#[test]
fn graph_loads_tutorial_and_caches() {
    let bridge = common::toolset_bridge_exe();
    let install = common::nwn2_install_path();
    if common::skip_if_toolset_prereqs_missing(&bridge, &install) {
        return;
    }
    let tutorial = install.join("modules/0_Tutorial.mod");
    if !tutorial.is_file() {
        eprintln!("skipping: 0_Tutorial.mod not present in install");
        return;
    }

    let tmp = TempDir::new().unwrap();
    let client =
        BridgeClient::new(bridge, install.clone(), tmp.path().to_path_buf()).expect("client");

    // Cold call: writes cache.
    let first = client.graph(&tutorial).expect("graph cold");
    assert!(
        !first.module.path.is_empty(),
        "graph must populate module.path"
    );
    assert!(
        !first.journal.categories.is_empty(),
        "tutorial must have journal categories"
    );
    assert!(
        !first.convo.nodes.is_empty(),
        "tutorial must have convo nodes"
    );

    // Warm call: cache hit; must produce equivalent graph.
    let second = client.graph(&tutorial).expect("graph warm");
    assert_eq!(
        first.journal.categories.len(),
        second.journal.categories.len(),
        "cached graph diverged from fresh graph (journal)"
    );
    assert_eq!(
        first.convo.nodes.len(),
        second.convo.nodes.len(),
        "cached graph diverged from fresh graph (convo)"
    );

    // Cache file exists on disk with the expected extension.
    let cache_entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".graph.json"))
        })
        .collect();
    assert_eq!(cache_entries.len(), 1, "expected one cache entry");
}
