//! Resolves the `.uti` blueprints quest rewards hand out (`ga_give_item`) in the game's
//! content priority: HAKs and override folders, then loose files in the campaign folder
//! (it overrides its modules), then the line's own module, then the base game.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use indexmap::IndexMap;
use tracing::debug;

use super::SaveGraph;
use super::script_source::{module_file, open_module_archive};
use crate::parsers::erf::ErfParser;
use crate::parsers::gff::{GffParser, GffValue};
use crate::services::resource_manager::{OverrideSource, ResourceManager};

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

    /// `file` from the campaign folder, else the module's archive or directory.
    fn module_file(&self, module: &str, file: &str) -> Option<Vec<u8>> {
        let path = self.modules.get(module);
        // `open_module_archive` has nothing for an expanded module directory.
        let mut archive = path.and_then(|p| self.archive(module, p));
        let module_dir = path.filter(|p| p.is_dir());
        module_file(
            self.campaign_dir.as_deref(),
            archive.as_deref_mut(),
            module_dir.map(PathBuf::as_path),
            file,
        )
    }

    fn load(&self, module: &str, resref: &str) -> Option<ItemBlueprint> {
        // The chain's campaign and module copies are the loaded module's; the line's own
        // campaign folder and module take their place.
        let chained = self.resources.and_then(|r| {
            let info = r.get_item_template_where(resref, |s| {
                !matches!(s, OverrideSource::Module | OverrideSource::Campaign)
            })?;
            Some((r, info))
        });
        let outranks_module = chained
            .as_ref()
            .is_some_and(|(_, info)| info.source.priority() > OverrideSource::Campaign.priority());
        if !outranks_module && let Some(bytes) = self.module_file(module, &format!("{resref}.uti"))
        {
            return parse_blueprint(&bytes);
        }
        let (resources, info) = chained?;
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
            archives.insert(module.to_string(), open_module_archive(path));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NWN2Paths;
    use crate::parsers::gff::GffWriter;
    use indexmap::indexmap;
    use tokio::sync::RwLock;

    fn write_uti(dir: &Path, tag: &str) -> PathBuf {
        let bytes = GffWriter::new("UTI ", "V3.2")
            .write(indexmap! { "Tag".into() => GffValue::String(tag.into()) })
            .unwrap();
        let path = dir.join("qe_sword.uti");
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn blueprints<'a>(campaign: &Path, resources: &'a ResourceManager) -> ItemBlueprints<'a> {
        ItemBlueprints {
            campaign_dir: Some(campaign.to_path_buf()),
            resources: Some(resources),
            ..ItemBlueprints::detached()
        }
    }

    fn tag(blueprint: Option<Arc<ItemBlueprint>>) -> String {
        match &blueprint.unwrap().fields["Tag"] {
            GffValue::String(tag) => tag.to_string(),
            other => panic!("Tag is {other:?}"),
        }
    }

    /// HAKs and override folders beat the campaign folder and module; the base game
    /// loses to them.
    #[test]
    fn override_chain_ranks_around_the_campaign_folder() {
        let campaign = tempfile::tempdir().unwrap();
        write_uti(campaign.path(), "campaign");
        let other = tempfile::tempdir().unwrap();
        let hak = write_uti(other.path(), "hak");
        let base = tempfile::tempdir().unwrap();
        let base = write_uti(base.path(), "base");
        let mut resources = ResourceManager::new(Arc::new(RwLock::new(NWN2Paths::new())));
        resources.index_test_file(OverrideSource::BaseGame, base);
        assert_eq!(
            tag(blueprints(campaign.path(), &resources).get("m", "QE_Sword")),
            "campaign"
        );
        resources.index_test_file(OverrideSource::Hak(0), hak);
        assert_eq!(
            tag(blueprints(campaign.path(), &resources).get("m", "qe_sword")),
            "hak"
        );
    }
}
