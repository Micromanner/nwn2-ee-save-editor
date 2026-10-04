use indexmap::IndexMap;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tracing::info;

use crate::parsers::erf::ErfParser;
use crate::parsers::gff::{GffParser, GffValue, GffWriter};

use crate::config::NWN2Paths;
use crate::services::campaign::backup::backup_module_z;
use crate::services::resource_manager::module_loader::find_installed_module;
use crate::services::savegame_handler::{SaveGameHandler, commit_outputs};

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
    /// Display name of the area the PC stands in (falls back to the module
    /// entry area); starts as the raw resref and is resolved through
    /// `area_name_strref` once a TLK is available.
    pub area_name: String,
    /// `Name` strref of that area's `.are`, -1 if absent.
    pub area_name_strref: i32,
    pub campaign: String,
    /// Module entry area; raw resref until resolved through `entry_area_strref`.
    pub entry_area: String,
    /// `Name` strref of the entry area's `.are`, -1 if absent.
    pub entry_area_strref: i32,
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
    let current_module_id = handler
        .extract_current_module()
        .map_err(|e| e.to_string())?;
    extract_module_info_by_id(handler, paths, &current_module_id)
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
                let name = match parse_module_z_file(&path, &file_stem, paths, None) {
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
                    is_current: file_stem.eq_ignore_ascii_case(&current_module_id),
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
    let is_current = handler
        .extract_current_module()
        .is_ok_and(|current| current.eq_ignore_ascii_case(module_id));
    let player_area_id = if is_current {
        player_area_id(handler)
    } else {
        None
    };
    parse_module_z_file(&z_path, module_id, paths, player_area_id)
}

/// `Mod_PlayerList[0].AreaId` from `playerlist.ifo`: the object id of the
/// area the PC is standing in, matched against `module.ifo` `Mod_Area_list`.
fn player_area_id(handler: &SaveGameHandler) -> Option<u32> {
    let bytes = handler.extract_player_data().ok()?;
    let gff = GffParser::from_bytes(bytes).ok()?;
    let root = gff.read_struct_fields(0).ok()?;
    let Some(GffValue::List(players)) = root.get("Mod_PlayerList") else {
        return None;
    };
    let player = players.first()?.force_load().ok()?;
    match player.get("AreaId") {
        Some(GffValue::Dword(id)) => Some(*id),
        Some(GffValue::Int(id)) => Some(*id as u32),
        _ => None,
    }
}

/// Resref of the area whose `Mod_Area_list` `ObjectId` matches `area_id`.
fn area_resref_by_object_id(root: &IndexMap<String, GffValue<'_>>, area_id: u32) -> Option<String> {
    let GffValue::List(areas) = root.get("Mod_Area_list")? else {
        return None;
    };
    areas.iter().find_map(|area| {
        let fields = area.force_load().ok()?;
        let matches = match fields.get("ObjectId") {
            Some(GffValue::Dword(id)) => *id == area_id,
            Some(GffValue::Int(id)) => *id as u32 == area_id,
            _ => false,
        };
        if !matches {
            return None;
        }
        match fields.get("Area_Name") {
            Some(GffValue::ResRef(s) | GffValue::String(s)) => Some(s.to_string()),
            _ => None,
        }
    })
}

/// Decompressed `<module>.z` archive and its parsed `module.ifo`.
fn read_module_ifo(path: &Path) -> Result<(ErfParser, Arc<GffParser>), String> {
    let decompressed = decompress_z_file(path)?;

    let mut erf_parser = ErfParser::new();
    erf_parser
        .parse_from_bytes(&decompressed)
        .map_err(|e| format!("Failed to parse ERF from .z file: {e}"))?;

    let module_ifo_bytes = erf_parser
        .extract_resource("module.ifo")
        .map_err(|e| format!("module.ifo not found in .z file: {e}"))?;

    let gff = GffParser::from_bytes(module_ifo_bytes)
        .map_err(|e| format!("Failed to parse module.ifo GFF: {e}"))?;
    Ok((erf_parser, gff))
}

