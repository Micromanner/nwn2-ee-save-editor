//! Looks up `.nss` sources for scripts referenced by dialogue lines: inside the
//! module's `.mod` archive (or expanded directory) first, then loose files in the
//! campaign folder, then the game's `data/scripts*.zip` (includes and `nwscript`
//! constants only; the generic `ga_*` family is already classified by the bridge).

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use tracing::debug;

use super::script_decoder::ScriptConstants;
use crate::parsers::erf::{ErfParser, SecurityLimits};

pub struct ScriptSources {
    archive: Option<ErfParser>,
    dirs: Vec<PathBuf>,
    /// `data/scripts.zip`, `scripts_x1.zip`, `scripts_x2.zip` of the install.
    game_zips: Vec<PathBuf>,
    /// Lowercase `<script>.nss` stem -> (zip, entry name), later packs winning;
    /// built on first use.
    game_index: Option<HashMap<String, (PathBuf, String)>>,
    cache: HashMap<String, Option<String>>,
    game_cache: HashMap<String, Option<String>>,
    nwscript: Option<ScriptConstants>,
    constants_cache: HashMap<String, ScriptConstants>,
}

impl ScriptSources {
    pub fn open(module_path: &Path, campaign_dir: Option<&Path>, game_dir: Option<&Path>) -> Self {
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
        let game_zips = game_dir
            .map(|dir| {
                ["scripts.zip", "scripts_x1.zip", "scripts_x2.zip"]
                    .iter()
                    .map(|name| dir.join("data").join(name))
                    .filter(|path| path.is_file())
                    .collect()
            })
            .unwrap_or_default();
        Self {
            archive,
            dirs,
            game_zips,
            game_index: None,
            cache: HashMap::new(),
            game_cache: HashMap::new(),
            nwscript: None,
            constants_cache: HashMap::new(),
        }
    }

    /// Named constants visible to `script`: `nwscript` (`FEAT_*` and friends) plus
    /// the `const int`/`const string` declarations of its `#include` chain, such as
    /// the OC's `kinc_companion` `TAG_KHELGAR = "khelgar"`.
    pub fn constants(&mut self, script: &str) -> ScriptConstants {
        let key = script.to_ascii_lowercase();
        if let Some(constants) = self.constants_cache.get(&key) {
            return constants.clone();
        }
        if self.nwscript.is_none() {
            let mut constants = ScriptConstants::default();
            if let Some(src) = self.game_script("nwscript") {
                collect_constants(&src, &mut constants);
            }
            self.nwscript = Some(constants);
        }
        let mut constants = self.nwscript.clone().unwrap_or_default();
        let mut visited = HashSet::new();
        self.collect_includes(&key, &mut constants, &mut visited);
        self.constants_cache.insert(key, constants.clone());
        constants
    }

    fn collect_includes(
        &mut self,
        script: &str,
        constants: &mut ScriptConstants,
        visited: &mut HashSet<String>,
    ) {
        let key = script.to_ascii_lowercase();
        if !visited.insert(key.clone()) || visited.len() > 64 {
            return;
        }
        let Some(source) = self
            .source(&key)
            .map(str::to_string)
            .or_else(|| self.game_script(&key))
        else {
            return;
        };
        for include in includes_of(&source) {
            self.collect_includes(&include, constants, visited);
        }
        collect_constants(&source, constants);
    }

    /// `<name>.nss` out of the game's script archives (later packs win).
    fn game_script(&mut self, script: &str) -> Option<String> {
        let key = script.to_ascii_lowercase();
        if let Some(cached) = self.game_cache.get(&key) {
            return cached.clone();
        }
        let index = self.game_index.get_or_insert_with(|| {
            let mut index = HashMap::new();
            for path in &self.game_zips {
                let Ok(file) = std::fs::File::open(path) else {
                    continue;
                };
                let Ok(zip) = zip::ZipArchive::new(file) else {
                    continue;
                };
                for name in zip.file_names() {
                    let lower = name.to_ascii_lowercase();
                    if let Some(stem) = lower
                        .rsplit('/')
                        .next()
                        .and_then(|f| f.strip_suffix(".nss"))
                    {
                        index.insert(stem.to_string(), (path.clone(), name.to_string()));
                    }
                }
            }
            index
        });
        let found = index.get(&key).and_then(|(path, name)| {
            let file = std::fs::File::open(path).ok()?;
            let mut zip = zip::ZipArchive::new(file).ok()?;
            let mut entry = zip.by_name(name).ok()?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ok()?;
            Some(decode_text(&bytes))
        });
        self.game_cache.insert(key, found.clone());
        found
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

/// `#include "name"` directives of a source, in order.
fn includes_of(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("#include")?.trim_start();
            let rest = rest.strip_prefix('"')?;
            let end = rest.find('"')?;
            Some(rest[..end].to_ascii_lowercase())
        })
        .collect()
}

/// Top-level `int NAME = 5;` / `const string NAME = "x";` declarations. Earlier
/// definitions win so a script's own declarations shadow its includes' only when
/// collected first; `nwscript` is collected before everything else.
fn collect_constants(source: &str, constants: &mut ScriptConstants) {
    for line in source.lines() {
        let line = line.trim();
        let line = line
            .strip_prefix("const")
            .map(str::trim_start)
            .unwrap_or(line);
        let Some((kind, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Some((name, value)) = rest.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let Some(value) = value.split_once(';').map(|(v, _)| v.trim()) else {
            continue;
        };
        match kind {
            "int" => {
                if let Ok(n) = value.parse::<i64>() {
                    constants.ints.entry(name.to_string()).or_insert(n);
                }
            }
            "string" => {
                if let Some(s) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
                    constants
                        .strings
                        .entry(name.to_string())
                        .or_insert_with(|| s.to_string());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_and_includes_are_parsed_from_source_lines() {
        let src = r#"
#include "kinc_companion"
#include "ginc_misc"
const string TAG_KHELGAR 	= "khelgar";
int FEAT_ARMOR_PROFICIENCY_HEAVY        = 2;
const int RR_KHELGAR = 7; // trailing
string sNotConst = GetTag(OBJECT_SELF);
void main() {}
"#;
        assert_eq!(includes_of(src), vec!["kinc_companion", "ginc_misc"]);
        let mut constants = ScriptConstants::default();
        collect_constants(src, &mut constants);
        assert_eq!(constants.strings["TAG_KHELGAR"], "khelgar");
        assert_eq!(constants.ints["FEAT_ARMOR_PROFICIENCY_HEAVY"], 2);
        assert_eq!(constants.ints["RR_KHELGAR"], 7);
        assert!(!constants.strings.contains_key("sNotConst"));
    }
}
