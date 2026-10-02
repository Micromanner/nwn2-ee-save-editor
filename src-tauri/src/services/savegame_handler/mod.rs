pub mod backup;
pub mod backup_manifest;
pub mod error;

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use tracing::debug;
use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::parsers::gff::GffParser;

pub use error::{SaveGameError, SaveGameResult};

static NWN2_DATE_TIME: LazyLock<zip::DateTime> =
    LazyLock::new(|| zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap_or_default());

const RESGFF_ZIP: &str = "resgff.zip";
const PLAYERLIST_IFO: &str = "playerlist.ifo";
const PLAYER_BIC: &str = "player.bic";
const GLOBALS_XML: &str = "globals.xml";
pub const CURRENTMODULE_TXT: &str = "currentmodule.txt";
const MODULE_IFO: &str = "module.ifo";
const PLAYERINFO_BIN: &str = "playerinfo.bin";
pub const SAVENAME_TXT: &str = "savename.txt";

/// Files an EE save keeps beside `resgff.zip` instead of inside it: the root
/// text/info files and the per-module `.z` state archives.
fn is_loose_in_ee(filename: &str) -> bool {
    matches!(
        filename,
        GLOBALS_XML | MODULE_IFO | CURRENTMODULE_TXT | PLAYERINFO_BIN | SAVENAME_TXT
    ) || filename.to_ascii_lowercase().ends_with(".z")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveFormat {
    /// Enhanced Edition (2025+): player files bundled inside `resgff.zip`.
    Ee,
    /// Original NWN2 (2006-2024, pre-Enhanced Edition — includes the Gold and
    /// Complete bundles): player files loose at the save root.
    Original,
}

impl SaveFormat {
    pub fn detect(save_dir: &Path) -> SaveGameResult<Self> {
        if save_dir.join(RESGFF_ZIP).exists() {
            return Ok(Self::Ee);
        }
        if save_dir.join(PLAYERLIST_IFO).exists() && save_dir.join(PLAYER_BIC).exists() {
            return Ok(Self::Original);
        }
        Err(SaveGameError::InvalidStructure(format!(
            "No save marker found in {} (expected {RESGFF_ZIP} or loose {PLAYER_BIC}+{PLAYERLIST_IFO})",
            save_dir.display()
        )))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
    pub compression: String,
}

pub struct PlayerSources<'a> {
    pub playerlist: &'a [u8],
    pub player_bic: Option<&'a [u8]>,
}

pub struct PlayerOutputs {
    pub playerlist: Vec<u8>,
    /// `None` preserves the source bytes (used for non-primary-slot MP writes).
    pub player_bic: Option<Vec<u8>>,
}

fn commit_outputs(outputs: Vec<(PathBuf, Vec<u8>)>) -> SaveGameResult<()> {
    commit_outputs_with(outputs, |from, to| fs::rename(from, to))
}

#[cfg(test)]
mod transaction_tests {
    use super::*;

    #[test]
    fn preparation_failure_leaves_all_originals_intact() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("globals.xml");
        fs::write(&first, b"original").unwrap();
        let result = commit_outputs(vec![
            (first.clone(), b"changed".to_vec()),
            (dir.path().join("missing/module.z"), vec![1]),
        ]);
        assert!(result.is_err());
        assert_eq!(fs::read(first).unwrap(), b"original");
    }

    #[test]
    fn replacement_failure_restores_previous_files_and_removes_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("globals.xml");
        let second = dir.path().join("playerinfo.bin");
        let third = dir.path().join("resgff.zip");
        fs::write(&first, b"original").unwrap();
        fs::write(&third, b"archive").unwrap();
        let mut calls = 0;
        let result = commit_outputs_with(
            vec![
                (first.clone(), vec![1]),
                (second.clone(), vec![2]),
                (third.clone(), vec![3]),
            ],
            |from, to| {
                calls += 1;
                if calls == 3 {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "injected failure",
                    ))
                } else {
                    fs::rename(from, to)
                }
            },
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("original files restored")
        );
        assert_eq!(fs::read(first).unwrap(), b"original");
        assert_eq!(fs::read(third).unwrap(), b"archive");
        assert!(!second.exists());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn loose_files_wait_for_batch_commit_and_abort_is_reversible() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("playerlist.ifo"), b"original player").unwrap();
        fs::write(dir.path().join("player.bic"), b"original bic").unwrap();
        fs::write(dir.path().join("globals.xml"), b"original globals").unwrap();
        let mut handler = SaveGameHandler::new(dir.path(), false).unwrap();
        handler.begin_batch();
        handler.update_file("globals.xml", b"new globals").unwrap();
        assert_eq!(handler.extract_file("globals.xml").unwrap(), b"new globals");
        assert_eq!(
            fs::read(dir.path().join("globals.xml")).unwrap(),
            b"original globals"
        );
        handler.abort_batch();
        assert_eq!(
            handler.extract_file("globals.xml").unwrap(),
            b"original globals"
        );
        handler.begin_batch();
        handler.update_file("globals.xml", b"new globals").unwrap();
        handler.commit_batch().unwrap();
        assert_eq!(
            fs::read(dir.path().join("globals.xml")).unwrap(),
            b"new globals"
        );
    }
}

