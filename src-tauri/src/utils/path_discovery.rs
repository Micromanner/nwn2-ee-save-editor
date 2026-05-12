use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

const KNOWN_GAME_FOLDER_NAMES: &[&str] = &[
    "NWN2 Enhanced Edition",
    "Neverwinter Nights 2 Enhanced Edition",
    "Neverwinter Nights 2",
    "NWN2",
    "Enhanced Edition",
    "Neverwinter Nights 2 Complete",
    "Neverwinter Nights 2 Platinum",
];

const STEAM_APP_IDS: &[&str] = &["2738630", "2760"];

const GOG_PRODUCT_IDS: &[&str] = &["1993442013", "1207659162"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathTiming {
    pub operation: String,
    pub duration_ms: u64,
    pub paths_checked: u32,
    pub paths_found: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResult {
    pub nwn2_paths: Vec<String>,
    pub steam_paths: Vec<String>,
    pub gog_paths: Vec<String>,
    pub total_time_ms: u64,
    pub timing_breakdown: Vec<PathTiming>,
}

pub fn discover_nwn2_paths_rust(
    search_paths: Option<Vec<String>>,
) -> Result<DiscoveryResult, String> {
    let start_time = Instant::now();
    let mut timing_breakdown = Vec::new();

    let candidate_start = Instant::now();
    let mut candidate_paths = if let Some(custom_paths) = search_paths {
        build_candidate_paths_from_roots(custom_paths.into_iter().map(PathBuf::from).collect())
    } else {
        get_default_candidate_paths()
    };
    timing_breakdown.push(PathTiming {
        operation: "candidate_collection".to_string(),
        duration_ms: candidate_start.elapsed().as_millis() as u64,
        paths_checked: candidate_paths.len() as u32,
        paths_found: 0,
    });

    let gog_start = Instant::now();
    let gog_registry_paths = get_gog_install_paths_from_registry();
    let gog_found = gog_registry_paths.len() as u32;
    candidate_paths.extend(gog_registry_paths);
    timing_breakdown.push(PathTiming {
        operation: "gog_registry".to_string(),
        duration_ms: gog_start.elapsed().as_millis() as u64,
        paths_checked: GOG_PRODUCT_IDS.len() as u32,
        paths_found: gog_found,
    });

    let validation_start = Instant::now();
    let mut nwn2_paths = Vec::new();
    let mut steam_paths = Vec::new();
    let mut gog_paths = Vec::new();
    let mut paths_found = 0;

    for candidate in &candidate_paths {
        if !candidate.exists() || !is_nwn2_installation(candidate) {
            continue;
        }

        paths_found += 1;
        let path_str = candidate.to_string_lossy().to_string();
        nwn2_paths.push(path_str.clone());

        let path_lower = path_str.to_lowercase();
        if path_lower.contains("steam") || path_lower.contains("steamapps") {
            steam_paths.push(path_str);
        } else if path_lower.contains("gog") {
            gog_paths.push(path_str);
        }
    }

    timing_breakdown.push(PathTiming {
        operation: "candidate_validation".to_string(),
        duration_ms: validation_start.elapsed().as_millis() as u64,
        paths_checked: candidate_paths.len() as u32,
        paths_found,
    });

    let total_time = start_time.elapsed();

    Ok(DiscoveryResult {
        nwn2_paths: dedupe_string_paths(nwn2_paths),
        steam_paths: dedupe_string_paths(steam_paths),
        gog_paths: dedupe_string_paths(gog_paths),
        total_time_ms: total_time.as_millis() as u64,
        timing_breakdown,
    })
}

pub fn profile_path_discovery_rust(iterations: u32) -> Result<HashMap<String, f64>, String> {
    let mut results = HashMap::new();
    let mut total_times = Vec::new();

    for _ in 0..iterations {
        let start = Instant::now();
        let _ = discover_nwn2_paths_rust(None)?;
        let duration = start.elapsed();
        total_times.push(duration.as_secs_f64());
    }

    if !total_times.is_empty() {
        let mean_time = total_times.iter().sum::<f64>() / total_times.len() as f64;
        let min_time = total_times.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_time = total_times.iter().fold(0.0f64, |a, &b| a.max(b));

        results.insert("mean_seconds".to_string(), mean_time);
        results.insert("min_seconds".to_string(), min_time);
        results.insert("max_seconds".to_string(), max_time);
        results.insert("iterations".to_string(), f64::from(iterations));
    }

    Ok(results)
}

fn get_default_candidate_paths() -> Vec<PathBuf> {
    build_candidate_paths_from_roots(get_default_search_roots())
}

fn build_candidate_paths_from_roots(roots: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = HashSet::new();
    let mut steam_roots = HashSet::new();
    let mut steam_library_roots = HashSet::new();

    for root in roots {
        add_root_candidates(
            &root,
            &mut candidates,
            &mut steam_roots,
            &mut steam_library_roots,
        );
    }

    for steam_root in &steam_roots {
        add_steam_root_candidates(steam_root, &mut candidates, &mut steam_library_roots);
    }

    for library_root in &steam_library_roots {
        add_steam_library_candidates(library_root, &mut candidates);
    }

    let mut candidate_paths: Vec<_> = candidates.into_iter().collect();
    candidate_paths.sort();
    candidate_paths
}

fn add_root_candidates(
    root: &Path,
    candidates: &mut HashSet<PathBuf>,
    steam_roots: &mut HashSet<PathBuf>,
    steam_library_roots: &mut HashSet<PathBuf>,
) {
    candidates.insert(root.to_path_buf());

    for subdir in [
        root.to_path_buf(),
        root.join("Games"),
        root.join("GOG Games"),
        root.join("Program Files").join("GOG Games"),
        root.join("Program Files (x86)").join("GOG Games"),
    ] {
        add_named_install_candidates(&subdir, candidates);
    }

    for steam_root in steam_root_candidates(root) {
        steam_roots.insert(steam_root.clone());
        if steam_root.join("steamapps").exists() {
            steam_library_roots.insert(steam_root);
        }
    }
}

fn steam_root_candidates(root: &Path) -> Vec<PathBuf> {
    vec![
        root.to_path_buf(),
        root.join("Steam"),
        root.join("SteamLibrary"),
        root.join("Program Files").join("Steam"),
        root.join("Program Files (x86)").join("Steam"),
        root.join(".steam").join("steam"),
        root.join(".steam").join("root"),
        root.join(".local").join("share").join("Steam"),
    ]
}

fn add_steam_root_candidates(
    steam_root: &Path,
    candidates: &mut HashSet<PathBuf>,
    steam_library_roots: &mut HashSet<PathBuf>,
) {
    if steam_root.join("steamapps").exists() {
        steam_library_roots.insert(steam_root.to_path_buf());
    }

    if let Some(parent) = steam_root.parent()
        && parent.join("SteamLibrary").exists()
    {
        steam_library_roots.insert(parent.join("SteamLibrary"));
    }

    let libraryfolders_path = steam_root.join("steamapps").join("libraryfolders.vdf");
    for library_root in parse_steam_libraryfolders(&libraryfolders_path) {
        steam_library_roots.insert(library_root);
    }

    add_steam_library_candidates(steam_root, candidates);
}

fn add_steam_library_candidates(library_root: &Path, candidates: &mut HashSet<PathBuf>) {
    add_named_install_candidates(&library_root.join("steamapps").join("common"), candidates);
    add_named_install_candidates(&library_root.join("common"), candidates);

    for install_path in find_steam_installs_via_appmanifest(library_root) {
        candidates.insert(install_path);
    }
}

fn find_steam_installs_via_appmanifest(library_root: &Path) -> Vec<PathBuf> {
    let steamapps = library_root.join("steamapps");
    let mut install_paths = Vec::new();

    for app_id in STEAM_APP_IDS {
        let acf_path = steamapps.join(format!("appmanifest_{app_id}.acf"));
        let Some(install_dir) = parse_acf_installdir(&acf_path) else {
            continue;
        };
        install_paths.push(steamapps.join("common").join(install_dir));
    }

    install_paths
}

fn parse_acf_installdir(acf_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(acf_path).ok()?;
    content
        .lines()
        .find_map(|line| parse_vdf_key_value(line, "installdir"))
}

fn add_named_install_candidates(base: &Path, candidates: &mut HashSet<PathBuf>) {
    candidates.insert(base.to_path_buf());

    for folder_name in KNOWN_GAME_FOLDER_NAMES {
        candidates.insert(base.join(folder_name));
    }
}

fn dedupe_string_paths(paths: Vec<String>) -> Vec<String> {
    let mut unique_paths = Vec::new();
    let mut seen_paths = HashSet::new();

    for path in paths {
        let canonical_path = std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(&path));
        let canonical_str = canonical_path.to_string_lossy().to_string();

        if seen_paths.insert(canonical_str) {
            unique_paths.push(path);
        }
    }

    unique_paths
}

fn get_default_search_roots() -> Vec<PathBuf> {
    let mut roots = HashSet::new();

    #[cfg(target_os = "windows")]
    {
        for drive_root in get_windows_drive_roots() {
            roots.insert(drive_root);
        }

        if let Ok(program_files) = std::env::var("ProgramFiles") {
            roots.insert(PathBuf::from(program_files));
        }

        if let Ok(program_files_x86) = std::env::var("ProgramFiles(x86)") {
            roots.insert(PathBuf::from(program_files_x86));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = dirs::home_dir() {
            roots.insert(home.join(".steam").join("steam"));
            roots.insert(home.join(".steam").join("root"));
            roots.insert(home.join(".local").join("share").join("Steam"));
            roots.insert(home.join("Games"));
        }

        for drive_root in get_wsl_windows_drive_roots() {
            roots.insert(drive_root);
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        if let Some(home) = dirs::home_dir() {
            roots.insert(home.join("Games"));
        }
    }

    let mut root_paths: Vec<_> = roots.into_iter().collect();
    root_paths.sort();
    root_paths
}

#[cfg(target_os = "windows")]
fn get_windows_drive_roots() -> Vec<PathBuf> {
    ('A'..='Z')
        .map(|drive| PathBuf::from(format!("{drive}:/")))
        .filter(|path| path.exists())
        .collect()
}

#[cfg(target_os = "linux")]
fn get_wsl_windows_drive_roots() -> Vec<PathBuf> {
    let mut drive_roots = Vec::new();
    let mnt_root = PathBuf::from("/mnt");

    if let Ok(entries) = std::fs::read_dir(mnt_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_drive = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.len() == 1 && name.chars().all(|ch| ch.is_ascii_alphabetic())
                });

            if is_drive {
                drive_roots.push(path);
            }
        }
    }

    drive_roots.sort();
    drive_roots
}

