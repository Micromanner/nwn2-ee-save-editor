use indexmap::IndexMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::character::{Character, FeatInfo};
use crate::loaders::GameData;
use crate::parsers::gff::{GffParser, GffValue, GffWriter};
use crate::services::PlayerInfo;
use crate::services::campaign::content::{ModuleInfo, ModuleVariables};
use crate::services::item_property_decoder::ItemPropertyDecoder;
use crate::services::load_diagnostics::{LoadError, LoadReport, LoadStage};
use crate::services::resource_manager::ResourceManager;
use crate::services::save_graph::SaveGraph;
use crate::services::savegame_handler::backup_manifest::{
    self, CharacterChanges, PartyChange, QuestChange, SaveRecord,
};
use crate::services::savegame_handler::{PlayerOutputs, SaveGameHandler};
use crate::state::standalone_bic;

const HISTORY_LIMIT: usize = 100;

pub struct HistoryEntry {
    pub label: String,
    gff_snapshot: IndexMap<String, GffValue<'static>>,
    modified_snapshot: bool,
}

/// Undo/redo stacks of one character. Parked together with the character so
/// switching party members keeps each one's history.
#[derive(Default)]
pub struct History {
    pub undo_stack: VecDeque<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
}

#[derive(serde::Serialize, specta::Type)]
pub struct CharacterHistory {
    /// `None` for the player / standalone character.
    pub ros_name: Option<String>,
    pub name: String,
    pub active: bool,
    /// Newest first.
    pub undo_labels: Vec<String>,
    /// Nearest first.
    pub redo_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterSource {
    Player,
    Companion { ros_name: String },
    Standalone,
}

pub struct RosterListing {
    pub ros_name: String,
    pub char_name: String,
    pub classes: Vec<(i32, i32)>,
    /// `RosAvailable == 0`: this companion is in the active party on load.
    pub in_party: bool,
    /// Staged (unsaved) membership differing from `in_party`, if any.
    pub staged_in_party: Option<bool>,
    pub gender: Option<i32>,
    /// `RosRace` mirrors the creature's `Subrace` byte (a `racialsubtypes.2da` row).
    pub subrace: Option<i32>,
}

pub struct SessionState {
    pub current_file_path: Option<PathBuf>,
    pub save_dir: Option<PathBuf>,
    pub savegame_handler: Option<SaveGameHandler>,
    pub character: Option<Character>,
    pub selected_player_index: usize,
    pub primary_player_index: Option<usize>,
    pub item_property_decoder: ItemPropertyDecoder,
    pub feat_cache: Option<Vec<FeatInfo>>,
    pub module_info_cache: Option<(ModuleInfo, ModuleVariables)>,
    /// Cached aggregated quest graph for the current save. Built on first
    /// `save_get_quest_graph` call so per-quest transition fetches are O(lookup).
    /// `Arc` so cache hits don't deep-clone the multi-MB graph — projection and
    /// per-quest fetch both read through the shared pointer.
    /// Cleared whenever the save changes (`load_character`, `close_character`).
    pub quest_graph_cache: Option<Arc<SaveGraph>>,
    pub quest_edits: crate::services::save_graph::editing::QuestEdits,
    pub history: History,
    pub character_source: CharacterSource,
    pub standalone_backup_done: bool,
    /// Characters edited earlier in this session but not currently active. Edits
    /// and undo history stay in memory across switches; `save_character` flushes
    /// all of them.
    parked: Vec<(CharacterSource, Character, History)>,
    /// Staged party-membership changes (lowercased ros_name -> desired
    /// in_party), held in memory until `save_character` flushes them.
    staged_party: std::collections::HashMap<String, bool>,
    /// Backup directories taken during this session (load, pre-save, manual).
    /// Every successful save appends its change record to each of them.
    session_backups: Vec<PathBuf>,
    /// Every history label of this session in order, never popped: undo and
    /// redo append their own entries. Feeds the backup manifests on save.
    change_log: Vec<(CharacterSource, String)>,
}

impl SessionState {
    #[instrument(name = "SessionState::new", skip_all)]
    pub fn new(resource_manager: Arc<tokio::sync::RwLock<ResourceManager>>) -> Self {
        debug!("Creating SessionState");

        debug!("Initializing ItemPropertyDecoder");
        let item_property_decoder = ItemPropertyDecoder::new(resource_manager);
        debug!("ItemPropertyDecoder created");

        info!("SessionState created successfully");

        Self {
            current_file_path: None,
            save_dir: None,
            savegame_handler: None,
            character: None,
            selected_player_index: 0,
            primary_player_index: None,
            item_property_decoder,
            feat_cache: None,
            module_info_cache: None,
            quest_graph_cache: None,
            quest_edits: crate::services::save_graph::editing::QuestEdits::default(),
            history: History::default(),
            character_source: CharacterSource::Player,
            standalone_backup_done: false,
            parked: Vec::new(),
            staged_party: std::collections::HashMap::new(),
            session_backups: Vec::new(),
            change_log: Vec::new(),
        }
    }

    #[instrument(name = "SessionState::load_character", skip(self, report), fields(file_path = %file_path))]
    pub fn load_character(
        &mut self,
        file_path: &str,
        player_index: Option<usize>,
        report: &mut LoadReport,
    ) -> Result<(), LoadError> {
        info!("Loading character from save file");
        let path = PathBuf::from(file_path);

        if path.is_file()
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("bic"))
        {
            return self.load_standalone_bic(&path, report);
        }

        crate::services::savegame_handler::backup::clear_backup_tracking();

        let fatal = |report: &mut LoadReport,
                     stage: LoadStage,
                     message: String,
                     context: serde_json::Value|
         -> LoadError {
            warn!("{message}");
            report.set_fatal(stage, message.clone(), context.clone());
            LoadError {
                stage,
                message,
                context,
            }
        };

        debug!("Creating SaveGameHandler");
        let handler = SaveGameHandler::new(&path, false, true).map_err(|e| {
            fatal(
                report,
                LoadStage::SaveOpen,
                format!("Failed to create save handler: {e}"),
                serde_json::json!({ "file_path": file_path }),
            )
        })?;
        debug!("SaveGameHandler created");

        debug!("Extracting playerlist.ifo from save archive");
        let playerlist_data = handler.extract_player_data().map_err(|e| {
            fatal(
                report,
                LoadStage::Playerlist,
                format!("Failed to extract playerlist.ifo: {e}"),
                serde_json::json!({ "file_path": file_path }),
            )
        })?;
        info!("playerlist.ifo extracted ({} bytes)", playerlist_data.len());

