pub mod cache;
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
use crate::utils::ZipContentReader;

pub use cache::{CacheStats, CachedModuleState, FileModificationTracker, ModuleLRUCache};
pub use error::{ResourceManagerError, ResourceManagerResult};
pub use override_chain::{
    CampaignInfo, ContainerType, ModuleInfo, OverrideSource, ResourceLocation, TemplateInfo,
};

const BASE_GAME_ZIPS: &[&str] = &["2da.zip", "2da_x1.zip", "2da_x2.zip"];
const TEMPLATE_ZIPS: &[&str] = &["Templates.zip", "Templates_X1.zip", "Templates_X2.zip"];
const SOUNDSET_ZIPS: &[&str] = &["soundsets.zip", "soundsets_x1.zip", "soundsets_x2.zip"];
const SOUND_ZIPS: &[&str] = &["sounds.zip", "sounds_x1.zip", "sounds_x2.zip"];
const MODEL_ZIPS: &[&str] = &[
    "nwn2_models.zip",
    "nwn2_models_x1.zip",
    "nwn2_models_x2.zip",
];
const MATERIAL_ZIPS: &[&str] = &["nwn2_materials.zip"];
const ACTOR_ZIPS: &[&str] = &["actors.zip", "actors_x1.zip", "actors_x2.zip"];
const LOD_ZIPS: &[&str] = &[
    "lod-merged.zip",
    "lod-merged_x1.zip",
    "lod-merged_x2.zip",
    "lod-merged_v101.zip",
    "lod-merged_v107.zip",
    "lod-merged_v121.zip",
    "lod-merged_x1_v121.zip",
    "lod-merged_x2_v121.zip",
];
const VO_ZIPS: &[&str] = &["vo.zip", "vo_x1.zip", "vo_x2.zip"];

pub struct ResourceManager {
    paths: Arc<RwLock<NWN2Paths>>,
    zip_reader: Mutex<ZipContentReader>,

    template_locations: HashMap<String, ResourceLocation>,

    tlk_cache: Option<Arc<StdRwLock<TLKParser>>>,
    custom_tlk_cache: Option<Arc<StdRwLock<TLKParser>>>,

    hak_overrides: Vec<HashMap<String, Arc<TDAParser>>>,
    module_overrides: DashMap<String, Arc<TDAParser>>,
    tda_cache: DashMap<(String, OverrideSource), Arc<TDAParser>>,

    current_module: Option<String>,
    module_path: Option<PathBuf>,

    module_info: Option<ModuleInfo>,

    current_campaign_id: Option<String>,
    current_campaign_folder: Option<PathBuf>,

    module_cache: ModuleLRUCache,
    file_mod_tracker: FileModificationTracker,

    data_zip_paths: Vec<PathBuf>,

    resource_index: HashMap<String, Vec<ResourceLocation>>,
    icon_file_paths: HashMap<String, PathBuf>,

    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    initialized: bool,
}

impl ResourceManager {
    pub fn new(paths: Arc<RwLock<NWN2Paths>>) -> Self {
        Self {
            paths,
            zip_reader: Mutex::new(ZipContentReader::new()),
            template_locations: HashMap::new(),
            tlk_cache: None,
            custom_tlk_cache: None,
            hak_overrides: Vec::new(),
            module_overrides: DashMap::new(),
            tda_cache: DashMap::new(),
            current_module: None,
            module_path: None,
            module_info: None,
            current_campaign_id: None,
            current_campaign_folder: None,
            data_zip_paths: Vec::new(),
            resource_index: HashMap::new(),
            icon_file_paths: HashMap::new(),
            module_cache: ModuleLRUCache::new(),
            file_mod_tracker: FileModificationTracker::new(),
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

        self.scan_base_game_zips().await?;
        self.scan_workshop_directories().await?;
        self.scan_override_directories().await?;
        self.scan_icon_directories().await?;
        self.load_base_tlk().await?;
        self.cache_data_zip_paths().await;

        self.initialized = true;
        info!(
            "ResourceManager initialized: {} total resource keys, {} templates, {} icons",
            self.resource_index.len(),
            self.template_locations.len(),
            self.icon_file_paths.len(),
        );

        Ok(())
    }

    pub async fn update_paths(&mut self, new_paths: NWN2Paths) {
        {
            let mut paths = self.paths.write().await;
            *paths = new_paths;
        }

        self.initialized = false;
        self.template_locations.clear();
        self.tlk_cache = None;
        self.custom_tlk_cache = None;
        self.hak_overrides.clear();
        self.module_overrides.clear();
        self.tda_cache.clear();
        self.current_module = None;
        self.module_path = None;
        self.module_info = None;
        self.current_campaign_id = None;
        self.current_campaign_folder = None;
        self.module_cache.clear();
        self.file_mod_tracker.clear();
        self.data_zip_paths.clear();
        self.resource_index.clear();
        self.icon_file_paths.clear();
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        *self.zip_reader.lock() = ZipContentReader::new();
    }

    async fn scan_base_game_zips(&mut self) -> ResourceManagerResult<()> {
        let paths = self.paths.read().await;
        let data_dir = paths
            .data()
            .ok_or_else(|| ResourceManagerError::PathNotConfigured("NWN2 data directory".into()))?;
        let game_folder = paths.game_folder().cloned();
        drop(paths);

        let all_zip_groups: &[&[&str]] = &[
            BASE_GAME_ZIPS,
            TEMPLATE_ZIPS,
            SOUNDSET_ZIPS,
            SOUND_ZIPS,
            MODEL_ZIPS,
            MATERIAL_ZIPS,
            ACTOR_ZIPS,
            LOD_ZIPS,
        ];

        let mut zip_paths: Vec<PathBuf> = Vec::new();
        for group in all_zip_groups {
            for name in *group {
                let p = data_dir.join(name);
                if p.exists() {
                    zip_paths.push(p);
                }
            }
        }

        if let Some(ref gf) = game_folder {
            let vo_dir = gf.join("localization/english/data");
            for name in VO_ZIPS {
                let p = vo_dir.join(name);
                if p.exists() {
                    zip_paths.push(p);
                }
            }
        }

        // Cache mtime per zip path (avoids thousands of redundant stat calls)
        let zip_mtimes: HashMap<PathBuf, f64> = zip_paths
            .iter()
            .map(|p| {
                let mtime = std::fs::metadata(p)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0.0, |d| d.as_secs_f64());
                (p.clone(), mtime)
            })
            .collect();

        let entries = crate::utils::zip_scanner::scan_zips_parallel(&zip_paths)
            .map_err(ResourceManagerError::ZipError)?;

        let zip_count = zip_paths.len();
        let entry_count = entries.len();

        for entry in entries {
            let key = resource_key(&entry.stem, &entry.extension);
            let mtime = zip_mtimes.get(&entry.zip_path).copied().unwrap_or(0.0);

            let location = ResourceLocation::from_zip(
                OverrideSource::BaseGame,
                entry.zip_path,
                entry.internal_path,
                mtime,
            );

            self.resource_index
                .entry(key)
                .or_default()
                .push(location.clone());

            if entry.extension == "uti" {
                self.template_locations.insert(entry.stem, location);
            }
        }

        let tda_count = self
            .resource_index
            .iter()
            .filter(|(k, _)| k.ends_with(".2da"))
            .count();

        info!(
            "Indexed {} zips: {} resource keys, {} 2DAs, {} templates",
            zip_count,
            entry_count,
            tda_count,
            self.template_locations.len()
        );

        Ok(())
    }

