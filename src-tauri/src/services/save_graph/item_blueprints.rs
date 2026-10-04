//! Resolves the `.uti` blueprints quest rewards hand out (`ga_give_item`): the
//! line's own module archive first, then loose files in the campaign folder,
//! then the resource manager's override chain (base game, override, HAKs).

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use tracing::debug;

use super::SaveGraph;
use crate::parsers::erf::{ErfParser, SecurityLimits};
use crate::parsers::gff::{GffParser, GffValue};
use crate::services::resource_manager::ResourceManager;

#[derive(Debug, Clone)]
pub struct ItemBlueprint {
    pub fields: IndexMap<String, GffValue<'static>>,
    /// Embedded `LocalizedName` text, when the blueprint carries one.
    pub name: Option<String>,
    /// `LocalizedName` strref for names that live in the TLK.
    pub name_strref: Option<i32>,
}

/// Lookup result by `(module, lowercase resref)`; `None` = no source ships it.
type BlueprintCache = HashMap<(String, String), Option<Arc<ItemBlueprint>>>;

pub struct ItemBlueprints<'a> {
    modules: BTreeMap<String, PathBuf>,
    campaign_dir: Option<PathBuf>,
    resources: Option<&'a ResourceManager>,
    archives: RefCell<HashMap<String, Option<ErfParser>>>,
    cache: RefCell<BlueprintCache>,
}

impl<'a> ItemBlueprints<'a> {
    pub fn new(graph: &SaveGraph, resources: Option<&'a ResourceManager>) -> Self {
        Self {
            modules: graph
                .modules
                .iter()
                .filter(|m| !m.resolved_path.is_empty())
                .map(|m| (m.name.clone(), PathBuf::from(&m.resolved_path)))
                .collect(),
            campaign_dir: graph.campaign.campaign_path.as_deref().map(PathBuf::from),
            resources,
            archives: RefCell::new(HashMap::new()),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// No sources at all: every lookup misses.
    #[cfg(test)]
    pub fn detached() -> Self {
        Self {
            modules: BTreeMap::new(),
            campaign_dir: None,
            resources: None,
            archives: RefCell::new(HashMap::new()),
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// The blueprint `resref` resolves to for a line in `module`, or `None` when
    /// no source ships it.
    pub fn get(&self, module: &str, resref: &str) -> Option<Arc<ItemBlueprint>> {
        let key = (module.to_string(), resref.to_ascii_lowercase());
        if let Some(found) = self.cache.borrow().get(&key) {
            return found.clone();
        }
        let found = self.load(module, &key.1).map(Arc::new);
        self.cache.borrow_mut().insert(key, found.clone());
        found
    }

    fn load(&self, module: &str, resref: &str) -> Option<ItemBlueprint> {
        let file = format!("{resref}.uti");
        if let Some(path) = self.modules.get(module) {
            if path.is_dir() {
                if let Ok(bytes) = std::fs::read(path.join(&file)) {
                    return parse_blueprint(&bytes);
                }
            } else if let Some(mut archive) = self.archive(module, path)
                && let Ok(bytes) = archive.extract_resource(&file)
            {
                return parse_blueprint(&bytes);
            }
        }
        if let Some(dir) = &self.campaign_dir
            && let Ok(bytes) = std::fs::read(dir.join(&file))
        {
            return parse_blueprint(&bytes);
        }
        let resources = self.resources?;
        let info = resources.get_item_template(resref)?;
        match resources.get_item_template_fields(&info) {
            Ok(fields) => Some(blueprint_from_fields(fields)),
            Err(e) => {
                debug!("item blueprint {resref} unreadable: {e}");
                None
            }
        }
    }

    fn archive(&self, module: &str, path: &Path) -> Option<std::cell::RefMut<'_, ErfParser>> {
        let mut archives = self.archives.borrow_mut();
        if !archives.contains_key(module) {
            let mut parser = ErfParser::new().with_limits(SecurityLimits {
                max_file_size: 4 * 1024 * 1024 * 1024,
                ..SecurityLimits::default()
            });
            let parsed = match parser.read(path) {
                Ok(()) => Some(parser),
                Err(e) => {
                    debug!("item blueprints unavailable for {}: {e}", path.display());
                    None
                }
            };
            archives.insert(module.to_string(), parsed);
        }
        std::cell::RefMut::filter_map(archives, |a| a.get_mut(module)?.as_mut()).ok()
    }
}

fn parse_blueprint(bytes: &[u8]) -> Option<ItemBlueprint> {
    let parser = GffParser::from_bytes(bytes.to_vec()).ok()?;
    let fields = parser.read_owned_struct(0).ok()?;
    Some(blueprint_from_fields(fields))
}

/// An item struct's `LocalizedName`: embedded text and TLK strref, either may be missing.
pub(super) fn localized_name(
    fields: &IndexMap<String, GffValue<'static>>,
) -> (Option<String>, Option<i32>) {
    match fields.get("LocalizedName") {
        Some(GffValue::LocString(ls)) => (
            ls.substrings
                .iter()
                .map(|s| s.string.to_string())
                .find(|s| !s.is_empty()),
            (ls.string_ref >= 0).then_some(ls.string_ref),
        ),
        _ => (None, None),
    }
}

fn blueprint_from_fields(fields: IndexMap<String, GffValue<'static>>) -> ItemBlueprint {
    let (name, name_strref) = localized_name(&fields);
    ItemBlueprint {
        fields,
        name,
        name_strref,
    }
}
