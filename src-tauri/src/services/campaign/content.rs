use indexmap::IndexMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs::{self};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tracing::{error, info, warn};
use xz2::read::XzDecoder;
use xz2::stream::Stream;

use crate::parsers::erf::ErfParser;
use crate::parsers::gff::{GffParser, GffValue, GffWriter};

use crate::config::NWN2Paths;
use crate::services::campaign::backup::backup_module_z;
use crate::services::savegame_handler::SaveGameHandler;

/// True if the path's extension is `z` regardless of case.
/// EE saves use lowercase `.z` (LZMA); original-NWN2 saves use uppercase `.Z` (PK zip).
fn has_z_extension(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("z"))
}

/// Write a `VarTable` entry's Type/Value, preserving the existing `Value`
/// variant when updating an int (real `module.ifo` files mix `Int` and `Dword`
/// for integer values; the engine cares which one).
fn apply_var_entry_update(
    entry: &mut IndexMap<String, GffValue<'static>>,
    var_name: &str,
    value: &str,
    var_type: &str,
) -> Result<(), String> {
    let (type_id, new_value): (u32, GffValue<'static>) = match var_type {
        "int" => {
            let v: i32 = value
                .parse()
                .map_err(|e| format!("Invalid int value for '{var_name}': {e}"))?;
            let value = match entry.get("Value") {
                Some(GffValue::Dword(_)) => GffValue::Dword(v as u32),
                _ => GffValue::Int(v),
            };
            (1, value)
        }
        "float" => {
            let v: f32 = value
                .parse()
                .map_err(|e| format!("Invalid float value for '{var_name}': {e}"))?;
            (2, GffValue::Float(v))
        }
        "string" => (3, GffValue::String(Cow::Owned(value.to_string()))),
        _ => return Err(format!("Unknown variable type: {var_type}")),
    };
    entry.insert("Type".to_string(), GffValue::Dword(type_id));
    entry.insert("Value".to_string(), new_value);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleInfo {
    pub module_name: String,
    pub area_name: String,
    pub campaign: String,
    pub entry_area: String,
    pub module_description: String,
    pub campaign_id: String,
    pub current_module: String,
    pub hak_list: Vec<String>,
    pub custom_tlk: String,
    pub game_year: u32,
    pub game_month: u8,
    pub game_day: u8,
    pub game_hour: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModuleVariables {
    pub integers: HashMap<String, i32>,
    pub floats: HashMap<String, f32>,
    pub strings: HashMap<String, String>,
}

pub fn extract_module_info(
    handler: &SaveGameHandler,
    paths: &NWN2Paths,
) -> Result<(ModuleInfo, ModuleVariables), String> {
    let current_module_id = handler.extract_current_module().unwrap_or_default();
    let save_dir = handler.save_dir();

    info!("ContentManager: Current module is '{}'", current_module_id);

    // Find all .z files
    let mut z_files = Vec::new();
    if let Ok(entries) = fs::read_dir(save_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && has_z_extension(&path) {
                z_files.push(path);
            }
        }
    }

    if z_files.is_empty() {
        warn!("ContentManager: No .z files found in save directory");
        return Ok((ModuleInfo::default(), ModuleVariables::default()));
    }

    let mut found_module_info = None;
    let mut found_module_vars = None;

    // Try to find the current module first
    for z_file in &z_files {
        let file_stem = z_file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if file_stem == current_module_id {
            info!(
                "ContentManager: Parsing current module .z file: {}",
                file_stem
            );
            if let Ok(res) = parse_module_z_file(z_file, &current_module_id, paths) {
                found_module_info = Some(res.0);
                found_module_vars = Some(res.1);
                break;
            }
        }
    }

    // If not found, fallback to first .z file
    if found_module_info.is_none() && !z_files.is_empty() {
        let z_file = &z_files[0];
        let file_stem = z_file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        info!("ContentManager: Fallback to parsing: {}", file_stem);
        if let Ok(res) = parse_module_z_file(z_file, &file_stem, paths) {
            found_module_info = Some(res.0);
            found_module_vars = Some(res.1);
        }
    }

    let info = found_module_info.unwrap_or_default();

    Ok((info, found_module_vars.unwrap_or_default()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSummary {
    pub id: String,
    pub name: String,
    pub is_current: bool,
}

pub fn list_modules(
    handler: &SaveGameHandler,
    paths: &NWN2Paths,
) -> Result<Vec<ModuleSummary>, String> {
    let current_module_id = handler.extract_current_module().unwrap_or_default();
    let save_dir = handler.save_dir();

    let mut modules = Vec::new();
    if let Ok(entries) = fs::read_dir(save_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && has_z_extension(&path) {
                let file_stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                let name = match parse_module_z_file(&path, &file_stem, paths) {
                    Ok((info, _)) => {
                        if info.module_name.is_empty() {
                            file_stem.clone()
                        } else {
                            info.module_name
                        }
                    }
                    Err(_) => file_stem.clone(),
                };
                modules.push(ModuleSummary {
                    is_current: file_stem == current_module_id,
                    id: file_stem,
                    name,
                });
            }
        }
    }

    modules.sort_by(|a, b| {
        b.is_current
            .cmp(&a.is_current)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(modules)
}

pub fn extract_module_info_by_id(
    handler: &SaveGameHandler,
    paths: &NWN2Paths,
    module_id: &str,
) -> Result<(ModuleInfo, ModuleVariables), String> {
    let save_dir = handler.save_dir();
    let z_path = find_module_z_file(save_dir, module_id)?;
    parse_module_z_file(&z_path, module_id, paths)
}

fn parse_module_z_file(
    path: &Path,
    module_id: &str,
    nwn2_paths: &NWN2Paths,
) -> Result<(ModuleInfo, ModuleVariables), String> {
    let decompressed = decompress_z_file(path)?;

    // Parse ERF
    let mut erf_parser = ErfParser::new();
    erf_parser
        .parse_from_bytes(&decompressed)
        .map_err(|e| format!("Failed to parse ERF from .z file: {e}"))?;

    // Extract module.ifo
    let module_ifo_bytes = erf_parser
        .extract_resource("module.ifo")
        .map_err(|e| format!("module.ifo not found in .z file: {e}"))?;

    // Parse GFF
    let gff = GffParser::from_bytes(module_ifo_bytes)
        .map_err(|e| format!("Failed to parse module.ifo GFF: {e}"))?;
    let root = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root struct: {e}"))?;

    // helper functions
    let get_string = |key: &str| -> String {
        match root.get(key) {
            Some(GffValue::String(s)) => s.to_string(),
            Some(GffValue::ResRef(s)) => s.to_string(),
            Some(GffValue::LocString(ls)) => ls
                .substrings
                .first()
                .map(|sub| sub.string.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        }
    };
    let get_u32 = |key: &str| -> u32 {
        match root.get(key) {
            Some(GffValue::Dword(v)) => *v,
            Some(GffValue::Int(v)) => *v as u32,
            _ => 0,
        }
    };

    // Process Module Info
    let module_name = get_string("Mod_Name");
    let module_description = get_string("Mod_Description");
    let entry_area = get_string("Mod_Entry_Area");
    let campaign_id = match root.get("Campaign_ID") {
        Some(GffValue::Void(bytes)) => hex::encode(bytes),
        Some(GffValue::String(s) | GffValue::ResRef(s)) => s.to_string(),
        _ => String::new(),
    };

    let campaign = if campaign_id.is_empty() {
        String::new()
    } else {
        find_campaign_name(&campaign_id, nwn2_paths).unwrap_or_else(|| {
            format!(
                "Campaign ({})",
                &campaign_id[..std::cmp::min(8, campaign_id.len())]
            )
        })
    };

    let custom_tlk = get_string("Mod_CustomTlk");

    let mut hak_list = Vec::new();
    if let Some(GffValue::List(haks)) = root.get("Mod_HakList") {
        for hak_lazy in haks {
            let fields = hak_lazy.force_load();
            if let Some(GffValue::String(s)) = fields.get("Mod_Hak") {
                hak_list.push(s.to_string());
            }
        }
    }

    let get_byte = |key: &str| -> u8 {
        match root.get(key) {
            Some(GffValue::Byte(v)) => *v,
            Some(GffValue::Dword(v)) => *v as u8,
            Some(GffValue::Int(v)) => *v as u8,
            _ => 0,
        }
    };

    let info = ModuleInfo {
        module_name: if module_name.is_empty() {
            module_id.to_string()
        } else {
            module_name
        },
        area_name: entry_area.clone(),
        campaign,
        entry_area,
        module_description,
        campaign_id,
        current_module: module_id.to_string(),
        hak_list,
        custom_tlk,
        game_year: get_u32("Mod_StartYear"),
        game_month: get_byte("Mod_StartMonth"),
        game_day: get_byte("Mod_StartDay"),
        game_hour: get_byte("Mod_StartHour"),
    };

    // Process Variables (VarTable)
    let mut vars = ModuleVariables::default();
    if let Some(GffValue::List(var_entries)) = root.get("VarTable") {
        for var_lazy in var_entries {
            let fields = var_lazy.force_load();
            let name = match fields.get("Name") {
                Some(GffValue::String(s)) => s.to_string(),
                _ => continue,
            };
            let type_id = match fields.get("Type") {
                Some(GffValue::Dword(v)) => *v,
                Some(GffValue::Int(v)) => *v as u32,
                _ => continue,
            };

            if let Some(val_field) = fields.get("Value") {
                match (type_id, val_field) {
                    (1, GffValue::Int(v)) => {
                        vars.integers.insert(name, *v);
                    }
                    (1, GffValue::Dword(v)) => {
                        vars.integers.insert(name, *v as i32);
                    }
                    (2, GffValue::Float(v)) => {
                        vars.floats.insert(name, *v);
                    }
                    (3, GffValue::String(s)) => {
                        vars.strings.insert(name, s.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((info, vars))
}

pub fn update_module_variable(
    handler: &SaveGameHandler,
    var_name: &str,
    value: &str,
    var_type: &str,
    module_id: Option<&str>,
) -> Result<(), String> {
    let result = update_module_variable_inner(handler, var_name, value, var_type, module_id);
    if let Err(ref e) = result {
        error!("Failed to update module variable '{}': {}", var_name, e);
    }
    result
}

fn update_module_variable_inner(
    handler: &SaveGameHandler,
    var_name: &str,
    value: &str,
    var_type: &str,
    module_id: Option<&str>,
) -> Result<(), String> {
    let current_module_id = handler.extract_current_module().unwrap_or_default();
    let target_module = module_id.unwrap_or(&current_module_id);
    let save_dir = handler.save_dir();

    let z_path = find_module_z_file(save_dir, target_module)?;

    info!(
        "Updating module variable '{}' (type={}) in {:?}",
        var_name, var_type, z_path
    );

    let decompressed = decompress_z_file(&z_path)?;

    info!("Decompressed {} bytes from .z file", decompressed.len());

    let mut erf_parser = ErfParser::new();
    erf_parser
        .parse_from_bytes(&decompressed)
        .map_err(|e| format!("Failed to parse ERF: {e}"))?;
    erf_parser
        .load_all_resources()
        .map_err(|e| format!("Failed to load ERF resources: {e}"))?;

    let module_ifo_bytes = erf_parser
        .extract_resource("module.ifo")
        .map_err(|e| format!("module.ifo not found: {e}"))?;

    info!("Extracted module.ifo ({} bytes)", module_ifo_bytes.len());

    let gff = GffParser::from_bytes(module_ifo_bytes)
        .map_err(|e| format!("Failed to parse module.ifo GFF: {e}"))?;
    let root = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root struct: {e}"))?;

    let mut owned_fields: IndexMap<String, GffValue<'static>> = root
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let var_table = owned_fields
        .entry("VarTable".to_string())
        .or_insert_with(|| GffValue::ListOwned(Vec::new()));

    let GffValue::ListOwned(entries) = var_table else {
        return Err(format!(
            "VarTable is not a ListOwned, got: {:?}",
            std::mem::discriminant(var_table)
        ));
    };

    let existing = entries.iter_mut().find(|e| {
        matches!(
            e.get("Name"),
            Some(GffValue::String(s)) if s.as_ref() == var_name
        )
    });

    if let Some(entry) = existing {
        apply_var_entry_update(entry, var_name, value, var_type)?;
        info!("Updated existing VarTable entry '{}'", var_name);
    } else {
        let mut new_entry = IndexMap::new();
        new_entry.insert(
            "Name".to_string(),
            GffValue::String(Cow::Owned(var_name.to_string())),
        );
        apply_var_entry_update(&mut new_entry, var_name, value, var_type)?;
        entries.push(new_entry);
        info!("Added new VarTable entry '{}'", var_name);
    }

    let new_ifo_bytes = GffWriter::new("IFO ", "V3.2")
        .write(owned_fields)
        .map_err(|e| format!("Failed to serialize module.ifo: {e:?}"))?;

    info!("Serialized module.ifo ({} bytes)", new_ifo_bytes.len());

    erf_parser
        .update_resource("module.ifo", new_ifo_bytes)
        .map_err(|e| format!("Failed to update module.ifo in ERF: {e}"))?;

    let erf_bytes = erf_parser
        .to_bytes()
        .map_err(|e| format!("Failed to serialize ERF: {e}"))?;

    let compressed = compress_z_matching_existing(&erf_bytes, &z_path)?;

    info!(
        "Compressed {} -> {} bytes, writing to {:?}",
        erf_bytes.len(),
        compressed.len(),
        z_path
    );

    if let Err(e) = backup_module_z(handler, target_module) {
        warn!("Failed to backup {}.z: {}", target_module, e);
    }

    fs::write(&z_path, compressed).map_err(|e| format!("Failed to write .z file: {e}"))?;

    info!(
        "Successfully updated module variable '{}' in {:?}",
        var_name, z_path
    );
    Ok(())
}

pub fn batch_update_module_variables(
    handler: &SaveGameHandler,
    updates: &[(String, String, String)],
    module_id: Option<&str>,
) -> Result<(), String> {
    if updates.is_empty() {
        return Ok(());
    }

    let current_module_id = handler.extract_current_module().unwrap_or_default();
    let target_module = module_id.unwrap_or(&current_module_id);
    let save_dir = handler.save_dir();

    let z_path = find_module_z_file(save_dir, target_module)?;

    info!(
        "Batch updating {} module variables in {:?}",
        updates.len(),
        z_path
    );

    let decompressed = decompress_z_file(&z_path)?;

    info!("Decompressed {} bytes from .z file", decompressed.len());

    let mut erf_parser = ErfParser::new();
    erf_parser
        .parse_from_bytes(&decompressed)
        .map_err(|e| format!("Failed to parse ERF: {e}"))?;
    erf_parser
        .load_all_resources()
        .map_err(|e| format!("Failed to load ERF resources: {e}"))?;

    let module_ifo_bytes = erf_parser
        .extract_resource("module.ifo")
        .map_err(|e| format!("module.ifo not found: {e}"))?;

    info!("Extracted module.ifo ({} bytes)", module_ifo_bytes.len());

    let gff = GffParser::from_bytes(module_ifo_bytes)
        .map_err(|e| format!("Failed to parse module.ifo GFF: {e}"))?;
    let root = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root struct: {e}"))?;

    let mut owned_fields: IndexMap<String, GffValue<'static>> = root
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let var_table = owned_fields
        .entry("VarTable".to_string())
        .or_insert_with(|| GffValue::ListOwned(Vec::new()));

    let GffValue::ListOwned(entries) = var_table else {
        return Err(format!(
            "VarTable is not a ListOwned, got: {:?}",
            std::mem::discriminant(var_table)
        ));
    };

    for (var_name, value, var_type) in updates {
        let existing = entries.iter_mut().find(|e| {
            matches!(
                e.get("Name"),
                Some(GffValue::String(s)) if s.as_ref() == var_name.as_str()
            )
        });

        if let Some(entry) = existing {
            apply_var_entry_update(entry, var_name, value, var_type)?;
            info!("Updated existing VarTable entry '{}'", var_name);
        } else {
            let mut new_entry = IndexMap::new();
            new_entry.insert(
                "Name".to_string(),
                GffValue::String(Cow::Owned(var_name.clone())),
            );
            apply_var_entry_update(&mut new_entry, var_name, value, var_type)?;
            entries.push(new_entry);
            info!("Added new VarTable entry '{}'", var_name);
        }
    }

    let new_ifo_bytes = GffWriter::new("IFO ", "V3.2")
        .write(owned_fields)
        .map_err(|e| format!("Failed to serialize module.ifo: {e:?}"))?;

    info!("Serialized module.ifo ({} bytes)", new_ifo_bytes.len());

    erf_parser
        .update_resource("module.ifo", new_ifo_bytes)
        .map_err(|e| format!("Failed to update module.ifo in ERF: {e}"))?;

    let erf_bytes = erf_parser
        .to_bytes()
        .map_err(|e| format!("Failed to serialize ERF: {e}"))?;

    let compressed = compress_z_matching_existing(&erf_bytes, &z_path)?;

    info!(
        "Compressed {} -> {} bytes, writing to {:?}",
        erf_bytes.len(),
        compressed.len(),
        z_path
    );

    if let Err(e) = backup_module_z(handler, target_module) {
        warn!("Failed to backup {}.z: {}", target_module, e);
    }

    fs::write(&z_path, compressed).map_err(|e| format!("Failed to write .z file: {e}"))?;

    info!(
        "Successfully batch-updated {} module variables in {:?}",
        updates.len(),
        z_path
    );
    Ok(())
}

fn decompress_z_file(path: &Path) -> Result<Vec<u8>, String> {
    // EE saves use LZMA-alone compression for .z; original NWN2 saves
    // wrap the same ERF payload in a PK zip (.Z) instead. Sniff the first 4
    // bytes and dispatch — the ERF parser sees the same bytes either way.
    let mut header = [0u8; 4];
    {
        let mut file = fs::File::open(path).map_err(|e| format!("Failed to open .z file: {e}"))?;
        // Short reads (< 4 bytes) leave the rest zero; either branch below
        // will then fail with a clearer error than a silent truncation.
        let _ = file.read(&mut header);
    }

    if &header == b"PK\x03\x04" {
        decompress_z_file_zip(path)
    } else {
        decompress_z_file_lzma(path)
    }
}

fn decompress_z_file_lzma(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Failed to open .z file: {e}"))?;
    let stream = Stream::new_lzma_decoder(u64::MAX)
        .map_err(|e| format!("Failed to create LZMA decoder: {e}"))?;
    let mut decoder = XzDecoder::new_stream(file, stream);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Failed to decompress .z file (LZMA): {e}"))?;
    Ok(decompressed)
}

fn decompress_z_file_zip(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| format!("Failed to open .Z file: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to open .Z file as zip: {e}"))?;
    if archive.is_empty() {
        return Err("Original-NWN2 .Z zip is empty".to_string());
    }
    // Original-NWN2 .Z files contain exactly one (unnamed) entry: the ERF payload.
    let mut entry = archive
        .by_index(0)
        .map_err(|e| format!("Failed to read first entry of .Z zip: {e}"))?;
    let mut decompressed = Vec::with_capacity(entry.size() as usize);
    entry
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("Failed to inflate .Z entry: {e}"))?;
    Ok(decompressed)
}

fn compress_lzma_for_nwn2(data: &[u8]) -> Result<Vec<u8>, String> {
    // NWN2 .z files use LZMA alone format with known uncompressed size and 4MB dictionary.
    // The lzma-rs crate produces the correct header with uncompressed_size set.
    let mut output = Vec::new();
    lzma_rs::lzma_compress_with_options(
        &mut &data[..],
        &mut output,
        &lzma_rs::compress::Options {
            unpacked_size: lzma_rs::compress::UnpackedSize::WriteToHeader(Some(data.len() as u64)),
        },
    )
    .map_err(|e| format!("LZMA compression failed: {e}"))?;

    Ok(output)
}

/// Wrap an ERF payload in the original-NWN2 PK zip envelope (single unnamed entry).
fn compress_zip_for_original(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Cursor;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    let mut buf = Vec::new();
    {
        let mut writer = ZipWriter::new(Cursor::new(&mut buf));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        // Original NWN2's .Z entries have empty filenames — match the observed layout.
        writer
            .start_file::<&str, ()>("", options)
            .map_err(|e| format!("Failed to start zip entry: {e}"))?;
        use std::io::Write;
        writer
            .write_all(data)
            .map_err(|e| format!("Failed to write zip entry: {e}"))?;
        writer
            .finish()
            .map_err(|e| format!("Failed to finish zip: {e}"))?;
    }
    Ok(buf)
}

/// Compress an ERF payload to match the format of `existing_path`. If the file
/// currently uses the original-NWN2 PK-zip envelope, produce a PK zip; otherwise produce
/// LZMA-alone. Writing the wrong format would break the engine's loader.
fn compress_z_matching_existing(data: &[u8], existing_path: &Path) -> Result<Vec<u8>, String> {
    let mut header = [0u8; 4];
    if let Ok(mut file) = fs::File::open(existing_path) {
        let _ = file.read(&mut header);
    }
    if &header == b"PK\x03\x04" {
        compress_zip_for_original(data)
    } else {
        compress_lzma_for_nwn2(data)
    }
}

fn find_module_z_file(save_dir: &Path, module_id: &str) -> Result<PathBuf, String> {
    if let Ok(entries) = fs::read_dir(save_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && has_z_extension(&path)
                && path
                    .file_stem()
                    .is_some_and(|s| s.to_string_lossy() == module_id)
            {
                return Ok(path);
            }
        }
    }
    Err(format!(
        "Module .z file not found for '{module_id}' in {}",
        save_dir.display()
    ))
}

struct CampaignEntry {
    path: PathBuf,
    display_name: Option<String>,
}

type CampaignIndex = HashMap<String, CampaignEntry>;
type CampaignIndexCache = Mutex<Option<(Vec<PathBuf>, Arc<CampaignIndex>)>>;

fn campaign_index(paths: &NWN2Paths) -> Arc<CampaignIndex> {
    static CACHE: OnceLock<CampaignIndexCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let mut dirs = Vec::new();
    if let Some(p) = paths.user_campaigns() {
        dirs.push(p);
    }
    if let Some(p) = paths.campaigns() {
        dirs.push(p);
    }

    let mut guard = cache.lock();
    if let Some((cached_dirs, cached_idx)) = guard.as_ref()
        && cached_dirs == &dirs
    {
        return Arc::clone(cached_idx);
    }

    let mut idx = CampaignIndex::new();
    for dir in &dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let cam_file = path.join("campaign.cam");
                if !cam_file.exists() {
                    continue;
                }
                let Ok(parser) = GffParser::new(&cam_file) else {
                    continue;
                };
                let guid = match parser.read_field_by_label(0, "GUID") {
                    Ok(GffValue::Void(bytes)) => hex::encode(bytes),
                    Ok(GffValue::String(s) | GffValue::ResRef(s)) => s.to_string(),
                    _ => continue,
                };
                let display_name = match parser.read_field_by_label(0, "DisplayName") {
                    Ok(GffValue::LocString(ls)) => {
                        ls.substrings.first().map(|s| s.string.to_string())
                    }
                    _ => None,
                };
                idx.insert(
                    guid.to_lowercase(),
                    CampaignEntry {
                        path: cam_file,
                        display_name,
                    },
                );
            }
        }
    }

    let arc = Arc::new(idx);
    *guard = Some((dirs, Arc::clone(&arc)));
    arc
}

pub fn find_campaign_path(campaign_id: &str, paths: &NWN2Paths) -> Option<PathBuf> {
    campaign_index(paths)
        .get(&campaign_id.to_lowercase())
        .map(|e| e.path.clone())
}

pub fn find_campaign_name(campaign_id: &str, paths: &NWN2Paths) -> Option<String> {
    campaign_index(paths)
        .get(&campaign_id.to_lowercase())
        .and_then(|e| e.display_name.clone())
}