fn commit_outputs_with(
    outputs: Vec<(PathBuf, Vec<u8>)>,
    mut replace: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> SaveGameResult<()> {
    let mut prepared = Vec::new();
    for (target, bytes) in outputs {
        let parent = target
            .parent()
            .ok_or_else(|| SaveGameError::Transform("Missing output directory".into()))?;
        let mut new = tempfile::NamedTempFile::new_in(parent)?;
        new.write_all(&bytes)?;
        new.as_file().sync_all()?;
        let original = match fs::read(&target) {
            Ok(bytes) => {
                let mut backup = tempfile::NamedTempFile::new_in(parent)?;
                backup.write_all(&bytes)?;
                backup.as_file().sync_all()?;
                Some(backup.into_temp_path())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        prepared.push((target, new.into_temp_path(), original));
    }
    for (index, (target, new, _)) in prepared.iter().enumerate() {
        if let Err(error) = replace(new, target) {
            let mut failures = Vec::new();
            for (target, _, original) in prepared.iter().take(index).rev() {
                let restored = if let Some(original) = original {
                    fs::copy(original, target).map(|_| ())
                } else {
                    fs::remove_file(target)
                };
                if let Err(e) = restored {
                    failures.push(format!("{}: {e}", target.display()));
                }
            }
            if !failures.is_empty() {
                let mut retained = Vec::new();
                for (_, _, original) in prepared {
                    if let Some(original) = original {
                        match original.keep() {
                            Ok(path) => retained.push(path.display().to_string()),
                            Err(e) => retained.push(e.to_string()),
                        }
                    }
                }
                return Err(SaveGameError::Transform(format!(
                    "Save failed: {error}. Rollback failed: {}. Recovery copies: {}",
                    failures.join("; "),
                    retained.join(", ")
                )));
            }
            return Err(SaveGameError::Transform(format!(
                "Save failed; original files restored: {error}"
            )));
        }
    }
    Ok(())
}

pub struct SaveGameHandler {
    save_dir: PathBuf,
    zip_path: PathBuf,
    format: SaveFormat,
    /// Backup taken by [`Self::new`] before any edit of this session.
    load_backup_path: Option<PathBuf>,
    /// `Some` while a batch is open: `update_file` writes are staged here and
    /// flushed by [`Self::commit_batch`] with one archive rewrite.
    batch: Option<Batch>,
}

#[derive(Default)]
struct Batch {
    zip: indexmap::IndexMap<String, Vec<u8>>,
    loose: indexmap::IndexMap<String, Vec<u8>>,
}

impl SaveGameHandler {
    pub fn new(save_path: impl AsRef<Path>, create_load_backup: bool) -> SaveGameResult<Self> {
        let save_path = save_path.as_ref();

        let (save_dir, zip_path) = if save_path.is_dir() {
            let zip = save_path.join(RESGFF_ZIP);
            (save_path.to_path_buf(), zip)
        } else if save_path.extension().is_some_and(|e| e == "zip") {
            let dir = save_path
                .parent()
                .ok_or_else(|| SaveGameError::InvalidStructure("No parent directory".into()))?;
            (dir.to_path_buf(), save_path.to_path_buf())
        } else {
            return Err(SaveGameError::InvalidStructure(format!(
                "Invalid save path: {}",
                save_path.display()
            )));
        };

        let format = SaveFormat::detect(&save_dir)?;

        let load_backup_path = if create_load_backup {
            Some(backup::create_backup(
                &save_dir,
                backup_manifest::BackupReason::Load,
            )?)
        } else {
            None
        };

        Ok(Self {
            save_dir,
            zip_path,
            format,
            load_backup_path,
            batch: None,
        })
    }

    /// Stage subsequent `update_file` writes in memory so
    /// [`Self::commit_batch`] rewrites `resgff.zip` once for the whole save
    /// operation instead of once per file. Reads observe staged bytes.
    pub fn begin_batch(&mut self) {
        self.batch.get_or_insert_with(Batch::default);
    }

    /// Write all staged files in one transaction and end the batch. A failed
    /// commit drops the staged bytes so callers can retry from their
    /// still-modified in-memory state.
    pub fn commit_batch(&mut self) -> SaveGameResult<()> {
        let Some(batch) = self.batch.take() else {
            return Ok(());
        };
        let mut outputs: Vec<(PathBuf, Vec<u8>)> = batch
            .loose
            .into_iter()
            .map(|(name, bytes)| (self.save_dir.join(name), bytes))
            .collect();
        if !batch.zip.is_empty() {
            outputs.push((self.zip_path.clone(), self.build_zip(&batch.zip)?));
        }
        commit_outputs(outputs)
    }

    pub fn abort_batch(&mut self) {
        self.batch = None;
    }

    /// Run `stage` inside the open batch, or inside a batch of its own that is
    /// committed right after.
    fn in_batch(
        &mut self,
        stage: impl FnOnce(&mut Self) -> SaveGameResult<()>,
    ) -> SaveGameResult<()> {
        if self.batch.is_some() {
            return stage(self);
        }
        self.begin_batch();
        if let Err(e) = stage(self) {
            self.abort_batch();
            return Err(e);
        }
        self.commit_batch()
    }

    pub fn format(&self) -> SaveFormat {
        self.format
    }

    fn is_loose(&self, filename: &str) -> bool {
        self.format == SaveFormat::Original || is_loose_in_ee(filename)
    }

    pub fn extract_file(&self, filename: &str) -> SaveGameResult<Vec<u8>> {
        let not_in_save = || SaveGameError::FileNotInSave {
            filename: filename.into(),
        };

        let loose = self.is_loose(filename);
        if let Some(batch) = &self.batch {
            let staged = if loose { &batch.loose } else { &batch.zip };
            if let Some(bytes) = staged.get(filename) {
                return Ok(bytes.clone());
            }
        }

        if loose {
            return match fs::read(self.save_dir.join(filename)) {
                Ok(bytes) => Ok(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(not_in_save()),
                Err(e) => Err(e.into()),
            };
        }

        let mut archive = ZipArchive::new(File::open(&self.zip_path)?)?;
        let mut entry = archive.by_name(filename).map_err(|e| match e {
            ZipError::FileNotFound => not_in_save(),
            e => e.into(),
        })?;
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn extract_player_data(&self) -> SaveGameResult<Vec<u8>> {
        self.extract_file(PLAYERLIST_IFO)
    }

    pub fn extract_player_bic(&self) -> SaveGameResult<Option<Vec<u8>>> {
        match self.extract_file(PLAYER_BIC) {
            Ok(data) => Ok(Some(data)),
            Err(SaveGameError::FileNotInSave { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn extract_companion(&self, companion_name: &str) -> SaveGameResult<Vec<u8>> {
        let filename = if companion_name.to_lowercase().ends_with(".ros") {
            companion_name.to_string()
        } else {
            format!("{companion_name}.ros")
        };

        match self.extract_file(&filename) {
            Ok(bytes) => Ok(bytes),
            Err(SaveGameError::FileNotInSave { .. }) if self.format == SaveFormat::Original => {
                // 2006-era original-NWN2 saves prefix companion files with "ROS-".
                let prefixed = format!("ROS-{filename}");
                self.extract_file(&prefixed)
            }
            Err(e) => Err(e),
        }
    }

    pub fn extract_globals_xml(&self) -> SaveGameResult<String> {
        String::from_utf8(self.extract_file(GLOBALS_XML)?)
            .map_err(|e| SaveGameError::Transform(format!("Invalid globals.xml encoding: {e}")))
    }

    pub fn extract_current_module(&self) -> SaveGameResult<String> {
        String::from_utf8(self.extract_file(CURRENTMODULE_TXT)?)
            .map(|module| module.trim().to_string())
            .map_err(|e| SaveGameError::Transform(format!("Invalid {CURRENTMODULE_TXT}: {e}")))
    }

    pub fn update_file(&mut self, filename: &str, content: &[u8]) -> SaveGameResult<()> {
        let loose = self.is_loose(filename);
        self.in_batch(|handler| {
            let batch = handler.batch.get_or_insert_with(Batch::default);
            let staged = if loose {
                &mut batch.loose
            } else {
                &mut batch.zip
            };
            staged.insert(filename.to_string(), content.to_vec());
            Ok(())
        })?;
        debug!("Updated file in save: {}", filename);
        Ok(())
    }

    /// Build a copy of `resgff.zip` in memory with every entry in `entries`
    /// substituted (matched case-insensitively) and the ones the archive
    /// doesn't hold yet appended. Untouched entries are copied raw.
    fn build_zip(&self, entries: &indexmap::IndexMap<String, Vec<u8>>) -> SaveGameResult<Vec<u8>> {
        let mut src = ZipArchive::new(File::open(&self.zip_path)?)?;
        let mut dst = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(*NWN2_DATE_TIME);

        let mut written: HashSet<&str> = HashSet::new();
        for i in 0..src.len() {
            let entry = src.by_index_raw(i)?;
            let name = entry.name().to_string();
            match entries
                .iter()
                .find(|(staged, _)| staged.eq_ignore_ascii_case(&name))
            {
                Some((staged, content)) => {
                    dst.start_file(name, options)?;
                    dst.write_all(content)?;
                    written.insert(staged.as_str());
                }
                None => dst.raw_copy_file(entry)?,
            }
        }
        for (name, content) in entries {
            if !written.contains(name.as_str()) {
                dst.start_file(name.as_str(), options)?;
                dst.write_all(content)?;
            }
        }
        Ok(dst.finish()?.into_inner())
    }

    /// Rewrite playerlist.ifo and player.bic in one step.
    ///
    /// The transform closure receives the source bytes for both files and returns
    /// the new bytes. Returning `player_bic: None` preserves the source bytes
    /// unchanged - used for multiplayer non-primary-slot writes where the primary
    /// player's BIC must stay intact.
    pub fn rewrite_player_files<F>(&mut self, transform: F) -> SaveGameResult<()>
    where
        F: FnOnce(PlayerSources<'_>) -> Result<PlayerOutputs, String>,
    {
        let playerlist = self.extract_player_data()?;
        let player_bic = self.extract_player_bic()?;
        let outputs = transform(PlayerSources {
            playerlist: &playerlist,
            player_bic: player_bic.as_deref(),
        })
        .map_err(SaveGameError::Transform)?;

        self.in_batch(|handler| {
            handler.update_file(PLAYERLIST_IFO, &outputs.playerlist)?;
            if let Some(bytes) = &outputs.player_bic {
                handler.update_file(PLAYER_BIC, bytes)?;
            }
            Ok(())
        })
    }

    pub fn list_files(&self) -> SaveGameResult<Vec<FileInfo>> {
        let mut files = Vec::new();

        match self.format {
            SaveFormat::Ee => {
                let file = File::open(&self.zip_path)?;
                let mut archive = ZipArchive::new(file)?;
                for i in 0..archive.len() {
                    let entry = archive.by_index(i)?;
                    files.push(FileInfo {
                        name: entry.name().to_string(),
                        size: entry.size(),
                        compressed_size: entry.compressed_size(),
                        compression: format!("{:?}", entry.compression()),
                    });
                }
            }
            SaveFormat::Original => {
                for entry in fs::read_dir(&self.save_dir)? {
                    let entry = entry?;
                    if !entry.path().is_file() {
                        continue;
                    }
                    let meta = entry.metadata()?;
                    files.push(FileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        size: meta.len(),
                        compressed_size: meta.len(),
                        compression: "Stored".to_string(),
                    });
                }
            }
        }

        Ok(files)
    }

    fn entry_names(&self) -> SaveGameResult<Vec<String>> {
        match self.format {
            SaveFormat::Ee => {
                let archive = ZipArchive::new(File::open(&self.zip_path)?)?;
                let mut names: Vec<String> = archive.file_names().map(str::to_string).collect();
                if let Some(pending) = self.batch.as_ref() {
                    for name in pending.zip.keys() {
                        if !names.iter().any(|n| n == name) {
                            names.push(name.clone());
                        }
                    }
                }
                Ok(names)
            }
            SaveFormat::Original => {
                let mut names = Vec::new();
                for entry in fs::read_dir(&self.save_dir)? {
                    let entry = entry?;
                    if entry.file_type()?.is_file() {
                        names.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
                Ok(names)
            }
        }
    }

    pub fn extract_roster(&self) -> SaveGameResult<Option<(String, Vec<u8>)>> {
        let name = self
            .entry_names()?
            .into_iter()
            .find(|n| n.to_lowercase().ends_with(".rst"));
        match name {
            Some(n) => {
                let bytes = self.extract_file(&n)?;
                Ok(Some((n, bytes)))
            }
            None => Ok(None),
        }
    }

    pub fn companion_stored_name(&self, ros_name: &str) -> SaveGameResult<String> {
        self.entry_names()?
            .into_iter()
            .find(|n| {
                let lower = n.to_lowercase();
                if !lower.ends_with(".ros") {
                    return false;
                }
                let stem = &n[..n.len() - 4];
                let stem = stem.strip_prefix("ROS-").unwrap_or(stem);
                stem.eq_ignore_ascii_case(ros_name)
            })
            .ok_or_else(|| SaveGameError::FileNotInSave {
                filename: format!("{ros_name}.ros"),
            })
    }

    pub fn list_companions(&self) -> SaveGameResult<Vec<String>> {
        let mut companions = Vec::new();

        let names = self.entry_names()?;

        for name in names {
            if !name.to_lowercase().ends_with(".ros") {
                continue;
            }
            let stem = name
                .strip_suffix(".ros")
                .or_else(|| name.strip_suffix(".ROS"))
                .unwrap_or(&name);
            // Strip 2006-era "ROS-" prefix so the canonical companion key matches
            // what callers (and modern original-NWN2 saves) use.
            let canonical = stem.strip_prefix("ROS-").unwrap_or(stem);
            companions.push(canonical.to_string());
        }

        Ok(companions)
    }

    /// `FirstName LastName` from `player.bic`, `None` when the save has no
    /// player.bic or the name is empty.
    pub fn read_character_name(&self) -> SaveGameResult<Option<String>> {
        let Some(data) = self.extract_player_bic()? else {
            return Ok(None);
        };
        let gff = GffParser::from_bytes(data)
            .map_err(|e| SaveGameError::GffParse(format!("Failed to parse player.bic: {e}")))?;
        let fields = gff.read_struct_fields(0).map_err(|e| {
            SaveGameError::GffParse(format!("Failed to read player.bic fields: {e}"))
        })?;
        let name = ["FirstName", "LastName"]
            .into_iter()
            .filter_map(|key| extract_locstring(&fields, key))
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        Ok((!name.is_empty()).then_some(name))
    }

    pub fn load_backup_path(&self) -> Option<&Path> {
        self.load_backup_path.as_deref()
    }

    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }
}

fn extract_locstring(
    fields: &indexmap::IndexMap<String, crate::parsers::gff::GffValue<'_>>,
    key: &str,
) -> Option<String> {
    use crate::parsers::gff::GffValue;

    match fields.get(key)? {
        GffValue::String(s) => Some(s.to_string()),
        GffValue::LocString(ls) => ls.substrings.first().map(|sub| sub.string.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_ee_when_resgff_present() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("resgff.zip"), b"dummy").unwrap();
        assert_eq!(SaveFormat::detect(tmp.path()).unwrap(), SaveFormat::Ee);
    }

    #[test]
    fn test_detect_format_original_when_loose_player_files_present() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        assert_eq!(
            SaveFormat::detect(tmp.path()).unwrap(),
            SaveFormat::Original
        );
    }

    #[test]
    fn test_detect_format_errors_when_neither_marker_present() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            SaveFormat::detect(tmp.path()),
            Err(SaveGameError::InvalidStructure(_))
        ));
    }

    #[test]
    fn test_detect_format_prefers_ee_when_both_markers_present() {
        // An original-NWN2 save opened once in EE will gain a resgff.zip; treat as EE.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("resgff.zip"), b"dummy").unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        assert_eq!(SaveFormat::detect(tmp.path()).unwrap(), SaveFormat::Ee);
    }

    #[test]
    fn test_handler_new_accepts_original_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false)
            .expect("Original-format save directory must be accepted");
        assert_eq!(handler.format(), SaveFormat::Original);
        assert_eq!(handler.save_dir(), tmp.path());
    }

    #[test]
    fn test_extract_file_reads_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC dummy bic payload").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO dummy ifo payload").unwrap();
        std::fs::write(tmp.path().join("companion.ros"), b"ROS dummy ros payload").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();

        assert_eq!(
            handler.extract_file("player.bic").unwrap(),
            b"BIC dummy bic payload"
        );
        assert_eq!(
            handler.extract_file("playerlist.ifo").unwrap(),
            b"IFO dummy ifo payload"
        );
        assert_eq!(
            handler.extract_file("companion.ros").unwrap(),
            b"ROS dummy ros payload"
        );
    }

    #[test]
    fn test_extract_companion_with_ros_prefix_fallback() {
        // 2006-era original-NWN2 saves prefix companion files with "ROS-".
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        std::fs::write(tmp.path().join("ROS-khelgar.ros"), b"ROS khelgar bytes").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();

        assert_eq!(
            handler.extract_companion("khelgar").unwrap(),
            b"ROS khelgar bytes"
        );
    }

    #[test]
    fn test_list_companions_original_strips_ros_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        std::fs::write(tmp.path().join("ROS-khelgar.ros"), b"K").unwrap();
        std::fs::write(tmp.path().join("neeshka.ros"), b"N").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        let mut companions = handler.list_companions().unwrap();
        companions.sort();
        assert_eq!(
            companions,
            vec!["khelgar".to_string(), "neeshka".to_string()]
        );
    }

    #[test]
    fn test_list_files_returns_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        std::fs::write(tmp.path().join("globals.xml"), b"<xml/>").unwrap();
        std::fs::write(tmp.path().join("khelgar.ros"), b"K").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        let files = handler.list_files().unwrap();
        let names: std::collections::HashSet<String> = files.into_iter().map(|f| f.name).collect();
        assert!(names.contains("player.bic"));
        assert!(names.contains("playerlist.ifo"));
        assert!(names.contains("globals.xml"));
        assert!(names.contains("khelgar.ros"));
    }

    #[test]
    fn test_update_file_writes_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC original").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO original").unwrap();

        let mut handler = SaveGameHandler::new(tmp.path(), false).unwrap();

        handler
            .update_file("player.bic", b"BIC rewritten")
            .expect("update_file must succeed on Original");

        assert_eq!(
            std::fs::read(tmp.path().join("player.bic")).unwrap(),
            b"BIC rewritten"
        );
        assert!(
            !tmp.path().join("resgff.zip").exists(),
            "Original-format writes must not synthesize a resgff.zip"
        );
    }

    #[test]
    fn test_batched_updates_commit_in_single_rewrite() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("resgff.zip");
        {
            let file = std::fs::File::create(&zip_path).unwrap();
            let mut zw = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            for (name, content) in [
                ("playerlist.ifo", &b"IFO old"[..]),
                ("khelgar.ros", &b"ROS khelgar old"[..]),
                ("neeshka.ros", &b"ROS neeshka old"[..]),
            ] {
                zw.start_file(name, options).unwrap();
                std::io::Write::write_all(&mut zw, content).unwrap();
            }
            zw.finish().unwrap();
        }

        let mut handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        handler.begin_batch();
        handler
            .update_file("khelgar.ros", b"ROS khelgar new")
            .unwrap();
        handler
            .update_file("neeshka.ros", b"ROS neeshka new")
            .unwrap();
        handler.update_file("added.ros", b"ROS added").unwrap();

        // Staged bytes are visible to reads before the commit.
        assert_eq!(
            handler.extract_file("khelgar.ros").unwrap(),
            b"ROS khelgar new"
        );
        assert!(
            handler
                .entry_names()
                .unwrap()
                .iter()
                .any(|n| n == "added.ros")
        );
        // Nothing hit the archive yet.
        {
            let file = std::fs::File::open(&zip_path).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();
            assert!(archive.by_name("added.ros").is_err());
        }

        handler.commit_batch().unwrap();

        let file = std::fs::File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        for (name, expected) in [
            ("playerlist.ifo", &b"IFO old"[..]),
            ("khelgar.ros", &b"ROS khelgar new"[..]),
            ("neeshka.ros", &b"ROS neeshka new"[..]),
            ("added.ros", &b"ROS added"[..]),
        ] {
            let mut entry = archive.by_name(name).unwrap();
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).unwrap();
            assert_eq!(buf, expected, "{name}");
        }

        // A committed batch is closed: later writes go straight to the zip.
        handler
            .update_file("khelgar.ros", b"ROS khelgar direct")
            .unwrap();
        let file = std::fs::File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut entry = archive.by_name("khelgar.ros").unwrap();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"ROS khelgar direct");
    }

    #[test]
    fn test_rewrite_player_files_writes_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC old").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO old").unwrap();

        let mut handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        handler
            .rewrite_player_files(|src| {
                assert_eq!(src.playerlist, b"IFO old");
                assert_eq!(src.player_bic, Some(&b"BIC old"[..]));
                Ok(PlayerOutputs {
                    playerlist: b"IFO new".to_vec(),
                    player_bic: Some(b"BIC new".to_vec()),
                })
            })
            .unwrap();

        assert_eq!(
            std::fs::read(tmp.path().join("playerlist.ifo")).unwrap(),
            b"IFO new"
        );
        assert_eq!(
            std::fs::read(tmp.path().join("player.bic")).unwrap(),
            b"BIC new"
        );
    }

    #[test]
    fn test_rewrite_player_files_preserves_bic_when_transform_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC keep me").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO old").unwrap();

        let mut handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        handler
            .rewrite_player_files(|_| {
                Ok(PlayerOutputs {
                    playerlist: b"IFO new".to_vec(),
                    player_bic: None,
                })
            })
            .unwrap();

        assert_eq!(
            std::fs::read(tmp.path().join("player.bic")).unwrap(),
            b"BIC keep me"
        );
        assert_eq!(
            std::fs::read(tmp.path().join("playerlist.ifo")).unwrap(),
            b"IFO new"
        );
    }

    #[test]
    fn test_extract_roster_finds_uppercase_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO dummy").unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC dummy").unwrap();
        std::fs::write(tmp.path().join("ROSTER.rst"), b"RST dummy roster").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        let (name, bytes) = handler.extract_roster().unwrap().expect("roster present");
        assert_eq!(name, "ROSTER.rst");
        assert_eq!(bytes, b"RST dummy roster");
    }

    #[test]
    fn test_extract_roster_absent_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO dummy").unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC dummy").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        assert!(handler.extract_roster().unwrap().is_none());
    }

    #[test]
    fn test_companion_stored_name_resolves_prefix_and_case() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO dummy").unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC dummy").unwrap();
        std::fs::write(tmp.path().join("ROS-khelgar.ros"), b"K").unwrap();
        std::fs::write(tmp.path().join("neeshka.ros"), b"N").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        assert_eq!(
            handler.companion_stored_name("khelgar").unwrap(),
            "ROS-khelgar.ros"
        );
        assert_eq!(
            handler.companion_stored_name("NEESHKA").unwrap(),
            "neeshka.ros"
        );
        assert!(handler.companion_stored_name("missing").is_err());
    }

    #[test]
    fn test_handler_new_accepts_ee_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("resgff.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        zw.start_file("playerlist.ifo", zip::write::SimpleFileOptions::default())
            .unwrap();
        use std::io::Write;
        zw.write_all(b"IFO ").unwrap();
        zw.finish().unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).expect("EE save must be accepted");
        assert_eq!(handler.format(), SaveFormat::Ee);
    }

    fn write_stored_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zw = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let options =
            zip::write::SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for (name, content) in entries {
            zw.start_file(*name, options).unwrap();
            zw.write_all(content).unwrap();
        }
        zw.finish().unwrap();
    }

    #[test]
    fn test_ee_writes_zip_entry_even_when_loose_copy_exists() {
        let tmp = tempfile::tempdir().unwrap();
        write_stored_zip(
            &tmp.path().join("resgff.zip"),
            &[("playerlist.ifo", b"IFO"), ("khelgar.ros", b"ROS zip")],
        );
        std::fs::write(tmp.path().join("khelgar.ros"), b"ROS loose").unwrap();

        let mut handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        assert_eq!(handler.extract_file("khelgar.ros").unwrap(), b"ROS zip");
        handler.update_file("khelgar.ros", b"ROS new").unwrap();

        assert_eq!(handler.extract_file("khelgar.ros").unwrap(), b"ROS new");
        assert_eq!(
            std::fs::read(tmp.path().join("khelgar.ros")).unwrap(),
            b"ROS loose"
        );
    }

    #[test]
    fn test_zip_rewrite_copies_untouched_entries_raw_and_matches_names_case_insensitively() {
        let tmp = tempfile::tempdir().unwrap();
        let zip_path = tmp.path().join("resgff.zip");
        write_stored_zip(
            &zip_path,
            &[("playerlist.ifo", b"IFO"), ("Player.bic", b"BIC old")],
        );

        let mut handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        handler.update_file("player.bic", b"BIC new").unwrap();

        let mut archive = ZipArchive::new(std::fs::File::open(&zip_path).unwrap()).unwrap();
        assert_eq!(archive.len(), 2);
        assert_eq!(
            archive.by_name("playerlist.ifo").unwrap().compression(),
            CompressionMethod::Stored
        );
        let mut bic = Vec::new();
        archive
            .by_name("Player.bic")
            .unwrap()
            .read_to_end(&mut bic)
            .unwrap();
        assert_eq!(bic, b"BIC new");
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[test]
    fn test_loose_read_error_is_not_reported_as_missing() {
        let tmp = tempfile::tempdir().unwrap();
        write_stored_zip(
            &tmp.path().join("resgff.zip"),
            &[("playerlist.ifo", b"IFO")],
        );
        std::fs::create_dir(tmp.path().join("globals.xml")).unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false).unwrap();
        assert!(matches!(
            handler.extract_file("globals.xml"),
            Err(SaveGameError::Io(_))
        ));
        assert!(matches!(
            handler.extract_file("savename.txt"),
            Err(SaveGameError::FileNotInSave { .. })
        ));
    }
}
