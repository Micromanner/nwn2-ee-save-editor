pub mod error;
pub mod module_loader;
pub mod override_chain;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};

use dashmap::DashMap;
use indexmap::IndexMap;
use parking_lot::Mutex;
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};

use crate::config::NWN2Paths;
use crate::parsers::erf::ErfParser;
use crate::parsers::gff::GffParser;
use crate::parsers::tda::TDAParser;
use crate::parsers::tlk::TLKParser;
use crate::services::model_loader::SkeletonData;
use crate::utils::ZipContentReader;
use crate::utils::directory_scanner::{self, ScannedFile};

pub use error::{ResourceManagerError, ResourceManagerResult};
pub use override_chain::{ContainerType, ModuleContent, OverrideSource, ResourceLocation};

const VO_ZIPS: &[&str] = &["vo.zip", "vo_x1.zip", "vo_x2.zip"];

const ENHANCED_ZIPS: &[&str] = &[
    "nwn2_models.zip",
    "lod-merged.zip",
    "lod-merged_x1.zip",
    "lod-merged_x2.zip",
    "nwn2_materials_noupscale.zip",
    "upscale_initial.zip",
    "upscale_main.zip",
    "upscale_x1.zip",
    "upscale_x2.zip",
];

type IndexEntries = Vec<(String, ResourceLocation)>;

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_ratio: f64,
}

pub struct ResourceManager {
    paths: Arc<RwLock<NWN2Paths>>,
    zip_reader: Mutex<ZipContentReader>,

    tlk_cache: Option<Arc<StdRwLock<TLKParser>>>,
    /// The loaded module's `Mod_CustomTlk`.
    custom_tlk: Option<Arc<StdRwLock<TLKParser>>>,

    tda_cache: DashMap<(String, OverrideSource), Arc<TDAParser>>,

    module_content: Option<ModuleContent>,
    module_path: Option<PathBuf>,
    /// Opened HAK/module archives, so a read does not re-parse the key list.
    erf_cache: DashMap<PathBuf, Arc<ErfParser>>,

    data_zip_paths: Vec<PathBuf>,

    resource_index: HashMap<String, Vec<ResourceLocation>>,
    icon_file_paths: HashMap<String, PathBuf>,

    /// Lazily built per-extension sorted key list, so `list_resources_by_prefix`
    /// binary-searches a range instead of walking the whole index. Dropped
    /// whenever `resource_index` changes.
    prefix_index: DashMap<String, Arc<Vec<String>>>,
    /// Parsed skeleton GR2s by lowercase resref. Every skinned item model
    /// resolves the same body skeleton, so it is parsed once per index state.
    skeleton_cache: DashMap<String, Arc<SkeletonData>>,

    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    initialized: bool,
}

/// Everything `initialize` reads from disk, gathered off the async runtime.
struct BaseScan {
    data_zip_paths: Vec<PathBuf>,
    entries: IndexEntries,
    icon_file_paths: HashMap<String, PathBuf>,
    icon_dir: Option<PathBuf>,
    tlk: Option<TLKParser>,
}

/// The resources and custom TLK of one module, gathered by
/// `ResourceManager::scan_module_content` without holding the manager's lock.
#[derive(Default)]
pub struct ModuleScan {
    content: Option<ModuleContent>,
    module_path: Option<PathBuf>,
    entries: IndexEntries,
    erfs: Vec<(PathBuf, Arc<ErfParser>)>,
    custom_tlk: Option<TLKParser>,
}

impl ResourceManager {
    pub fn new(paths: Arc<RwLock<NWN2Paths>>) -> Self {
        Self {
            paths,
            zip_reader: Mutex::new(ZipContentReader::new()),
            tlk_cache: None,
            custom_tlk: None,
            tda_cache: DashMap::new(),
            module_content: None,
            module_path: None,
            erf_cache: DashMap::new(),
            data_zip_paths: Vec::new(),
            resource_index: HashMap::new(),
            icon_file_paths: HashMap::new(),
            prefix_index: DashMap::new(),
            skeleton_cache: DashMap::new(),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            initialized: false,
        }
    }

