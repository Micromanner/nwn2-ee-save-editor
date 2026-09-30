use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;

use crate::services::savegame_handler::SAVENAME_TXT;

/// Save slots reserved for generated test saves. Everything destructive is confined to it.
const TEST_SLOTS: RangeInclusive<u32> = 950..=999;

#[derive(Debug, Serialize)]
pub struct TestSave {
    pub slot: u32,
    pub folder: String,
    pub name: Option<String>,
    pub loaded: bool,
}

/// The slot number a save folder name starts with (`000067 - Quest-Edit-Test` -> 67).
fn slot_number(folder_name: &str) -> Option<u32> {
    let digits = folder_name.get(..6)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let rest = &folder_name[6..];
    if !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    digits.parse().ok()
}

fn folders(saves: &Path) -> Result<Vec<(u32, PathBuf)>, String> {
    let entries =
        std::fs::read_dir(saves).map_err(|e| format!("cannot read {}: {e}", saves.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| Some((slot_number(&e.file_name().to_string_lossy())?, e.path())))
        .collect())
}

/// The save folder using `slot`. Templates are only read, so any save qualifies.
pub fn find_template(saves: &Path, slot: u32) -> Result<PathBuf, String> {
    let matches: Vec<PathBuf> = folders(saves)?
        .into_iter()
        .filter(|(n, _)| *n == slot)
        .map(|(_, p)| p)
        .collect();
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!(
            "save slot {slot:06} not found in {}",
            saves.display()
        )),
        _ => Err(format!("several save folders use slot {slot:06}")),
    }
}

pub fn test_range() -> String {
    format!("{:06}-{:06}", TEST_SLOTS.start(), TEST_SLOTS.end())
}

/// Held while picking a slot and copying into it, so concurrent calls cannot pick the same one.
static CREATE_LOCK: Mutex<()> = Mutex::new(());

/// Copies `template` into the first free test slot and returns the new folder.
pub fn create(saves: &Path, template: &Path, name: &str) -> Result<PathBuf, String> {
    let _guard = CREATE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let used: Vec<u32> = folders(saves)?.into_iter().map(|(n, _)| n).collect();
    let slot = TEST_SLOTS
        .clone()
        .find(|n| !used.contains(n))
        .ok_or_else(|| {
            format!(
                "all test slots {} are used; delete some first",
                test_range()
            )
        })?;
    let folder = saves.join(format!("{slot:06} - {}", folder_safe(name)));
    crate::services::savegame_handler::backup::copy_save_files(template, &folder)
        .map_err(|e| format!("failed to copy template: {e}"))?;
    Ok(folder)
}

pub fn list(saves: &Path, loaded: Option<&Path>) -> Result<Vec<TestSave>, String> {
    let mut out: Vec<TestSave> = folders(saves)?
        .into_iter()
        .filter(|(n, _)| TEST_SLOTS.contains(n))
        .map(|(slot, path)| TestSave {
            slot,
            folder: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            name: std::fs::read_to_string(path.join(SAVENAME_TXT))
                .ok()
                .map(|s| s.trim().to_string()),
            loaded: loaded.is_some_and(|l| same_path(l, &path)),
        })
        .collect();
    out.sort_by_key(|s| s.slot);
    Ok(out)
}

/// Deletes test saves. `slots: None` deletes every test save except the loaded one.
pub fn delete(
    saves: &Path,
    slots: Option<&[u32]>,
    loaded: Option<&Path>,
) -> Result<Vec<u32>, String> {
    let targets: Vec<(u32, PathBuf)> = folders(saves)?
        .into_iter()
        .filter(|(n, _)| slots.map_or(TEST_SLOTS.contains(n), |s| s.contains(n)))
        .filter(|(_, p)| slots.is_some() || !loaded.is_some_and(|l| same_path(l, p)))
        .collect();
    for (_, path) in &targets {
        ensure_deletable(saves, path, loaded)?;
    }
    let mut deleted = Vec::new();
    for (slot, path) in targets {
        std::fs::remove_dir_all(&path).map_err(|e| {
            format!(
                "failed to delete {}: {e}; already deleted: {deleted:?}",
                path.display()
            )
        })?;
        deleted.push(slot);
    }
    Ok(deleted)
}

/// The only gate in front of `remove_dir_all`: a direct child of the saves folder, in the
/// test slot range, and not the save the editor has open.
fn ensure_deletable(saves: &Path, path: &Path, loaded: Option<&Path>) -> Result<(), String> {
    let slot = path
        .file_name()
        .and_then(|n| slot_number(&n.to_string_lossy()))
        .ok_or_else(|| format!("{} is not a save slot folder", path.display()))?;
    if !TEST_SLOTS.contains(&slot) {
        return Err(format!(
            "slot {slot:06} is outside the test range {}",
            test_range()
        ));
    }
    let parent = path.parent().ok_or("path has no parent")?;
    if !same_path(parent, saves) {
        return Err(format!(
            "{} is not inside {}",
            path.display(),
            saves.display()
        ));
    }
    if loaded.is_some_and(|l| same_path(l, path)) {
        return Err(format!("slot {slot:06} is loaded in the editor"));
    }
    Ok(())
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn folder_safe(name: &str) -> String {
    name.chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_number_reads_leading_six_digits() {
        assert_eq!(slot_number("000067 - Quest-Edit-Test"), Some(67));
        assert_eq!(slot_number("000950"), Some(950));
        assert_eq!(slot_number("backups"), None);
        assert_eq!(slot_number("0009501 - x"), None);
        assert_eq!(slot_number("00095"), None);
    }

    #[test]
    fn delete_guard_rejects_everything_outside_the_test_range() {
        let root = tempfile::tempdir().expect("tempdir");
        let saves = root.path();
        for name in [
            "000905 - QT Party",
            "000950 - TEST",
            "000999 - TEST",
            "001000 - x",
        ] {
            std::fs::create_dir(saves.join(name)).expect("mkdir");
        }
        let nested = saves.join("000950 - TEST").join("000951 - nested");
        std::fs::create_dir(&nested).expect("mkdir");

        assert!(ensure_deletable(saves, &saves.join("000905 - QT Party"), None).is_err());
        assert!(ensure_deletable(saves, &saves.join("001000 - x"), None).is_err());
        assert!(ensure_deletable(saves, &nested, None).is_err());
        assert!(ensure_deletable(saves, &saves.join("000950 - TEST"), None).is_ok());
        let loaded = saves.join("000999 - TEST");
        assert!(ensure_deletable(saves, &loaded, Some(&loaded)).is_err());
    }

    #[test]
    fn delete_all_keeps_the_loaded_save_and_non_test_slots() {
        let root = tempfile::tempdir().expect("tempdir");
        let saves = root.path();
        for name in ["000067 - Template", "000950 - A", "000951 - B"] {
            std::fs::create_dir(saves.join(name)).expect("mkdir");
        }
        let loaded = saves.join("000951 - B");
        let deleted = delete(saves, None, Some(&loaded)).expect("delete");
        assert_eq!(deleted, vec![950]);
        assert!(saves.join("000067 - Template").exists());
        assert!(loaded.exists());
        assert!(delete(saves, Some(&[951]), Some(&loaded)).is_err());
    }
}
