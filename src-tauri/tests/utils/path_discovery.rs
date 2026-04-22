use app_lib::config::NWN2Paths;
use app_lib::utils::{discover_nwn2_paths_rust, profile_path_discovery_rust};

fn get_paths() -> NWN2Paths {
    NWN2Paths::new()
}

#[test]
fn test_discover_nwn2_paths() {
    let result = discover_nwn2_paths_rust(None);

    assert!(result.is_ok(), "Path discovery should succeed");
    let discovery = result.unwrap();

    println!("=== NWN2 Path Discovery ===");
    println!("Total time: {}ms", discovery.total_time_ms);
    println!("NWN2 paths found: {}", discovery.nwn2_paths.len());
    println!("Steam paths: {}", discovery.steam_paths.len());
    println!("GOG paths: {}", discovery.gog_paths.len());

    for path in &discovery.nwn2_paths {
        println!("  Found: {path}");
    }

    assert!(
        !discovery.nwn2_paths.is_empty(),
        "Should find at least one NWN2 installation"
    );
}

#[test]
fn test_timing_breakdown() {
    let result = discover_nwn2_paths_rust(None);

    assert!(result.is_ok());
    let discovery = result.unwrap();

    assert!(
        !discovery.timing_breakdown.is_empty(),
        "Should have timing breakdown"
    );

    println!("=== Timing Breakdown ===");
    for timing in &discovery.timing_breakdown {
        println!(
            "  {}: {}ms ({} paths checked, {} found)",
            timing.operation, timing.duration_ms, timing.paths_checked, timing.paths_found
        );
    }
}

#[test]
fn test_steam_path_classification() {
    let result = discover_nwn2_paths_rust(None).unwrap();

    for path in &result.nwn2_paths {
        let is_steam = path.contains("Steam") || path.contains("steamapps");
        let in_steam_paths = result.steam_paths.contains(path);

        if is_steam {
            assert!(in_steam_paths, "Steam path {path} should be in steam_paths");
        }
    }
}

#[test]
fn test_gog_path_classification() {
    let result = discover_nwn2_paths_rust(None).unwrap();

    for path in &result.nwn2_paths {
        let is_gog = path.contains("GOG");
        let in_gog_paths = result.gog_paths.contains(path);

        if is_gog {
            assert!(in_gog_paths, "GOG path {path} should be in gog_paths");
        }
    }
}

#[test]
fn test_profile_path_discovery_benchmark() {
    let result = profile_path_discovery_rust(3);

    assert!(result.is_ok());
    let profile = result.unwrap();

    println!("=== Path Discovery Profile (3 iterations) ===");
    println!(
        "  Mean: {:.4}s",
        profile.get("mean_seconds").unwrap_or(&0.0)
    );
    println!("  Min:  {:.4}s", profile.get("min_seconds").unwrap_or(&0.0));
    println!("  Max:  {:.4}s", profile.get("max_seconds").unwrap_or(&0.0));

    let mean = *profile.get("mean_seconds").unwrap_or(&0.0);
    assert!(
        mean < 5.0,
        "Path discovery should complete in under 5 seconds"
    );
}

#[test]
fn test_discovered_path_is_valid_installation() {
    let result = discover_nwn2_paths_rust(None).unwrap();

    for path_str in &result.nwn2_paths {
        let path = std::path::Path::new(path_str);
        assert!(path.exists(), "Discovered path should exist: {path_str}");

        let has_data = path.join("data").exists();
        let has_tlk = path.join("dialog.tlk").exists();
        let has_exe = path.join("nwn2main.exe").exists() || path.join("nwn2.exe").exists();
        let has_enhanced = path.join("enhanced").exists();

        let is_valid = has_data || has_tlk || has_exe || has_enhanced;

        assert!(
            is_valid,
            "Discovered path should have NWN2 indicators: {path_str}"
        );

        println!(
            "Validated: {path_str} (data={has_data}, tlk={has_tlk}, exe={has_exe}, enhanced={has_enhanced})"
        );
    }
}

