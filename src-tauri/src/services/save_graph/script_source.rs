//! Looks up `.nss` sources for scripts referenced by dialogue lines: inside the
//! module's `.mod` archive (or expanded directory) first, then loose files in the
//! campaign folder, then the game's `data/scripts*.zip` (includes and `nwscript`
//! constants only; the generic `ga_*` family is already classified by the bridge).

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tracing::debug;

use super::script_decoder::ScriptConstants;
use crate::parsers::erf::{ErfParser, SecurityLimits};

/// The install's `data/scripts.zip`, `scripts_x1.zip` and `scripts_x2.zip`, shared by
/// every module of one quest-graph build so the archives are indexed and `nwscript`
/// is parsed once instead of once per module.
pub struct GameScripts {
    zips: Vec<zip::ZipArchive<File>>,
    /// Lowercase `<script>.nss` stem -> (zip, entry name), later packs winning.
    index: HashMap<String, (usize, String)>,
    cache: HashMap<String, Option<String>>,
    nwscript: Option<ScriptConstants>,
}

impl GameScripts {
    pub fn open(game_dir: Option<&Path>) -> Self {
        let zips: Vec<_> = game_dir
            .map(|dir| {
                ["scripts.zip", "scripts_x1.zip", "scripts_x2.zip"]
                    .iter()
                    .filter_map(|name| File::open(dir.join("data").join(name)).ok())
                    .filter_map(|file| zip::ZipArchive::new(file).ok())
                    .collect()
            })
            .unwrap_or_default();
        let mut index = HashMap::new();
        for (i, zip) in zips.iter().enumerate() {
            for name in zip.file_names() {
                let lower = name.to_ascii_lowercase();
                if let Some(stem) = lower
                    .rsplit('/')
                    .next()
                    .and_then(|f| f.strip_suffix(".nss"))
                {
                    index.insert(stem.to_string(), (i, name.to_string()));
                }
            }
        }
        Self {
            zips,
            index,
            cache: HashMap::new(),
            nwscript: None,
        }
    }

    /// `<script>.nss` out of the archives; `script` must be lowercase.
    fn script(&mut self, script: &str) -> Option<String> {
        if let Some(cached) = self.cache.get(script) {
            return cached.clone();
        }
        let found = self.index.get(script).and_then(|(i, name)| {
            let mut entry = self.zips[*i].by_name(name).ok()?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).ok()?;
            Some(decode_text(&bytes))
        });
        self.cache.insert(script.to_string(), found.clone());
        found
    }

    fn nwscript(&mut self) -> &ScriptConstants {
        if self.nwscript.is_none() {
            let mut constants = ScriptConstants::default();
            if let Some(src) = self.script("nwscript") {
                collect_constants(&src, &mut constants);
            }
            self.nwscript = Some(constants);
        }
        self.nwscript
            .as_ref()
            .expect("nwscript constants just parsed")
    }
}

pub struct ScriptSources<'a> {
    archive: Option<ErfParser>,
    dirs: Vec<PathBuf>,
    game: &'a mut GameScripts,
    cache: HashMap<String, Option<String>>,
    constants_cache: HashMap<String, Arc<ScriptConstants>>,
}

impl<'a> ScriptSources<'a> {
    pub fn open(
        module_path: &Path,
        campaign_dir: Option<&Path>,
        game: &'a mut GameScripts,
    ) -> Self {
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
            game,
            cache: HashMap::new(),
            constants_cache: HashMap::new(),
        }
    }

    /// Named constants visible to `script`: `nwscript` (`FEAT_*` and friends) plus
    /// the `const int`/`const string` declarations of its `#include` chain, such as
    /// the OC's `kinc_companion` `TAG_KHELGAR = "khelgar"`. Cached per module, since
    /// a module can ship its own copy of an include.
    pub fn constants(&mut self, script: &str) -> Arc<ScriptConstants> {
        let key = script.to_ascii_lowercase();
        if let Some(constants) = self.constants_cache.get(&key) {
            return constants.clone();
        }
        let mut constants = self.game.nwscript().clone();
        let mut visited = HashSet::new();
        self.collect_includes(&key, &mut constants, &mut visited);
        let constants = Arc::new(constants);
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
            .or_else(|| self.game.script(&key))
        else {
            return;
        };
        for include in includes_of(&source) {
            self.collect_includes(&include, constants, visited);
        }
        collect_constants(&source, constants);
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
