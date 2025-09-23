"""
FastAPI Pydantic models organized by manager
Each model file corresponds to a specific character manager
"""

# Shared/base models used across all managers
from .shared_models import (
    # Base responses
    BaseResponse,
    ErrorResponse,
    ValidationError,
    ValidationResponse,
    
    # System
    HealthResponse,
    SystemInfo,
    CacheStatus,
    ConfigResponse,
    # CacheRebuildResponse,
    # ConfigUpdateRequest,
    # ConfigUpdateResponse,
    NWN2PathResponse,
    AutoDiscoverResponse,
    # BackgroundLoadingTriggerResponse,
    # BackgroundLoadingStatusResponse,
    # InitializationStatusResponse,
    
    # Session
    SessionInfo,
    SessionStatus,
    ActiveSessionsList,
    
    # Change tracking
    ChangeRecord,
    CascadingEffect,
    ChangeResult,
    
    # Pagination
    PaginationParams,
    PaginationInfo,
    PaginatedResponse,
    
    # Files/Resources
    FileInfo,
    ResourceInfo,
    
    # Custom content
    CustomContentItem,
    CustomContentSummary,
    
    # GFF/Data
    GFFFieldInfo,
    RawDataRequest,
    RawDataResponse,
    FieldStructureResponse,
    RawFieldUpdateRequest,
    RawFieldUpdateResponse,
    
    # Batch operations
    BatchOperation,
    BatchOperationResult,
    BatchRequest,
    BatchResponse,
    
    # Export/Import
    ExportOptions,
    ImportOptions,
    
    # Alignment
    AlignmentResponse,
    AlignmentUpdateRequest,
    AlignmentShiftRequest,
    AlignmentShiftResponse
)

# Ability/Attribute models
from .ability_models import (
    AbilityScore,
    AbilityScores,
    AbilityModifiers,
    DetailedModifiers,
    AttributeDependencies,
    EncumbranceLimits,
    CharacterBiography,
    AttributeState,
    AttributeChangeRequest,
    AttributeSetRequest,
    AttributeChangeResponse,
    PointBuyRequest,
    PointBuyResponse,
    AttributeRollRequest,
    AttributeRollResponse,
    AttributeSummary,
    AttributeValidation,
    AttributeModifiersResponse
)

# Combat models
from .combat_models import (
    ArmorClassBreakdown,
    BaseAttackBonusInfo,
    AttackBonusBreakdown,
    DamageBonusBreakdown,
    WeaponInfo,
    EquippedWeapons,
    DefensiveAbilities,
    CombatManeuvers,
    InitiativeInfo,
    CombatSummary,
    CombatState,
    CombatUpdateRequest,
    CombatUpdateResponse,
    CombatModeToggleRequest,
    CombatModeToggleResponse,
    DefensiveStats,
    NaturalArmorUpdateRequest,
    NaturalArmorUpdateResponse,
    InitiativeBonusUpdateRequest,
    InitiativeBonusUpdateResponse
)

# Skill models
from .skill_models import (
    SkillInfo,
    SkillPoints,
    SkillSummary,
    SkillUpdateRequest,
    SkillChange,
    SkillUpdateResponse,
    SkillBatchUpdateRequest,
    SkillBatchUpdateResponse,
    SkillResetRequest,
    SkillResetResponse,
    SkillCheckRequest,
    SkillCheckResponse,
    SkillPrerequisites,
    SkillBuild,
    SkillBuildImportRequest,
    SkillBuildImportResponse,
    AllSkillsResponse,

)

# Feat models
from .feat_models import (
    FeatPrerequisites,
    DetailedPrerequisite,
    FeatInfo,
    FeatChain,
    FeatSlots,
    FeatCategories,
    CurrentFeats,
    FeatSummary,
    FeatState,
    FeatAddRequest,
    FeatAddResponse,
    FeatRemoveRequest,
    FeatRemoveResponse,
    FeatValidationRequest,
    FeatValidationResponse,
    FeatSearchRequest,
    FeatSearchResponse,
    FeatBuild,
    FeatRespecRequest,
    FeatRespecResponse,
    FeatUpdateRequest,
    FeatUpdateResponse,
    AvailableFeatsResponse,
    LegitimateFeatsResponse,
    FeatDetails,
    FeatsByCategoryResponse
)

# Character models (top-level orchestration)
from .character_models import (
    CharacterInfo,
    CharacterSummary,
    ManagerStatus,
    ManagersStatus,
    Transaction,
    TransactionHistory,
    ValidationResult,
    BatchUpdateRequest,
    BatchUpdateResult,
    CharacterState,
    CharacterExport,
    CharacterImportRequest,
    CharacterImportResponse,
    CharacterSaveRequest,
    CharacterSaveResponse,
    CharacterBackup,
    CharacterBackupList,
    CharacterComparisonRequest,
    CharacterComparisonResult,
    CharacterCloneRequest,
    CharacterCloneResponse,
    CharacterEventData,
    CharacterEvents,
    CharacterDebugInfo
)