        debug!("Parsing playerlist.ifo GFF data");
        let gff = GffParser::from_bytes(playerlist_data).map_err(|e| {
            fatal(
                report,
                LoadStage::Gff,
                format!("GFF Parse error: {e}"),
                serde_json::json!({ "file_path": file_path, "which": "playerlist.ifo" }),
            )
        })?;
        debug!("playerlist.ifo parsed successfully");

        let player_entries = read_playerlist_entries(gff).map_err(|e| {
            fatal(
                report,
                LoadStage::Playerlist,
                format!("Failed to read playerlist entries: {e}"),
                serde_json::Value::Null,
            )
        })?;

        let bic_warning = |report: &mut LoadReport, reason: String| {
            warn!("{reason}");
            report.add_warning(
                LoadStage::Bic,
                reason,
                serde_json::json!({ "fallback": "playerlist_slot" }),
            );
        };
        let player_bic_fields = match handler.extract_player_bic() {
            Ok(Some(player_bic_data)) => match read_player_bic_entry(player_bic_data) {
                Ok(fields) => Some(fields),
                Err(err) => {
                    bic_warning(report, format!("Failed to parse player.bic: {err}"));
                    None
                }
            },
            Ok(None) => None,
            Err(err) => {
                bic_warning(report, format!("Failed to extract player.bic: {err}"));
                None
            }
        };
        let primary_player_index =
            resolve_primary_player_index(&player_entries, player_bic_fields.as_ref());
        let selected_player_index = player_index.unwrap_or(primary_player_index.unwrap_or(0));

        let fields = match player_bic_fields {
            Some(fields) if primary_player_index == Some(selected_player_index) => {
                debug!(
                    "Using player.bic as authoritative source for playerlist slot {}",
                    selected_player_index
                );
                fields
            }
            _ => read_playerlist_entry_from_entries(&player_entries, selected_player_index)
                .map_err(|e| {
                    fatal(
                        report,
                        LoadStage::Playerlist,
                        format!("Failed to read playerlist slot: {e}"),
                        serde_json::json!({ "selected_player_index": selected_player_index }),
                    )
                })?,
        };
        info!("Character data extracted ({} fields)", fields.len());

        debug!("Creating Character from GFF fields");
        let character = Character::from_gff(fields);
        info!(
            "Character created: {} (Level {})",
            character.full_name(),
            character.total_level()
        );

        let save_dir = if path.is_dir() {
            path.clone()
        } else if let Some(parent) = path.parent().map(PathBuf::from) {
            parent
        } else {
            return Err(fatal(
                report,
                LoadStage::SaveOpen,
                "Failed to determine save directory".to_string(),
                serde_json::json!({ "file_path": file_path }),
            ));
        };

        self.character = Some(character);
        self.savegame_handler = Some(handler);
        self.current_file_path = Some(path);
        self.save_dir = Some(save_dir);
        self.module_info_cache = None;
        self.quest_graph_cache = None;
        self.selected_player_index = selected_player_index;
        self.primary_player_index = primary_player_index;
        self.character_source = CharacterSource::Player;
        self.parked.clear();
        self.staged_party.clear();
        self.quest_edits.clear();
        self.clear_history();
        if let Some(path) = self
            .savegame_handler
            .as_ref()
            .and_then(|h| h.load_backup_path())
        {
            self.session_backups.push(path.to_path_buf());
        }

        info!("Character loaded successfully");
        Ok(())
    }

    #[instrument(name = "SessionState::load_standalone_bic", skip(self, report), fields(path = %path.display()))]
    fn load_standalone_bic(
        &mut self,
        path: &Path,
        report: &mut LoadReport,
    ) -> Result<(), LoadError> {
        info!("Loading standalone character file");
        crate::services::savegame_handler::backup::clear_backup_tracking();

        let fields = standalone_bic::load_fields(path).map_err(|message| {
            warn!("{message}");
            let context = serde_json::json!({ "file_path": path.display().to_string() });
            report.set_fatal(LoadStage::Bic, message.clone(), context.clone());
            LoadError {
                stage: LoadStage::Bic,
                message,
                context,
            }
        })?;

        let character = Character::from_gff(fields);
        info!(
            "Standalone character created: {} (Level {})",
            character.full_name(),
            character.total_level()
        );

        self.character = Some(character);
        self.savegame_handler = None;
        self.current_file_path = Some(path.to_path_buf());
        self.save_dir = None;
        self.feat_cache = None;
        self.module_info_cache = None;
        self.quest_graph_cache = None;
        self.selected_player_index = 0;
        self.primary_player_index = None;
        self.character_source = CharacterSource::Standalone;
        self.standalone_backup_done = false;
        self.parked.clear();
        self.staged_party.clear();
        self.quest_edits.clear();
        self.clear_history();

        info!("Standalone character loaded successfully");
        Ok(())
    }

