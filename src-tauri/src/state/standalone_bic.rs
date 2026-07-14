//! Load and save standalone .bic character files (localvault import).
//!
//! Unlike save games, a vault character is a single GFF file with the same
//! root layout as a playerlist entry, so it reuses the player.bic parsing
//! pipeline and `merge_fields_into_gff` for write-back.

use indexmap::IndexMap;
use std::path::Path;

use crate::parsers::gff::GffValue;
use crate::state::session_state::read_player_bic_entry;

pub fn load_fields(path: &Path) -> Result<IndexMap<String, GffValue<'static>>, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("Failed to read character file {}: {e}", path.display()))?;
    if bytes.len() < 4 || &bytes[0..4] != b"BIC " {
        return Err(format!(
            "{} is not a valid character file (missing BIC header)",
            path.display()
        ));
    }
    read_player_bic_entry(bytes)
}

pub fn save(
    bic_path: &Path,
    char_fields: &IndexMap<String, GffValue<'static>>,
    backup_keep_count: usize,
    create_backup: bool,
) -> Result<(), String> {
    let original = std::fs::read(bic_path)
        .map_err(|e| format!("Failed to read character file {}: {e}", bic_path.display()))?;

    if create_backup {
        backup_original(bic_path, &original, backup_keep_count)?;
    }

    let bytes =
        crate::parsers::gff::merge_fields_into_gff(Some(&original), char_fields, "BIC ", false)?;

    let tmp = bic_path.with_extension("bic.tmp");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("Failed to write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, bic_path)
        .map_err(|e| format!("Failed to replace {}: {e}", bic_path.display()))?;

    tracing::info!("Standalone character saved: {}", bic_path.display());
    Ok(())
}

fn backup_original(bic_path: &Path, original: &[u8], keep_count: usize) -> Result<(), String> {
    let vault_dir = bic_path
        .parent()
        .ok_or_else(|| "Character file has no parent directory".to_string())?;
    let stem = bic_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("character");
    let backup_dir = vault_dir.join("backups").join(stem);
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup directory: {e}"))?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("backup_{timestamp}.bic"));
    std::fs::write(&backup_path, original).map_err(|e| format!("Failed to write backup: {e}"))?;
    tracing::info!("Created vault backup: {}", backup_path.display());

    prune_backups(&backup_dir, keep_count);
    Ok(())
}

/// Timestamped names sort lexicographically in chronological order, so a
/// plain sort puts the oldest first. Prune failures are non-fatal.
fn prune_backups(backup_dir: &Path, keep_count: usize) {
    let keep_count = keep_count.max(1);
    let Ok(entries) = std::fs::read_dir(backup_dir) else {
        return;
    };
    let mut backups: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("backup_") && n.ends_with(".bic"))
        })
        .collect();
    backups.sort();
    while backups.len() > keep_count {
        let oldest = backups.remove(0);
        if std::fs::remove_file(&oldest).is_err() {
            tracing::warn!("Failed to prune old vault backup: {}", oldest.display());
        }
    }
}
