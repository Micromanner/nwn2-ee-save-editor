use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use flate2::read::DeflateDecoder;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use zip::read::ZipFile;
use zip::{CompressionMethod, ZipArchive};

const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const LOCAL_HEADER_SIZE: u64 = 30;

/// Where a stored or deflated entry's bytes live, so it can be read through a
/// private file handle without opening the archive or holding the shared
/// reader: concurrent texture and model reads then inflate in parallel, and
/// the startup scan already has everything needed to build it.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ZipEntryLocation {
    header_start: u64,
    compressed_size: u64,
    size: u64,
    deflated: bool,
}

impl ZipEntryLocation {
    /// `None` for compression methods `read_entry` cannot handle.
    pub fn from_raw_entry(file: &ZipFile<'_>) -> Option<Self> {
        let deflated = match file.compression() {
            CompressionMethod::Stored => false,
            CompressionMethod::Deflated => true,
            _ => return None,
        };
        Some(Self {
            header_start: file.header_start(),
            compressed_size: file.compressed_size(),
            size: file.size(),
            deflated,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct IndexedEntry {
    index: usize,
    direct: Option<ZipEntryLocation>,
}

#[derive(Clone)]
pub struct ZipReadRequest {
    pub zip_path: String,
    pub internal_path: String,
    pub request_id: String,
}

pub struct ZipReadResult {
    pub request_id: String,
    pub success: bool,
    pub data: Option<Vec<u8>>,
    pub error: Option<String>,
}

pub struct ZipContentReader {
    open_archives: HashMap<String, ZipArchive<BufReader<File>>>,
    file_indices: HashMap<String, HashMap<String, IndexedEntry>>,
    basename_indices: HashMap<String, HashMap<String, String>>,
    files_read: u64,
    bytes_read: u64,
    archives_opened: u64,
    cache_hits: u64,
}

impl ZipContentReader {
    pub fn new() -> Self {
        ZipContentReader {
            open_archives: HashMap::new(),
            file_indices: HashMap::new(),
            basename_indices: HashMap::new(),
            files_read: 0,
            bytes_read: 0,
            archives_opened: 0,
            cache_hits: 0,
        }
    }

    pub fn read_file_from_zip(
        &mut self,
        zip_path: String,
        internal_path: String,
    ) -> Result<Vec<u8>, String> {
        let entry = self.locate(&zip_path, &internal_path)?;
        if let Some(direct) = entry.direct {
            let contents = Self::read_entry(Path::new(&zip_path), &direct)?;
            self.files_read += 1;
            self.bytes_read += contents.len() as u64;
            return Ok(contents);
        }

        let archive = self
            .open_archives
            .get_mut(&zip_path)
            .ok_or_else(|| format!("Failed to access ZIP archive: {zip_path}"))?;

        let mut file = archive
            .by_index(entry.index)
            .map_err(|e| format!("Failed to read file at index {}: {e}", entry.index))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .map_err(|e| format!("Failed to read file {internal_path}: {e}"))?;

        self.files_read += 1;
        self.bytes_read += contents.len() as u64;
        Ok(contents)
    }

    fn locate(&mut self, zip_path: &str, internal_path: &str) -> Result<IndexedEntry, String> {
        if self.open_archives.contains_key(zip_path) {
            self.cache_hits += 1;
        } else {
            self.open_archive(zip_path)?;
        }

        self.file_indices
            .get(zip_path)
            .and_then(|indices| indices.get(internal_path).copied())
            .ok_or_else(|| format!("File not found in ZIP index: {internal_path}"))
    }

    /// Reads one stored or deflated entry through a fresh file handle. Needs
    /// no `&self`, so callers drop the reader lock before calling it.
    pub fn read_entry(zip_path: &Path, entry: &ZipEntryLocation) -> Result<Vec<u8>, String> {
        let zip = zip_path.display();
        let mut file =
            File::open(zip_path).map_err(|e| format!("Failed to open ZIP {zip}: {e}"))?;
        file.seek(SeekFrom::Start(entry.header_start))
            .map_err(|e| format!("Failed to seek ZIP {zip}: {e}"))?;
        let mut header = [0u8; LOCAL_HEADER_SIZE as usize];
        file.read_exact(&mut header)
            .map_err(|e| format!("Failed to read local header in {zip}: {e}"))?;
        if u32::from_le_bytes(header[0..4].try_into().unwrap()) != LOCAL_HEADER_SIGNATURE {
            return Err(format!("Bad local header signature in {zip}"));
        }
        let name_len = u64::from(u16::from_le_bytes(header[26..28].try_into().unwrap()));
        let extra_len = u64::from(u16::from_le_bytes(header[28..30].try_into().unwrap()));
        file.seek(SeekFrom::Start(
            entry.header_start + LOCAL_HEADER_SIZE + name_len + extra_len,
        ))
        .map_err(|e| format!("Failed to seek ZIP {zip}: {e}"))?;

        // DeflateDecoder buffers internally; stored entries copy straight
        // into the output.
        let mut raw = file.take(entry.compressed_size);
        let mut contents = Vec::with_capacity(entry.size as usize);
        if entry.deflated {
            DeflateDecoder::new(raw).read_to_end(&mut contents)
        } else {
            raw.read_to_end(&mut contents)
        }
        .map_err(|e| format!("Failed to read entry in {zip}: {e}"))?;
        Ok(contents)
    }

    pub fn read_multiple_files(&mut self, requests: Vec<ZipReadRequest>) -> Vec<ZipReadResult> {
        let mut results = Vec::new();

        let mut grouped: HashMap<String, Vec<ZipReadRequest>> = HashMap::new();
        for request in requests {
            grouped
                .entry(request.zip_path.clone())
                .or_default()
                .push(request);
        }

        for (zip_path, file_requests) in grouped {
            if !self.open_archives.contains_key(&zip_path)
                && let Err(e) = self.open_archive(&zip_path)
            {
                for req in file_requests {
                    results.push(ZipReadResult {
                        request_id: req.request_id,
                        success: false,
                        data: None,
                        error: Some(format!("Failed to open ZIP: {e}")),
                    });
                }
                continue;
            }

            if let Some(archive) = self.open_archives.get_mut(&zip_path) {
                for req in file_requests {
                    match archive.by_name(&req.internal_path) {
                        Ok(mut file) => {
                            let mut contents = Vec::new();
                            match file.read_to_end(&mut contents) {
                                Ok(_) => {
                                    self.files_read += 1;
                                    self.bytes_read += contents.len() as u64;
                                    results.push(ZipReadResult {
                                        request_id: req.request_id,
                                        success: true,
                                        data: Some(contents),
                                        error: None,
                                    });
                                }
                                Err(e) => {
                                    results.push(ZipReadResult {
                                        request_id: req.request_id,
                                        success: false,
                                        data: None,
                                        error: Some(e.to_string()),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            results.push(ZipReadResult {
                                request_id: req.request_id,
                                success: false,
                                data: None,
                                error: Some(format!("File not found: {e}")),
                            });
                        }
                    }
                }
            }
        }

        results
    }

    pub fn read_multiple_files_parallel(
        &self,
        requests: Vec<ZipReadRequest>,
    ) -> Vec<ZipReadResult> {
        requests
            .par_iter()
            .map(
                |req| match Self::read_single_file(&req.zip_path, &req.internal_path) {
                    Ok(data) => ZipReadResult {
                        request_id: req.request_id.clone(),
                        success: true,
                        data: Some(data),
                        error: None,
                    },
                    Err(e) => ZipReadResult {
                        request_id: req.request_id.clone(),
                        success: false,
                        data: None,
                        error: Some(e),
                    },
                },
            )
            .collect()
    }

    fn read_single_file(zip_path: &str, internal_path: &str) -> Result<Vec<u8>, String> {
        let file = File::open(zip_path).map_err(|e| format!("Failed to open ZIP: {e}"))?;

        let reader = BufReader::with_capacity(64 * 1024, file);
        let mut archive =
            ZipArchive::new(reader).map_err(|e| format!("Failed to read ZIP: {e}"))?;

        let mut entry = archive
            .by_name(internal_path)
            .map_err(|e| format!("File not found: {e}"))?;

        let mut contents = Vec::with_capacity(entry.size() as usize);
        entry
            .read_to_end(&mut contents)
            .map_err(|e| format!("Failed to read: {e}"))?;

        Ok(contents)
    }

    pub fn preopen_zip_archives(&mut self, zip_paths: Vec<String>) -> Result<(), String> {
        for zip_path in zip_paths {
            if !self.open_archives.contains_key(&zip_path) {
                self.open_archive(&zip_path)?;
            }
        }
        Ok(())
    }

    pub fn close_archive(&mut self, zip_path: String) {
        self.open_archives.remove(&zip_path);
    }

    pub fn close_all_archives(&mut self) {
        self.open_archives.clear();
    }

    pub fn get_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        stats.insert("files_read".to_string(), serde_json::json!(self.files_read));
        stats.insert("bytes_read".to_string(), serde_json::json!(self.bytes_read));
        stats.insert(
            "archives_opened".to_string(),
            serde_json::json!(self.archives_opened),
        );
        stats.insert("cache_hits".to_string(), serde_json::json!(self.cache_hits));
        stats.insert(
            "open_archives".to_string(),
            serde_json::json!(self.open_archives.len()),
        );

        let bytes_read_mb = self.bytes_read as f64 / (1024.0 * 1024.0);
        stats.insert(
            "bytes_read_mb".to_string(),
            serde_json::json!(bytes_read_mb),
        );

        stats
    }

    pub fn file_exists_in_zip(
        &mut self,
        zip_path: String,
        internal_path: String,
    ) -> Result<bool, String> {
        if !self.open_archives.contains_key(&zip_path) {
            self.open_archive(&zip_path)?;
        }

        if let Some(archive) = self.open_archives.get_mut(&zip_path) {
            Ok(archive.by_name(&internal_path).is_ok())
        } else {
            Ok(false)
        }
    }

    pub fn list_files_by_extension(
        &mut self,
        zip_path: &str,
        extension: &str,
    ) -> Result<Vec<String>, String> {
        if !self.open_archives.contains_key(zip_path) {
            self.open_archive(zip_path)?;
        }

        let ext_lower = format!(".{}", extension.to_lowercase());
        Ok(self
            .file_indices
            .get(zip_path)
            .map(|indices| {
                indices
                    .keys()
                    .filter(|name| name.to_lowercase().ends_with(&ext_lower))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    pub fn list_files_by_prefix(
        &mut self,
        zip_path: &str,
        prefix: &str,
        extension: &str,
    ) -> Result<Vec<String>, String> {
        if !self.open_archives.contains_key(zip_path) {
            self.open_archive(zip_path)?;
        }

        let prefix_lower = prefix.to_lowercase();
        let ext_lower = format!(".{}", extension.to_lowercase());
        Ok(self
            .basename_indices
            .get(zip_path)
            .map(|basenames| {
                basenames
                    .keys()
                    .filter(|name| name.starts_with(&prefix_lower) && name.ends_with(&ext_lower))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    fn open_archive(&mut self, zip_path: &str) -> Result<(), String> {
        let path = Path::new(zip_path);
        if !path.exists() {
            return Err(format!("ZIP file not found: {zip_path}"));
        }

        let file =
            File::open(path).map_err(|e| format!("Failed to open ZIP file {zip_path}: {e}"))?;

        const BUFFER_SIZE: usize = 64 * 1024;
        let reader = BufReader::with_capacity(BUFFER_SIZE, file);

        let mut archive = ZipArchive::new(reader)
            .map_err(|e| format!("Failed to read ZIP archive {zip_path}: {e}"))?;

        let mut indices = HashMap::new();
        let mut basenames = HashMap::new();
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index_raw(i) {
                let name = file.name().to_string();
                let basename = name.rsplit('/').next().unwrap_or(&name).to_lowercase();
                basenames.insert(basename, name.clone());
                indices.insert(
                    name,
                    IndexedEntry {
                        index: i,
                        direct: ZipEntryLocation::from_raw_entry(&file),
                    },
                );
            }
        }

        self.file_indices.insert(zip_path.to_string(), indices);
        self.basename_indices
            .insert(zip_path.to_string(), basenames);
        self.open_archives.insert(zip_path.to_string(), archive);
        self.archives_opened += 1;

        Ok(())
    }
}

impl Default for ZipContentReader {
    fn default() -> Self {
        Self::new()
    }
}