pub fn find_steam_workshop_for_app(app_id: &str) -> Option<PathBuf> {
    find_steam_workshop_for_app_with_roots(app_id, get_default_search_roots())
}

fn find_steam_workshop_for_app_with_roots(
    app_id: &str,
    search_roots: Vec<PathBuf>,
) -> Option<PathBuf> {
    for library_root in discover_steam_library_roots(search_roots) {
        let workshop = library_root
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join(app_id);
        if workshop.is_dir() {
            return Some(workshop);
        }
    }
    None
}

fn discover_steam_library_roots(search_roots: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut library_roots = HashSet::new();

    for root in search_roots {
        for steam_root in steam_root_candidates(&root) {
            if steam_root.join("steamapps").exists() {
                library_roots.insert(steam_root.clone());
            }
            let vdf = steam_root.join("steamapps").join("libraryfolders.vdf");
            for lib in parse_steam_libraryfolders(&vdf) {
                library_roots.insert(lib);
            }
        }
    }

    let mut sorted: Vec<_> = library_roots.into_iter().collect();
    sorted.sort();
    sorted
}

fn parse_steam_libraryfolders(vdf_path: &Path) -> Vec<PathBuf> {
    let Ok(content) = std::fs::read_to_string(vdf_path) else {
        return Vec::new();
    };

    let mut libraries = Vec::new();

    for line in content.lines() {
        let Some(value) = parse_vdf_key_value(line, "path") else {
            continue;
        };

        let normalized = value.replace("\\\\", "\\");
        libraries.push(PathBuf::from(normalized));
    }

    libraries
}