    pub async fn initialize(&mut self) -> ResourceManagerResult<()> {
        if self.initialized {
            return Ok(());
        }

        info!("Initializing ResourceManager");

        let paths = self.paths.read().await.clone();
        let scan = tokio::task::spawn_blocking(move || scan_base(&paths))
            .await
            .map_err(|e| ResourceManagerError::Parse(format!("Resource scan failed: {e}")))??;

        self.data_zip_paths = scan.data_zip_paths;
        self.insert_entries(scan.entries);
        self.icon_file_paths = scan.icon_file_paths;
        self.tlk_cache = scan.tlk.map(|tlk| Arc::new(StdRwLock::new(tlk)));

        info!(
            "Indexed {} data zips: {} 2DAs, {} workshop resources, {} icons from {}",
            self.data_zip_paths.len(),
            self.count_2das(|_| true),
            self.count_locations(|s| matches!(s, OverrideSource::Workshop)),
            self.icon_file_paths.len(),
            scan.icon_dir
                .map_or_else(|| "-".to_string(), |d| d.display().to_string()),
        );

        self.initialized = true;
        info!(
            "ResourceManager initialized: {} total resource keys, {} templates, {} icons",
            self.resource_index.len(),
            self.template_count(),
            self.icon_file_paths.len(),
        );

        Ok(())
    }

    pub async fn update_paths(&mut self, new_paths: NWN2Paths) {
        *self.paths.write().await = new_paths;
        *self = Self::new(Arc::clone(&self.paths));
    }

    /// Drops everything derived from `resource_index`; call whenever it changes.
    fn invalidate_index_caches(&self) {
        self.prefix_index.clear();
        self.skeleton_cache.clear();
    }

    pub fn cached_skeleton(&self, key: &str) -> Option<Arc<SkeletonData>> {
        self.skeleton_cache.get(key).map(|s| Arc::clone(&s))
    }

    pub fn cache_skeleton(&self, key: String, skeleton: Arc<SkeletonData>) {
        self.skeleton_cache.insert(key, skeleton);
    }

    fn insert_entries(&mut self, entries: IndexEntries) {
        for (key, location) in entries {
            self.resource_index.entry(key).or_default().push(location);
        }
        self.invalidate_index_caches();
    }

    fn count_locations(&self, source: impl Fn(&OverrideSource) -> bool) -> usize {
        self.resource_index
            .values()
            .flatten()
            .filter(|l| source(&l.source))
            .count()
    }

    fn count_2das(&self, source: impl Fn(&OverrideSource) -> bool) -> usize {
        self.resource_index
            .iter()
            .filter(|(k, locs)| k.ends_with(".2da") && locs.iter().any(|l| source(&l.source)))
            .count()
    }

    pub fn module_content(&self) -> Option<&ModuleContent> {
        self.module_content.as_ref()
    }

    /// Reads the module-specific content (module, campaign folder, HAKs,
    /// custom TLK) of `content` from disk. Blocking; runs without the
    /// manager's lock, `set_module_content` then swaps it in.
    pub fn scan_module_content(paths: &NWN2Paths, content: Option<ModuleContent>) -> ModuleScan {
        match content {
            Some(content) => scan_module(paths, content),
            None => ModuleScan::default(),
        }
    }

    /// Replaces the module-specific content with a scan's.
    pub fn set_module_content(&mut self, scan: ModuleScan) {
        for locs in self.resource_index.values_mut() {
            locs.retain(|l| !l.source.is_module_content());
        }
        self.resource_index.retain(|_, locs| !locs.is_empty());
        self.tda_cache.retain(|k, _| !k.1.is_module_content());
        self.erf_cache.clear();

        if let Some(content) = &scan.content {
            let count_2das = |source: fn(&OverrideSource) -> bool| {
                scan.entries
                    .iter()
                    .filter(|(key, loc)| key.ends_with(".2da") && source(&loc.source))
                    .count()
            };
            info!(
                "Module content for '{}': {} HAK 2DAs, {} campaign 2DAs, {} module 2DAs, custom TLK: {}",
                content.name,
                count_2das(|s| matches!(s, OverrideSource::Hak(_))),
                count_2das(|s| matches!(s, OverrideSource::Campaign)),
                count_2das(|s| matches!(s, OverrideSource::Module)),
                scan.custom_tlk.is_some(),
            );
        } else {
            info!("Module content cleared");
        }

        for (path, erf) in scan.erfs {
            self.erf_cache.insert(path, erf);
        }
        self.insert_entries(scan.entries);
        self.custom_tlk = scan.custom_tlk.map(|tlk| Arc::new(StdRwLock::new(tlk)));
        self.module_path = scan.module_path;
        self.module_content = scan.content;
    }

