//! Integration tests for the `toolset-bridge` client.
//!
//! These tests depend on the user's real NWN2:EE install and the committed
//! `toolset-bridge/dist/toolset-bridge.exe`. If either is missing the tests
//! self-skip so they don't explode on CI machines that lack the game.
//!
//! Env var `NWN2_INSTALL` overrides the default Steam path.

use std::path::{Path, PathBuf};

use app_lib::services::toolset_bridge::{BridgeClient, ResolutionKind};
use tempfile::TempDir;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

fn project_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == src-tauri; the bridge lives one dir up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn bridge_exe() -> PathBuf {
    project_root().join("toolset-bridge/dist/toolset-bridge.exe")
}

fn nwn2_install() -> PathBuf {
    if let Ok(env_path) = std::env::var("NWN2_INSTALL") {
        return PathBuf::from(env_path);
    }
    PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common/NWN2 Enhanced Edition")
}

fn skip_if_prereqs_missing(bridge: &Path, install: &Path) -> bool {
    if !bridge.is_file() {
        eprintln!(
            "skipping: bridge not built at {} — run `dotnet build -c Release -p:Platform=x64` in toolset-bridge/",
            bridge.display()
        );
        return true;
    }
    if !install.is_dir() {
        eprintln!("skipping: NWN2 install not found at {}", install.display());
        return true;
    }
    false
}

#[test]
fn list_modules_parses_oc_campaign() {
    let bridge = bridge_exe();
    let install = nwn2_install();
    if skip_if_prereqs_missing(&bridge, &install) {
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
    let bridge = bridge_exe();
    let install = nwn2_install();
    if skip_if_prereqs_missing(&bridge, &install) {
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
    // Shape sanity: graph output must contain the five top-level sections.
    for key in ["module", "journal", "factions", "module_variables", "convo"] {
        assert!(first.get(key).is_some(), "missing key in graph json: {key}");
    }
    let journal_cats = first["journal"]["categories"]
        .as_array()
        .expect("categories");
    assert!(
        !journal_cats.is_empty(),
        "tutorial must have journal categories"
    );
    let convo_nodes = first["convo"]["nodes"].as_array().expect("nodes");
    assert!(!convo_nodes.is_empty(), "tutorial must have convo nodes");

    // Warm call: cache hit; output must match.
    let second = client.graph(&tutorial).expect("graph warm");
    assert_eq!(first, second, "cached graph diverged from fresh graph");

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