    async fn scan_override_directories(&mut self) -> ResourceManagerResult<()> {
        let paths = self.paths.read().await;
        let override_dir = paths.override_dir();
        let custom_folders = paths.custom_override_folders().to_vec();
        drop(paths);

        if let Some(ref dir) = override_dir {
            let files = crate::utils::directory_scanner::scan_directory(dir, true);
            self.index_scanned_files(files, OverrideSource::OverrideDir);
        }

        for dir in &custom_folders {
            let files = crate::utils::directory_scanner::scan_directory(dir, true);
            self.index_scanned_files(files, OverrideSource::CustomOverride);
        }

        Ok(())
    }

    async fn scan_workshop_directories(&mut self) -> ResourceManagerResult<()> {
        let paths = self.paths.read().await;
        let workshop_dir = paths.steam_workshop_folder().cloned();
        drop(paths);

        let Some(workshop_dir) = workshop_dir else {
            return Ok(());
        };
        if !workshop_dir.exists() {
            return Ok(());
        }

        debug!(
            "Scanning Steam Workshop directory: {}",
            workshop_dir.display()
        );

        let files = crate::utils::directory_scanner::scan_workshop(&workshop_dir);
        self.index_scanned_files(files, OverrideSource::Workshop);

        let workshop_2da_count = self
            .resource_index
            .iter()
            .filter(|(k, locs)| {
                k.ends_with(".2da")
                    && locs
                        .iter()
                        .any(|l| matches!(l.source, OverrideSource::Workshop))
            })
            .count();
        let workshop_total = self
            .resource_index
            .values()
            .flat_map(|v| v.iter())
            .filter(|l| matches!(l.source, OverrideSource::Workshop))
            .count();

        info!(
            "Indexed {} workshop 2DAs, {} total workshop resources",
            workshop_2da_count, workshop_total
        );

        Ok(())
    }

    async fn scan_icon_directories(&mut self) -> ResourceManagerResult<()> {
        let paths = self.paths.read().await;
        let game_folder = paths.game_folder().cloned();
        drop(paths);

        let Some(game_folder) = game_folder else {
            return Ok(());
        };

        let upscaled = game_folder.join("ui").join("upscaled").join("icons");
        let icon_dir = if upscaled.exists() {
            upscaled
        } else {
            let default = game_folder.join("ui").join("default").join("icons");
            if default.exists() {
                default
            } else {
                return Ok(());
            }
        };

        for file in crate::utils::directory_scanner::scan_directory(&icon_dir, true) {
            if file.extension == "dds" || file.extension == "tga" || file.extension == "png" {
                self.icon_file_paths
                    .insert(file.stem.clone(), file.path.clone());

                let key = resource_key(&file.stem, &file.extension);
                let location =
                    ResourceLocation::from_file(OverrideSource::BaseGame, file.path, file.mtime);
                self.resource_index.entry(key).or_default().push(location);
            }
        }

        info!(
            "Indexed {} icon files from {}",
            self.icon_file_paths.len(),
            icon_dir.display()
        );
        Ok(())
    }

    fn index_scanned_files(
        &mut self,
        files: Vec<crate::utils::directory_scanner::ScannedFile>,
        source: OverrideSource,
    ) {
        for file in files {
            let key = resource_key(&file.stem, &file.extension);
            self.file_mod_tracker.track(file.path.clone(), file.mtime);

            let location = ResourceLocation::from_file(source.clone(), file.path, file.mtime);
            self.resource_index.entry(key).or_default().push(location);
        }
    }

