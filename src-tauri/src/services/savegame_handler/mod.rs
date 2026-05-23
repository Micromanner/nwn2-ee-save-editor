pub mod backup;
pub mod error;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::parsers::gff::GffParser;

pub use backup::{BackupInfo, CleanupResult, RestoreResult};
pub use error::{SaveGameError, SaveGameResult};

static NWN2_DATE_TIME: LazyLock<zip::DateTime> =
    LazyLock::new(|| zip::DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap_or_default());

const RESGFF_ZIP: &str = "resgff.zip";
const PLAYERLIST_IFO: &str = "playerlist.ifo";
const PLAYER_BIC: &str = "player.bic";
const GLOBALS_XML: &str = "globals.xml";
const CURRENTMODULE_TXT: &str = "currentmodule.txt";
const MODULE_IFO: &str = "module.ifo";
const PLAYERINFO_BIN: &str = "playerinfo.bin";

const FILE_HEADERS: &[(&str, &[u8; 4])] = &[
    (".bic", b"BIC "),
    (".ros", b"ROS "),
    (".ifo", b"IFO "),
    (".uti", b"UTI "),
    (".utc", b"UTC "),
    (".ute", b"UTE "),
];

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSummary {
    pub first_name: String,
    pub last_name: String,
    pub race: String,
    pub subrace: String,
    pub deity: String,
    pub gender: u8,
    pub classes: Vec<(String, u8)>,
    pub alignment: (u8, u8),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterStats {
    pub str: u8,
    pub dex: u8,
    pub con: u8,
    pub int: u8,
    pub wis: u8,
    pub cha: u8,
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

fn read_archive_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    filename: &str,
) -> SaveGameResult<Vec<u8>> {
    let mut entry = archive
        .by_name(filename)
        .map_err(|_| SaveGameError::FileNotInSave {
            filename: filename.into(),
        })?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

pub struct SaveGameHandler {
    save_dir: PathBuf,
    zip_path: PathBuf,
    format: SaveFormat,
    validate: bool,
    temp_files: Vec<PathBuf>,
}

impl SaveGameHandler {
    pub fn new(
        save_path: impl AsRef<Path>,
        validate: bool,
        create_load_backup: bool,
    ) -> SaveGameResult<Self> {
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

        if create_load_backup && !backup::has_backup_been_created(&save_dir) {
            backup::create_backup(&save_dir)?;
        }

        Ok(Self {
            save_dir,
            zip_path,
            format,
            validate,
            temp_files: Vec::new(),
        })
    }

    pub fn format(&self) -> SaveFormat {
        self.format
    }

    pub fn extract_file(&self, filename: &str) -> SaveGameResult<Vec<u8>> {
        // Root-level loose files exist in both formats and always live on disk.
        let is_disk_root_file =
            filename == GLOBALS_XML || filename == MODULE_IFO || filename == CURRENTMODULE_TXT;

        if is_disk_root_file {
            let disk_path = self.save_dir.join(filename);
            if let Ok(content) = fs::read(&disk_path) {
                return Ok(content);
            }
        }

        let contents = match self.format {
            SaveFormat::Ee => {
                let file = File::open(&self.zip_path)?;
                let mut archive = ZipArchive::new(file)?;
                let mut zip_file =
                    archive
                        .by_name(filename)
                        .map_err(|_| SaveGameError::FileNotInSave {
                            filename: filename.into(),
                        })?;
                let mut buf = Vec::with_capacity(zip_file.size() as usize);
                zip_file.read_to_end(&mut buf)?;
                buf
            }
            SaveFormat::Original => {
                let disk_path = self.save_dir.join(filename);
                fs::read(&disk_path).map_err(|_| SaveGameError::FileNotInSave {
                    filename: filename.into(),
                })?
            }
        };

        if self.validate {
            self.validate_file_content(filename, &contents)?;
        }

        Ok(contents)
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

    pub fn batch_read_character_files(&self) -> SaveGameResult<HashMap<String, Vec<u8>>> {
        match self.format {
            SaveFormat::Ee => self.batch_read_character_files_ee(),
            SaveFormat::Original => self.batch_read_character_files_original(),
        }
    }

    fn batch_read_character_files_ee(&self) -> SaveGameResult<HashMap<String, Vec<u8>>> {
        let file = File::open(&self.zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        let mut character_files = Vec::new();
        for i in 0..archive.len() {
            let entry = archive.by_index_raw(i)?;
            let name = entry.name().to_string();

            let is_character_file = name == PLAYERLIST_IFO
                || name == PLAYER_BIC
                || name.to_lowercase().ends_with(".ros");

            if is_character_file {
                character_files.push(name);
            }
        }

        let zip_path = self.zip_path.clone();
        let validate = self.validate;

        let results: Vec<(String, Vec<u8>)> = character_files
            .par_iter()
            .filter_map(|name| {
                let file = File::open(&zip_path).ok()?;
                let reader = BufReader::with_capacity(64 * 1024, file);
                let mut archive = ZipArchive::new(reader).ok()?;
                let mut zip_file = archive.by_name(name).ok()?;

                let mut contents = Vec::with_capacity(zip_file.size() as usize);
                zip_file.read_to_end(&mut contents).ok()?;

                if validate {
                    Self::validate_file_header(name, &contents).ok()?;
                }

                Some((name.clone(), contents))
            })
            .collect();

        Ok(results.into_iter().collect())
    }

    fn batch_read_character_files_original(&self) -> SaveGameResult<HashMap<String, Vec<u8>>> {
        let mut out: HashMap<String, Vec<u8>> = HashMap::new();
        for entry in fs::read_dir(&self.save_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            let is_character_file =
                lower == PLAYERLIST_IFO || lower == PLAYER_BIC || lower.ends_with(".ros");
            if !is_character_file {
                continue;
            }
            let contents = fs::read(&path)?;
            if self.validate {
                Self::validate_file_header(&name, &contents)?;
            }
            out.insert(name, contents);
        }
        Ok(out)
    }

    fn validate_file_header(filename: &str, content: &[u8]) -> SaveGameResult<()> {
        if content.len() < 4 {
            return Err(SaveGameError::ValidationFailed {
                filename: filename.into(),
                reason: "File too small".into(),
            });
        }

        for (ext, expected_header) in FILE_HEADERS {
            if filename.to_lowercase().ends_with(ext) {
                let actual = &content[0..4];
                if actual != *expected_header {
                    return Err(SaveGameError::InvalidHeader {
                        filename: filename.into(),
                        expected: String::from_utf8_lossy(*expected_header).into(),
                    });
                }
                break;
            }
        }

        Ok(())
    }

    pub fn extract_globals_xml(&self) -> SaveGameResult<String> {
        let path = self.save_dir.join(GLOBALS_XML);
        if !path.exists() {
            return Err(SaveGameError::FileNotInSave {
                filename: GLOBALS_XML.into(),
            });
        }
        Ok(fs::read_to_string(path)?)
    }

    pub fn extract_current_module(&self) -> SaveGameResult<String> {
        let path = self.save_dir.join(CURRENTMODULE_TXT);
        if !path.exists() {
            return Err(SaveGameError::FileNotInSave {
                filename: CURRENTMODULE_TXT.into(),
            });
        }
        Ok(fs::read_to_string(path)?.trim().to_string())
    }

    pub fn extract_module_ifo(&self) -> SaveGameResult<Vec<u8>> {
        let path = self.save_dir.join(MODULE_IFO);
        if !path.exists() {
            return Err(SaveGameError::FileNotInSave {
                filename: MODULE_IFO.into(),
            });
        }
        Ok(fs::read(path)?)
    }

    pub fn update_file(&mut self, filename: &str, content: &[u8]) -> SaveGameResult<()> {
        if self.validate {
            self.validate_file_content(filename, content)?;
        }

        let disk_path = self.save_dir.join(filename);

        // Always-loose files (both formats) and Original format write straight to disk.
        let force_disk = filename == GLOBALS_XML
            || filename == MODULE_IFO
            || filename == CURRENTMODULE_TXT
            || disk_path.exists();

        if self.format == SaveFormat::Original || force_disk {
            fs::write(&disk_path, content)?;
            debug!("Updated file on disk: {}", filename);
            return Ok(());
        }

        let temp_path = self.zip_path.with_extension("zip.tmp");

        {
            let src_file = File::open(&self.zip_path)?;
            let mut src_archive = ZipArchive::new(src_file)?;

            let dst_file = File::create(&temp_path)?;
            let mut dst_archive = ZipWriter::new(dst_file);

            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .last_modified_time(*NWN2_DATE_TIME);

            let mut file_written = false;

            for i in 0..src_archive.len() {
                let mut src_entry = src_archive.by_index(i)?;
                let name = src_entry.name().to_string();

                if name == filename {
                    dst_archive.start_file(&name, options)?;
                    dst_archive.write_all(content)?;
                    file_written = true;
                } else {
                    dst_archive.start_file(&name, options)?;
                    let mut buffer = Vec::with_capacity(src_entry.size() as usize);
                    src_entry.read_to_end(&mut buffer)?;
                    dst_archive.write_all(&buffer)?;
                }
            }

            if !file_written {
                dst_archive.start_file(filename, options)?;
                dst_archive.write_all(content)?;
            }

            dst_archive.finish()?;
        }

        fs::rename(&temp_path, &self.zip_path)?;

        debug!("Updated file in save: {}", filename);
        Ok(())
    }

    /// Rewrite playerlist.ifo and player.bic inside the save zip in a single pass.
    ///
    /// The transform closure receives the source bytes for both files and returns
    /// the new bytes. Returning `player_bic: None` preserves the source bytes
    /// unchanged - used for multiplayer non-primary-slot writes where the primary
    /// player's BIC must stay intact.
    pub fn rewrite_player_files<F>(&mut self, transform: F) -> SaveGameResult<()>
    where
        F: FnOnce(PlayerSources<'_>) -> Result<PlayerOutputs, String>,
    {
        match self.format {
            SaveFormat::Ee => self.rewrite_player_files_ee(transform),
            SaveFormat::Original => self.rewrite_player_files_original(transform),
        }
    }

    fn rewrite_player_files_ee<F>(&mut self, transform: F) -> SaveGameResult<()>
    where
        F: FnOnce(PlayerSources<'_>) -> Result<PlayerOutputs, String>,
    {
        let src_file = File::open(&self.zip_path)?;
        let mut src_archive = ZipArchive::new(src_file)?;

        let playerlist_src = read_archive_entry(&mut src_archive, PLAYERLIST_IFO)?;
        let player_bic_src = match read_archive_entry(&mut src_archive, PLAYER_BIC) {
            Ok(b) => Some(b),
            Err(SaveGameError::FileNotInSave { .. }) => None,
            Err(e) => return Err(e),
        };

        let outputs = transform(PlayerSources {
            playerlist: &playerlist_src,
            player_bic: player_bic_src.as_deref(),
        })
        .map_err(SaveGameError::Transform)?;

        let temp_path = self.zip_path.with_extension("zip.tmp");
        {
            let dst_file = File::create(&temp_path)?;
            let mut dst = ZipWriter::new(dst_file);

            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .last_modified_time(*NWN2_DATE_TIME);

            let mut bic_seen = false;
            for i in 0..src_archive.len() {
                let mut entry = src_archive.by_index(i)?;
                let name = entry.name().to_string();
                dst.start_file(&name, options)?;
                match name.as_str() {
                    PLAYERLIST_IFO => dst.write_all(&outputs.playerlist)?,
                    PLAYER_BIC => {
                        bic_seen = true;
                        if let Some(bytes) = &outputs.player_bic {
                            dst.write_all(bytes)?;
                        } else {
                            let mut buf = Vec::with_capacity(entry.size() as usize);
                            entry.read_to_end(&mut buf)?;
                            dst.write_all(&buf)?;
                        }
                    }
                    _ => {
                        let mut buf = Vec::with_capacity(entry.size() as usize);
                        entry.read_to_end(&mut buf)?;
                        dst.write_all(&buf)?;
                    }
                }
            }
            if !bic_seen && let Some(bytes) = &outputs.player_bic {
                dst.start_file(PLAYER_BIC, options)?;
                dst.write_all(bytes)?;
            }
            dst.finish()?;
        }

        drop(src_archive);
        fs::rename(&temp_path, &self.zip_path)?;

        info!("Updated player files in save (EE)");
        Ok(())
    }

    fn rewrite_player_files_original<F>(&mut self, transform: F) -> SaveGameResult<()>
    where
        F: FnOnce(PlayerSources<'_>) -> Result<PlayerOutputs, String>,
    {
        let playerlist_path = self.save_dir.join(PLAYERLIST_IFO);
        let player_bic_path = self.save_dir.join(PLAYER_BIC);

        let playerlist_src = fs::read(&playerlist_path)?;
        let player_bic_src = if player_bic_path.exists() {
            Some(fs::read(&player_bic_path)?)
        } else {
            None
        };

        let outputs = transform(PlayerSources {
            playerlist: &playerlist_src,
            player_bic: player_bic_src.as_deref(),
        })
        .map_err(SaveGameError::Transform)?;

        // Write to .tmp first then rename, so a crash mid-write doesn't leave
        // a half-written playerlist.ifo behind.
        let pl_tmp = playerlist_path.with_extension("ifo.tmp");
        fs::write(&pl_tmp, &outputs.playerlist)?;
        fs::rename(&pl_tmp, &playerlist_path)?;

        if let Some(bytes) = outputs.player_bic {
            let bic_tmp = player_bic_path.with_extension("bic.tmp");
            fs::write(&bic_tmp, &bytes)?;
            fs::rename(&bic_tmp, &player_bic_path)?;
        }
        // Transform returned None: caller asked to preserve the original; it's
        // already on disk untouched.

        info!("Updated player files in save (Original)");
        Ok(())
    }

    pub fn sync_playerinfo_bin(
        &self,
        fields: &indexmap::IndexMap<String, crate::parsers::gff::GffValue<'_>>,
        subrace_name: &str,
        alignment_name: &str,
        classes: &[(String, u8)],
    ) -> SaveGameResult<()> {
        let playerinfo_path = self.save_dir.join(PLAYERINFO_BIN);

        if !playerinfo_path.exists() {
            return Err(SaveGameError::PlayerInfoSync(
                "playerinfo.bin not found - save is invalid".into(),
            ));
        }

        let mut player_info = crate::services::playerinfo::PlayerInfo::load(&playerinfo_path)
            .map_err(|e| {
                SaveGameError::PlayerInfoSync(format!("playerinfo.bin is corrupted: {e}"))
            })?;

        player_info.update_from_gff_data(fields, subrace_name, alignment_name, classes);

        player_info.save(&playerinfo_path).map_err(|e| {
            SaveGameError::PlayerInfoSync(format!("Failed to write playerinfo.bin: {e}"))
        })?;

        info!("Successfully synced playerinfo.bin");
        Ok(())
    }

    pub fn update_module_ifo(&self, data: &[u8]) -> SaveGameResult<()> {
        let path = self.save_dir.join(MODULE_IFO);
        fs::write(path, data)?;
        Ok(())
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

    pub fn list_companions(&self) -> SaveGameResult<Vec<String>> {
        let mut companions = Vec::new();

        let names: Vec<String> = match self.format {
            SaveFormat::Ee => {
                let file = File::open(&self.zip_path)?;
                let mut archive = ZipArchive::new(file)?;
                (0..archive.len())
                    .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
                    .collect()
            }
            SaveFormat::Original => fs::read_dir(&self.save_dir)?
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect(),
        };

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

    pub fn extract_for_editing(&mut self, temp_dir: &Path) -> SaveGameResult<PathBuf> {
        if self.format != SaveFormat::Ee {
            return Err(SaveGameError::InvalidStructure(
                "extract_for_editing is only supported for EE-format saves".into(),
            ));
        }
        fs::create_dir_all(temp_dir)?;

        let file = File::open(&self.zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let out_path = temp_dir.join(entry.name());

            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut out_file = File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;

            self.temp_files.push(out_path);
        }

        for entry in fs::read_dir(&self.save_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path != self.zip_path && path.is_file() {
                let dest = temp_dir.join(entry.file_name());
                fs::copy(&path, &dest)?;
                self.temp_files.push(dest);
            }
        }

        Ok(temp_dir.to_path_buf())
    }

    pub fn repack_from_directory(&mut self, source_dir: &Path) -> SaveGameResult<()> {
        if self.format != SaveFormat::Ee {
            return Err(SaveGameError::InvalidStructure(
                "repack_from_directory is only supported for EE-format saves".into(),
            ));
        }
        let temp_path = self.zip_path.with_extension("zip.tmp");

        {
            let dst_file = File::create(&temp_path)?;
            let mut dst_archive = ZipWriter::new(dst_file);

            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .last_modified_time(*NWN2_DATE_TIME);

            for entry in fs::read_dir(source_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();

                    if name_str.to_lowercase().ends_with(".xml")
                        || name_str.to_lowercase().ends_with(".txt")
                        || name_str.to_lowercase() == "module.ifo"
                        || name_str.to_lowercase() == "playerinfo.bin"
                    {
                        continue;
                    }

                    dst_archive.start_file(name_str.as_ref(), options)?;
                    let mut file = File::open(&path)?;
                    std::io::copy(&mut file, &mut dst_archive)?;
                }
            }

            dst_archive.finish()?;
        }

        fs::rename(&temp_path, &self.zip_path)?;

        info!("Repacked save from directory: {}", source_dir.display());
        Ok(())
    }

    pub fn list_backups(&self) -> SaveGameResult<Vec<BackupInfo>> {
        backup::list_backups(&self.save_dir)
    }

    pub fn restore_from_backup(
        &mut self,
        backup_path: &Path,
        create_pre_restore_backup: bool,
    ) -> SaveGameResult<RestoreResult> {
        backup::restore_from_backup(backup_path, &self.save_dir, create_pre_restore_backup)
    }

    pub fn cleanup_old_backups(&self, keep_count: usize) -> SaveGameResult<CleanupResult> {
        backup::cleanup_old_backups(&self.save_dir, keep_count)
    }

    pub fn read_character_summary(&self) -> SaveGameResult<Option<CharacterSummary>> {
        let data = match self.extract_player_bic()? {
            Some(d) => d,
            None => return Ok(None),
        };

        let gff = GffParser::from_bytes(data)
            .map_err(|e| SaveGameError::GffParse(format!("Failed to parse player.bic: {e}")))?;

        let fields = gff.read_struct_fields(0).map_err(|e| {
            SaveGameError::GffParse(format!("Failed to read player.bic fields: {e}"))
        })?;

        let first_name = extract_locstring(&fields, "FirstName").unwrap_or_default();
        let last_name = extract_locstring(&fields, "LastName").unwrap_or_default();
        let subrace = extract_string(&fields, "Subrace").unwrap_or_default();
        let deity = extract_string(&fields, "Deity").unwrap_or_default();
        let gender = extract_byte(&fields, "Gender").unwrap_or(0);

        let race_id = extract_byte(&fields, "Race").unwrap_or(0);
        let race = format!("Race_{race_id}");

        let lawful = extract_byte(&fields, "LawfulChaotic").unwrap_or(50);
        let good = extract_byte(&fields, "GoodEvil").unwrap_or(50);

        let classes = Vec::new();

        Ok(Some(CharacterSummary {
            first_name,
            last_name,
            race,
            subrace,
            deity,
            gender,
            classes,
            alignment: (lawful, good),
        }))
    }

    pub fn get_file_info(&self, filename: &str) -> SaveGameResult<Option<FileInfo>> {
        match self.format {
            SaveFormat::Ee => {
                let file = File::open(&self.zip_path)?;
                let mut archive = ZipArchive::new(file)?;
                match archive.by_name(filename) {
                    Ok(entry) => Ok(Some(FileInfo {
                        name: entry.name().to_string(),
                        size: entry.size(),
                        compressed_size: entry.compressed_size(),
                        compression: format!("{:?}", entry.compression()),
                    })),
                    Err(_) => Ok(None),
                }
            }
            SaveFormat::Original => {
                let path = self.save_dir.join(filename);
                if !path.is_file() {
                    return Ok(None);
                }
                let meta = fs::metadata(&path)?;
                Ok(Some(FileInfo {
                    name: filename.to_string(),
                    size: meta.len(),
                    compressed_size: meta.len(),
                    compression: "Stored".to_string(),
                }))
            }
        }
    }

    pub fn infer_save_path_from_backup(&self, backup_path: &Path) -> Option<PathBuf> {
        backup::infer_save_path_from_backup(backup_path)
    }

    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    pub fn zip_path(&self) -> &Path {
        &self.zip_path
    }

    fn validate_file_content(&self, filename: &str, content: &[u8]) -> SaveGameResult<()> {
        Self::validate_file_header(filename, content)
    }

    fn cleanup_temp_files(&mut self) {
        for path in self.temp_files.drain(..) {
            if path.exists()
                && let Err(e) = fs::remove_file(&path)
            {
                warn!("Failed to cleanup temp file {}: {}", path.display(), e);
            }
        }
    }
}

impl Drop for SaveGameHandler {
    fn drop(&mut self) {
        self.cleanup_temp_files();
    }
}

fn extract_string(
    fields: &indexmap::IndexMap<String, crate::parsers::gff::GffValue<'_>>,
    key: &str,
) -> Option<String> {
    use crate::parsers::gff::GffValue;

    match fields.get(key)? {
        GffValue::String(s) => Some(s.to_string()),
        GffValue::ResRef(s) => Some(s.to_string()),
        _ => None,
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

fn extract_byte(
    fields: &indexmap::IndexMap<String, crate::parsers::gff::GffValue<'_>>,
    key: &str,
) -> Option<u8> {
    use crate::parsers::gff::GffValue;

    match fields.get(key)? {
        GffValue::Byte(v) => Some(*v),
        GffValue::Word(v) => Some(*v as u8),
        GffValue::Dword(v) => Some(*v as u8),
        GffValue::Int(v) => Some(*v as u8),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_header_validation() {
        let handler = SaveGameHandler {
            save_dir: PathBuf::from("/test"),
            zip_path: PathBuf::from("/test/resgff.zip"),
            format: SaveFormat::Ee,
            validate: true,
            temp_files: Vec::new(),
        };

        let valid_bic = b"BIC test data here";
        assert!(handler.validate_file_content("test.bic", valid_bic).is_ok());

        let invalid_bic = b"XXXX test data here";
        assert!(
            handler
                .validate_file_content("test.bic", invalid_bic)
                .is_err()
        );
    }

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

        let handler = SaveGameHandler::new(tmp.path(), false, false)
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

        let handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();

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

        let handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();

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

        let handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();
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

        let handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();
        let files = handler.list_files().unwrap();
        let names: std::collections::HashSet<String> = files.into_iter().map(|f| f.name).collect();
        assert!(names.contains("player.bic"));
        assert!(names.contains("playerlist.ifo"));
        assert!(names.contains("globals.xml"));
        assert!(names.contains("khelgar.ros"));
    }

    #[test]
    fn test_get_file_info_returns_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC payload").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();
        let info = handler
            .get_file_info("player.bic")
            .unwrap()
            .expect("present");
        assert_eq!(info.name, "player.bic");
        assert_eq!(info.size, b"BIC payload".len() as u64);
    }

    #[test]
    fn test_update_file_writes_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC original").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO original").unwrap();

        let mut handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();

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
    fn test_rewrite_player_files_writes_loose_for_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC old").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO old").unwrap();

        let mut handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();
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

        let mut handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();
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
    fn test_batch_read_character_files_original() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("player.bic"), b"BIC ").unwrap();
        std::fs::write(tmp.path().join("playerlist.ifo"), b"IFO ").unwrap();
        std::fs::write(tmp.path().join("neeshka.ros"), b"NEESHKA").unwrap();

        let handler = SaveGameHandler::new(tmp.path(), false, false).unwrap();
        let map = handler.batch_read_character_files().unwrap();
        assert!(map.contains_key("player.bic"));
        assert!(map.contains_key("playerlist.ifo"));
        assert!(map.contains_key("neeshka.ros"));
        assert_eq!(map.get("neeshka.ros").unwrap(), b"NEESHKA");
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

        let handler =
            SaveGameHandler::new(tmp.path(), false, false).expect("EE save must be accepted");
        assert_eq!(handler.format(), SaveFormat::Ee);
    }
}
