//! Looks up `.nss` sources for scripts referenced by dialogue lines: inside the
//! module's `.mod` archive (or expanded directory) first, then loose files in the
//! campaign folder. Base-game scripts are not searched; the generic `ga_*` family
//! is already classified by the bridge.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::debug;

use crate::parsers::erf::{ErfParser, SecurityLimits};

pub struct ScriptSources {
    archive: Option<ErfParser>,
    dirs: Vec<PathBuf>,
    cache: HashMap<String, Option<String>>,
}

impl ScriptSources {
    pub fn open(module_path: &Path, campaign_dir: Option<&Path>) -> Self {
        let mut dirs = Vec::new();
        let mut archive = None;
        if module_path.is_dir() {
            dirs.push(module_path.to_path_buf());
        } else if module_path.is_file() {
            let mut parser = ErfParser::new().with_limits(SecurityLimits {
                max_file_size: 4 * 1024 * 1024 * 1024,
                ..SecurityLimits::default()
            });
            match parser.read(module_path) {
                Ok(()) => archive = Some(parser),
                Err(e) => debug!(
                    "script sources unavailable for {}: {e}",
                    module_path.display()
                ),
            }
        }
        if let Some(dir) = campaign_dir {
            dirs.push(dir.to_path_buf());
        }
        Self {
            archive,
            dirs,
            cache: HashMap::new(),
        }
    }

    /// Source text of `<script>.nss`, or `None` when no source ships.
    pub fn source(&mut self, script: &str) -> Option<&str> {
        let key = script.to_ascii_lowercase();
        if !self.cache.contains_key(&key) {
            let found = self.load(&key);
            self.cache.insert(key.clone(), found);
        }
        self.cache.get(&key).and_then(|s| s.as_deref())
    }

    fn load(&mut self, script: &str) -> Option<String> {
        let file = format!("{script}.nss");
        if let Some(archive) = self.archive.as_mut()
            && let Ok(bytes) = archive.extract_resource(&file)
        {
            return Some(decode_text(&bytes));
        }
        for dir in &self.dirs {
            if let Ok(bytes) = std::fs::read(dir.join(&file)) {
                return Some(decode_text(&bytes));
            }
        }
        None
    }
}

/// NWScript sources are Windows-1252; string literals only matter for variable
/// names, which are ASCII in practice.
fn decode_text(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}