    pub fn get_2da(&self, name: &str) -> ResourceManagerResult<Arc<TDAParser>> {
        self.load_best_2da(name, |source| matches!(source, OverrideSource::BaseGame))
    }

    pub fn get_2da_with_overrides(&self, name: &str) -> ResourceManagerResult<Arc<TDAParser>> {
        self.load_best_2da(name, |_| true)
    }

    fn load_best_2da(
        &self,
        name: &str,
        source: impl Fn(&OverrideSource) -> bool,
    ) -> ResourceManagerResult<Arc<TDAParser>> {
        let name_lower = name.to_lowercase();
        let name_lower = name_lower.strip_suffix(".2da").unwrap_or(&name_lower);

        let location = self
            .resource_index
            .get(&resource_key(name_lower, "2da"))
            .and_then(|locs| best_location(locs.iter().filter(|l| source(&l.source))));
        let Some(location) = location else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
            return Err(ResourceManagerError::TdaNotFound { name: name.into() });
        };

        let cache_key = (name_lower.to_string(), location.source.clone());
        if let Some(parser) = self.tda_cache.get(&cache_key) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Arc::clone(&parser));
        }
        self.cache_misses.fetch_add(1, Ordering::Relaxed);

        let data = self.read_location(location)?;
        let mut parser = TDAParser::new();
        parser.parse_from_bytes(&data).map_err(|e| {
            ResourceManagerError::InvalidTdaFormat(format!("Failed to parse {name_lower}.2da: {e}"))
        })?;
        let parser = Arc::new(parser);
        self.tda_cache.insert(cache_key, Arc::clone(&parser));
        Ok(parser)
    }

    pub fn get_string(&self, str_ref: i32) -> String {
        if str_ref < 0 {
            return String::new();
        }
        crate::parsers::tlk::resolve_strref(
            self.tlk_cache.as_deref(),
            self.custom_tlk.as_deref(),
            str_ref,
        )
        .unwrap_or_else(|| format!("<StrRef:{str_ref}>"))
    }

    pub fn get_strings_batch(&self, str_refs: &[i32]) -> HashMap<i32, String> {
        str_refs
            .iter()
            .map(|&str_ref| (str_ref, self.get_string(str_ref)))
            .collect()
    }

    pub fn get_item_template(&self, resref: &str) -> Option<ResourceLocation> {
        let key = resource_key(&resref.to_lowercase(), "uti");
        best_location(self.resource_index.get(&key)?).cloned()
    }

    /// Every item template by lowercase resref.
    pub fn get_all_item_templates(&self) -> HashMap<String, ResourceLocation> {
        self.sorted_keys_for_extension("uti")
            .iter()
            .filter_map(|key| {
                let location = best_location(self.resource_index.get(key)?)?;
                Some((key.strip_suffix(".uti")?.to_string(), location.clone()))
            })
            .collect()
    }

    pub fn get_item_template_fields(
        &self,
        location: &ResourceLocation,
    ) -> ResourceManagerResult<IndexMap<String, crate::parsers::gff::GffValue<'static>>> {
        let data = self.read_location(location)?;

        let gff = GffParser::from_bytes(data).map_err(|e| {
            ResourceManagerError::InvalidGffFormat(format!("Failed to parse template: {e}"))
        })?;

        let fields = gff.read_struct_fields(0).map_err(|e| {
            ResourceManagerError::InvalidGffFormat(format!("Failed to read template fields: {e}"))
        })?;

        Ok(fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned()))
            .collect())
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);

        CacheStats {
            size: self.tda_cache.len(),
            hits,
            misses,
            hit_ratio: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }

    /// The `.mod` file or module directory of the loaded module, if installed.
    pub fn module_path(&self) -> Option<&Path> {
        self.module_path.as_deref()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn resource_count(&self) -> usize {
        self.resource_index.len()
    }

    pub fn template_count(&self) -> usize {
        self.resource_index
            .keys()
            .filter(|k| k.ends_with(".uti"))
            .count()
    }

    pub fn data_zip_paths(&self) -> &[PathBuf] {
        &self.data_zip_paths
    }

    pub fn resource_source_counts(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for loc in self.resource_index.values().flatten() {
            *counts.entry(loc.source.key().to_string()).or_default() += 1;
        }
        counts
    }

    pub fn get_available_2da_files(&self) -> Vec<String> {
        self.resource_index
            .keys()
            .filter_map(|k| k.strip_suffix(".2da"))
            .map(str::to_string)
            .collect()
    }

    pub fn get_tlk_parser(&self) -> Option<Arc<StdRwLock<TLKParser>>> {
        self.tlk_cache.clone()
    }

    pub fn get_custom_tlk_parser(&self) -> Option<Arc<StdRwLock<TLKParser>>> {
        self.custom_tlk.clone()
    }

    pub fn get_icon_path(&self, resref: &str) -> Option<PathBuf> {
        self.icon_file_paths.get(&resref.to_lowercase()).cloned()
    }

    pub fn has_resource(&self, resref: &str, extension: &str) -> bool {
        let key = resource_key(&resref.to_lowercase(), &extension.to_lowercase());
        self.resource_index.contains_key(&key)
    }

    pub fn get_resource_bytes(
        &self,
        resref: &str,
        extension: &str,
    ) -> ResourceManagerResult<Vec<u8>> {
        let key = resource_key(&resref.to_lowercase(), &extension.to_lowercase());

        trace!(
            "ResourceManager: get_resource_bytes searching for key: {}",
            key
        );

        if let Some(location) = self.resource_index.get(&key).and_then(best_location) {
            trace!(
                "ResourceManager: Found resource '{}' in source: {:?}",
                key, location.source
            );
            let started = std::time::Instant::now();
            let result = self.read_location(location);
            let elapsed = started.elapsed();
            if elapsed.as_millis() >= 50 {
                info!(
                    "Slow resource read '{key}': {elapsed:?} from {:?} {} (scan-located: {})",
                    location.source,
                    location.container_path.display(),
                    location.zip_entry.is_some()
                );
            }
            return result;
        }

        Err(ResourceManagerError::FileNotFound(PathBuf::from(format!(
            "{resref}.{extension}"
        ))))
    }

    pub fn read_location(&self, location: &ResourceLocation) -> ResourceManagerResult<Vec<u8>> {
        match &location.container_type {
            ContainerType::Directory => Ok(std::fs::read(&location.container_path)?),
            ContainerType::Zip => {
                let internal = location
                    .internal_path
                    .as_ref()
                    .ok_or_else(|| ResourceManagerError::Parse("Missing internal path".into()))?;
                // Entries the startup scan located are read straight from
                // the file, so concurrent reads (a thumbnail's textures,
                // prefetched models) inflate in parallel instead of queueing
                // on the shared reader.
                match location.zip_entry {
                    Some(entry) => ZipContentReader::read_entry(&location.container_path, &entry)
                        .map_err(ResourceManagerError::ZipError),
                    None => self
                        .zip_reader
                        .lock()
                        .read_file_from_zip(
                            location.container_path.to_string_lossy().to_string(),
                            internal.clone(),
                        )
                        .map_err(ResourceManagerError::ZipError),
                }
            }
            ContainerType::Erf => {
                let internal = location
                    .internal_path
                    .as_ref()
                    .ok_or_else(|| ResourceManagerError::Parse("Missing internal path".into()))?;
                let erf = self.open_erf(&location.container_path)?;
                erf.read_resource(internal).map_err(|e| {
                    warn!("Failed to read {internal} from ERF: {e}");
                    ResourceManagerError::ExtractionFailed {
                        resource: internal.clone(),
                        container: location.container_path.display().to_string(),
                    }
                })
            }
        }
    }

    fn open_erf(&self, path: &Path) -> ResourceManagerResult<Arc<ErfParser>> {
        if let Some(erf) = self.erf_cache.get(path) {
            return Ok(Arc::clone(&erf));
        }
        let erf = Arc::new(read_erf(path)?);
        self.erf_cache.insert(path.to_path_buf(), Arc::clone(&erf));
        Ok(erf)
    }

    pub fn list_resources_by_extension(&self, extension: &str) -> Vec<(String, String)> {
        let ext_suffix = format!(".{}", extension.to_lowercase());
        let mut results: Vec<(String, String)> = self
            .resource_index
            .iter()
            .filter(|(key, _)| key.ends_with(&ext_suffix))
            .filter_map(|(key, locs)| {
                let source = best_location(locs)?
                    .container_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                Some((key.clone(), source))
            })
            .collect();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    pub fn list_resources_by_prefix(&self, prefix: &str, extension: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let ext_lower = extension.to_lowercase();

        trace!(
            "ResourceManager: list_resources_by_prefix searching index for prefix: '{}' ext: '{}'",
            prefix, extension
        );

        let keys = self.sorted_keys_for_extension(&ext_lower);
        let start = keys.partition_point(|k| k.as_str() < prefix_lower.as_str());
        keys[start..]
            .iter()
            .take_while(|k| k.starts_with(&prefix_lower))
            .cloned()
            .collect()
    }

    /// Sorted, deduplicated resource keys for one extension, built once and
    /// memoized. The list is sorted by full key (`stem.ext`), which for a fixed
    /// extension orders by stem, so a prefix match is a contiguous range.
    fn sorted_keys_for_extension(&self, ext_lower: &str) -> Arc<Vec<String>> {
        if let Some(cached) = self.prefix_index.get(ext_lower) {
            return Arc::clone(cached.value());
        }

        let ext_suffix = format!(".{ext_lower}");
        let mut keys: Vec<String> = self
            .resource_index
            .keys()
            .filter(|key| key.ends_with(&ext_suffix))
            .cloned()
            .collect();
        keys.sort_unstable();
        keys.dedup();

        let keys = Arc::new(keys);
        self.prefix_index
            .insert(ext_lower.to_string(), Arc::clone(&keys));
        keys
    }
}

fn resource_key(stem: &str, extension: &str) -> String {
    format!("{stem}.{extension}")
}

/// Highest-priority location. Ties go to the location indexed last, which is
/// why the data zips are indexed in sorted order (`2da_x2.zip` after
/// `2da_x1.zip` after `2da.zip`).
fn best_location<'a>(
    locations: impl IntoIterator<Item = &'a ResourceLocation>,
) -> Option<&'a ResourceLocation> {
    locations.into_iter().max_by_key(|l| l.source.priority())
}

