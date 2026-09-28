use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::config::NWN2Paths;
use crate::parsers::erf::ErfParser;
use crate::services::campaign::content::{find_campaign_folder, save_module_content};
use crate::services::resource_manager::ModuleContent;
use crate::services::resource_manager::module_loader::{
    find_installed_module, find_named_file, hak_dirs,
};
use crate::services::savegame_handler::CURRENTMODULE_TXT;
use crate::utils::directory_scanner::{ScannedFile, scan_directory, scan_workshop};

/// The mod content of a save as oracle layers, lowest priority first, following the editor's
/// order: module < campaign < override folders (Documents, Workshop, custom) < HAKs, the first
/// listed HAK winning. Base game data is loaded by the oracle itself.
///
/// The toolset resource manager searches every ERF before any directory, so the module's 2DAs
/// are extracted into `scratch` and sent as a directory; HAKs stay ERFs because they outrank
/// everything anyway. Folders are sent as the flat directories that hold 2DAs, the same file
/// set the editor's recursive scan sees. Without a save (standalone `.bic`) only the override
/// folders apply.
pub fn layers(
    paths: &NWN2Paths,
    save_dir: Option<&Path>,
    scratch: &Path,
) -> Result<Vec<Value>, String> {
    let content = save_dir.map(module_content).transpose()?;

    let mut dirs = Vec::new();
    if let Some(module) = content
        .as_ref()
        .and_then(|c| find_installed_module(paths, &c.name))
    {
        if module.is_dir() {
            dirs.extend(twoda_dirs(&module));
        } else {
            let extracted = scratch.join("module");
            let mut erf = ErfParser::new();
            erf.read(&module)
                .and_then(|()| erf.extract_all_2da(&extracted))
                .map_err(|e| format!("failed to read module {}: {e}", module.display()))?;
            if extracted.is_dir() {
                dirs.push(extracted);
            }
        }
    }
    if let Some(campaign) = content
        .as_ref()
        .and_then(|c| c.campaign_id.as_deref().filter(|g| !g.is_empty()))
        .and_then(|guid| find_campaign_folder(guid, paths))
    {
        dirs.extend(twoda_dirs(&campaign));
    }
    if let Some(documents_override) = paths.override_dir() {
        dirs.extend(twoda_dirs(&documents_override));
    }
    if let Some(workshop) = paths.steam_workshop_folder() {
        dirs.extend(twoda_parents(scan_workshop(workshop)));
    }
    for custom in paths.custom_override_folders() {
        dirs.extend(twoda_dirs(custom));
    }

    let mut layers: Vec<Value> = dirs
        .iter()
        .map(|d| json!({ "kind": "dir", "path": d }))
        .collect();
    if let Some(content) = &content {
        let hak_dirs = hak_dirs(paths);
        let haks: Vec<PathBuf> = content
            .hak_list
            .iter()
            .filter_map(|name| find_named_file(name, "hak", &hak_dirs))
            .collect();
        layers.extend(
            haks.iter()
                .rev()
                .map(|h| json!({ "kind": "erf", "path": h })),
        );
    }
    Ok(layers)
}

fn module_content(save_dir: &Path) -> Result<ModuleContent, String> {
    let module_id = std::fs::read_to_string(save_dir.join(CURRENTMODULE_TXT))
        .map_err(|e| format!("save has no {CURRENTMODULE_TXT}: {e}"))?;
    save_module_content(save_dir, module_id.trim())
}

fn twoda_dirs(root: &Path) -> Vec<PathBuf> {
    twoda_parents(scan_directory(root, true))
}

/// The distinct folders holding the 2DA files among `files`.
fn twoda_parents(files: Vec<ScannedFile>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for file in files {
        if file.extension.eq_ignore_ascii_case("2da")
            && let Some(parent) = file.path.parent()
            && !dirs.iter().any(|d| d == parent)
        {
            dirs.push(parent.to_path_buf());
        }
    }
    dirs
}