fn parse_vdf_key_value(line: &str, key: &str) -> Option<String> {
    let tokens: Vec<&str> = line.split('"').collect();
    if tokens.len() >= 4 && tokens[1] == key {
        return Some(tokens[3].to_string());
    }

    None
}

#[cfg(target_os = "windows")]
fn get_gog_install_paths_from_registry() -> Vec<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let mut install_paths = Vec::new();

    for product_id in GOG_PRODUCT_IDS {
        for prefix in [
            "SOFTWARE\\WOW6432Node\\GOG.com\\Games",
            "SOFTWARE\\GOG.com\\Games",
        ] {
            let subkey = format!("{prefix}\\{product_id}");
            let Ok(key) = hklm.open_subkey(&subkey) else {
                continue;
            };
            if let Ok(path) = key.get_value::<String, _>("path") {
                install_paths.push(PathBuf::from(path));
                break;
            }
        }
    }

    install_paths
}

#[cfg(not(target_os = "windows"))]
fn get_gog_install_paths_from_registry() -> Vec<PathBuf> {
    Vec::new()
}

fn is_nwn2_installation(path: &Path) -> bool {
    let indicators = ["data", "dialog.tlk", "nwn2main.exe", "nwn2.exe", "enhanced"];

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let name_lower = name.to_lowercase();
                for indicator in &indicators {
                    if name_lower == indicator.to_lowercase() {
                        return true;
                    }
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{
        KNOWN_GAME_FOLDER_NAMES, build_candidate_paths_from_roots, discover_steam_library_roots,
        find_steam_installs_via_appmanifest, find_steam_workshop_for_app_with_roots,
        parse_acf_installdir, parse_steam_libraryfolders,
    };
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn test_parse_steam_libraryfolders_reads_library_paths() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let steamapps_dir = temp_dir.path().join("steamapps");
        fs::create_dir_all(&steamapps_dir).expect("steamapps dir");

        let vdf_path = steamapps_dir.join("libraryfolders.vdf");
        fs::write(
            &vdf_path,
            "\"libraryfolders\"\n{\n    \"0\"\n    {\n        \"path\"        \"D:\\\\SteamLibrary\"\n    }\n    \"1\"\n    {\n        \"path\"        \"E:\\\\Games\"\n    }\n}\n",
        )
        .expect("write vdf");

        let libraries = parse_steam_libraryfolders(&vdf_path);

        assert_eq!(libraries.len(), 2);
        assert_eq!(libraries[0], PathBuf::from("D:\\SteamLibrary"));
        assert_eq!(libraries[1], PathBuf::from("E:\\Games"));
    }

    #[test]
    fn test_build_candidate_paths_uses_libraryfolders_without_directory_walk() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let steam_root = temp_dir.path().join("Steam");
        let steamapps_dir = steam_root.join("steamapps");
        fs::create_dir_all(&steamapps_dir).expect("steamapps dir");

        let vdf_path = steamapps_dir.join("libraryfolders.vdf");
        let library_root = temp_dir.path().join("SteamLibrary");
        fs::write(
            &vdf_path,
            format!(
                "\"libraryfolders\"\n{{\n    \"0\"\n    {{\n        \"path\"        \"{}\"\n    }}\n}}\n",
                library_root.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write vdf");

        let candidates = build_candidate_paths_from_roots(vec![steam_root]);
        let expected = library_root
            .join("steamapps")
            .join("common")
            .join("NWN2 Enhanced Edition");

        assert!(
            candidates.contains(&expected),
            "steam libraryfolders path should produce a direct candidate"
        );
    }

    #[test]
    fn test_parse_acf_reads_installdir() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let acf_path = temp_dir.path().join("appmanifest_2738630.acf");
        fs::write(
            &acf_path,
            "\"AppState\"\n{\n    \"appid\"        \"2738630\"\n    \"name\"         \"NWN2 Enhanced Edition\"\n    \"installdir\"   \"Enhanced Edition\"\n}\n",
        )
        .expect("write acf");

        assert_eq!(
            parse_acf_installdir(&acf_path),
            Some("Enhanced Edition".to_string())
        );
    }

    #[test]
    fn test_find_steam_installs_via_appmanifest_returns_renamed_folder() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let library_root = temp_dir.path();
        let steamapps = library_root.join("steamapps");
        fs::create_dir_all(&steamapps).expect("steamapps dir");
        fs::write(
            steamapps.join("appmanifest_2738630.acf"),
            "\"AppState\"\n{\n    \"installdir\"   \"Custom Renamed Folder\"\n}\n",
        )
        .expect("write acf");

        let installs = find_steam_installs_via_appmanifest(library_root);

        let expected = steamapps.join("common").join("Custom Renamed Folder");
        assert_eq!(installs, vec![expected]);
    }

    #[test]
    fn test_build_candidate_paths_includes_renamed_folder_via_appmanifest() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let steam_root = temp_dir.path().join("Steam");
        let steamapps_dir = steam_root.join("steamapps");
        fs::create_dir_all(&steamapps_dir).expect("steamapps dir");

        let vdf_path = steamapps_dir.join("libraryfolders.vdf");
        let library_root = temp_dir.path().join("SteamLibrary");
        fs::create_dir_all(library_root.join("steamapps")).expect("library steamapps");
        fs::write(
            &vdf_path,
            format!(
                "\"libraryfolders\"\n{{\n    \"0\"\n    {{\n        \"path\"        \"{}\"\n    }}\n}}\n",
                library_root.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write vdf");
        fs::write(
            library_root
                .join("steamapps")
                .join("appmanifest_2738630.acf"),
            "\"AppState\"\n{\n    \"installdir\"   \"Custom Renamed Folder\"\n}\n",
        )
        .expect("write acf");

        let candidates = build_candidate_paths_from_roots(vec![steam_root]);
        let expected = library_root
            .join("steamapps")
            .join("common")
            .join("Custom Renamed Folder");

        assert!(
            candidates.contains(&expected),
            "appmanifest install dir should be a candidate even when folder is renamed"
        );
    }

    #[test]
    fn test_discover_steam_library_roots_includes_libraryfolders_entries() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let steam_root = temp_dir.path().join("Steam");
        let steamapps_dir = steam_root.join("steamapps");
        fs::create_dir_all(&steamapps_dir).expect("steamapps dir");

        let alt_library = temp_dir.path().join("AltLibrary");
        fs::create_dir_all(alt_library.join("steamapps")).expect("alt library steamapps");

        let vdf_path = steamapps_dir.join("libraryfolders.vdf");
        fs::write(
            &vdf_path,
            format!(
                "\"libraryfolders\"\n{{\n    \"0\"\n    {{\n        \"path\"        \"{}\"\n    }}\n}}\n",
                alt_library.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write vdf");

        let roots = discover_steam_library_roots(vec![temp_dir.path().to_path_buf()]);

        assert!(roots.contains(&steam_root), "primary Steam root detected");
        assert!(
            roots.contains(&alt_library),
            "alt library from VDF detected"
        );
    }

    #[test]
    fn test_find_steam_workshop_for_app_discovers_workshop_in_alt_library() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let steam_root = temp_dir.path().join("Steam");
        let steamapps_dir = steam_root.join("steamapps");
        fs::create_dir_all(&steamapps_dir).expect("steamapps dir");

        let alt_library = temp_dir.path().join("AltLibrary");
        let workshop_dir = alt_library
            .join("steamapps")
            .join("workshop")
            .join("content")
            .join("2738630");
        fs::create_dir_all(&workshop_dir).expect("workshop dir");

        let vdf_path = steamapps_dir.join("libraryfolders.vdf");
        fs::write(
            &vdf_path,
            format!(
                "\"libraryfolders\"\n{{\n    \"0\"\n    {{\n        \"path\"        \"{}\"\n    }}\n}}\n",
                alt_library.to_string_lossy().replace('\\', "\\\\")
            ),
        )
        .expect("write vdf");

        let found =
            find_steam_workshop_for_app_with_roots("2738630", vec![temp_dir.path().to_path_buf()]);

        assert_eq!(found, Some(workshop_dir));
    }

    #[test]
    fn test_find_steam_workshop_for_app_returns_none_when_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let found =
            find_steam_workshop_for_app_with_roots("2738630", vec![temp_dir.path().to_path_buf()]);
        assert_eq!(found, None);
    }

    #[test]
    fn test_known_folder_names_include_new_entries() {
        for expected in [
            "Enhanced Edition",
            "Neverwinter Nights 2 Complete",
            "Neverwinter Nights 2 Platinum",
        ] {
            assert!(
                KNOWN_GAME_FOLDER_NAMES.contains(&expected),
                "KNOWN_GAME_FOLDER_NAMES should include {expected}"
            );
        }
    }
}