    /// Write every modified character of this session (active and parked) to the
    /// save, then any staged party changes, batching all zip-bound writes into a
    /// single archive rewrite. Every character is attempted even when one fails;
    /// the collected failures (with any warnings) come back as one error, and
    /// serialization failures abort the batch. Archive and loose-file replacements
    /// roll back together; nothing is marked saved until the batch commits.
    pub fn save_character(
        &mut self,
        game_data: &GameData,
        backup_keep_count: usize,
    ) -> Result<Option<String>, String> {
        if self.character_source == CharacterSource::Standalone {
            return self.save_standalone_inner(backup_keep_count);
        }

        if !self.quest_edits.pending.is_empty() {
            let handler = self.savegame_handler.as_ref().ok_or("No save loaded")?;
            self.quest_edits.validate_saved(
                handler,
                &crate::config::NWN2Paths::new(),
                self.primary_player_index
                    .unwrap_or(self.selected_player_index),
            )?;
        }
        let record = self.build_save_record(game_data);
        let mut active = self.character.take().ok_or("No character loaded")?;
        if let Some(handler) = self.savegame_handler.as_mut() {
            handler.begin_batch();
        }

        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        let active_source = self.character_source.clone();
        let active_saved = match self.save_one(&active_source, &mut active, game_data) {
            Ok(warning) => {
                warnings.extend(warning);
                true
            }
            Err(e) => {
                errors.push(format!("{}: {e}", describe_source(&active_source)));
                false
            }
        };
        self.character = Some(active);

        let parked = std::mem::take(&mut self.parked);
        let mut restored = Vec::with_capacity(parked.len());
        let mut saved_parked = Vec::new();
        for (source, mut character, history) in parked {
            match self.save_one(&source, &mut character, game_data) {
                Ok(warning) => {
                    warnings.extend(warning);
                    saved_parked.push(restored.len());
                }
                Err(e) => errors.push(format!("{}: {e}", describe_source(&source))),
            }
            restored.push((source, character, history));
        }
        self.parked = restored;

        let flushed_party = match self.flush_staged_party() {
            Ok(warning) => {
                warnings.extend(warning);
                true
            }
            Err(e) => {
                errors.push(format!("party changes: {e}"));
                false
            }
        };

        let mut rewarded_fields = Vec::new();
        if let Some(handler) = self.savegame_handler.as_mut() {
            if errors.is_empty() {
                match self.quest_edits.flush(
                    handler,
                    self.primary_player_index
                        .unwrap_or(self.selected_player_index),
                    self.primary_player_index.is_some(),
                    game_data,
                ) {
                    Ok(written) => rewarded_fields = written,
                    Err(e) => errors.push(e),
                }
            }
            if !errors.is_empty() {
                handler.abort_batch();
                return Err(errors.join("\n"));
            }
        }

        if let Some(handler) = self.savegame_handler.as_mut()
            && let Err(e) = handler.commit_batch()
        {
            errors.push(format!("Failed to write save file: {e}"));
            return Err(errors.join("\n"));
        }

        if active_saved && let Some(character) = self.character.as_mut() {
            character.mark_saved();
        }
        for idx in saved_parked {
            self.parked[idx].1.mark_saved();
        }
        // Quest rewards landed in the save on top of the serialized player; mirror them
        // into the loaded player so the character tabs match without a reload.
        if !rewarded_fields.is_empty()
            && self
                .primary_player_index
                .unwrap_or(self.selected_player_index)
                == self.selected_player_index
            && let Some(player) = self.player_character_mut()
        {
            for (name, value) in rewarded_fields {
                player.gff_mut().insert(name, value);
            }
            player.mark_saved();
        }
        if flushed_party {
            self.staged_party.clear();
        }
        self.quest_edits.clear();
        self.module_info_cache = None;
        if !record.is_empty() {
            for dir in &self.session_backups {
                backup_manifest::append_save(dir, &record);
            }
        }
        self.change_log.clear();

        if !errors.is_empty() {
            if !warnings.is_empty() {
                errors.extend(warnings);
            }
            return Err(errors.join("\n"));
        }

        Ok(if warnings.is_empty() {
            None
        } else {
            Some(warnings.join("\n"))
        })
    }

    fn save_one(
        &mut self,
        source: &CharacterSource,
        character: &mut Character,
        game_data: &GameData,
    ) -> Result<Option<String>, String> {
        match source {
            CharacterSource::Companion { ros_name } => {
                let prefix = Self::companion_blueprint_prefix(character, game_data);
                self.save_companion_inner(character, ros_name, prefix.as_deref())
            }
            CharacterSource::Player => self.save_player_inner(character, game_data),
            CharacterSource::Standalone => Err("Standalone character cannot be parked".into()),
        }
    }

    fn save_player_inner(
        &mut self,
        character: &mut Character,
        game_data: &GameData,
    ) -> Result<Option<String>, String> {
        if !character.is_modified() {
            info!("No changes to save");
            return Ok(None);
        }
        let companion_prefix = Self::companion_blueprint_prefix(character, game_data);
        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or_else(|| "No active save handler".to_string())?;

        let char_fields = character.clone_gff();
        let selected_player_index = self.selected_player_index;
        let update_primary_player_files = self.primary_player_index == Some(selected_player_index);

        handler
            .rewrite_player_files(|src| {
                let playerlist = serialize_playerlist_bytes(
                    src.playerlist,
                    &char_fields,
                    selected_player_index,
                    companion_prefix.as_deref(),
                )?;
                let player_bic = if update_primary_player_files {
                    Some(crate::parsers::gff::merge_fields_into_gff(
                        src.player_bic,
                        &char_fields,
                        "BIC ",
                        false,
                    )?)
                } else {
                    None
                };
                Ok(PlayerOutputs {
                    playerlist,
                    player_bic,
                })
            })
            .map_err(|e| format!("Failed to write save file: {e}"))?;

        if update_primary_player_files {
            let save_dir = self.save_dir.as_ref().ok_or("No current save path")?;
            let bytes = playerinfo_for_character(save_dir, character, game_data)?;
            handler
                .update_file("playerinfo.bin", &bytes)
                .map_err(|e| e.to_string())?;
        }

        info!(
            "Character saved successfully (player.bic_updated={})",
            update_primary_player_files
        );
        Ok(None)
    }

    fn companion_blueprint_prefix(character: &Character, game_data: &GameData) -> Option<String> {
        crate::character::pets::companion_blueprint_prefix(
            character.animal_companion_type(),
            game_data,
        )
    }

    fn save_standalone_inner(
        &mut self,
        backup_keep_count: usize,
    ) -> Result<Option<String>, String> {
        let path = self
            .current_file_path
            .clone()
            .ok_or("No character file loaded")?;
        let character = self.character.as_ref().ok_or("No character loaded")?;

        if !character.is_modified() {
            info!("No changes to save");
            return Ok(None);
        }

        standalone_bic::save(
            &path,
            &character.clone_gff(),
            backup_keep_count,
            !self.standalone_backup_done,
        )?;
        self.standalone_backup_done = true;
        self.character.as_mut().unwrap().mark_saved();
        Ok(None)
    }

    fn save_companion_inner(
        &mut self,
        character: &mut Character,
        ros_name: &str,
        companion_prefix: Option<&str>,
    ) -> Result<Option<String>, String> {
        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or("No active save handler")?;

        if !character.is_modified() {
            info!("No changes to save");
            return Ok(None);
        }

        let stored_name = handler
            .companion_stored_name(ros_name)
            .map_err(|e| format!("Failed to locate companion file: {e}"))?;
        let original = handler
            .extract_file(&stored_name)
            .map_err(|e| format!("Failed to read companion file: {e}"))?;
        let mut fields = character.clone_gff();
        if let Some(prefix) = companion_prefix {
            crate::character::pets::sync_mod_companion_variable(&mut fields, prefix);
        }
        // roster.rst RosAvailable is the party-membership record; the engine
        // mirrors it into the .ros FactionID. Re-assert it from the roster so
        // a membership change made after this companion was parsed isn't
        // clobbered by the stale in-memory value.
        if let Some(faction) = companion_faction_from_roster(handler, ros_name) {
            crate::services::roster::set_faction_in_fields(&mut fields, faction);
        }
        let bytes =
            crate::parsers::gff::merge_fields_into_gff(Some(&original), &fields, "ROS ", true)?;
        handler
            .update_file(&stored_name, &bytes)
            .map_err(|e| format!("Failed to write companion file: {e}"))?;

        let classes: Vec<(i32, i32)> = character
            .class_entries()
            .into_iter()
            .map(|e| (e.class_id.0, e.level))
            .collect();
        let identity = crate::services::roster::RosterIdentity {
            gender: character.gender(),
            subrace: character.subrace_index().unwrap_or(0),
        };
        let warning = match sync_roster(handler, ros_name, &classes, identity) {
            Ok(()) => None,
            Err(e) => {
                warn!("Roster sync failed after companion save: {e}");
                Some(format!(
                    "Companion saved, but updating the party roster failed: {e}"
                ))
            }
        };

        info!("Companion saved: {stored_name}");
        Ok(warning)
    }