    async fn load_base_tlk(&mut self) -> ResourceManagerResult<()> {
        let paths = self.paths.read().await;
        let base_tlk_path = paths.dialog_tlk();
        let documents_folder = paths.documents_folder().cloned();
        let workshop_folder = paths.steam_workshop_folder().cloned();
        drop(paths);

        let mut best_parser: Option<TLKParser> = None;
        let mut best_source = String::new();

        // Load base dialog.tlk
        if let Some(ref tlk_path) = base_tlk_path
            && tlk_path.exists()
        {
            match module_loader::load_tlk(tlk_path) {
                Ok(parser) => {
                    info!(
                        "Found base TLK: {} ({} entries)",
                        tlk_path.display(),
                        parser.string_count()
                    );
                    best_source = tlk_path.display().to_string();
                    best_parser = Some(parser);
                }
                Err(e) => warn!("Failed to parse base TLK: {}", e),
            }
        }

        // Mods like Kaedrin's PrC Pack (Nexus install) ship a merged dialog.tlk
        // into the user Documents folder. Engine-documented user TLK override.
        if let Some(ref docs_dir) = documents_folder {
            for name in ["dialog.tlk", "dialog.TLK"] {
                let candidate = docs_dir.join(name);
                if !candidate.exists() {
                    continue;
                }
                match module_loader::load_tlk(&candidate) {
                    Ok(parser) => {
                        let count = parser.string_count();
                        let best_count = best_parser.as_ref().map_or(0, TLKParser::string_count);
                        if count > best_count {
                            info!(
                                "Found Documents TLK with more entries: {} ({} > {})",
                                candidate.display(),
                                count,
                                best_count
                            );
                            best_source = candidate.display().to_string();
                            best_parser = Some(parser);
                        }
                        break;
                    }
                    Err(e) => debug!("Skipping Documents TLK {}: {}", candidate.display(), e),
                }
            }
        }

        // Mods like Kaedrin's PrC Pack (Workshop install) ship a merged dialog.tlk
        // inside their workshop item folder. Same "highest string count wins" heuristic.
        if let Some(ref workshop_dir) = workshop_folder
            && let Ok(entries) = std::fs::read_dir(workshop_dir)
        {
            for entry in entries.flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let tlk_path = entry.path().join("dialog.TLK");
                if !tlk_path.exists() {
                    // Also check lowercase
                    let tlk_lower = entry.path().join("dialog.tlk");
                    if !tlk_lower.exists() {
                        continue;
                    }
                }
                let actual_path = if entry.path().join("dialog.TLK").exists() {
                    entry.path().join("dialog.TLK")
                } else {
                    entry.path().join("dialog.tlk")
                };

                match module_loader::load_tlk(&actual_path) {
                    Ok(parser) => {
                        let count = parser.string_count();
                        let best_count = best_parser.as_ref().map_or(0, TLKParser::string_count);
                        if count > best_count {
                            info!(
                                "Found Workshop TLK with more entries: {} ({} > {})",
                                actual_path.display(),
                                count,
                                best_count
                            );
                            best_source = actual_path.display().to_string();
                            best_parser = Some(parser);
                        }
                    }
                    Err(e) => debug!("Skipping Workshop TLK {}: {}", actual_path.display(), e),
                }
            }
        }

        if let Some(parser) = best_parser {
            info!(
                "Using TLK: {} ({} entries)",
                best_source,
                parser.string_count()
            );
            self.tlk_cache = Some(Arc::new(StdRwLock::new(parser)));
        }

