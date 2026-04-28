use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tracing::{debug, info, instrument};

use crate::commands::models::ModelEntry;
use crate::config::{AppConfig, NWN2Paths};
use crate::loaders::GameData;
use crate::services::resource_manager::ResourceManager;
use crate::services::save_graph::QuestGraphProgress;
use crate::services::toolset_bridge::BridgeClient;
use crate::state::session_state::SessionState;

#[derive(Clone, serde::Serialize)]
pub struct InitStatus {
    pub step: String,
    pub progress: f32,
    pub message: String,
}

impl Default for InitStatus {
    fn default() -> Self {
        Self {
            step: "idle".to_string(),
            progress: 0.0,
            message: "Waiting to start...".to_string(),
        }
    }
}

pub struct AppState {
    pub paths: RwLock<NWN2Paths>,
    pub config: RwLock<AppConfig>,
    pub resource_manager: Arc<tokio::sync::RwLock<ResourceManager>>,
    pub game_data: Arc<RwLock<GameData>>,
    pub session: Arc<RwLock<SessionState>>,
    pub init_status: Arc<RwLock<InitStatus>>,
    pub quest_graph_progress: Arc<RwLock<QuestGraphProgress>>,
    pub model_list_cache: Mutex<Option<Vec<ModelEntry>>>,
    /// Shared long-lived toolset-bridge client. Spawned lazily on first use so
    /// startup doesn't pay for it unless/until a Quests-tab or toolset command
    /// fires; kept across commands to amortize ~1s of bridge init.
    pub bridge_client: Mutex<Option<Arc<BridgeClient>>>,
}

impl AppState {
    #[instrument(name = "AppState::new", skip_all)]
    pub fn new() -> Self {
        info!("Creating AppState");

        debug!("Initializing NWN2Paths");
        let paths = NWN2Paths::new();
        debug!("NWN2Paths initialized");

        debug!("Loading AppConfig");
        let config = AppConfig::load();
        debug!("AppConfig loaded");

        debug!("Creating ResourceManager");
        let paths_arc = Arc::new(tokio::sync::RwLock::new(paths.clone()));
        let resource_manager = Arc::new(tokio::sync::RwLock::new(ResourceManager::new(paths_arc)));
        debug!("ResourceManager created");

        debug!("Initializing TLK parser");
        let tlk = Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::default(),
        ));
        debug!("TLK parser initialized");

        debug!("Creating GameData");
        let game_data = Arc::new(RwLock::new(GameData::new(tlk)));
        info!("GameData initialized (empty - will load on initialize_game_data)");

        debug!("Creating SessionState");
        let session = SessionState::new(Arc::clone(&resource_manager));
        debug!("SessionState created");

        info!("AppState created successfully");

        Self {
            paths: RwLock::new(paths),
            config: RwLock::new(config),
            resource_manager,
            game_data,
            session: Arc::new(RwLock::new(session)),
            init_status: Arc::new(RwLock::new(InitStatus::default())),
            quest_graph_progress: Arc::new(RwLock::new(QuestGraphProgress::default())),
            model_list_cache: Mutex::new(None),
            bridge_client: Mutex::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
