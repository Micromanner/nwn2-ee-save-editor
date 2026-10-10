use app_lib::config::NWN2Paths;
use app_lib::services::campaign::CampaignManager;
use app_lib::services::savegame_handler::SaveGameHandler;
// use std::fs;
use tempfile::TempDir;

#[path = "../common/mod.rs"]
mod common;

#[test]
fn test_campaign_manager_read_operations() {
    let fixture_path = common::fixtures_path().join("saves/Classic_Campaign");

    // Copy to temp dir to avoid SaveGameHandler creating backups in the fixture directory.
    let temp_dir = TempDir::new().unwrap();
    let save_path = temp_dir.path().join("Classic_Campaign");

    // Copy directory
    common::copy_dir_recursive(&fixture_path, &save_path).expect("Failed to copy fixture");

    let handler = SaveGameHandler::new(&save_path).expect("Failed to create handler");
    let paths = NWN2Paths::new(); // Default paths, likely mostly empty but should work for basic logic

    // Test get_summary
    let summary = CampaignManager::get_summary(&handler).expect("Failed to get summary");
    // Assert on some known values from Classic_Campaign if possible, or just presence
    assert!(
        summary.general_info.contains_key("game_act")
            || summary.general_info.contains_key("player_name")
    );

    // Test get_module_info
    let (info, vars) =
        CampaignManager::get_module_info(&handler, &paths).expect("Failed to get module info");
    assert!(!info.module_name.is_empty());
    assert_eq!(
        info.area_name, "{1019}tombs",
        "player AreaId should map to the current area, not the module entry area"
    );
    assert!(
        info.area_name_strref >= 0,
        "area {} should resolve its .are Name strref",
        info.area_name
    );
    assert!(!vars.integers.is_empty() || !vars.strings.is_empty() || !vars.floats.is_empty());

    // Test analyze_quest_progress
    let overview =
        CampaignManager::analyze_quest_progress(&handler).expect("Failed to analyze quests");
    assert!(overview.active_count + overview.completed_count > 0);
}

#[test]
fn area_name_fallback_drops_numeric_prefix() {
    use app_lib::services::campaign::content::area_name_from_resref;
    assert_eq!(area_name_from_resref("1100_west_harbor"), "West Harbor");
    assert_eq!(area_name_from_resref("1008gentry"), "Gentry");
    assert_eq!(area_name_from_resref("1234"), "1234");
    assert_eq!(area_name_from_resref("{1013}merchant q"), "Merchant Q");
}

#[test]
fn strip_area_code_removes_brace_prefix() {
    use app_lib::services::campaign::content::strip_area_code;
    assert_eq!(
        strip_area_code("{1008}Lord Dalren's House"),
        "Lord Dalren's House"
    );
    assert_eq!(strip_area_code("West Harbor"), "West Harbor");
    assert_eq!(strip_area_code("{abc}Keep"), "{abc}Keep");
}

// Fixture reality: MOTB/globals.xml carries a single companion variable
// (00_nInfluencesafiya); gann/okku/dove/oneofmany exist only as .ros files.
// The influence page must still list the whole MotB cast and none of the OC.
#[test]
fn companion_influence_lists_whole_campaign_cast_from_roster() {
    let handler =
        SaveGameHandler::new(common::fixtures_path().join("saves/MOTB")).expect("handler");
    let companions = CampaignManager::get_companion_influence(&handler, |_| {}).expect("influence");

    let mut ids: Vec<&str> = companions
        .iter()
        .filter(|(_, s)| s.source == "explicit")
        .map(|(id, _)| id.as_str())
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["gann", "kaelyn", "okku", "one_of_many", "safiya"]);
    assert_eq!(companions["safiya"].influence, Some(-6));
    assert_eq!(companions["kaelyn"].influence, None);
}

// Classic_Campaign has .ros files for the whole OC cast from campaign start;
// every OC companion is listed and no MotB companion leaks in.
#[test]
fn companion_influence_lists_whole_oc_cast() {
    let handler = SaveGameHandler::new(common::fixtures_path().join("saves/Classic_Campaign"))
        .expect("handler");
    let companions = CampaignManager::get_companion_influence(&handler, |_| {}).expect("influence");

    let explicit = companions
        .values()
        .filter(|s| s.source == "explicit")
        .count();
    assert_eq!(explicit, 12);
    assert!(
        !companions.contains_key("safiya"),
        "MotB cast must not leak into an OC save"
    );
}

fn mid_playthrough_influence(
    save: &str,
) -> indexmap::IndexMap<String, app_lib::parsers::xml::CompanionStatus> {
    let handler = SaveGameHandler::new(
        common::fixtures_path()
            .join("saves/mid-playthrough-saves")
            .join(save),
    )
    .expect("handler");
    CampaignManager::get_companion_influence(&handler, |_| {}).expect("influence")
}

// Westgate keeps influence in `HF_COINF_<tag>` (westgate2.hak `CompanionInfluence`);
// Mantides has a .ros but no influence global yet, so he takes the shared prefix.
#[test]
fn westgate_influence_is_matched_to_roster_companions() {
    let companions = mid_playthrough_influence("Westgate");

    let rinara = &companions["c_rinara"];
    assert_eq!(rinara.name, "Rinara");
    assert_eq!(rinara.influence_var, "HF_COINF_c_rinara");
    assert_eq!(rinara.source, "roster");
    assert_eq!(
        companions["c_charissa"].influence_var,
        "HF_COINF_c_charissa"
    );
    let mantides = &companions["c_mantides"];
    assert_eq!(mantides.influence_var, "HF_COINF_c_mantides");
    assert_eq!(mantides.influence, None);
    assert_eq!(companions.len(), 3);
}

// The OC Act 2 save carries influence-like globals that belong to no companion.
#[test]
fn oc_influence_ignores_globals_without_a_roster_companion() {
    let companions = mid_playthrough_influence("OC_Act2");

    let vars: Vec<&str> = companions
        .values()
        .map(|s| s.influence_var.as_str())
        .collect();
    for noise in [
        "0bWolfInfluence",
        "00_bInfluence_Glina",
        "__00_kr_influence_id",
        "0nDarmonInFlagon",
    ] {
        assert!(!vars.contains(&noise), "{noise} is not companion influence");
    }
    assert_eq!(
        companions["npc_bevil"].influence_var,
        "00_nInfluencenpc_bevil"
    );
    assert_eq!(
        companions
            .values()
            .filter(|s| s.source == "explicit")
            .count(),
        12
    );
}

// Campaigns without an influence system list nothing.
#[test]
fn campaigns_without_influence_list_no_companions() {
    for save in ["BG_Reloaded", "SoZ", "IWD"] {
        let companions = mid_playthrough_influence(save);
        assert!(
            companions.is_empty(),
            "{save}: {:?}",
            companions.keys().collect::<Vec<_>>()
        );
    }
}
