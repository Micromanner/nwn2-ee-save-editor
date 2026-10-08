use app_lib::file_operations::{list_backups, list_saves};
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[path = "common/mod.rs"]
mod common;

#[test]
fn browse_backups_root_populates_character_and_save_name_from_newest_inner_backup() {
    let temp = TempDir::new().expect("tempdir");
    let backups_root = temp.path().join("backups");
    let save_folder = backups_root.join("000001 - Save");
    let older = save_folder.join("backup_20260101_000000");
    let newer = save_folder.join("backup_20260401_120000");

    fs::create_dir_all(&older).expect("create older");
    // Ensure the newer directory is created with a strictly later mtime/ctime.
    thread::sleep(Duration::from_millis(20));
    fs::create_dir_all(&newer).expect("create newer");

    // Older backup has a different savename and no playerinfo.bin so we can detect
    // the wrong one being picked up.
    fs::write(older.join("savename.txt"), "Old Save").expect("write old savename");

    // Copy a real playerinfo.bin from fixtures so PlayerInfo::get_player_name parses cleanly.
    let fixture_player_info = common::fixtures_path().join("saves/Classic_Campaign/playerinfo.bin");
    fs::copy(&fixture_player_info, newer.join("playerinfo.bin"))
        .expect("copy playerinfo.bin fixture");
    fs::write(newer.join("savename.txt"), "My Newest Save").expect("write new savename");
    // Add one small file so size > 0 is reported.
    fs::write(newer.join("player.bic"), b"dummy bytes").expect("write player.bic");

    let result = list_backups(&backups_root, true).expect("list_backups ok");

    assert_eq!(result.len(), 1, "expected one save folder at root");
    let entry = &result[0];
    assert_eq!(entry.name, "000001 - Save");
    assert_eq!(
        entry.save_name.as_deref(),
        Some("My Newest Save"),
        "save_name should come from the newer backup, not the older one"
    );
    assert!(
        entry.character_name.is_some(),
        "character_name should be read from the newer backup's playerinfo.bin"
    );
    assert!(
        entry.size > 0,
        "size should reflect files in the newer backup"
    );
}

#[test]
fn browse_backups_save_folder_lists_only_timestamped_backups() {
    let temp = TempDir::new().expect("tempdir");
    let save_folder = temp.path().join("000001 - Save");
    fs::create_dir_all(save_folder.join("backup_20260101_000000")).expect("create backup");
    fs::create_dir_all(save_folder.join("not a backup")).expect("create other dir");

    let result = list_backups(&save_folder, false).expect("list_backups ok");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "backup_20260101_000000");
}

/// Writes one EE-layout save folder per name (`savename.txt`), oldest first,
/// a second apart because save times have second precision.
fn write_saves(dir: &std::path::Path, names: &[&str]) {
    for (i, name) in names.iter().enumerate() {
        let save = dir.join(format!("{:06}", i + 1));
        fs::create_dir_all(&save).expect("create save");
        fs::write(save.join("resgff.zip"), b"dummy").expect("write resgff.zip");
        fs::write(save.join("savename.txt"), name).expect("write savename");
        thread::sleep(Duration::from_millis(1100));
    }
}

#[test]
fn browse_saves_pages_newest_first() {
    let temp = TempDir::new().expect("tempdir");
    write_saves(temp.path(), &["One", "Two", "Three"]);
    fs::create_dir_all(temp.path().join("backups")).expect("create backups");

    let first = list_saves(temp.path(), 2, 0, None).expect("list_saves ok");
    let second = list_saves(temp.path(), 2, 2, None).expect("list_saves ok");

    let names = |r: &app_lib::file_operations::BrowseSavesResponse| {
        r.files
            .iter()
            .map(|f| f.save_name.clone().unwrap_or_default())
            .collect::<Vec<_>>()
    };
    assert_eq!(first.total_count, 3);
    assert_eq!(names(&first), ["Three", "Two"]);
    assert_eq!(names(&second), ["One"]);
    assert!(first.files.iter().all(|f| f.size > 0));
}

#[test]
fn browse_saves_search_filters_before_paging() {
    let temp = TempDir::new().expect("tempdir");
    write_saves(
        temp.path(),
        &["Act 1 Docks", "Act 2 Crossroad", "act 1 Flagon"],
    );

    let result = list_saves(temp.path(), 1, 0, Some("  ACT 1 ")).expect("list_saves ok");

    assert_eq!(result.total_count, 2);
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].save_name.as_deref(), Some("act 1 Flagon"));
}