    pub fn close_character(&mut self) {
        self.character = None;
        self.savegame_handler = None;
        self.current_file_path = None;
        self.save_dir = None;
        self.selected_player_index = 0;
        self.primary_player_index = None;
        self.feat_cache = None;
        self.module_info_cache = None;
        self.quest_graph_cache = None;
        self.character_source = CharacterSource::Player;
        self.standalone_backup_done = false;
        self.parked.clear();
        self.staged_party.clear();
        self.quest_edits.clear();
        self.clear_history();
        crate::services::savegame_handler::backup::clear_backup_tracking();
    }

    pub fn record_history(&mut self, label: impl Into<String>) {
        let Some(character) = self.character.as_ref() else {
            return;
        };
        let entry = HistoryEntry {
            label: label.into(),
            gff_snapshot: character.clone_gff(),
            modified_snapshot: character.is_modified(),
        };
        self.change_log
            .push((self.character_source.clone(), entry.label.clone()));
        self.history.undo_stack.push_back(entry);
        if self.history.undo_stack.len() > HISTORY_LIMIT {
            self.history.undo_stack.pop_front();
        }
        self.history.redo_stack.clear();
    }

    pub fn track_session_backup(&mut self, path: PathBuf) {
        self.session_backups.push(path);
    }

    /// Everything this save will write, described for the backup manifests:
    /// history labels per character, pending quest edits and staged party
    /// changes. Built before the save mutates any of them.
    fn build_save_record(&self, game_data: &GameData) -> SaveRecord {
        let mut characters: Vec<CharacterChanges> = Vec::new();
        for (source, label) in &self.change_log {
            let ros_name = match source {
                CharacterSource::Companion { ros_name } => Some(ros_name.clone()),
                CharacterSource::Player | CharacterSource::Standalone => None,
            };
            if let Some(existing) = characters.iter_mut().find(|c| c.ros_name == ros_name) {
                existing.changes.push(label.clone());
                continue;
            }
            let character = if *source == self.character_source {
                self.character.as_ref()
            } else {
                self.parked
                    .iter()
                    .find(|(s, _, _)| s == source)
                    .map(|(_, c, _)| c)
            };
            let name = character.map_or_else(
                || ros_name.clone().unwrap_or_default(),
                |c| crate::character::resolved_full_name(c, game_data),
            );
            characters.push(CharacterChanges {
                name,
                ros_name,
                changes: vec![label.clone()],
            });
        }

        let quests = self
            .quest_edits
            .pending
            .iter()
            .map(|edit| {
                let category = self.quest_graph_cache.as_ref().and_then(|graph| {
                    graph
                        .quests
                        .iter()
                        .find(|q| q.tag == edit.tag)
                        .map(|q| &q.category)
                });
                QuestChange {
                    tag: edit.tag.clone(),
                    name: category
                        .map(|c| c.name.clone())
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| edit.tag.clone()),
                    from_state: edit.saved_state,
                    to_state: edit.target,
                    state_text: category.and_then(|c| {
                        c.entries
                            .iter()
                            .find(|e| u32::try_from(e.id).ok() == Some(edit.target))
                            .map(|e| e.text.clone())
                    }),
                }
            })
            .collect();

        let party = if self.staged_party.is_empty() {
            Vec::new()
        } else {
            let roster = self.list_roster().unwrap_or_default();
            self.staged_party
                .iter()
                .map(|(ros_name, in_party)| PartyChange {
                    ros_name: ros_name.clone(),
                    name: roster
                        .iter()
                        .find(|r| r.ros_name.eq_ignore_ascii_case(ros_name))
                        .map(|r| r.char_name.clone()),
                    in_party: *in_party,
                })
                .collect()
        };