# Note: base_models.py was removed - models moved to other files as needed

# Spell models
from .spell_models import (
    SpellInfo,
    SpellSchool,
    SpellcastingClass,
    MetamagicFeat,
    MemorizedSpell,
    SpellSummary,
    SpellsState,
    SpellSummaryClass,
    AvailableSpellsResponse,
    AllSpellsResponse,
    SpellManageRequest,
    SpellManageResponse
)

# Inventory models
from .inventory_models import (
    ItemProperty,
    ItemInfo,
    EquipmentSlot,
    EncumbranceInfo,
    EquipmentBonuses,
    InventoryValidation,
    CarryCapacity
)

# Save models - Updated to match SaveManager methods
from .save_models import (
    SaveDetails,
    SaveSummaryResponse,
    SaveBreakdownResponse,
    SaveTotalsResponse,
    SaveCheckRequest,
    SaveCheckResponse,
    TemporaryModifierRequest,
    TemporaryModifierResponse,
    MiscSaveBonusRequest,
    MiscSaveBonusResponse,
    RacialSavesResponse,
    ClearModifiersResponse,
    # Aliases for backward compatibility
    SavesState,
    SaveState,
    SaveBreakdown
)

# Savegame models
from .savegame_models import (
    SavegameImportRequest,
    SavegameImportResponse,
    CompanionInfo,
    SavegameCompanionsResponse,
    BackupInfo,
    SavegameInfoResponse,
    SavegameUpdateRequest,
    SavegameUpdateResponse,
    SavegameRestoreRequest,
    SavegameRestoreResponse,
    SavegameExportRequest,
    SavegameExportResponse,
    SavegameValidationRequest,
    SavegameValidationResponse,
    SavegameBrowseRequest,
    SavegameListItem,
    SavegameBrowseResponse
)

# Race models
from .race_models import (
    CurrentRace,
    RaceSummary,
    RaceChangeRequest,
    RaceChangeResponse,
    RaceValidationRequest,
    RaceValidationResponse,
    SubraceInfo,
    AvailableSubracesResponse,
    SubraceValidationResponse
)

# Content models
from .content_models import (
    ModuleInfo,
    CampaignInfo,
    CampaignInfoResponse,
    CustomContentItem,
    CustomContentSummary
)

# Class models
from .class_models import (
    ClassInfo,
    ClassLevel,
    ClassFeature,
    MulticlassInfo,
    CombatProgression,
    ClassSummary,
    ClassState,
    ClassChangeRequest,
    ClassChangeResponse,
    PrestigeClassOption,
    ClassProgressionPreview,
    ClassValidationRequest,
    ClassValidationResponse,
    ClassSearchRequest,
    ClassSearchResponse,
    ClassBuildExport,
    ClassBuildImportRequest,
    ClassBuildImportResponse,
    ClassesState,
    LevelUpRequest,
    ClassChangeResult,
    ClassChangePreview,
    LevelUpResult,
    LevelUpPreview,
    FocusInfo,
    SearchClassesResult,
    CategorizedClassesResponse,
    ClassFeaturesRequest,
    ClassFeaturesResponse
)

# Gamedata models
from .gamedata_models import (
    NWN2PathInfo,
    PathInfo,
    CustomFolderInfo,
    PathConfig,
    NWN2PathsResponse,
    GameDataTableInfo,
    GameDataTablesResponse,
    GameDataRowRequest,
    GameDataRowResponse,
    TLKInfo,
    TLKStringRequest,
    TLKStringEntry,
    TLKStringResponse,
    HAKInfo,
    HAKListResponse,
    ModuleInfo,
    ModuleListResponse,
    GameDataCacheInfo,
    GameDataCacheResponse,
    GameDataRefreshRequest,
    GameDataRefreshResponse,
    GameDataConfigResponse,
    # New FastAPI endpoint models
    GameDataTableResponse,
    GameDataTablesResponse,
    GameDataSchemaResponse,
    GameDataModulesResponse
)

