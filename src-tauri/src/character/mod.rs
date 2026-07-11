//! Character module - idiomatic Rust character representation
//!
//! Replaces the 11-manager architecture with a single Character struct
//! that directly owns GFF data without Arc<RwLock<>> overhead.

use crate::parsers::gff::GffValue;
use indexmap::IndexMap;
use tracing::{debug, instrument};

pub mod abilities;
mod appearance;
pub(crate) mod appearance_helpers;
pub mod classes;
pub mod combat;
pub mod combat_summary;
pub mod error;
pub(crate) mod feats;
pub mod gff_helpers;
mod identity;
mod inventory;
mod item_appearance;
pub mod overview;
mod race;
pub mod save_summary;
pub mod saves;
mod skills;
mod spells;
pub mod types;

pub use abilities::{AbilitiesState, AbilityIncrease, AbilityPointsSummary};
pub use appearance::{AppearanceOption, AppearanceState, CharacterModelParts};
pub use appearance_helpers::{TintChannel, TintChannels, cosmetic_gloves_tint, swap_tint_2_3};
pub use classes::ClassesState;
pub use classes::{
    AlignmentRestriction, BabType, ClassEntry, ClassInfo, ClassSummaryEntry, LevelHistoryEntry,
    PrestigeClassOption, PrestigeClassValidation, PrestigeRequirements, SkillRankEntry, XpProgress,
};
pub use combat::{CombatStats, DamageBonuses};
pub use combat_summary::{
    ACBreakdown, ArmorClass, AttackBonuses, AttackBreakdown, CombatManeuverBonus, CombatSummary,
    DamageReduction, Initiative, InitiativeChange, MovementSpeed, NaturalArmorChange,
};
pub use error::CharacterError;
pub use feats::{
    DomainInfo, FeatAvailability, FeatCategory, FeatEntry, FeatInfo, FeatSlots, FeatSource,
    FeatSummary, FeatType, FeatsState, PrerequisiteResult,
};
pub use identity::{Alignment, resolve_localized_name};
pub use inventory::{
    AddItemResult, BasicItemInfo, EncumbranceInfo, EquipResult, EquipmentSlot, EquipmentSummary,
    FullEncumbrance, FullEquippedItem, FullInventoryItem, FullInventorySummary, InventoryItem,
    ItemProficiencyInfo, ProficiencyRequirement, RemoveItemResult, UnequipResult, WeightStatus,
};
pub use item_appearance::{
    AccessorySlot, AccessorySlotData, ArmorAccessories, ItemAppearance, ItemAppearanceOptions,
    RenderableAccessory, resref_attach_bone_and_slot,
};
pub(crate) use item_appearance::{detect_armor_slot, is_bracer_label, parse_equip_slots};
pub use overview::OverviewState;
pub use race::{SizeCategory, SubraceInfo};
pub use save_summary::{SaveBreakdown, SaveChange, SaveCheck, SaveSummary, SaveType, SavingThrows};
pub use saves::SaveBonuses;
pub use skills::{ABLE_LEARNER_FEAT_ID, SkillPointsSummary, SkillSummaryEntry};
pub use spells::{
    AbilitySpellEntry, CasterClassSummary, KnownSpellEntry, MAX_SPELL_LEVEL, MemorizedSpellEntry,
    MemorizedSpellRaw, MetamagicFeat, SpellDetails, SpellSummary, SpellcastingClass, SpellsState,
    is_displayable_spell, is_mod_prefixed_name,
};
pub use types::*;

/// Character data with direct GFF ownership.
///
/// Unlike the old CharacterData/ManagerContext design, Character owns
/// its GFF fields directly without Arc<RwLock<>>. All methods are sync
/// (not async) - the single lock at AppState/SessionState level is sufficient.
pub struct Character {
    /// GFF fields - fully owned, no lazy loading
    gff: IndexMap<String, GffValue<'static>>,
    /// Track if character has been modified since load/save
    modified: bool,
}