        SaveRecord {
            saved_at: chrono::Local::now().to_rfc3339(),
            characters,
            quests,
            party,
        }
    }

    pub fn undo(&mut self) -> Option<String> {
        let entry = self.history.undo_stack.pop_back()?;
        let label = entry.label.clone();
        self.change_log
            .push((self.character_source.clone(), format!("Undo: {label}")));
        let redo_entry = self.character.as_ref().map(|ch| HistoryEntry {
            label: label.clone(),
            gff_snapshot: ch.clone_gff(),
            modified_snapshot: ch.is_modified(),
        });
        if let Some(character) = self.character.as_mut() {
            character.restore_snapshot(entry.gff_snapshot, entry.modified_snapshot);
        }
        if let Some(redo) = redo_entry {
            self.history.redo_stack.push(redo);
        }
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let entry = self.history.redo_stack.pop()?;
        let label = entry.label.clone();
        self.change_log
            .push((self.character_source.clone(), format!("Redo: {label}")));
        let undo_entry = self.character.as_ref().map(|ch| HistoryEntry {
            label: label.clone(),
            gff_snapshot: ch.clone_gff(),
            modified_snapshot: ch.is_modified(),
        });
        if let Some(character) = self.character.as_mut() {
            character.restore_snapshot(entry.gff_snapshot, entry.modified_snapshot);
        }
        if let Some(undo) = undo_entry {
            self.history.undo_stack.push_back(undo);
        }
        Some(label)
    }

    pub fn clear_history(&mut self) {
        self.history.undo_stack.clear();
        self.history.redo_stack.clear();
        self.change_log.clear();
        self.session_backups.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.history.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.history.redo_stack.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.history.undo_stack.back().map(|e| e.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.history.redo_stack.last().map(|e| e.label.as_str())
    }

    /// Undo labels newest first.
    pub fn undo_labels(&self) -> Vec<String> {
        self.history
            .undo_stack
            .iter()
            .rev()
            .map(|e| e.label.clone())
            .collect()
    }

    /// Redo labels nearest first.
    pub fn redo_labels(&self) -> Vec<String> {
        self.history
            .redo_stack
            .iter()
            .rev()
            .map(|e| e.label.clone())
            .collect()
    }

    /// Every character of this session that has undo history: the active one
    /// first, then parked ones.
    pub fn session_history(&self, game_data: &GameData) -> Vec<CharacterHistory> {
        let entry =
            |source: &CharacterSource, character: &Character, history: &History, active: bool| {
                CharacterHistory {
                    ros_name: match source {
                        CharacterSource::Companion { ros_name } => Some(ros_name.clone()),
                        CharacterSource::Player | CharacterSource::Standalone => None,
                    },
                    name: crate::character::resolved_full_name(character, game_data),
                    active,
                    undo_labels: history
                        .undo_stack
                        .iter()
                        .rev()
                        .map(|e| e.label.clone())
                        .collect(),
                    redo_labels: history
                        .redo_stack
                        .iter()
                        .rev()
                        .map(|e| e.label.clone())
                        .collect(),
                }
            };
        let mut out = Vec::new();
        if let Some(character) = self.character.as_ref() {
            out.push(entry(
                &self.character_source,
                character,
                &self.history,
                true,
            ));
        }
        for (source, character, history) in &self.parked {
            if !history.undo_stack.is_empty() || !history.redo_stack.is_empty() {
                out.push(entry(source, character, history, false));
            }
        }
        out
    }

    pub fn invalidate_feat_cache(&mut self) {
        self.feat_cache = None;
    }

    pub fn invalidate_module_info_cache(&mut self) {
        self.module_info_cache = None;
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.character.as_ref().is_some_and(Character::is_modified)
            || self.parked.iter().any(|(_, c, _)| c.is_modified())
            || !self.staged_party.is_empty()
            || !self.quest_edits.pending.is_empty()
    }

    pub fn character(&self) -> Option<&Character> {
        self.character.as_ref()
    }

    pub fn character_mut(&mut self) -> Option<&mut Character> {
        self.character.as_mut()
    }

    /// Make `ros_name` the active character. Pending edits on the previous
    /// character are parked in memory, not written; `save_character` flushes them.
    pub fn load_companion(&mut self, ros_name: &str) -> Result<(), String> {
        let target = CharacterSource::Companion {
            ros_name: ros_name.to_string(),
        };
        if self.character_source == target && self.character.is_some() {
            return Ok(());
        }
        if let Some((character, history)) = self.take_parked(&target) {
            self.activate(target, character, history);
            return Ok(());
        }
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let bytes = handler
            .extract_companion(ros_name)
            .map_err(|e| format!("Failed to extract companion '{ros_name}': {e}"))?;
        let gff = GffParser::from_bytes(bytes)
            .map_err(|e| format!("Failed to parse companion file: {e}"))?;
        let fields: IndexMap<String, GffValue<'static>> = gff
            .read_struct_fields(0)
            .map_err(|e| format!("Failed to read companion fields: {e}"))?
            .into_iter()
            .map(|(k, v)| (k, v.force_owned()))
            .collect();

        let character = Character::from_gff(fields);
        info!(
            "Companion loaded: {} ({ros_name}, level {})",
            character.full_name(),
            character.total_level()
        );
        self.activate(target, character, History::default());
        Ok(())
    }

    /// Read-only copy of `ros_name`'s character without changing the active
    /// source: the active or parked in-memory copy when present (unsaved edits
    /// included), else parsed fresh from the save. Used for portrait rendering.
    pub fn companion_snapshot(&self, ros_name: &str) -> Result<Character, String> {
        let target = CharacterSource::Companion {
            ros_name: ros_name.to_string(),
        };
        if self.character_source == target
            && let Some(character) = &self.character
        {
            return Ok(Character::from_gff(character.clone_gff()));
        }
        if let Some((_, character, _)) = self.parked.iter().find(|(s, _, _)| *s == target) {
            return Ok(Character::from_gff(character.clone_gff()));
        }
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let bytes = handler
            .extract_companion(ros_name)
            .map_err(|e| format!("Failed to extract companion '{ros_name}': {e}"))?;
        let gff = GffParser::from_bytes(bytes)
            .map_err(|e| format!("Failed to parse companion file: {e}"))?;
        let fields: IndexMap<String, GffValue<'static>> = gff
            .read_struct_fields(0)
            .map_err(|e| format!("Failed to read companion fields: {e}"))?
            .into_iter()
            .map(|(k, v)| (k, v.force_owned()))
            .collect();
        Ok(Character::from_gff(fields))
    }

    /// Make the player the active character again, restoring in-memory edits if
    /// the player was parked, else re-reading it from the save.
    pub fn load_player(&mut self) -> Result<(), String> {
        if self.character_source == CharacterSource::Player && self.character.is_some() {
            return Ok(());
        }
        if let Some((character, history)) = self.take_parked(&CharacterSource::Player) {
            self.activate(CharacterSource::Player, character, history);
            return Ok(());
        }
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let fields = read_player_fields(
            handler,
            self.selected_player_index,
            self.primary_player_index,
        )?;
        let character = Character::from_gff(fields);
        info!(
            "Player reloaded: {} (level {})",
            character.full_name(),
            character.total_level()
        );
        self.activate(CharacterSource::Player, character, History::default());
        Ok(())
    }

    /// The player character wherever it currently lives (active or parked).
    fn player_character_mut(&mut self) -> Option<&mut Character> {
        if self.character_source == CharacterSource::Player {
            return self.character.as_mut();
        }
        self.parked
            .iter_mut()
            .find(|(source, _, _)| *source == CharacterSource::Player)
            .map(|(_, character, _)| character)
    }

    fn take_parked(&mut self, source: &CharacterSource) -> Option<(Character, History)> {
        let idx = self.parked.iter().position(|(s, _, _)| s == source)?;
        let (_, character, history) = self.parked.swap_remove(idx);
        Some((character, history))
    }

    /// Park the current character and its history (if any) and install
    /// `character` as active. Same save, so module/quest caches stay;
    /// character-scoped state does not.
    fn activate(&mut self, source: CharacterSource, character: Character, history: History) {
        if let Some(previous) = self.character.take() {
            let previous_source = self.character_source.clone();
            self.parked.retain(|(s, _, _)| *s != previous_source);
            let previous_history = std::mem::take(&mut self.history);
            self.parked
                .push((previous_source, previous, previous_history));
        }
        self.character = Some(character);
        self.character_source = source;
        self.feat_cache = None;
        self.history = history;
    }

    pub fn list_roster(&self) -> Result<Vec<RosterListing>, String> {
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let Some((_, bytes)) = handler
            .extract_roster()
            .map_err(|e| format!("Failed to read roster: {e}"))?
        else {
            warn!("No roster file found in save; companion list empty");
            return Ok(Vec::new());
        };
        let members = match crate::services::roster::parse_roster_members(bytes) {
            Ok(m) => m,
            Err(e) => {
                warn!("Roster file unparseable, treating as empty: {e}");
                return Ok(Vec::new());
            }
        };
        let companion_files = handler
            .list_companions()
            .map_err(|e| format!("Failed to list companions: {e}"))?;

        // Neither globals.xml `*_Joined` vars nor the RosAvailable/RosSelectable/
        // RosCampaignNPC flags identify a companion: the game skips the vars for
        // some companions and clears all three flags on an active party member.
        Ok(members
            .into_iter()
            .filter(|m| {
                companion_files
                    .iter()
                    .any(|c| c.eq_ignore_ascii_case(&m.ros_name))
            })
            .map(|m| RosterListing {
                staged_in_party: self
                    .staged_party
                    .get(&m.ros_name.to_ascii_lowercase())
                    .copied(),
                in_party: !m.available,
                ros_name: m.ros_name,
                char_name: m.char_name,
                classes: m.classes,
                gender: m.gender,
                subrace: m.subrace,
            })
            .collect())
    }

    pub fn party_limit(&self) -> Result<Option<i32>, String> {
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let Some((_, bytes)) = handler
            .extract_roster()
            .map_err(|e| format!("Failed to read roster: {e}"))?
        else {
            return Ok(None);
        };
        crate::services::roster::parse_party_limit(bytes)
    }

    /// Stage the desired party composition in the session. Nothing touches
    /// disk here: only entries differing from the file are remembered, and
    /// `save_character` flushes them via [`Self::apply_party_membership`].
    /// Staging a member back to their on-disk state un-stages them.
    pub fn stage_party_membership(&mut self, desired: &[(String, bool)]) -> Result<(), String> {
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let Some((_, bytes)) = handler
            .extract_roster()
            .map_err(|e| format!("Failed to read roster: {e}"))?
        else {
            return Err("Save has no roster file".into());
        };
        let members = crate::services::roster::parse_roster_members(bytes)?;
        for (name, in_party) in desired {
            let Some(member) = members
                .iter()
                .find(|m| m.ros_name.eq_ignore_ascii_case(name))
            else {
                return Err(format!("roster member not found: {name}"));
            };
            let key = member.ros_name.to_ascii_lowercase();
            if member.available == *in_party {
                self.staged_party.insert(key, *in_party);
            } else {
                self.staged_party.remove(&key);
            }
        }
        Ok(())
    }

    /// Write any staged party changes to the save (or the open batch). The
    /// caller clears `staged_party` once the write is committed. Returns a
    /// warning string when the roster was written but the faction mirror
    /// partially failed.
    fn flush_staged_party(&mut self) -> Result<Option<String>, String> {
        if self.staged_party.is_empty() {
            return Ok(None);
        }
        let desired: Vec<(String, bool)> = self
            .staged_party
            .iter()
            .map(|(name, in_party)| (name.clone(), *in_party))
            .collect();
        self.apply_party_membership(&desired)
    }

    fn apply_party_membership(
        &mut self,
        desired: &[(String, bool)],
    ) -> Result<Option<String>, String> {
        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or("No active save handler")?;
        crate::services::roster::apply_party_membership(handler, desired)
    }

    #[instrument(name = "SessionState::export_to_localvault", skip(self, paths))]
    pub fn export_to_localvault(
        &self,
        paths: &crate::config::nwn2_paths::NWN2Paths,
    ) -> Result<String, String> {
        if self.character_source != CharacterSource::Player {
            return Err("Export to local vault is only available for the player character".into());
        }

        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        let character = self.character.as_ref().ok_or("No character loaded")?;

        let vault_path = paths
            .localvault()
            .ok_or("Could not determine NWN2 localvault path")?;

        if !vault_path.exists() {
            std::fs::create_dir_all(&vault_path)
                .map_err(|e| format!("Failed to create localvault directory: {e}"))?;
        }

        let player_bic_data = handler
            .extract_player_bic()
            .map_err(|e| format!("Failed to extract player.bic: {e}"))?
            .ok_or("No player.bic found in save")?;

        let first_name = character.first_name();
        let last_name = character.last_name();
        let filename = if last_name.is_empty() {
            format!("{first_name}.bic")
        } else {
            format!("{first_name} {last_name}.bic")
        };

        let sanitized_filename = filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '.' || *c == '-' || *c == '_')
            .collect::<String>();

        let dest_path = vault_path.join(&sanitized_filename);

        std::fs::write(&dest_path, &player_bic_data)
            .map_err(|e| format!("Failed to write character to vault: {e}"))?;

        info!("Exported character to vault: {}", dest_path.display());

        Ok(dest_path.to_string_lossy().to_string())
    }
}

