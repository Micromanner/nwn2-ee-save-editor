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
    pub history: History,
    pub character_source: CharacterSource,
    pub standalone_backup_done: bool,
    /// Characters edited earlier in this session but not currently active. Edits
    /// and undo history stay in memory across switches; `save_character` flushes
    /// all of them.
    parked: Vec<(CharacterSource, Character, History)>,
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
            history: History::default(),
            character_source: CharacterSource::Player,
            standalone_backup_done: false,
            parked: Vec::new(),
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
        self.clear_history();

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
        self.clear_history();

        info!("Standalone character loaded successfully");
        Ok(())
    }

    /// Write every modified character of this session (active and parked) to the save.
    pub fn save_character(
        &mut self,
        game_data: &GameData,
        backup_keep_count: usize,
    ) -> Result<Option<String>, String> {
        if self.character_source == CharacterSource::Standalone {
            return self.save_standalone_inner(backup_keep_count);
        }

        let mut warnings = Vec::new();
        let mut active = self.character.take().ok_or("No character loaded")?;
        let active_source = self.character_source.clone();
        let result = self.save_one(&active_source, &mut active, game_data);
        self.character = Some(active);
        warnings.extend(result?);

        let parked = std::mem::take(&mut self.parked);
        let mut restored = Vec::with_capacity(parked.len());
        let mut first_error = None;
        for (source, mut character, history) in parked {
            if first_error.is_none() {
                match self.save_one(&source, &mut character, game_data) {
                    Ok(warning) => warnings.extend(warning),
                    Err(e) => first_error = Some(e),
                }
            }
            restored.push((source, character, history));
        }
        self.parked = restored;
        if let Some(e) = first_error {
            return Err(e);
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
            write_playerinfo_for_character(save_dir, character, game_data)?;
        }

        character.mark_saved();

        info!(
            "Character saved successfully (player.bic_updated={})",
            update_primary_player_files
        );
        Ok(None)
    }

    /// Save only the active companion. `save_character` is the normal path; this
    /// exists for callers that must not touch other parked characters.
    pub fn save_companion(&mut self, game_data: &GameData) -> Result<Option<String>, String> {
        let CharacterSource::Companion { ros_name } = &self.character_source else {
            return Err("Active character is not a companion".into());
        };
        let ros_name = ros_name.clone();
        let mut character = self.character.take().ok_or("No character loaded")?;
        let prefix = Self::companion_blueprint_prefix(&character, game_data);
        let result = self.save_companion_inner(&mut character, &ros_name, prefix.as_deref());
        self.character = Some(character);
        result
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

        character.mark_saved();
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
        self.history.undo_stack.push_back(entry);
        if self.history.undo_stack.len() > HISTORY_LIMIT {
            self.history.undo_stack.pop_front();
        }
        self.history.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<String> {
        let entry = self.history.undo_stack.pop_back()?;
        let label = entry.label.clone();
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
                ros_name: m.ros_name,
                char_name: m.char_name,
                classes: m.classes,
            })
            .collect())
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

fn write_playerinfo_for_character(
    save_dir: &Path,
    character: &Character,
    game_data: &GameData,
) -> Result<(), String> {
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
    player_info
        .save(&playerinfo_path)
        .map_err(|e| format!("Failed to write playerinfo.bin: {e}"))?;

    Ok(())
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