/// The HAKs, custom TLK and campaign a `module.ifo` root names.
fn module_content(
    root: &IndexMap<String, GffValue<'_>>,
    module_id: &str,
) -> Result<crate::services::resource_manager::ModuleContent, String> {
    let campaign_id = match root.get("Campaign_ID") {
        Some(GffValue::Void(bytes)) => hex::encode(bytes),
        Some(GffValue::String(s) | GffValue::ResRef(s)) => s.to_string(),
        _ => String::new(),
    };
    let custom_tlk = match root.get("Mod_CustomTlk") {
        Some(GffValue::String(s) | GffValue::ResRef(s)) => s.to_string(),
        _ => String::new(),
    };

    let mut hak_list = Vec::new();
    if let Some(GffValue::List(haks)) = root.get("Mod_HakList") {
        for hak_lazy in haks {
            let fields = hak_lazy
                .force_load()
                .map_err(|e| format!("Failed to read Mod_HakList entry: {e}"))?;
            if let Some(GffValue::String(s) | GffValue::ResRef(s)) = fields.get("Mod_Hak")
                && !s.is_empty()
            {
                hak_list.push(s.to_string());
            }
        }
    }

    Ok(crate::services::resource_manager::ModuleContent {
        name: module_id.to_string(),
        custom_tlk,
        hak_list,
        campaign_id: (!campaign_id.is_empty()).then_some(campaign_id),
    })
}

/// The module the save plays in (`currentmodule.txt`), with the HAKs, custom
/// TLK and campaign named by the `module.ifo` inside its `.z`.
pub fn save_module_content(
    save_dir: &Path,
    module_id: &str,
) -> Result<crate::services::resource_manager::ModuleContent, String> {
    let z_path = find_module_z_file(save_dir, module_id)?;
    let (_, gff) = read_module_ifo(&z_path)?;
    let root = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read root struct: {e}"))?;
    module_content(&root, module_id)
}

