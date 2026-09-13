//! `editor_changes.json` inside a backup directory: which editor changes were
//! saved after the backup was taken, so the user can tell backups apart when
//! choosing one to restore.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::warn;

pub const MANIFEST_FILE: &str = "editor_changes.json";
const MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BackupReason {
    #[default]
    Load,
    PreSave,
    PreRestore,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupManifest {
    pub version: u32,
    pub reason: BackupReason,
    /// One record per successful save made after this backup, oldest first.
    pub saves: Vec<SaveRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SaveRecord {
    /// RFC 3339 local time of the save.
    pub saved_at: String,
    pub characters: Vec<CharacterChanges>,
    pub quests: Vec<QuestChange>,
    pub party: Vec<PartyChange>,
}

impl SaveRecord {
    pub fn is_empty(&self) -> bool {
        self.characters.is_empty() && self.quests.is_empty() && self.party.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterChanges {
    pub name: String,
    /// `None` for the player character.
    pub ros_name: Option<String>,
    /// Undo-history labels in the order they were made.
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestChange {
    pub tag: String,
    pub name: String,
    pub from_state: Option<u32>,
    pub to_state: u32,
    pub state_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyChange {
    pub ros_name: String,
    pub name: Option<String>,
    pub in_party: bool,
}

pub fn write_initial(backup_dir: &Path, reason: BackupReason) {
    let manifest = BackupManifest {
        version: MANIFEST_VERSION,
        reason,
        saves: Vec::new(),
    };
    write(backup_dir, &manifest);
}

/// Append a save record to an existing manifest. A backup deleted earlier in
/// the session is skipped silently; a manifest that fails to parse is replaced.
pub fn append_save(backup_dir: &Path, record: &SaveRecord) {
    if !backup_dir.is_dir() {
        return;
    }
    let mut manifest = read(backup_dir).unwrap_or(BackupManifest {
        version: MANIFEST_VERSION,
        ..Default::default()
    });
    manifest.saves.push(record.clone());
    write(backup_dir, &manifest);
}

/// `None` when the backup predates manifests or the file is unreadable.
pub fn read(backup_dir: &Path) -> Option<BackupManifest> {
    let bytes = fs::read(backup_dir.join(MANIFEST_FILE)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write(backup_dir: &Path, manifest: &BackupManifest) {
    let path = backup_dir.join(MANIFEST_FILE);
    let result = serde_json::to_vec_pretty(manifest)
        .map_err(|e| e.to_string())
        .and_then(|bytes| fs::write(&path, bytes).map_err(|e| e.to_string()));
    if let Err(e) = result {
        warn!("Failed to write backup manifest {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(label: &str) -> SaveRecord {
        SaveRecord {
            saved_at: "2026-09-13T14:00:00+02:00".into(),
            characters: vec![CharacterChanges {
                name: "Occidio".into(),
                ros_name: None,
                changes: vec![label.into()],
            }],
            quests: vec![],
            party: vec![],
        }
    }

    #[test]
    fn missing_manifest_reads_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read(dir.path()).is_none());
    }

    #[test]
    fn initial_then_two_appends_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        write_initial(dir.path(), BackupReason::PreSave);
        let initial = read(dir.path()).unwrap();
        assert_eq!(initial.reason, BackupReason::PreSave);
        assert!(initial.saves.is_empty());

        append_save(dir.path(), &record("Set gold to 5000"));
        append_save(dir.path(), &record("Set experience to 1"));
        let manifest = read(dir.path()).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.reason, BackupReason::PreSave);
        assert_eq!(manifest.saves.len(), 2);
        assert_eq!(
            manifest.saves[1].characters[0].changes[0],
            "Set experience to 1"
        );
    }

    #[test]
    fn append_to_deleted_backup_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone");
        append_save(&gone, &record("x"));
        assert!(!gone.exists());
    }

    #[test]
    fn reason_serializes_snake_case() {
        let json = serde_json::to_string(&BackupReason::PreRestore).unwrap();
        assert_eq!(json, "\"pre_restore\"");
    }
}