impl Character {
    /// Create a new Character from parsed GFF fields.
    ///
    /// Uses `force_owned()` to recursively convert all LazyStruct values
    /// to StructOwned, eliminating Arc<RwLock<>> from nested data.
    #[instrument(name = "Character::from_gff", skip_all, fields(field_count = fields.len()))]
    pub fn from_gff(fields: IndexMap<String, GffValue<'static>>) -> Self {
        debug!("Converting {} GFF fields to owned values", fields.len());

        let owned_gff: IndexMap<String, GffValue<'static>> = fields
            .into_iter()
            .map(|(k, v)| (k, v.force_owned()))
            .collect();

        debug!("GFF fields converted to owned values");

        Self {
            gff: owned_gff,
            modified: false,
        }
    }

    /// Convert Character back to GFF fields for saving.
    ///
    /// Consumes the Character - caller should clone if needed.
    pub fn into_gff(self) -> IndexMap<String, GffValue<'static>> {
        self.gff
    }

    /// Get a reference to GFF fields for inspection.
    pub fn gff(&self) -> &IndexMap<String, GffValue<'static>> {
        &self.gff
    }

    /// Get a mutable reference to GFF fields.
    /// Marks the character as modified.
    pub fn gff_mut(&mut self) -> &mut IndexMap<String, GffValue<'static>> {
        self.modified = true;
        &mut self.gff
    }

    /// Check if the character has unsaved modifications.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark the character as saved (clear modified flag).
    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// Mark the character as modified.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// Clone the GFF data (useful for saving without consuming).
    pub fn clone_gff(&self) -> IndexMap<String, GffValue<'static>> {
        self.gff.clone()
    }

    pub fn restore_snapshot(&mut self, gff: IndexMap<String, GffValue<'static>>, modified: bool) {
        self.gff = gff;
        self.modified = modified;
    }

    /// Run `f` against the character; if it returns `Err`, roll back all GFF
    /// mutations so the character is left in its pre-call state. Useful for
    /// multi-step operations (`swap_class`, looping level-up/down) where a
    /// partial failure would otherwise leave `ClassList` / `LvlStatList` /
    /// derived stats inconsistent.
    pub fn transactional<F, T, E>(&mut self, f: F) -> Result<T, E>
    where
        F: FnOnce(&mut Self) -> Result<T, E>,
    {
        let gff_snapshot = self.gff.clone();
        let modified_snapshot = self.modified;
        match f(self) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.gff = gff_snapshot;
                self.modified = modified_snapshot;
                Err(e)
            }
        }
    }

    /// Basic character validation.
    ///
    /// Performs fundamental checks to ensure character integrity.
    /// Returns ValidationResult with errors/warnings.
    pub fn validate(&self, game_data: &crate::loaders::GameData) -> ValidationResult {
        let mut result = ValidationResult::ok();

        if self.first_name().is_empty() {
            result.add_warning("Character has no first name");
        }

        let race_id = self.race_id();
        if game_data.get_table("racialtypes").is_some()
            && let Some(races) = game_data.get_table("racialtypes")
            && races.get_by_id(race_id.0).is_none()
        {
            result.add_error(format!("Invalid race ID: {}", race_id.0));
        }

        let level = self.total_level();
        if level < 1 {
            result.add_error("Character level is less than 1");
        }
        if level > types::MAX_TOTAL_LEVEL {
            result.add_error(format!(
                "Character level {} exceeds maximum {}",
                level,
                types::MAX_TOTAL_LEVEL
            ));
        }

        let alignment = self.alignment();
        if alignment.law_chaos < types::ALIGNMENT_MIN || alignment.law_chaos > types::ALIGNMENT_MAX
        {
            result.add_error(format!(
                "Law/Chaos alignment {} out of range",
                alignment.law_chaos
            ));
        }
        if alignment.good_evil < types::ALIGNMENT_MIN || alignment.good_evil > types::ALIGNMENT_MAX
        {
            result.add_error(format!(
                "Good/Evil alignment {} out of range",
                alignment.good_evil
            ));
        }

        result
    }
}

impl std::fmt::Debug for Character {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Character")
            .field("fields_count", &self.gff.len())
            .field("modified", &self.modified)
            .finish()
    }
}
