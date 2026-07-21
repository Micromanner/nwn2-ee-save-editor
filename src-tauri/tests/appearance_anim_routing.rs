//! Routing helper coverage for tail/wing idle animation selection.
//!
//! The plain `#[test]` cases exercise `is_synced_base_anims` in isolation
//! (no ResourceManager needed) and must pass in CI. The `#[tokio::test]`
//! creature smoke test is `#[ignore]`d because it depends on a real NWN2 EE
//! install; run it manually with `-- --ignored`.

use app_lib::services::model_loader::is_synced_base_anims;

#[test]
fn tiefling_tail_is_synced() {
    assert!(is_synced_base_anims("P_HHM_skel", Some("P_HHM")));
    assert!(is_synced_base_anims("P_HHF_Skel", Some("P_HHF")));
}

#[test]
fn dragon_tail_is_not_synced() {
    assert!(!is_synced_base_anims("P_HHM_skel", Some("c_dragon")));
}

#[test]
fn gargoyle_wings_are_not_synced() {
    assert!(!is_synced_base_anims("P_HHM_skel", Some("c_wingleg")));
}

#[test]
fn blank_base_anims_is_not_synced() {
    assert!(!is_synced_base_anims("P_HHM_skel", None));
}

#[test]
fn matching_is_case_insensitive() {
    assert!(is_synced_base_anims("P_HHM_skel", Some("p_hhm")));
}

// Guarded: only runs when the NWN2 EE install is present. Mirrors the
// ResourceManager setup used in
// src-tauri/tests/debugging/diagnose_head_hair_mask_format.rs.
#[tokio::test]
#[ignore = "requires a real NWN2 EE install; run manually with -- --ignored"]
async fn dragon_idle_set_loads_tail_tracks() {
    use app_lib::services::resource_manager::ResourceManager;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let nwn2_paths = Arc::new(RwLock::new(app_lib::config::NWN2Paths::new()));
    let rm = Arc::new(RwLock::new(ResourceManager::new(nwn2_paths)));
    {
        let mut g = rm.write().await;
        let _ = g.initialize().await;
    }
    let rm = rm.read().await;

    let anims = app_lib::services::model_loader::load_idle_animations_for_prefix(&rm, "c_dragon");
    assert!(!anims.is_empty(), "c_dragon idle set should load clips");
    let has_tail = anims
        .iter()
        .any(|a| a.tracks.iter().any(|t| t.bone_name.starts_with("Tail")));
    assert!(has_tail, "c_dragon idle should contain Tail* bone tracks");
}