#[test]
fn test_no_duplicate_paths() {
    let result = discover_nwn2_paths_rust(None).unwrap();

    let mut seen = std::collections::HashSet::new();

    for path in &result.nwn2_paths {
        let canonical =
            std::fs::canonicalize(path).unwrap_or_else(|_| std::path::PathBuf::from(path));
        let canonical_str = canonical.to_string_lossy().to_string();

        assert!(
            seen.insert(canonical_str.clone()),
            "Duplicate path found: {path}"
        );
    }

    println!("All {} paths are unique", result.nwn2_paths.len());
}

#[test]
fn test_discovered_paths_match_nwn2_paths() {
    let paths = get_paths();

    if let Some(game_folder) = paths.game_folder() {
        let game_folder_str = game_folder.to_string_lossy().to_string();

        let discovery = discover_nwn2_paths_rust(None).unwrap();

        let found = discovery.nwn2_paths.iter().any(|p| {
            let p_canonical = std::fs::canonicalize(p).ok();
            let game_canonical = std::fs::canonicalize(game_folder).ok();

            match (p_canonical, game_canonical) {
                (Some(a), Some(b)) => a == b,
                _ => p == &game_folder_str,
            }
        });

        println!("NWN2Paths game folder: {game_folder_str}");
        println!("Discovered paths: {:?}", discovery.nwn2_paths);

        assert!(found, "NWN2Paths game folder should be in discovered paths");
    }
}

#[test]
fn test_is_enhanced_edition() {
    let paths = get_paths();

    let is_enhanced = paths.is_enhanced_edition();
    let has_enhanced_dir = paths.enhanced().map(|p| p.exists()).unwrap_or(false);

    println!("Is Enhanced Edition: {is_enhanced}");
    println!("Has enhanced directory: {has_enhanced_dir}");

    assert_eq!(
        is_enhanced, has_enhanced_dir,
        "Enhanced edition flag should match enhanced directory presence"
    );
}

#[test]
fn test_is_steam_installation() {
    let paths = get_paths();

    let is_steam = paths.is_steam_installation();

    if let Some(game_folder) = paths.game_folder() {
        let path_contains_steam = game_folder
            .to_string_lossy()
            .to_lowercase()
            .contains("steam");

        println!("Is Steam installation: {is_steam}");
        println!("Path contains 'steam': {path_contains_steam}");
    }
}

#[test]
fn test_is_gog_installation() {
    let paths = get_paths();

    let is_gog = paths.is_gog_installation();

    if let Some(game_folder) = paths.game_folder() {
        let path_contains_gog = game_folder.to_string_lossy().to_lowercase().contains("gog");

        println!("Is GOG installation: {is_gog}");
        println!("Path contains 'gog': {path_contains_gog}");
    }
}

#[cfg(target_os = "windows")]
#[test]
fn test_gog_registry_discovery_on_windows() {
    use std::path::Path;

    let discovery = discover_nwn2_paths_rust(None).expect("discovery succeeds");

    let gog_registry_timing = discovery
        .timing_breakdown
        .iter()
        .find(|t| t.operation == "gog_registry");

    assert!(
        gog_registry_timing.is_some(),
        "gog_registry timing entry should always be recorded on Windows"
    );

    let timing = gog_registry_timing.unwrap();
    println!(
        "gog_registry: {}ms, {} product IDs checked, {} paths found",
        timing.duration_ms, timing.paths_checked, timing.paths_found
    );

    if timing.paths_found == 0 {
        println!("No GOG NWN2 install in registry; test is a no-op.");
        return;
    }

    let found_gog_install = discovery.nwn2_paths.iter().any(|p| {
        let path = Path::new(p);
        path.exists()
            && (path.join("data").exists()
                || path.join("dialog.tlk").exists()
                || path.join("nwn2main.exe").exists())
    });

    assert!(
        found_gog_install,
        "Registry-reported path should appear as a validated install"
    );
}
