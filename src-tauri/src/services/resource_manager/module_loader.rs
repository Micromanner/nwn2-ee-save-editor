use std::path::{Path, PathBuf};

use crate::config::NWN2Paths;
use crate::parsers::tlk::TLKParser;

use super::error::{ResourceManagerError, ResourceManagerResult};

pub fn load_tlk(tlk_path: &Path) -> ResourceManagerResult<TLKParser> {
    let mut parser = TLKParser::new();
    parser.parse_from_file(tlk_path).map_err(|e| {
        ResourceManagerError::InvalidTlkFormat(format!(
            "Failed to parse TLK {}: {}",
            tlk_path.display(),
            e
        ))
    })?;
    Ok(parser)
}

/// First `<name>.<extension>` found in `dirs`, searched in order. `name` may
/// already carry the extension.
pub fn find_named_file(name: &str, extension: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let file_name = with_extension(name, extension);
    dirs.iter()
        .map(|dir| dir.join(&file_name))
        .find(|path| path.is_file())
}

fn with_extension(name: &str, extension: &str) -> String {
    let suffix = format!(".{extension}");
    if name.to_lowercase().ends_with(&suffix) {
        name.to_string()
    } else {
        format!("{name}{suffix}")
    }
}

/// HAK search folders in priority order: custom HAK folders, Documents `hak`, install `hak`.
pub fn hak_dirs(paths: &NWN2Paths) -> Vec<PathBuf> {
    paths
        .custom_hak_folders()
        .iter()
        .cloned()
        .chain(paths.hak_dir())
        .chain(paths.install_hak_dir())
        .collect()
}

/// An installed module's `.mod` file or directory module, searched in the
/// custom module folders, the Documents and install `modules` folders, then
/// the install campaign folders.
pub fn find_installed_module(paths: &NWN2Paths, module_name: &str) -> Option<PathBuf> {
    let module_dirs: Vec<PathBuf> = paths
        .custom_module_folders()
        .iter()
        .cloned()
        .chain(paths.modules_dir())
        .chain(paths.install_modules_dir())
        .collect();
    for folder in &module_dirs {
        if let Some(path) = find_named_file(module_name, "mod", std::slice::from_ref(folder)) {
            return Some(path);
        }
        let dir_path = folder.join(module_name);
        if dir_path.is_dir() {
            return Some(dir_path);
        }
    }

    let campaign_dirs: Vec<PathBuf> = std::fs::read_dir(paths.campaigns()?)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    find_named_file(module_name, "mod", &campaign_dirs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_named_file_adds_missing_extension() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("westgate.hak"), b"").unwrap();
        let dirs = vec![PathBuf::from("missing"), dir.path().to_path_buf()];

        assert_eq!(
            find_named_file("westgate", "hak", &dirs),
            Some(dir.path().join("westgate.hak"))
        );
        assert_eq!(
            find_named_file("westgate.hak", "hak", &dirs),
            Some(dir.path().join("westgate.hak"))
        );
        assert_eq!(find_named_file("other", "hak", &dirs), None);
    }
}
