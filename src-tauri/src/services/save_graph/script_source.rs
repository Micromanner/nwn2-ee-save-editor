//! Looks up `.nss` sources for scripts referenced by dialogue lines: loose files in the
//! campaign folder first (it overrides its modules), then the module's `.mod` archive
//! (or expanded directory), then the game's `data/scripts*.zip` (includes and `nwscript`
//! constants only; the generic `ga_*` family is already classified by the bridge).

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use tracing::debug;

use super::script_decoder::{ScriptConstants, ScriptDecode, collect_constants, decode_action};
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

/// A module `.mod` archive, readable up to the 4 GB ERF offset limit (Highcliff
/// alone is 457 MB). `None` for expanded module directories and unreadable files.
pub fn open_module_archive(module_path: &Path) -> Option<ErfParser> {
    if !module_path.is_file() {
        return None;
    }
    let mut parser = ErfParser::new().with_limits(SecurityLimits {
        max_file_size: 4 * 1024 * 1024 * 1024,
        ..SecurityLimits::default()
    });
    match parser.read(module_path) {
        Ok(()) => Some(parser),
        Err(e) => {
            debug!("module archive unreadable: {}: {e}", module_path.display());
            None
        }
    }
}

pub struct ScriptSources<'a> {
    archive: Option<ErfParser>,
    module_dir: Option<PathBuf>,
    campaign_dir: Option<PathBuf>,
    game: &'a mut GameScripts,
    cache: HashMap<String, Option<String>>,
    constants_cache: HashMap<String, Arc<ScriptConstants>>,
    /// Decodes by lowercase script name and JSON parameters: one dialogue line's
    /// call is decoded once however many nodes repeat it.
    decodes: HashMap<(String, String), Option<Arc<ScriptDecode>>>,
}

impl<'a> ScriptSources<'a> {
    /// Sources of a module: `archive` is its opened `.mod` (`open_module_archive`);
    /// an expanded module directory is searched as loose files.
    pub fn open(
        module_path: &Path,
        archive: Option<ErfParser>,
        campaign_dir: Option<&Path>,
        game: &'a mut GameScripts,
    ) -> Self {
        Self {
            archive,
            module_dir: module_path.is_dir().then(|| module_path.to_path_buf()),
            campaign_dir: campaign_dir.map(Path::to_path_buf),
            game,
            cache: HashMap::new(),
            constants_cache: HashMap::new(),
            decodes: HashMap::new(),
        }
    }

    /// `script_decoder::decode_action` of a call to `script`, `None` when no source
    /// ships or it has no `main`.
    pub fn decode(&mut self, script: &str, params: &[Value]) -> Option<Arc<ScriptDecode>> {
        let key = (
            script.to_ascii_lowercase(),
            Value::from(params.to_vec()).to_string(),
        );
        if let Some(decoded) = self.decodes.get(&key) {
            return decoded.clone();
        }
        let decoded = self.source(script).map(str::to_string).and_then(|src| {
            let constants = self.constants(script);
            decode_action(&src, params, &constants).map(Arc::new)
        });
        self.decodes.insert(key, decoded.clone());
        decoded
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
        module_file(
            self.campaign_dir.as_deref(),
            self.archive.as_mut(),
            self.module_dir.as_deref(),
            &file,
        )
        .map(|bytes| decode_text(&bytes))
    }
}

/// `file` as a module line sees it: the campaign folder overrides the module, which is
/// its `.mod` archive or an expanded module directory.
pub fn module_file(
    campaign_dir: Option<&Path>,
    archive: Option<&mut ErfParser>,
    module_dir: Option<&Path>,
    file: &str,
) -> Option<Vec<u8>> {
    if let Some(bytes) = campaign_dir.and_then(|dir| std::fs::read(dir.join(file)).ok()) {
        return Some(bytes);
    }
    if let Some(archive) = archive
        && let Ok(bytes) = archive.extract_resource(file)
    {
        return Some(bytes);
    }
    module_dir.and_then(|dir| std::fs::read(dir.join(file)).ok())
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
int nCounter = 0;
const int NEGATIVE = -3;
void Bump() { nCounter++; }
void main(int nDefault = 4)
{
    int bDone = 0;
    if (GetLocalInt(OBJECT_SELF, "x")) bDone = 1;
}
"#;
        assert_eq!(includes_of(src), vec!["kinc_companion", "ginc_misc"]);
        let mut constants = ScriptConstants::default();
        collect_constants(src, &mut constants);
        assert_eq!(constants.strings["TAG_KHELGAR"], "khelgar");
        assert_eq!(constants.ints["FEAT_ARMOR_PROFICIENCY_HEAVY"], 2);
        assert_eq!(constants.ints["RR_KHELGAR"], 7);
        assert_eq!(constants.ints["NEGATIVE"], -3);
        assert!(!constants.strings.contains_key("sNotConst"));
        // A function's locals, a global the script changes and parameter defaults are not constants.
        assert!(!constants.ints.contains_key("bDone"));
        assert!(!constants.ints.contains_key("nCounter"));
        assert!(!constants.ints.contains_key("nDefault"));
    }
}