fn playerinfo_for_character(
    save_dir: &Path,
    character: &Character,
    game_data: &GameData,
) -> Result<Vec<u8>, String> {
    let playerinfo_path = save_dir.join("playerinfo.bin");

    let mut player_info = if playerinfo_path.exists() {
        PlayerInfo::load(&playerinfo_path)
            .map_err(|e| format!("Failed to read playerinfo.bin: {e}"))?
    } else {
        PlayerInfo::new()
    };

    // playerinfo.bin's subrace field is the load-menu display text; NWN2 matches
    // the icon by TLK name, so resolve labels/indices through race_display_name.
    let subrace_label = character.race_display_name(game_data);
    let alignment_name = character.alignment().alignment_string();
    let classes = character
        .class_entries()
        .into_iter()
        .map(|entry| {
            let level = entry.level.clamp(0, i32::from(u8::MAX)) as u8;
            (character.get_class_name(entry.class_id, game_data), level)
        })
        .collect::<Vec<_>>();

    player_info.update_from_gff_data(character.gff(), &subrace_label, &alignment_name, &classes);
    let mut bytes = Vec::new();
    player_info.write(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn describe_source(source: &CharacterSource) -> String {
    match source {
        CharacterSource::Player => "player".into(),
        CharacterSource::Companion { ros_name } => ros_name.clone(),
        CharacterSource::Standalone => "standalone character".into(),
    }
}

fn sync_roster(
    handler: &mut SaveGameHandler,
    ros_name: &str,
    classes: &[(i32, i32)],
    identity: crate::services::roster::RosterIdentity,
) -> Result<(), String> {
    let Some((stored_name, bytes)) = handler.extract_roster().map_err(|e| e.to_string())? else {
        return Err("no roster file in save".into());
    };
    match crate::services::roster::sync_member(bytes, ros_name, classes, identity)? {
        Some(new_bytes) => handler
            .update_file(&stored_name, &new_bytes)
            .map_err(|e| e.to_string()),
        None => Err(format!("no roster entry named '{ros_name}'")),
    }
}

fn serialize_playerlist_bytes(
    playerlist_data: &[u8],
    character_fields: &IndexMap<String, GffValue<'static>>,
    player_index: usize,
    companion_prefix: Option<&str>,
) -> Result<Vec<u8>, String> {
    let gff = GffParser::from_bytes(playerlist_data.to_vec())
        .map_err(|e| format!("playerlist.ifo parse error: {e}"))?;

    let file_type = gff.file_type.clone();
    let file_version = gff.file_version.clone();

    let mut root_fields: IndexMap<String, GffValue<'static>> = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read playerlist.ifo root struct: {e}"))?
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    let Some(GffValue::ListOwned(players)) = root_fields.get_mut("Mod_PlayerList") else {
        return Err("playerlist.ifo Mod_PlayerList is missing or not a list".to_string());
    };

    let players_len = players.len();
    let player_entry = players.get_mut(player_index).ok_or_else(|| {
        format!(
            "Selected player index {player_index} is out of range for Mod_PlayerList with {players_len} entries"
        )
    })?;

    let mut merged = player_entry.clone();
    for (key, value) in character_fields {
        merged.insert(key.clone(), value.clone());
    }
    let is_journal = |entry: &IndexMap<String, GffValue<'static>>| matches!(entry.get("Name"), Some(GffValue::String(name)) if name.starts_with("NW_JOURNAL_"));
    if let Some(GffValue::ListOwned(saved_vars)) = player_entry.get("VarTable") {
        let vars = merged
            .entry("VarTable".into())
            .or_insert_with(|| GffValue::ListOwned(Vec::new()));
        if let GffValue::ListOwned(vars) = vars {
            vars.retain(|entry| !is_journal(entry));
            vars.extend(saved_vars.iter().filter(|entry| is_journal(entry)).cloned());
        }
    }
    if let Some(prefix) = companion_prefix {
        crate::character::pets::sync_mod_companion_variable(&mut merged, prefix);
    }
    *player_entry = merged;

    GffWriter::new(&file_type, &file_version)
        .write(root_fields)
        .map_err(|e| format!("playerlist.ifo serialization error: {e}"))
}

/// Player fields for `selected_index`: player.bic when that slot is the primary
/// player and the bic is readable, else the playerlist slot.
fn read_player_fields(
    handler: &SaveGameHandler,
    selected_index: usize,
    primary_index: Option<usize>,
) -> Result<IndexMap<String, GffValue<'static>>, String> {
    let playerlist_data = handler
        .extract_player_data()
        .map_err(|e| format!("Failed to extract playerlist.ifo: {e}"))?;
    let gff =
        GffParser::from_bytes(playerlist_data).map_err(|e| format!("GFF parse error: {e}"))?;
    let entries = read_playerlist_entries(gff)?;
    if primary_index == Some(selected_index)
        && let Ok(Some(bic)) = handler.extract_player_bic()
        && let Ok(fields) = read_player_bic_entry(bic)
    {
        return Ok(fields);
    }
    read_playerlist_entry_from_entries(&entries, selected_index)
}

/// FactionID a companion's `.ros` should carry per the roster's membership
/// record; `None` when the roster or playerlist can't be read.
fn companion_faction_from_roster(handler: &SaveGameHandler, ros_name: &str) -> Option<u16> {
    let (_, bytes) = handler.extract_roster().ok()??;
    let members = crate::services::roster::parse_roster_members(bytes).ok()?;
    let member = members
        .iter()
        .find(|m| m.ros_name.eq_ignore_ascii_case(ros_name))?;
    if member.available {
        Some(crate::services::roster::BENCHED_FACTION)
    } else {
        crate::services::roster::pc_party_faction(handler)
    }
}

pub(crate) fn read_playerlist_entries(
    gff: Arc<GffParser>,
) -> Result<Vec<IndexMap<String, GffValue<'static>>>, String> {
    debug!("Reading playerlist.ifo root struct");
    let root_fields = gff.read_struct_fields(0).map_err(|e| {
        warn!("Failed to read root struct: {}", e);
        format!("Failed to read root struct: {e}")
    })?;

    let mod_player_list = root_fields.get("Mod_PlayerList").ok_or_else(|| {
        warn!("Mod_PlayerList not found in playerlist.ifo");
        "Mod_PlayerList not found in playerlist.ifo".to_string()
    })?;

    if let GffValue::List(lazy_structs) = mod_player_list {
        if lazy_structs.is_empty() {
            warn!("Mod_PlayerList is empty");
            return Err("Mod_PlayerList is empty".to_string());
        }

        Ok(lazy_structs
            .iter()
            .map(|entry| entry.force_load())
            .collect())
    } else {
        warn!("Mod_PlayerList is not a list");
        Err("Mod_PlayerList is not a list".to_string())
    }
}