        Ok(())
    }

    async fn cache_data_zip_paths(&mut self) {
        let paths = self.paths.read().await;
        if let Some(data_dir) = paths.data()
            && let Ok(entries) = std::fs::read_dir(data_dir)
        {
            self.data_zip_paths = entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")))
                .collect();
        }
        debug!("Cached {} data zip paths", self.data_zip_paths.len());
    }

    pub fn get_2da(&self, name: &str) -> ResourceManagerResult<Arc<TDAParser>> {
        let name_lower = name.to_lowercase().replace(".2da", "");
        let cache_key = (name_lower.clone(), OverrideSource::BaseGame);

        if let Some(parser) = self.tda_cache.get(&cache_key) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Arc::clone(&parser));
        }

        let resource_key = format!("{name_lower}.2da");
        if let Some(locations) = self.resource_index.get(&resource_key)
            && let Some(location) = locations
                .iter()
                .rfind(|l| matches!(l.source, OverrideSource::BaseGame))
        {
            let parser = self.load_2da_from_location(location.clone())?;
            self.tda_cache.insert(cache_key, Arc::clone(&parser));
            return Ok(parser);
        }

        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        Err(ResourceManagerError::TdaNotFound { name: name.into() })
    }

    pub fn get_2da_with_overrides(&self, name: &str) -> ResourceManagerResult<Arc<TDAParser>> {
        let name_lower = name.to_lowercase().replace(".2da", "");
        let resource_key = format!("{name_lower}.2da");

        // Pick the highest-priority non-base-game location
        if let Some(locations) = self.resource_index.get(&resource_key)
            && let Some(location) = locations
                .iter()
                .filter(|l| !matches!(l.source, OverrideSource::BaseGame))
                .max_by_key(|l| l.source.priority())
        {
            let cache_key = (name_lower.clone(), location.source.clone());
            if let Some(parser) = self.tda_cache.get(&cache_key) {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(Arc::clone(&parser));
            }
            let parser = self.load_2da_from_location(location.clone())?;
            self.tda_cache.insert(cache_key, Arc::clone(&parser));
            return Ok(parser);
        }

        // HAK overrides (pre-parsed from ERF, not in resource_index)
        for hak_cache in &self.hak_overrides {
            if let Some(parser) = hak_cache.get(&name_lower) {
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Ok(Arc::clone(parser));
            }
        }

        // Module overrides (pre-parsed from .mod, not in resource_index)
        if let Some(parser) = self.module_overrides.get(&name_lower) {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(Arc::clone(&parser));
        }

        // Base game fallback
        self.get_2da(&name_lower)
    }

    fn load_2da_from_location(
        &self,
        location: ResourceLocation,
    ) -> ResourceManagerResult<Arc<TDAParser>> {
        match location.container_type {
            ContainerType::Zip => {
                let internal_path = location
                    .internal_path
                    .ok_or_else(|| ResourceManagerError::Parse("Missing internal path".into()))?;

                let data = self
                    .zip_reader
                    .lock()
                    .read_file_from_zip(
                        location.container_path.to_string_lossy().to_string(),
                        internal_path.clone(),
                    )
                    .map_err(|e| ResourceManagerError::ZipError(e.clone()))?;

                let mut parser = TDAParser::new();
                parser.parse_from_bytes(&data).map_err(|e| {
                    ResourceManagerError::InvalidTdaFormat(format!(
                        "Failed to parse {internal_path}: {e}"
                    ))
                })?;

                Ok(Arc::new(parser))
            }
            ContainerType::Erf => {
                let internal_path = location
                    .internal_path
                    .ok_or_else(|| ResourceManagerError::Parse("Missing internal path".into()))?;

                let mut erf = ErfParser::new();
                erf.read(&location.container_path).map_err(|e| {
                    ResourceManagerError::InvalidErfFormat(format!(
                        "Failed to open {}: {}",
                        location.container_path.display(),
                        e
                    ))
                })?;

                let resource = erf.resources.get(&internal_path).ok_or_else(|| {
                    ResourceManagerError::ExtractionFailed {
                        resource: internal_path.clone(),
                        container: location.container_path.display().to_string(),
                    }
                })?;

                let data = resource.data.as_ref().ok_or_else(|| {
                    ResourceManagerError::ExtractionFailed {
                        resource: internal_path.clone(),
                        container: location.container_path.display().to_string(),
                    }
                })?;

                let mut parser = TDAParser::new();
                parser.parse_from_bytes(data).map_err(|e| {
                    ResourceManagerError::InvalidTdaFormat(format!(
                        "Failed to parse {internal_path}: {e}"
                    ))
                })?;

                Ok(Arc::new(parser))
            }
            ContainerType::Directory => {
                let parser = self.parse_2da_file(&location.container_path)?;
                Ok(Arc::new(parser))
            }
        }
    }

    fn parse_2da_file(&self, path: &Path) -> ResourceManagerResult<TDAParser> {
        let mut parser = TDAParser::new();
        parser.parse_from_file(path).map_err(|e| {
            ResourceManagerError::InvalidTdaFormat(format!(
                "Failed to parse {}: {}",
                path.display(),
                e
            ))
        })?;
        Ok(parser)
    }

    pub fn get_string(&self, str_ref: i32) -> String {
        if str_ref < 0 {
            return String::new();
        }

        if let Some(custom_tlk) = &self.custom_tlk_cache
            && let Ok(mut tlk) = custom_tlk.write()
            && let Ok(Some(s)) = tlk.get_string(str_ref as usize)
            && !s.is_empty()
        {
            return s;
        }

        if let Some(base_tlk) = &self.tlk_cache
            && let Ok(mut tlk) = base_tlk.write()
            && let Ok(Some(s)) = tlk.get_string(str_ref as usize)
        {
            return s;
        }

        format!("<StrRef:{str_ref}>")
    }

    pub fn get_strings_batch(&self, str_refs: &[i32]) -> HashMap<i32, String> {
        let mut results = HashMap::with_capacity(str_refs.len());

        for &str_ref in str_refs {
            results.insert(str_ref, self.get_string(str_ref));
        }

        results
    }

    pub async fn set_module(&mut self, module_path: &Path) -> ResourceManagerResult<bool> {
        let module_key = module_path.display().to_string();

        if self.module_cache.contains(&module_key) {
            let cached = self.module_cache.get(&module_key).unwrap().clone();
            self.restore_from_cache(cached);
            info!("Restored module from cache: {}", module_key);
            return Ok(true);
        }

        let module_info = module_loader::extract_module_info(module_path)?;
        let module_2das = module_loader::load_module_2das(module_path, module_info.is_directory)?;

        self.module_info = Some(module_info.clone());
        self.module_path = Some(module_path.to_path_buf());
        self.module_overrides.clear();
        for (k, v) in module_2das {
            self.module_overrides.insert(k, v);
        }
        self.current_module = Some(module_key.clone());

        self.hak_overrides.clear();

        let paths = self.paths.read().await;
        let custom_hak_folders = paths.custom_hak_folders().to_vec();
        let user_hak = paths.hak_dir();
        let install_hak = paths.hak_dir();
        drop(paths);

        for hak_name in &module_info.hak_list {
            if let Some(hak_path) = module_loader::find_hak_path(
                hak_name,
                &custom_hak_folders,
                user_hak.as_ref(),
                install_hak.as_ref(),
            ) {
                match module_loader::load_hak_2das(&hak_path) {
                    Ok(hak_2das) => {
                        self.hak_overrides.push(hak_2das);

                        if let Some(tlk_path) = module_loader::check_hak_for_tlk(&hak_path)
                            && let Ok(tlk) = module_loader::load_tlk(&tlk_path)
                        {
                            self.custom_tlk_cache = Some(Arc::new(StdRwLock::new(tlk)));
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load HAK {}: {}", hak_name, e);
                    }
                }
            } else {
                warn!("HAK not found: {}", hak_name);
            }
        }

        if !module_info.custom_tlk.is_empty() && self.custom_tlk_cache.is_none() {
            self.load_custom_tlk(&module_info.custom_tlk).await;
        }

        if let Some(campaign_guid) = &module_info.campaign_id {
            self.set_campaign_by_guid(campaign_guid).await?;
        }

        let cached_state = CachedModuleState {
            module_info: module_info.clone(),
            hak_overrides: self.hak_overrides.clone(),
            module_overrides: self.module_overrides.clone().into_iter().collect(),
            campaign_overrides: self
                .tda_cache
                .iter()
                .filter(|entry| matches!(entry.key().1, OverrideSource::Campaign))
                .map(|entry| (entry.key().0.clone(), entry.value().clone()))
                .collect(),
            custom_tlk_path: None,
            last_accessed: 0,
        };
        self.module_cache.put(module_key, cached_state);

        info!(
            "Loaded module: {} with {} HAKs",
            module_info.name,
            module_info.hak_list.len()
        );
        Ok(true)
    }

    fn restore_from_cache(&mut self, cached: CachedModuleState) {
        self.module_info = Some(cached.module_info.clone());
        self.module_path = Some(cached.module_info.path.clone());
        self.hak_overrides = cached.hak_overrides;
        self.module_overrides.clear();
        for (k, v) in cached.module_overrides {
            self.module_overrides.insert(k, v);
        }
        for (k, v) in cached.campaign_overrides {
            self.tda_cache.insert((k, OverrideSource::Campaign), v);
        }
    }

    async fn load_custom_tlk(&mut self, tlk_name: &str) {
        let paths = self.paths.read().await;

        let tlk_filename = if tlk_name.to_lowercase().ends_with(".tlk") {
            tlk_name.to_string()
        } else {
            format!("{tlk_name}.tlk")
        };

        if let Some(tlk_dir) = paths.tlk_dir() {
            let tlk_path = tlk_dir.join(&tlk_filename);
            if tlk_path.exists()
                && let Ok(tlk) = module_loader::load_tlk(&tlk_path)
            {
                self.custom_tlk_cache = Some(Arc::new(StdRwLock::new(tlk)));
                return;
            }
        }

        if let Some(data_dir) = paths.data() {
            let tlk_path = data_dir.join(&tlk_filename);
            if tlk_path.exists()
                && let Ok(tlk) = module_loader::load_tlk(&tlk_path)
            {
                self.custom_tlk_cache = Some(Arc::new(StdRwLock::new(tlk)));
            }
        }
    }

    pub async fn set_campaign_by_guid(&mut self, guid: &str) -> ResourceManagerResult<bool> {
        let paths = self.paths.read().await;
        let install_campaigns = paths.campaigns();
        let user_campaigns = paths.user_campaigns();
        drop(paths);

        if let Some(campaign_folder) = module_loader::find_campaign_by_guid(
            guid,
            install_campaigns.as_ref(),
            user_campaigns.as_ref(),
        ) {
            self.current_campaign_folder = Some(campaign_folder.clone());
            self.current_campaign_id = Some(guid.to_string());

            // Remove old campaign entries from resource_index
            for locs in self.resource_index.values_mut() {
                locs.retain(|l| !matches!(l.source, OverrideSource::Campaign));
            }

            // Clear campaign entries from tda_cache
            self.tda_cache
                .retain(|k, _| !matches!(k.1, OverrideSource::Campaign));

            let files = crate::utils::directory_scanner::scan_directory(&campaign_folder, true);
            self.index_scanned_files(files, OverrideSource::Campaign);

            let campaign_2da_count = self
                .resource_index
                .iter()
                .filter(|(k, locs)| {
                    k.ends_with(".2da")
                        && locs
                            .iter()
                            .any(|l| matches!(l.source, OverrideSource::Campaign))
                })
                .count();

            info!(
                "Set campaign: {} with {} 2DA overrides",
                guid, campaign_2da_count
            );
            return Ok(true);
        }

        warn!("Campaign not found: {}", guid);
        Ok(false)
    }

    pub async fn load_haks_for_save(
        &mut self,
        hak_list: &[String],
        custom_tlk: &str,
        campaign_guid: &str,
    ) -> ResourceManagerResult<bool> {
        self.clear_override_caches();

        let paths = self.paths.read().await;
        let custom_hak_folders = paths.custom_hak_folders().to_vec();
        let user_hak = paths.hak_dir();
        let install_hak = paths.hak_dir();
        drop(paths);

        for hak_name in hak_list {
            if let Some(hak_path) = module_loader::find_hak_path(
                hak_name,
                &custom_hak_folders,
                user_hak.as_ref(),
                install_hak.as_ref(),
            ) {
                match module_loader::load_hak_2das(&hak_path) {
                    Ok(hak_2das) => {
                        self.hak_overrides.push(hak_2das);

                        if let Some(tlk_path) = module_loader::check_hak_for_tlk(&hak_path)
                            && let Ok(tlk) = module_loader::load_tlk(&tlk_path)
                        {
                            self.custom_tlk_cache = Some(Arc::new(StdRwLock::new(tlk)));
                        }
                    }
                    Err(e) => {
                        warn!("Failed to load HAK {}: {}", hak_name, e);
                    }
                }
            }
        }

        if !custom_tlk.is_empty() && self.custom_tlk_cache.is_none() {
            self.load_custom_tlk(custom_tlk).await;
        }

        if !campaign_guid.is_empty() {
            self.set_campaign_by_guid(campaign_guid).await?;
        }

        Ok(true)
    }

    pub fn add_custom_override_directory(&mut self, path: &Path) -> ResourceManagerResult<bool> {
        if !path.exists() || !path.is_dir() {
            return Err(ResourceManagerError::FileNotFound(path.to_path_buf()));
        }

        let files = crate::utils::directory_scanner::scan_directory(path, true);
        self.index_scanned_files(files, OverrideSource::CustomOverride);

        self.tda_cache
            .retain(|k, _| !matches!(k.1, OverrideSource::CustomOverride));

        let custom_2da_count = self
            .resource_index
            .iter()
            .filter(|(k, locs)| {
                k.ends_with(".2da")
                    && locs
                        .iter()
                        .any(|l| matches!(l.source, OverrideSource::CustomOverride))
            })
            .count();

        info!(
            "Added custom override directory: {} ({} total custom 2DAs)",
            path.display(),
            custom_2da_count
        );
        Ok(true)
    }

    pub fn remove_custom_override_directory(&mut self, path: &Path) -> bool {
        let path_str = path.display().to_string();
        let mut removed = 0usize;

        for locs in self.resource_index.values_mut() {
            let before = locs.len();
            locs.retain(|l| {
                !(matches!(l.source, OverrideSource::CustomOverride)
                    && l.container_path.starts_with(path))
            });
            removed += before - locs.len();
        }

        self.resource_index.retain(|_, v| !v.is_empty());

        self.tda_cache
            .retain(|k, _| !matches!(k.1, OverrideSource::CustomOverride));

        info!(
            "Removed custom override directory: {} ({} resources removed)",
            path_str, removed
        );
        removed > 0
    }

    pub fn clear_override_caches(&mut self) {
        self.hak_overrides.clear();
        self.module_overrides.clear();
        self.tda_cache.clear();
        self.custom_tlk_cache = None;
        self.current_module = None;
        self.module_path = None;
        self.module_info = None;
        self.current_campaign_id = None;
        self.current_campaign_folder = None;
    }

    pub fn check_for_modifications(&mut self) -> Vec<PathBuf> {
        let modified = self.file_mod_tracker.check_all();

        for path in &modified {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                let name_lower = name.to_lowercase();
                self.tda_cache.retain(|k, _| k.0 != name_lower);
            }
            self.file_mod_tracker.update(path);
        }

        modified
    }

    pub fn get_item_template(&self, resref: &str) -> Option<TemplateInfo> {
        let location = self.template_locations.get(resref)?;
        Some(TemplateInfo {
            resref: resref.to_string(),
            container_type: location.container_type.clone(),
            container_path: location.container_path.clone(),
            internal_path: location.internal_path.clone(),
            source: location.source.clone(),
        })
    }

    pub fn get_all_item_templates(&self) -> HashMap<String, TemplateInfo> {
        let mut templates = HashMap::new();

        for (resref, location) in &self.template_locations {
            templates.insert(
                resref.clone(),
                TemplateInfo {
                    resref: resref.clone(),
                    container_type: location.container_type.clone(),
                    container_path: location.container_path.clone(),
                    internal_path: location.internal_path.clone(),
                    source: location.source.clone(),
                },
            );
        }

        templates
    }

    pub fn get_item_template_data(
        &self,
        info: &TemplateInfo,
    ) -> ResourceManagerResult<IndexMap<String, serde_json::Value>> {
        let fields = self.get_item_template_fields(info)?;

        let mut result = IndexMap::new();
        for (key, value) in fields {
            result.insert(key, gff_value_to_json(&value));
        }

        Ok(result)
    }

    pub fn read_zip_file(
        &self,
        zip_path: &std::path::Path,
        internal_path: &str,
    ) -> ResourceManagerResult<Vec<u8>> {
        self.zip_reader
            .lock()
            .read_file_from_zip(
                zip_path.to_string_lossy().to_string(),
                internal_path.to_string(),
            )
            .map_err(|e| ResourceManagerError::ZipError(e.clone()))
    }

    pub fn get_item_template_fields(
        &self,
        info: &TemplateInfo,
    ) -> ResourceManagerResult<IndexMap<String, crate::parsers::gff::GffValue<'static>>> {
        let data =
            match &info.container_type {
                ContainerType::Zip => {
                    let internal_path = info.internal_path.as_ref().ok_or_else(|| {
                        ResourceManagerError::Parse("Missing internal path".into())
                    })?;

                    self.zip_reader
                        .lock()
                        .read_file_from_zip(
                            info.container_path.to_string_lossy().to_string(),
                            internal_path.clone(),
                        )
                        .map_err(|e| ResourceManagerError::ZipError(e.clone()))?
                }
                ContainerType::Erf => {
                    let internal_path = info.internal_path.as_ref().ok_or_else(|| {
                        ResourceManagerError::Parse("Missing internal path".into())
                    })?;

                    let mut erf = ErfParser::new();
                    erf.read(&info.container_path).map_err(|e| {
                        ResourceManagerError::InvalidErfFormat(format!(
                            "Failed to open {}: {}",
                            info.container_path.display(),
                            e
                        ))
                    })?;

                    erf.resources
                        .get(internal_path)
                        .ok_or_else(|| ResourceManagerError::ExtractionFailed {
                            resource: internal_path.clone(),
                            container: info.container_path.display().to_string(),
                        })?
                        .data
                        .clone()
                        .ok_or_else(|| ResourceManagerError::ExtractionFailed {
                            resource: internal_path.clone(),
                            container: info.container_path.display().to_string(),
                        })?
                }
                ContainerType::Directory => std::fs::read(&info.container_path)?,
            };

        let gff = GffParser::from_bytes(data).map_err(|e| {
            ResourceManagerError::InvalidGffFormat(format!("Failed to parse template: {e}"))
        })?;

        let fields = gff.read_struct_fields(0).map_err(|e| {
            ResourceManagerError::InvalidGffFormat(format!("Failed to read template fields: {e}"))
        })?;

        let mut owned_fields = IndexMap::new();
        for (k, v) in fields {
            owned_fields.insert(k, v.force_owned());
        }

        Ok(owned_fields)
    }

    pub fn get_item_template_summary(
        &self,
        info: &TemplateInfo,
    ) -> ResourceManagerResult<(Option<String>, i32)> {
        let data =
            match &info.container_type {
                ContainerType::Zip => {
                    let internal_path = info.internal_path.as_ref().ok_or_else(|| {
                        ResourceManagerError::Parse("Missing internal path".into())
                    })?;

                    self.zip_reader
                        .lock()
                        .read_file_from_zip(
                            info.container_path.to_string_lossy().to_string(),
                            internal_path.clone(),
                        )
                        .map_err(|e| ResourceManagerError::ZipError(e.clone()))?
                }
                ContainerType::Erf => {
                    let internal_path = info.internal_path.as_ref().ok_or_else(|| {
                        ResourceManagerError::Parse("Missing internal path".into())
                    })?;

                    let mut erf = ErfParser::new();
                    erf.read(&info.container_path).map_err(|e| {
                        ResourceManagerError::InvalidErfFormat(format!(
                            "Failed to open {}: {}",
                            info.container_path.display(),
                            e
                        ))
                    })?;

                    erf.resources
                        .get(internal_path)
                        .ok_or_else(|| ResourceManagerError::ExtractionFailed {
                            resource: internal_path.clone(),
                            container: info.container_path.display().to_string(),
                        })?
                        .data
                        .clone()
                        .ok_or_else(|| ResourceManagerError::ExtractionFailed {
                            resource: internal_path.clone(),
                            container: info.container_path.display().to_string(),
                        })?
                }
                ContainerType::Directory => std::fs::read(&info.container_path)?,
            };

        let gff = GffParser::from_bytes(data).map_err(|e| {
            ResourceManagerError::InvalidGffFormat(format!("Failed to parse template: {e}"))
        })?;

        let base_item = gff
            .read_field_by_label(0, "BaseItem")
            .ok()
            .and_then(|v| match v {
                crate::parsers::gff::GffValue::Int(i) => Some(i),
                crate::parsers::gff::GffValue::Short(s) => Some(i32::from(s)),
                crate::parsers::gff::GffValue::Byte(b) => Some(i32::from(b)),
                crate::parsers::gff::GffValue::Word(w) => Some(i32::from(w)),
                crate::parsers::gff::GffValue::Dword(d) => Some(d as i32),
                _ => None,
            })
            .unwrap_or(0);

        let name = gff
            .read_field_by_label(0, "LocalizedName")
            .ok()
            .and_then(|v| {
                if let crate::parsers::gff::GffValue::LocString(ls) = v {
                    if !ls.substrings.is_empty() {
                        Some(ls.substrings[0].string.to_string())
                    } else if ls.string_ref >= 0 {
                        Some(self.get_string(ls.string_ref))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .filter(|s| !s.is_empty());

        Ok((name, base_item))
    }

    pub fn get_cache_stats(&self) -> CacheStats {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let tda_count = self
            .resource_index
            .keys()
            .filter(|k| k.ends_with(".2da"))
            .count();

        CacheStats {
            size: tda_count,
            max_size: 0,
            hits,
            misses,
            hit_ratio: if hits + misses > 0 {
                hits as f64 / (hits + misses) as f64
            } else {
                0.0
            },
        }
    }

    pub fn get_module_cache_stats(&self) -> CacheStats {
        self.module_cache.get_stats()
    }

    pub fn get_zip_cache_stats(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.zip_reader.lock().get_stats()
    }

    pub fn get_module_info(&self) -> Option<&ModuleInfo> {
        self.module_info.as_ref()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn resource_count(&self) -> usize {
        self.resource_index.len()
    }

    pub fn template_count(&self) -> usize {
        self.template_locations.len()
    }

    pub fn data_zip_paths(&self) -> &[PathBuf] {
        &self.data_zip_paths
    }

    pub fn resource_source_counts(&self) -> std::collections::HashMap<String, usize> {
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for locations in self.resource_index.values() {
            for loc in locations {
                let key = match loc.source {
                    OverrideSource::BaseGame => "base_game",
                    OverrideSource::Expansion => "expansion",
                    OverrideSource::Module => "module",
                    OverrideSource::Campaign => "campaign",
                    OverrideSource::OverrideDir => "override_dir",
                    OverrideSource::Workshop => "workshop",
                    OverrideSource::CustomOverride => "custom_override",
                    OverrideSource::Hak(_) => "hak",
                };
                *counts.entry(key.to_string()).or_default() += 1;
            }
        }
        counts
    }

    pub fn get_available_2da_files(&self) -> Vec<String> {
        self.resource_index
            .keys()
            .filter(|k| k.ends_with(".2da"))
            .map(|k| k.trim_end_matches(".2da").to_string())
            .collect()
    }

    pub fn get_tlk_parser(&self) -> Option<Arc<StdRwLock<TLKParser>>> {
        self.tlk_cache.clone()
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

        if let Some(locations) = self.resource_index.get(&key)
            && let Some(location) = locations.iter().max_by_key(|l| l.source.priority())
        {
            trace!(
                "ResourceManager: Found resource '{}' in source: {:?}",
                key, location.source
            );
            return match &location.container_type {
                ContainerType::Directory => Ok(std::fs::read(&location.container_path)?),
                ContainerType::Zip => {
                    let internal = location.internal_path.as_ref().ok_or_else(|| {
                        ResourceManagerError::Parse("Missing internal path".into())
                    })?;
                    self.zip_reader
                        .lock()
                        .read_file_from_zip(
                            location.container_path.to_string_lossy().to_string(),
                            internal.clone(),
                        )
                        .map_err(ResourceManagerError::ZipError)
                }
                ContainerType::Erf => {
                    let internal = location.internal_path.as_ref().ok_or_else(|| {
                        ResourceManagerError::Parse("Missing internal path".into())
                    })?;
                    let mut erf = ErfParser::new();
                    erf.read(&location.container_path)
                        .map_err(|e| ResourceManagerError::InvalidErfFormat(format!("{e}")))?;
                    erf.resources
                        .get(internal)
                        .and_then(|r| r.data.clone())
                        .ok_or_else(|| ResourceManagerError::ExtractionFailed {
                            resource: internal.clone(),
                            container: location.container_path.display().to_string(),
                        })
                }
            };
        }

        // Fallback: search unindexed zips (should rarely fire)
        let mut zip_reader = self.zip_reader.lock();
        for path in &self.data_zip_paths {
            if let Ok(Some(bytes)) = zip_reader.find_file_by_name(&path.to_string_lossy(), &key) {
                debug!(
                    "Resource '{}' found via fallback zip scan (not in index)",
                    key
                );
                return Ok(bytes);
            }
        }

        Err(ResourceManagerError::FileNotFound(PathBuf::from(format!(
            "{resref}.{extension}"
        ))))
    }

    pub fn list_resources_by_extension(&self, extension: &str) -> Vec<(String, String)> {
        let ext_suffix = format!(".{}", extension.to_lowercase());
        let mut results: Vec<(String, String)> = self
            .resource_index
            .iter()
            .filter(|(key, _)| key.ends_with(&ext_suffix))
            .map(|(key, locs)| {
                let loc = locs.iter().max_by_key(|l| l.source.priority()).unwrap();
                let source = loc
                    .container_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                (key.clone(), source)
            })
            .collect();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results
    }

    pub fn list_resources_by_prefix(&self, prefix: &str, extension: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        let ext_lower = extension.to_lowercase();
        let ext_suffix = format!(".{ext_lower}");

        debug!(
            "ResourceManager: list_resources_by_prefix searching index for prefix: '{}' ext: '{}'",
            prefix, extension
        );

        let mut results: Vec<String> = self
            .resource_index
            .keys()
            .filter_map(|key| {
                if !key.ends_with(&ext_suffix) {
                    return None;
                }
                let stem = key.trim_end_matches(&ext_suffix);
                stem.starts_with(&prefix_lower).then(|| key.clone())
            })
            .collect();

        results.sort();
        results.dedup();
        results
    }
}

fn resource_key(stem: &str, extension: &str) -> String {
    format!("{stem}.{extension}")
}

fn gff_value_to_json(value: &crate::parsers::gff::GffValue<'_>) -> serde_json::Value {
    use crate::parsers::gff::GffValue;

    match value {
        GffValue::Byte(v) => serde_json::json!(*v),
        GffValue::Char(v) => serde_json::json!(*v as u8),
        GffValue::Word(v) => serde_json::json!(*v),
        GffValue::Short(v) => serde_json::json!(*v),
        GffValue::Dword(v) => serde_json::json!(*v),
        GffValue::Int(v) => serde_json::json!(*v),
        GffValue::Dword64(v) => serde_json::json!(*v),
        GffValue::Int64(v) => serde_json::json!(*v),
        GffValue::Float(v) => serde_json::json!(*v),
        GffValue::Double(v) => serde_json::json!(*v),
        GffValue::String(s) => serde_json::json!(s.as_ref()),
        GffValue::ResRef(s) => serde_json::json!(s.as_ref()),
        GffValue::LocString(ls) => {
            let mut obj = serde_json::Map::new();
            obj.insert("string_ref".to_string(), serde_json::json!(ls.string_ref));
            let substrings: Vec<serde_json::Value> = ls
                .substrings
                .iter()
                .map(|sub| {
                    serde_json::json!({
                        "string": sub.string.as_ref(),
                        "language": sub.language,
                        "gender": sub.gender
                    })
                })
                .collect();
            obj.insert(
                "substrings".to_string(),
                serde_json::Value::Array(substrings),
            );
            serde_json::Value::Object(obj)
        }
        GffValue::Void(data) => serde_json::json!(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            data.as_ref()
        )),
        GffValue::Struct(lazy) => {
            let fields = lazy.force_load();
            let obj: serde_json::Map<String, serde_json::Value> = fields
                .iter()
                .map(|(k, v)| (k.clone(), gff_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        GffValue::List(items) => {
            let arr: Vec<serde_json::Value> = items
                .iter()
                .map(|lazy| {
                    let fields = lazy.force_load();
                    let obj: serde_json::Map<String, serde_json::Value> = fields
                        .iter()
                        .map(|(k, v)| (k.clone(), gff_value_to_json(v)))
                        .collect();
                    serde_json::Value::Object(obj)
                })
                .collect();
            serde_json::Value::Array(arr)
        }
        GffValue::StructOwned(fields) => {
            let obj: serde_json::Map<String, serde_json::Value> = fields
                .iter()
                .map(|(k, v)| (k.clone(), gff_value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
        GffValue::ListOwned(items) => {
            let arr: Vec<serde_json::Value> = items
                .iter()
                .map(|fields| {
                    let obj: serde_json::Map<String, serde_json::Value> = fields
                        .iter()
                        .map(|(k, v)| (k.clone(), gff_value_to_json(v)))
                        .collect();
                    serde_json::Value::Object(obj)
                })
                .collect();
            serde_json::Value::Array(arr)
        }
        GffValue::StructRef(idx) => serde_json::json!({ "struct_ref": idx }),
        GffValue::ListRef(indices) => serde_json::json!({ "list_ref": indices }),
    }
}