__all__ = [
    # Shared models
    'BaseResponse', 'ErrorResponse', 'ValidationError', 'ValidationResponse',
    'HealthResponse', 'ReadyResponse', 'SystemInfo', 'CacheStatus', 'ConfigResponse',
    'CacheRebuildResponse', 'ConfigUpdateRequest', 'ConfigUpdateResponse', 'NWN2PathResponse',
    'AutoDiscoverResponse', 'BackgroundLoadingTriggerResponse', 'BackgroundLoadingStatusResponse',
    'InitializationStatusResponse',
    'SessionInfo', 'SessionStatus', 'ActiveSessionsList',
    'ChangeRecord', 'CascadingEffect', 'ChangeResult',
    'PaginationParams', 'PaginationInfo', 'PaginatedResponse',
    'FileInfo', 'ResourceInfo',
    'CustomContentItem', 'CustomContentSummary',
    'GFFFieldInfo', 'RawDataRequest', 'RawDataResponse', 'FieldStructureResponse',
    'RawFieldUpdateRequest', 'RawFieldUpdateResponse',
    'BatchOperation', 'BatchOperationResult', 'BatchRequest', 'BatchResponse',
    'ExportOptions', 'ImportOptions',
    'AlignmentResponse', 'AlignmentUpdateRequest', 'AlignmentShiftRequest', 'AlignmentShiftResponse',
    
    # Ability models
    'AbilityScore', 'AbilityScores', 'AbilityModifiers', 'DetailedModifiers',
    'AttributeDependencies', 'EncumbranceLimits', 'CharacterBiography',
    'AttributeState', 'AttributeChangeRequest', 'AttributeSetRequest',
    'AttributeChangeResponse', 'PointBuyRequest', 'PointBuyResponse',
    'AttributeRollRequest', 'AttributeRollResponse', 'AttributeSummary',
    'AttributeValidation', 'AttributeModifiersResponse',
    
    # Combat models
    'ArmorClassBreakdown', 'BaseAttackBonusInfo', 'AttackBonusBreakdown',
    'DamageBonusBreakdown', 'WeaponInfo', 'EquippedWeapons', 'DefensiveAbilities',
    'CombatManeuvers', 'InitiativeInfo', 'CombatSummary', 'CombatState',
    'CombatUpdateRequest',
    'CombatUpdateResponse', 'CombatModeToggleRequest', 'CombatModeToggleResponse',
    'DefensiveStats', 'NaturalArmorUpdateRequest', 'NaturalArmorUpdateResponse',
    'InitiativeBonusUpdateRequest', 'InitiativeBonusUpdateResponse',
    
    # Skill models
    'SkillInfo', 'SkillPoints', 'SkillSynergy', 'SkillSummary', 'SkillState',
    'SkillUpdateRequest', 'SkillChange', 'SkillUpdateResponse',
    'SkillBatchUpdateRequest', 'SkillBatchUpdateResponse', 'SkillResetRequest',
    'SkillResetResponse', 'SkillCheckRequest', 'SkillCheckResponse',
    'SkillPrerequisites', 'SkillBuild', 'SkillBuildImportRequest',
    'SkillBuildImportResponse', 'SkillValidation',
    
    # Feat models
    'FeatPrerequisites', 'DetailedPrerequisite', 'FeatInfo', 'FeatChain',
    'FeatSlots', 'FeatCategories', 'CurrentFeats', 'FeatSummary', 'FeatState',
    'FeatAddRequest', 'FeatAddResponse', 'FeatRemoveRequest', 'FeatRemoveResponse',
    'FeatValidationRequest', 'FeatValidationResponse', 'FeatSearchRequest',
    'FeatSearchResponse', 'FeatBuild', 'FeatRespecRequest', 'FeatRespecResponse',
    'FeatUpdateRequest', 'FeatUpdateResponse', 'AvailableFeatsResponse',
    'LegitimateFeatsResponse', 'FeatDetails', 'FeatsByCategoryResponse',
    
    # Character models
    'CharacterInfo', 'CharacterSummary', 'ManagerStatus', 'ManagersStatus',
    'Transaction', 'TransactionHistory', 'ValidationResult', 'BatchUpdateRequest',
    'BatchUpdateResult', 'CharacterState', 'CharacterExport', 'CharacterImportRequest',
    'CharacterImportResponse', 'CharacterSaveRequest', 'CharacterSaveResponse',
    'CharacterBackup', 'CharacterBackupList', 'CharacterComparisonRequest',
    'CharacterComparisonResult', 'CharacterCloneRequest', 'CharacterCloneResponse',
    'CharacterEventData', 'CharacterEvents', 'CharacterDebugInfo',
    
    # Spell models
    'SpellInfo', 'SpellSchool', 'SpellcastingClass', 'MetamagicFeat', 'MemorizedSpell',
    'SpellSummary', 'SpellsState', 'SpellSummaryClass',
    'AvailableSpellsResponse', 'AllSpellsResponse', 'SpellManageRequest', 'SpellManageResponse',
    
    # Inventory models
    'ItemProperty', 'ItemInfo', 'EquipmentSlot', 'EncumbranceInfo',
    'ItemCreateRequest', 'ItemCreateResponse', 'ItemModifyRequest',
    'ItemModifyResponse', 'ItemEquipRequest', 'ItemEquipResponse', 'ItemUnequipRequest',
    'ItemUnequipResponse', 'ItemMoveRequest', 'ItemMoveResponse', 'ItemStackRequest',
    'ItemStackResponse', 'ItemIdentifyRequest', 'ItemIdentifyResponse', 'ItemSearchRequest',
    'ItemSearchResponse', 'EquipmentBonuses', 'InventoryValidation',
    'InventoryUpdateRequest', 'InventoryUpdateResponse', 'CarryCapacity',
    
    # Savegame models
    'SavegameImportRequest', 'SavegameImportResponse', 'CompanionInfo',
    'SavegameCompanionsResponse', 'BackupInfo', 'SavegameInfoResponse',
    'SavegameUpdateRequest', 'SavegameUpdateResponse', 'SavegameRestoreRequest',
    'SavegameRestoreResponse', 'SavegameExportRequest', 'SavegameExportResponse',
    'SavegameValidationRequest', 'SavegameValidationResponse', 'SavegameBrowseRequest',
    'SavegameListItem', 'SavegameBrowseResponse',
    
    # Race models
    'CurrentRace', 'RaceSummary', 'RaceChangeRequest', 'RaceChangeResponse', 
    'RaceValidationRequest', 'RaceValidationResponse', 'SubraceInfo',
    'AvailableSubracesResponse', 'SubraceValidationResponse',
    
    # Save models - Updated to match SaveManager methods
    'SaveDetails', 'SaveSummaryResponse', 'SaveBreakdownResponse', 'SaveTotalsResponse',
    'SaveCheckRequest', 'SaveCheckResponse', 'TemporaryModifierRequest', 'TemporaryModifierResponse',
    'MiscSaveBonusRequest', 'MiscSaveBonusResponse', 'RacialSavesResponse', 'ClearModifiersResponse',
    'SavesState', 'SaveState', 'SaveBreakdown',
    
    # Content models
    'ModuleInfo', 'CampaignInfo', 'CampaignInfoResponse', 'CustomContentItem', 'CustomContentSummary',
    
    # Class models
    'ClassInfo', 'ClassLevel', 'ClassFeature', 'MulticlassInfo', 'CombatProgression',
    'ClassSummary', 'ClassState', 'ClassChangeRequest', 'ClassChangeResponse',
    'PrestigeClassOption', 'ClassProgressionPreview', 'ClassValidationRequest',
    'ClassValidationResponse', 'ClassSearchRequest', 'ClassSearchResponse',
    'ClassBuildExport', 'ClassBuildImportRequest', 'ClassBuildImportResponse',
    'ClassesState', 'LevelUpRequest', 'ClassChangeResult', 'ClassChangePreview',
    'LevelUpResult', 'LevelUpPreview', 'FocusInfo', 'SearchClassesResult',
    'CategorizedClassesResponse', 'ClassFeaturesRequest', 'ClassFeaturesResponse',
    
    # Savegame models
    'SavegameImportRequest', 'SavegameImportResponse', 'CompanionInfo', 'SavegameCompanionsResponse',
    'BackupInfo', 'SavegameInfoResponse', 'SavegameUpdateRequest', 'SavegameUpdateResponse',
    'SavegameRestoreRequest', 'SavegameRestoreResponse', 'SavegameExportRequest', 'SavegameExportResponse',
    'SavegameValidationRequest', 'SavegameValidationResponse', 'SavegameBrowseRequest',
    'SavegameListItem', 'SavegameBrowseResponse',
    
    # Gamedata models
    'NWN2PathInfo', 'PathInfo', 'CustomFolderInfo', 'PathConfig', 'NWN2PathsResponse', 
    'GameDataTableInfo', 'GameDataTablesResponse',
    'GameDataRowRequest', 'GameDataRowResponse', 'TLKInfo', 'TLKStringRequest',
    'TLKStringEntry', 'TLKStringResponse', 'HAKInfo', 'HAKListResponse',
    'ModuleInfo', 'ModuleListResponse', 'GameDataCacheInfo', 'GameDataCacheResponse',
    'GameDataRefreshRequest', 'GameDataRefreshResponse', 'GameDataConfigResponse',
    # New FastAPI endpoint models
    'GameDataTableResponse', 'GameDataTablesResponse', 'GameDataSchemaResponse', 'GameDataModulesResponse',
    
    # Legacy base models removed - functionality moved to appropriate model files
]