fn file_entries(files: Vec<ScannedFile>, source: &OverrideSource) -> IndexEntries {
    files
        .into_iter()
        .map(|file| {
            (
                resource_key(&file.stem, &file.extension),
                ResourceLocation::from_file(source.clone(), file.path),
            )
        })
        .collect()
}

fn read_erf(path: &Path) -> ResourceManagerResult<ErfParser> {
    let mut erf = ErfParser::new();
    erf.read(path).map_err(|e| {
        ResourceManagerError::InvalidErfFormat(format!("Failed to open {}: {e}", path.display()))
    })?;
    Ok(erf)
}

fn erf_entries(path: &Path, erf: &ErfParser, source: &OverrideSource) -> IndexEntries {
    erf.resources
        .keys()
        .filter(|name| name.contains('.'))
        .map(|name| {
            (
                name.clone(),
                ResourceLocation::from_erf(source.clone(), path.to_path_buf(), name.clone()),
            )
        })
        .collect()
}

fn scan_base(paths: &NWN2Paths) -> ResourceManagerResult<BaseScan> {
    let data_dir = paths
        .data()
        .ok_or_else(|| ResourceManagerError::PathNotConfigured("NWN2 data directory".into()))?;
    let game_folder = paths.game_folder().cloned();
    let enhanced_data = paths.enhanced_data();

    // Every archive in the data directory is indexed, so a lookup that
    // misses is a hash miss instead of a search that opens each unindexed
    // archive. Sorted, because equal-priority ties go to the last indexed.
    let mut data_zip_paths: Vec<PathBuf> = std::fs::read_dir(&data_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")))
        .collect();
    data_zip_paths.sort_by_key(|p| p.to_string_lossy().to_lowercase());

    let mut zip_paths = data_zip_paths.clone();
    if let Some(enhanced_data) = &enhanced_data {
        zip_paths.extend(
            ENHANCED_ZIPS
                .iter()
                .map(|name| enhanced_data.join(name))
                .filter(|p| p.is_file()),
        );
    }
    if let Some(vo_dir) = paths.localization_folder().map(|dir| dir.join("data")) {
        zip_paths.extend(
            VO_ZIPS
                .iter()
                .map(|name| vo_dir.join(name))
                .filter(|p| p.is_file()),
        );
    }

    let zip_entries = crate::utils::zip_scanner::scan_zips_parallel(&zip_paths)
        .map_err(ResourceManagerError::ZipError)?;

    let mut entries = IndexEntries::with_capacity(zip_entries.len());
    for entry in zip_entries {
        // EE LOD archives contain a newer Granny layout. Keep the original
        // GR2 skeletons/animations until that layout has parser support.
        if entry.extension == "gr2"
            && enhanced_data
                .as_ref()
                .is_some_and(|dir| entry.zip_path.starts_with(dir))
        {
            continue;
        }
        entries.push((
            resource_key(&entry.stem, &entry.extension),
            ResourceLocation::from_zip(
                OverrideSource::BaseGame,
                entry.zip_path,
                entry.internal_path,
                entry.location,
            ),
        ));
    }

    if let Some(workshop_dir) = paths.steam_workshop_folder() {
        debug!(
            "Scanning Steam Workshop directory: {}",
            workshop_dir.display()
        );
        entries.extend(file_entries(
            directory_scanner::scan_workshop(workshop_dir),
            &OverrideSource::Workshop,
        ));
    }

    if let Some(dir) = paths.override_dir() {
        entries.extend(file_entries(
            directory_scanner::scan_directory(&dir, true),
            &OverrideSource::OverrideDir,
        ));
    }
    for dir in paths.custom_override_folders() {
        entries.extend(file_entries(
            directory_scanner::scan_directory(dir, true),
            &OverrideSource::CustomOverride,
        ));
    }

    let (icon_dir, icon_file_paths) = scan_icons(game_folder.as_deref(), &mut entries);

    Ok(BaseScan {
        data_zip_paths,
        entries,
        icon_file_paths,
        icon_dir,
        tlk: load_base_tlk(paths),
    })
}

fn scan_icons(
    game_folder: Option<&Path>,
    entries: &mut IndexEntries,
) -> (Option<PathBuf>, HashMap<String, PathBuf>) {
    let mut icon_file_paths = HashMap::new();
    let Some(icon_dir) = game_folder.and_then(|gf| {
        ["upscaled", "default"]
            .iter()
            .map(|flavor| gf.join("ui").join(flavor).join("icons"))
            .find(|dir| dir.exists())
    }) else {
        return (None, icon_file_paths);
    };

    for file in directory_scanner::scan_directory(&icon_dir, true) {
        if matches!(file.extension.as_str(), "dds" | "tga" | "png") {
            icon_file_paths.insert(file.stem.clone(), file.path.clone());
            entries.push((
                resource_key(&file.stem, &file.extension),
                ResourceLocation::from_file(OverrideSource::BaseGame, file.path),
            ));
        }
    }
    (Some(icon_dir), icon_file_paths)
}

/// `dialog.tlk` of a folder, spelled either way (matters on case-sensitive
/// file systems).
fn dialog_tlk_in(dir: &Path) -> Option<PathBuf> {
    ["dialog.tlk", "dialog.TLK"]
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// The base talk table. Mods like Kaedrin's PrC Pack ship a merged
/// `dialog.tlk` into the Documents folder (Nexus install) or their Workshop
/// item folder; the candidate with the most strings wins.
fn load_base_tlk(paths: &NWN2Paths) -> Option<TLKParser> {
    let mut candidates: Vec<PathBuf> = paths.dialog_tlk().into_iter().collect();
    candidates.extend(paths.documents_folder().and_then(|d| dialog_tlk_in(d)));
    if let Some(workshop_dir) = paths.steam_workshop_folder()
        && let Ok(items) = std::fs::read_dir(workshop_dir)
    {
        candidates.extend(
            items
                .flatten()
                .map(|item| item.path())
                .filter(|p| p.is_dir())
                .filter_map(|item| dialog_tlk_in(&item)),
        );
    }

    let best = candidates
        .into_iter()
        .filter_map(|path| match TLKParser::string_count_of_file(&path) {
            Ok(count) => Some((path, count)),
            Err(e) => {
                debug!("Skipping TLK {}: {}", path.display(), e);
                None
            }
        })
        .fold(None::<(PathBuf, usize)>, |best, (path, count)| match best {
            Some((_, best_count)) if best_count >= count => best,
            _ => Some((path, count)),
        })?;

    match module_loader::load_tlk(&best.0) {
        Ok(parser) => {
            info!(
                "Using TLK: {} ({} entries)",
                best.0.display(),
                parser.string_count()
            );
            Some(parser)
        }
        Err(e) => {
            warn!("Failed to parse TLK {}: {}", best.0.display(), e);
            None
        }
    }
}

fn scan_module(paths: &NWN2Paths, content: ModuleContent) -> ModuleScan {
    let mut entries = IndexEntries::new();
    let mut erfs = Vec::new();

    let module_path = module_loader::find_installed_module(paths, &content.name);
    match &module_path {
        Some(path) if path.is_dir() => entries.extend(file_entries(
            directory_scanner::scan_directory(path, true),
            &OverrideSource::Module,
        )),
        Some(path) => match read_erf(path) {
            Ok(erf) => {
                entries.extend(erf_entries(path, &erf, &OverrideSource::Module));
                erfs.push((path.clone(), Arc::new(erf)));
            }
            Err(e) => warn!("Failed to open module {}: {}", path.display(), e),
        },
        None => warn!("Module not installed: {}", content.name),
    }

    if let Some(guid) = content.campaign_id.as_deref().filter(|g| !g.is_empty()) {
        if let Some(folder) = crate::services::campaign::content::find_campaign_path(guid, paths)
            .and_then(|cam_file| cam_file.parent().map(Path::to_path_buf))
        {
            entries.extend(file_entries(
                directory_scanner::scan_directory(&folder, true),
                &OverrideSource::Campaign,
            ));
        } else {
            warn!("Campaign not found: {}", guid);
        }
    }

    let hak_dirs: Vec<PathBuf> = paths
        .custom_hak_folders()
        .iter()
        .cloned()
        .chain(paths.hak_dir())
        .chain(paths.install_hak_dir())
        .collect();
    for (idx, hak_name) in content
        .hak_list
        .iter()
        .take(usize::from(u8::MAX))
        .enumerate()
    {
        let Some(hak_path) = module_loader::find_named_file(hak_name, "hak", &hak_dirs) else {
            warn!("HAK not found: {}", hak_name);
            continue;
        };
        match read_erf(&hak_path) {
            Ok(erf) => {
                entries.extend(erf_entries(
                    &hak_path,
                    &erf,
                    &OverrideSource::Hak(idx as u8),
                ));
                erfs.push((hak_path, Arc::new(erf)));
            }
            Err(e) => warn!("Failed to open HAK {}: {}", hak_name, e),
        }
    }

    ModuleScan {
        custom_tlk: load_custom_tlk(paths, &content.custom_tlk),
        content: Some(content),
        module_path,
        entries,
        erfs,
    }
}

/// The module's `Mod_CustomTlk`, from the Documents or install `tlk` folder.
fn load_custom_tlk(paths: &NWN2Paths, name: &str) -> Option<TLKParser> {
    if name.is_empty() {
        return None;
    }
    let tlk_dirs: Vec<PathBuf> = paths
        .tlk_dir()
        .into_iter()
        .chain(paths.install_tlk_dir())
        .collect();
    let Some(path) = module_loader::find_named_file(name, "tlk", &tlk_dirs) else {
        warn!("Custom TLK not found: {name}");
        return None;
    };
    module_loader::load_tlk(&path)
        .map_err(|e| warn!("{e}"))
        .ok()
}