fn parse_module_z_file(
    path: &Path,
    module_id: &str,
    nwn2_paths: &NWN2Paths,
    player_area_id: Option<u32>,
) -> Result<(ModuleInfo, ModuleVariables), String> {
    let (mut erf_parser, gff) = read_module_ifo(path)?;
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
    let current_area = player_area_id
        .and_then(|id| area_resref_by_object_id(&root, id))
        .unwrap_or_else(|| entry_area.clone());
    let area_name_strref =
        read_area_name_strref(&mut erf_parser, module_id, &current_area, nwn2_paths);
    let entry_area_strref =
        read_area_name_strref(&mut erf_parser, module_id, &entry_area, nwn2_paths);
    let content = module_content(&root, module_id)?;
    let campaign_id = content.campaign_id.clone().unwrap_or_default();

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
        area_name: current_area,
        area_name_strref,
        campaign,
        entry_area,
        entry_area_strref,
        module_description,
        campaign_id,
        current_module: module_id.to_string(),
        hak_list: content.hak_list,
        custom_tlk: content.custom_tlk,
        game_year: get_u32("Mod_StartYear"),
        game_month: get_byte("Mod_StartMonth"),
        game_day: get_byte("Mod_StartDay"),
        game_hour: get_byte("Mod_StartHour"),
    };

    // Process Variables (VarTable)
    let mut vars = ModuleVariables::default();
    if let Some(GffValue::List(var_entries)) = root.get("VarTable") {
        for var_lazy in var_entries {
            let fields = var_lazy
                .force_load()
                .map_err(|e| format!("Failed to read VarTable entry: {e}"))?;
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

pub fn batch_update_module_variables(
    handler: &SaveGameHandler,
    updates: &[(String, String, String)],
    module_id: Option<&str>,
) -> Result<(), String> {
    if updates.is_empty() {
        return Ok(());
    }
    let (path, bytes) = prepare_module_variable_updates(handler, updates, module_id, |_| Ok(()))?;
    let module = module_id
        .map(str::to_string)
        .unwrap_or(handler.extract_current_module().unwrap_or_default());
    backup_module_z(handler, &module)?;
    commit_outputs(vec![(path, bytes)]).map_err(|e| format!("Failed to write .z file: {e}"))
}

/// The saved state of a module (`<module>.z`) as a fully loaded ERF, ready to
/// edit and re-serialize, with the path it came from. Reads through the handler,
/// so a `.z` already staged in the open batch is edited on top of, not replaced.
pub fn open_module_state(
    handler: &SaveGameHandler,
    module_id: &str,
) -> Result<(PathBuf, ErfParser), String> {
    let z_path = find_module_z_file(handler.save_dir(), module_id)?;
    let file_name = z_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid module filename")?;
    let bytes = handler
        .extract_file(file_name)
        .map_err(|e| format!("Failed to read {file_name}: {e}"))?;
    let decompressed = decompress_z_bytes(&bytes)?;
    let mut erf_parser = ErfParser::new();
    erf_parser
        .parse_from_bytes(&decompressed)
        .map_err(|e| format!("Failed to parse ERF: {e}"))?;
    erf_parser
        .load_all_resources()
        .map_err(|e| format!("Failed to load ERF resources: {e}"))?;
    Ok((z_path, erf_parser))
}

/// Module variables plus any other edit of the module's saved state (area `.git`s),
/// in one pass so they cannot overwrite each other.
pub fn prepare_module_variable_updates(
    handler: &SaveGameHandler,
    updates: &[(String, String, String)],
    module_id: Option<&str>,
    edit_state: impl FnOnce(&mut ErfParser) -> Result<(), String>,
) -> Result<(PathBuf, Vec<u8>), String> {
    let current_module_id = handler.extract_current_module().unwrap_or_default();
    let target_module = module_id.unwrap_or(&current_module_id);
    let (z_path, mut erf_parser) = open_module_state(handler, target_module)?;

    info!(
        "Batch updating {} module variables in {:?}",
        updates.len(),
        z_path
    );

    edit_state(&mut erf_parser)?;
    if !updates.is_empty() {
        update_module_ifo_vars(&mut erf_parser, updates)?;
    }

    let compressed = serialize_module_state(&erf_parser, &z_path)?;
    Ok((z_path, compressed))
}

/// An edited module state as `.z` bytes, in the envelope `z_path` already uses.
pub fn serialize_module_state(erf_parser: &ErfParser, z_path: &Path) -> Result<Vec<u8>, String> {
    let erf_bytes = erf_parser
        .to_bytes()
        .map_err(|e| format!("Failed to serialize ERF: {e}"))?;

    let compressed = compress_z_matching_existing(&erf_bytes, z_path)?;

    info!(
        "Compressed {} -> {} bytes, writing to {:?}",
        erf_bytes.len(),
        compressed.len(),
        z_path
    );
    Ok(compressed)
}

/// Module ids of every module state (`<module>.z`) in a save folder.
pub fn module_state_ids(save_dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(save_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && has_z_extension(path))
        .filter_map(|path| Some(path.file_stem()?.to_string_lossy().into_owned()))
        .collect()
}

fn update_module_ifo_vars(
    erf_parser: &mut ErfParser,
    updates: &[(String, String, String)],
) -> Result<(), String> {
    let module_ifo_bytes = erf_parser
        .extract_resource("module.ifo")
        .map_err(|e| format!("module.ifo not found: {e}"))?;

    info!("Extracted module.ifo ({} bytes)", module_ifo_bytes.len());

    let gff = GffParser::from_bytes(module_ifo_bytes)
        .map_err(|e| format!("Failed to parse module.ifo GFF: {e}"))?;
    let mut owned_fields = gff
        .read_owned_struct(0)
        .map_err(|e| format!("Failed to read module.ifo: {e}"))?;

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
    Ok(())
}

/// Read the `Name` strref of `<area>.are`. Original-NWN2 saves carry the
/// `.are` inside the module state ERF; EE saves do not, so fall back to the
/// installed module (`Modules/<id>.mod` or a campaign folder). -1 when the
/// area file or field is missing.
fn read_area_name_strref(
    save_erf: &mut ErfParser,
    module_id: &str,
    area: &str,
    nwn2_paths: &NWN2Paths,
) -> i32 {
    if area.is_empty() {
        return -1;
    }
    let file_name = format!("{area}.are");
    if let Ok(bytes) = save_erf.extract_resource(&file_name) {
        return area_name_strref_from_gff(bytes);
    }

    let Some(module_path) = find_installed_module(nwn2_paths, module_id) else {
        return -1;
    };

    let bytes = if module_path.is_dir() {
        fs::read(module_path.join(&file_name)).ok()
    } else {
        let mut erf = ErfParser::new();
        erf.read(&module_path)
            .ok()
            .and_then(|()| erf.extract_resource(&file_name).ok())
    };
    bytes.map_or(-1, area_name_strref_from_gff)
}

fn area_name_strref_from_gff(bytes: Vec<u8>) -> i32 {
    let Ok(gff) = GffParser::from_bytes(bytes) else {
        return -1;
    };
    let Ok(root) = gff.read_struct_fields(0) else {
        return -1;
    };
    match root.get("Name") {
        Some(GffValue::LocString(ls)) => ls.string_ref,
        _ => -1,
    }
}

/// Drop the toolset's `{1008}` area-code prefix that OC area names and
/// resrefs carry (`{1008}Lord Dalren's House` -> `Lord Dalren's House`).
pub fn strip_area_code(name: &str) -> &str {
    let trimmed = name.trim_start();
    match trimmed
        .strip_prefix('{')
        .and_then(|rest| rest.split_once('}'))
    {
        Some((code, rest)) if code.chars().all(|c| c.is_ascii_digit()) => rest.trim_start(),
        _ => trimmed,
    }
}

/// Human-readable fallback for an area resref: drops the numeric area-code
/// prefix (`1100_west_harbor` -> `West Harbor`).
pub fn area_name_from_resref(resref: &str) -> String {
    let resref = strip_area_code(resref);
    let stem = resref.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_');
    let stem = if stem.is_empty() { resref } else { stem };
    stem.split(['_', ' '])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn decompress_z_file(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to open .z file: {e}"))?;
    decompress_z_bytes(&bytes)
}

fn decompress_z_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    // EE saves use LZMA-alone compression for .z; original NWN2 saves
    // wrap the same ERF payload in a PK zip (.Z) instead. Sniff the first 4
    // bytes and dispatch — the ERF parser sees the same bytes either way.
    if bytes.starts_with(b"PK\x03\x04") {
        decompress_z_zip(bytes)
    } else {
        decompress_z_lzma(bytes)
    }
}

fn decompress_z_lzma(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let decode = || -> std::io::Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        lzma_rust2::LzmaReader::new_mem_limit(bytes, u32::MAX, None)?
            .read_to_end(&mut decompressed)?;
        Ok(decompressed)
    };
    decode().map_err(|e| format!("Failed to decompress .z file (LZMA): {e}"))
}

fn decompress_z_zip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| format!("Failed to open .Z file as zip: {e}"))?;
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

