pub mod campaign;
pub mod class_categorizer;
pub mod item_cost_calculator;
pub mod item_property_decoder;
pub mod load_diagnostics;
pub mod model_loader;
pub mod playerinfo;
pub mod resource_manager;
pub mod roster;
pub mod rule_detector;
pub mod save_graph;
pub mod savegame_handler;
pub mod texture_decode;
pub mod toolset_bridge;

pub use resource_manager::{
    CacheStats, CampaignInfo, ContainerType, ModuleInfo, OverrideSource, ResourceLocation,
    ResourceManager, ResourceManagerError, ResourceManagerResult, TemplateInfo,
};

pub use rule_detector::{ColumnPurpose, RuleDetector, TableType};

pub use savegame_handler::{
    BackupInfo, CharacterStats, CharacterSummary, CleanupResult, FileInfo, RestoreResult,
    SaveGameError, SaveGameHandler, SaveGameResult,
};

pub use playerinfo::{
    PlayerClassEntry, PlayerInfo, PlayerInfoData, PlayerInfoParseError, PlayerInfoResult,
};

pub use item_property_decoder::{
    DecodedProperty, ItemBonuses, ItemPropertyDecoder, ItemPropertyError, ItemPropertyResult,
    PropertyDefinition, PropertyMetadata,
};

pub use class_categorizer::{
    Categories, CategorizedClasses, ClassFocus, ClassInfo, ClassType, FocusInfo,
    get_categorized_classes,
};

pub use item_cost_calculator::ItemCostCalculator;