fn read_playerlist_entry_from_entries(
    entries: &[IndexMap<String, GffValue<'static>>],
    player_index: usize,
) -> Result<IndexMap<String, GffValue<'static>>, String> {
    entries.get(player_index).cloned().ok_or_else(|| {
        format!(
            "Selected player index {player_index} is out of range for Mod_PlayerList with {} entries",
            entries.len()
        )
    })
}

pub(crate) fn read_player_bic_entry(
    player_bic_data: Vec<u8>,
) -> Result<IndexMap<String, GffValue<'static>>, String> {
    let gff = GffParser::from_bytes(player_bic_data).map_err(|e| {
        warn!("Failed to parse player.bic: {}", e);
        format!("Failed to parse player.bic: {e}")
    })?;

    let root_fields = gff.read_struct_fields(0).map_err(|e| {
        warn!("Failed to read player.bic root struct: {}", e);
        format!("Failed to read player.bic root struct: {e}")
    })?;

    Ok(root_fields
        .into_iter()
        .map(|(key, value)| (key, value.force_owned()))
        .collect())
}

pub(crate) fn resolve_primary_player_index(
    player_entries: &[IndexMap<String, GffValue<'static>>],
    player_bic_fields: Option<&IndexMap<String, GffValue<'static>>>,
) -> Option<usize> {
    if player_entries.len() == 1 {
        return Some(0);
    }

    let player_bic_fields = player_bic_fields?;

    let player_bic_name = Character::from_gff(player_bic_fields.clone()).full_name();
    if player_bic_name.trim().is_empty() {
        warn!("player.bic has no character name; refusing to infer a primary multiplayer slot");
        return None;
    }

    let matching_indices = player_entries
        .iter()
        .enumerate()
        .filter_map(|(index, fields)| {
            (Character::from_gff(fields.clone()).full_name() == player_bic_name).then_some(index)
        })
        .collect::<Vec<_>>();

    match matching_indices.as_slice() {
        [index] => Some(*index),
        [] => {
            warn!(
                "player.bic name '{}' did not match any Mod_PlayerList entry; refusing to infer a primary multiplayer slot",
                player_bic_name
            );
            None
        }
        _ => {
            warn!(
                "player.bic name '{}' matched multiple Mod_PlayerList entries; refusing to infer a primary multiplayer slot",
                player_bic_name
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::gff::GffValue;

    fn make_gff_with_str(str_val: i32) -> IndexMap<String, GffValue<'static>> {
        let mut map = IndexMap::new();
        map.insert("Str".to_string(), GffValue::Byte(str_val as u8));
        map
    }

    fn read_str(gff: &IndexMap<String, GffValue<'static>>) -> i32 {
        match gff.get("Str") {
            Some(GffValue::Byte(v)) => i32::from(*v),
            _ => -1,
        }
    }

    fn make_session_with_character(gff: IndexMap<String, GffValue<'static>>) -> SessionState {
        let paths = Arc::new(tokio::sync::RwLock::new(
            crate::config::nwn2_paths::NWN2Paths::new(),
        ));
        let rm = Arc::new(tokio::sync::RwLock::new(
            crate::services::resource_manager::ResourceManager::new(paths),
        ));
        let mut session = SessionState::new(rm);
        session.character = Some(Character::from_gff(gff));
        session
    }

    fn quest_save_session() -> (tempfile::TempDir, SessionState, GameData) {
        use indexmap::indexmap;
        let dir = tempfile::tempdir().unwrap();
        let save = dir.path().join("save");
        std::fs::create_dir(&save).unwrap();
        let fields = indexmap! {
            "Str".into() => GffValue::Byte(10),
            "VarTable".into() => GffValue::ListOwned(vec![indexmap! {
                "Name".into() => GffValue::String("NW_JOURNAL_ENTRYtest".into()),
                "Type".into() => GffValue::Dword(1), "Value".into() => GffValue::Int(10),
            }]),
        };
        let root =
            indexmap! { "Mod_PlayerList".into() => GffValue::ListOwned(vec![fields.clone()]) };
        std::fs::write(
            save.join("playerlist.ifo"),
            GffWriter::new("IFO ", "V3.2").write(root).unwrap(),
        )
        .unwrap();
        std::fs::write(
            save.join("player.bic"),
            GffWriter::new("BIC ", "V3.2")
                .write(fields.clone())
                .unwrap(),
        )
        .unwrap();
        let mut session = make_session_with_character(fields);
        session.save_dir = Some(save.clone());
        session.savegame_handler = Some(SaveGameHandler::new(save, false, false).unwrap());
        session.primary_player_index = Some(0);
        session
            .quest_edits
            .pending
            .push(crate::services::save_graph::editing::PendingQuestEdit {
                tag: "test".into(),
                target: 20,
                saved_state: Some(10),
                source: None,
                writes: Vec::new(),
                blueprints: std::collections::BTreeMap::new(),
            });
        let data = GameData::new(Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::new(),
        )));
        (dir, session, data)
    }

    #[test]
    fn quest_save_survives_later_character_saves_and_undo() {
        let (_dir, mut session, data) = quest_save_session();
        session.record_history("Before character edit");
        session
            .character
            .as_mut()
            .unwrap()
            .gff_mut()
            .insert("Str".into(), GffValue::Byte(12));
        assert!(session.has_unsaved_changes());
        session.save_character(&data, 5).unwrap();
        assert!(!session.has_unsaved_changes());
        assert!(session.quest_edits.pending.is_empty());
        session.undo();
        session.character.as_mut().unwrap().mark_modified();
        session.save_character(&data, 5).unwrap();
        let journal = crate::services::save_graph::read_live_journal(
            session.savegame_handler.as_ref().unwrap(),
            0,
        )
        .unwrap();
        assert_eq!(journal[0].state, 20);
    }

    #[test]
    fn quest_save_targets_primary_player_when_companion_is_selected() {
        let (_dir, mut session, data) = quest_save_session();
        session.character_source = CharacterSource::Companion {
            ros_name: "companion".into(),
        };
        session.character = Some(Character::from_gff(make_gff_with_str(10)));
        session.save_character(&data, 5).unwrap();
        let journal = crate::services::save_graph::read_live_journal(
            session.savegame_handler.as_ref().unwrap(),
            0,
        )
        .unwrap();
        assert_eq!(journal[0].state, 20);
    }

    #[test]
    fn failed_quest_preparation_keeps_character_changes_and_pending_edits() {
        let (_dir, mut session, data) = quest_save_session();
        let handler = session.savegame_handler.as_ref().unwrap();
        let before = handler.extract_player_data().unwrap();
        session.quest_edits.pending[0].target = u32::MAX;
        session
            .character
            .as_mut()
            .unwrap()
            .gff_mut()
            .insert("Str".into(), GffValue::Byte(12));
        assert!(session.save_character(&data, 5).is_err());
        assert!(session.has_unsaved_changes());
        assert_eq!(session.quest_edits.pending.len(), 1);
        assert_eq!(
            session
                .savegame_handler
                .as_ref()
                .unwrap()
                .extract_player_data()
                .unwrap(),
            before
        );
        session.quest_edits.pending[0].target = 20;
        session.save_character(&data, 5).unwrap();
        assert!(!session.has_unsaved_changes());
    }

    #[test]
    fn undo_redo_round_trips_attribute_change() {
        let initial_gff = make_gff_with_str(10);
        let mut session = make_session_with_character(initial_gff);

        session.record_history("Set STR to 18");

        let mutated = make_gff_with_str(18);
        session
            .character
            .as_mut()
            .unwrap()
            .restore_snapshot(mutated, true);

        assert_eq!(read_str(session.character.as_ref().unwrap().gff()), 18);

        let label = session.undo();
        assert_eq!(label.as_deref(), Some("Set STR to 18"));
        assert_eq!(read_str(session.character.as_ref().unwrap().gff()), 10);

        let label = session.redo();
        assert_eq!(label.as_deref(), Some("Set STR to 18"));
        assert_eq!(read_str(session.character.as_ref().unwrap().gff()), 18);
    }

    #[test]
    fn redo_cleared_on_new_history() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_history("first");
        session.undo();
        assert_eq!(session.history.redo_stack.len(), 1);

        session.record_history("second");
        assert_eq!(session.history.redo_stack.len(), 0);
    }

    #[test]
    fn clear_history_empties_both_stacks() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_history("op1");
        session.record_history("op2");
        session.undo();
        session.clear_history();

        assert!(session.history.undo_stack.is_empty());
        assert!(session.history.redo_stack.is_empty());
    }

    #[test]
    fn history_limit_enforced() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        for i in 0..=HISTORY_LIMIT {
            session.record_history(format!("op{i}"));
        }

        assert_eq!(session.history.undo_stack.len(), HISTORY_LIMIT);
        assert_eq!(session.history.undo_stack.front().unwrap().label, "op1");
    }
}