/// LZMA-alone the way the game writes `.z`: props `0x5d`, 4 MiB dictionary, the real
/// size in the header and no end marker. A header with unknown size breaks the save.
/// Preset 1 matches the game's file size (OC module: 343 KB vs 341 KB) at a seventh of
/// preset 6's time, which matters because saves encode under the session write lock.
fn compress_lzma_for_nwn2(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut options = lzma_rust2::LzmaOptions::with_preset(1);
    options.dict_size = 4 * 1024 * 1024;
    let encode = || -> std::io::Result<Vec<u8>> {
        let mut writer =
            lzma_rust2::LzmaWriter::new_use_header(Vec::new(), &options, Some(data.len() as u64))?;
        writer.write_all(data)?;
        writer.finish()
    };
    encode().map_err(|e| format!("LZMA compression failed: {e}"))
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
                // campaign.cam names modules `1000_Neverwinter_A1`; the save writes
                // `1000_neverwinter_a1.z`.
                && path
                    .file_stem()
                    .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(module_id))
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
                // Documents copy wins over the install copy of the same campaign.
                idx.entry(guid.to_lowercase()).or_insert(CampaignEntry {
                    path: cam_file,
                    display_name,
                });
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

/// The folder holding the campaign's `campaign.cam`.
pub fn find_campaign_folder(campaign_id: &str, paths: &NWN2Paths) -> Option<PathBuf> {
    find_campaign_path(campaign_id, paths)?
        .parent()
        .map(Path::to_path_buf)
}

pub fn find_campaign_name(campaign_id: &str, paths: &NWN2Paths) -> Option<String> {
    campaign_index(paths)
        .get(&campaign_id.to_lowercase())
        .and_then(|e| e.display_name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lzma_matches_game_header_and_round_trips() {
        let data: Vec<u8> = (0..200_000u32)
            .flat_map(|i| (i % 1000).to_le_bytes())
            .collect();
        let compressed = compress_lzma_for_nwn2(&data).unwrap();

        assert_eq!(&compressed[..5], &[0x5d, 0x00, 0x00, 0x40, 0x00]);
        assert_eq!(&compressed[5..13], &(data.len() as u64).to_le_bytes());
        assert_eq!(decompress_z_bytes(&compressed).unwrap(), data);
    }
}
