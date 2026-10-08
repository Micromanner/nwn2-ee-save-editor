use indexmap::IndexMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

use crate::character::{Character, FeatInfo};
use crate::commands::error::CommandError;
use crate::loaders::GameData;
use crate::parsers::gff::{GffParser, GffValue, GffWriter};
use crate::services::PlayerInfo;
use crate::services::campaign::content::{ModuleInfo, ModuleVariables};
use crate::services::item_property_decoder::ItemPropertyDecoder;
use crate::services::load_diagnostics::{LoadError, LoadReport, LoadStage};
use crate::services::save_graph::SaveGraph;
use crate::services::save_graph::editing::{self, PendingQuestEdit, QuestPreview};
use crate::services::savegame_handler::backup_manifest::{
    self, CharacterChanges, PartyChange, QuestChange, SaveRecord, VariableChange,
};
use crate::services::savegame_handler::{PlayerOutputs, SAVENAME_TXT, SaveFormat, SaveGameHandler};
use crate::state::standalone_bic;

const HISTORY_LIMIT: usize = 100;

pub struct HistoryEntry {
    pub label: String,
    gff_snapshot: IndexMap<String, GffValue<'static>>,
    modified_snapshot: bool,
    /// The quest edit this entry staged (undo stack) or took back (redo stack).
    quest: Option<PendingQuestEdit>,
}

/// The loaded player and its undo snapshots with removed quest edits taken back,
/// not yet in place.
struct QuestRevert {
    player: Character,
    /// Per undo entry: the snapshot with the changes taken back, `None` = unchanged.
    snapshots: Vec<Option<Character>>,
    /// Whether the player stays modified once the edits are gone.
    modified: bool,
}

/// Whether an undo entry staged one of `edits`.
fn staged_one_of(entry: &HistoryEntry, edits: &[PendingQuestEdit]) -> bool {
    entry
        .quest
        .as_ref()
        .is_some_and(|q| edits.iter().any(|edit| edit.tag == q.tag))
}

/// Undo/redo stacks of one character. Parked together with the character so
/// switching party members keeps each one's history.
#[derive(Default)]
pub struct History {
    pub undo_stack: VecDeque<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
}

impl History {
    /// Record a new edit: older entries beyond the limit drop off, redo is lost.
    fn push(&mut self, entry: HistoryEntry) {
        self.undo_stack.push_back(entry);
        if self.undo_stack.len() > HISTORY_LIMIT {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    /// Undo labels newest first.
    pub fn undo_labels(&self) -> Vec<String> {
        self.undo_stack
            .iter()
            .rev()
            .map(|e| e.label.clone())
            .collect()
    }

    /// Redo labels nearest first.
    pub fn redo_labels(&self) -> Vec<String> {
        self.redo_stack
            .iter()
            .rev()
            .map(|e| e.label.clone())
            .collect()
    }
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
    /// Bumped whenever the loaded save changes, so background work started for
    /// an earlier load can tell its result is stale.
    pub load_generation: u64,
    /// Bumped whenever the active character changes (edit, undo, switch, load),
    /// so caches built in the background can tell they are stale.
    pub character_revision: u64,
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
    /// Staged `savename.txt` content, written by `save_character`. The save folder
    /// is never renamed.
    staged_save_name: Option<String>,
    /// Every history label of this session in order, never popped: undo and
    /// redo append their own entries. Feeds the backup manifests on save.
    change_log: Vec<(CharacterSource, String)>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    #[instrument(name = "SessionState::new", skip_all)]
    pub fn new() -> Self {
        info!("SessionState created successfully");

        Self {
            current_file_path: None,
            save_dir: None,
            savegame_handler: None,
            character: None,
            selected_player_index: 0,
            primary_player_index: None,
            item_property_decoder: ItemPropertyDecoder::default(),
            feat_cache: None,
            module_info_cache: None,
            quest_graph_cache: None,
            load_generation: 0,
            character_revision: 0,
            quest_edits: crate::services::save_graph::editing::QuestEdits::default(),
            history: History::default(),
            character_source: CharacterSource::Player,
            standalone_backup_done: false,
            parked: Vec::new(),
            staged_party: std::collections::HashMap::new(),
            staged_save_name: None,
            change_log: Vec::new(),
        }
    }

    #[instrument(name = "SessionState::load_character", skip(self, report), fields(file_path = %file_path))]
    pub fn load_character(
        &mut self,
        file_path: &str,
        player_index: Option<usize>,
        create_load_backup: bool,
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

        debug!("Creating SaveGameHandler");
        let mut handler = SaveGameHandler::new(&path).map_err(|e| {
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
        let character = Character::from_gff(fields).map_err(|e| {
            fatal(
                report,
                LoadStage::Playerlist,
                format!("Failed to read character data: {e}"),
                serde_json::json!({ "selected_player_index": selected_player_index }),
            )
        })?;
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

        if create_load_backup && let Err(e) = handler.create_load_backup() {
            return Err(fatal(
                report,
                LoadStage::SaveOpen,
                format!("Failed to back up save: {e}"),
                serde_json::json!({ "file_path": file_path }),
            ));
        }

        self.character = Some(character);
        self.savegame_handler = Some(handler);
        self.current_file_path = Some(path);
        self.save_dir = Some(save_dir);
        self.reset_save_caches();
        self.selected_player_index = selected_player_index;
        self.primary_player_index = primary_player_index;
        self.character_source = CharacterSource::Player;
        self.parked.clear();
        self.staged_party.clear();
        self.staged_save_name = None;
        self.quest_edits.clear();
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

        let bic_error = |report: &mut LoadReport, message: String| {
            let context = serde_json::json!({ "file_path": path.display().to_string() });
            fatal(report, LoadStage::Bic, message, context)
        };

        let fields =
            standalone_bic::load_fields(path).map_err(|message| bic_error(report, message))?;

        let character =
            Character::from_gff(fields).map_err(|e| bic_error(report, e.to_string()))?;
        info!(
            "Standalone character created: {} (Level {})",
            character.full_name(),
            character.total_level()
        );

        self.character = Some(character);
        self.savegame_handler = None;
        self.current_file_path = Some(path.to_path_buf());
        self.save_dir = None;
        self.reset_save_caches();
        self.selected_player_index = 0;
        self.primary_player_index = None;
        self.character_source = CharacterSource::Standalone;
        self.standalone_backup_done = false;
        self.parked.clear();
        self.staged_party.clear();
        self.staged_save_name = None;
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
    /// roll back together; nothing is marked saved until the batch commits. A
    /// successful save clears every undo history: older snapshots no longer
    /// match the save on disk.
    pub fn save_character(
        &mut self,
        game_data: &GameData,
        paths: &crate::config::NWN2Paths,
        backup_keep_count: usize,
    ) -> Result<Option<String>, String> {
        if self.character_source == CharacterSource::Standalone {
            return self.save_standalone_inner(backup_keep_count);
        }

        let player_index = self
            .primary_player_index
            .unwrap_or(self.selected_player_index);
        if !self.quest_edits.pending.is_empty() {
            let handler = self.savegame_handler.as_ref().ok_or("No save loaded")?;
            self.quest_edits
                .validate_saved(handler, paths, player_index)?;
        }
        let record = self.build_save_record(game_data);
        self.savegame_handler
            .as_mut()
            .ok_or("No active save handler")?
            .begin_batch();
        // Whatever happens while staging, the batch must not stay open: later
        // writes would be staged into it and never committed.
        let staged = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.stage_save_writes(game_data, paths, player_index)
        }));
        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or("No active save handler")?;
        let (rewarded_fields, flushed_party, warnings) = match staged {
            Ok(Ok(staged)) => staged,
            Ok(Err(e)) => {
                handler.abort_batch();
                return Err(e);
            }
            Err(panic) => {
                handler.abort_batch();
                std::panic::resume_unwind(panic);
            }
        };
        handler
            .commit_batch()
            .map_err(|e| format!("Failed to write save file: {e}"))?;

        for character in self
            .character
            .iter_mut()
            .chain(self.parked.iter_mut().map(|(_, character, _)| character))
        {
            character.mark_saved();
        }
        // Quest rewards landed in the save on top of the serialized player; mirror them
        // into the loaded player so the character tabs match without a reload.
        let rewarded = !rewarded_fields.player.is_empty() || !rewarded_fields.companions.is_empty();
        if !rewarded_fields.player.is_empty()
            && player_index == self.selected_player_index
            && let Some(player) = self.player_character_mut()
        {
            for (name, value) in rewarded_fields.player {
                player.gff_mut().insert(name, value);
            }
            player.mark_saved();
        }
        // Same for companions loaded in this session, or their next save would put
        // the pre-quest alignment and feats back.
        for (ros_name, fields) in rewarded_fields.companions {
            let matches = |source: &CharacterSource| matches!(source, CharacterSource::Companion { ros_name: name } if name.eq_ignore_ascii_case(&ros_name));
            let loaded = self
                .parked
                .iter_mut()
                .filter(|(source, _, _)| matches(source))
                .map(|(_, character, _)| character)
                .chain(
                    matches(&self.character_source)
                        .then_some(self.character.as_mut())
                        .flatten(),
                );
            for character in loaded {
                for (name, value) in &fields {
                    character.gff_mut().insert(name.clone(), value.clone());
                }
                character.mark_saved();
            }
        }
        if rewarded {
            self.on_character_changed();
        }
        if flushed_party {
            self.staged_party.clear();
        }
        self.staged_save_name = None;
        self.quest_edits.clear();
        self.module_info_cache = None;
        self.append_to_load_backup(&record);
        self.clear_history();

        Ok((!warnings.is_empty()).then(|| warnings.join("\n")))
    }

    /// Stage every write of a save into the open batch: all modified characters
    /// (each attempted even when one fails), staged party changes, then quest
    /// edits, save name and cheat flags once everything before succeeded.
    fn stage_save_writes(
        &mut self,
        game_data: &GameData,
        paths: &crate::config::NWN2Paths,
        player_index: usize,
    ) -> Result<(editing::FlushedWrites, bool, Vec<String>), String> {
        let slot = PlayerSlot {
            selected_index: self.selected_player_index,
            primary_index: self.primary_player_index,
        };
        let active = self.character.as_mut().ok_or("No character loaded")?;
        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or("No active save handler")?;

        let mut warnings = Vec::new();
        let mut errors = Vec::new();
        let characters = std::iter::once((&self.character_source, active)).chain(
            self.parked
                .iter_mut()
                .map(|(source, character, _)| (&*source, character)),
        );
        for (source, character) in characters {
            match save_one(handler, &slot, source, character, game_data) {
                Ok(warning) => warnings.extend(warning),
                Err(e) => errors.push(format!("{}: {e}", describe_source(source))),
            }
        }

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
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }

        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or("No active save handler")?;
        let mut rewarded_fields = self.quest_edits.flush(
            handler,
            paths,
            player_index,
            self.primary_player_index.is_some(),
            game_data,
        )?;
        warnings.append(&mut rewarded_fields.warnings);
        warnings.extend(
            crate::services::roster::remove_staged_companion_area_copies(handler)
                .map_err(|e| format!("companion area copies: {e}"))?,
        );
        if let Some(name) = &self.staged_save_name {
            handler
                .update_file(SAVENAME_TXT, name.as_bytes())
                .map_err(|e| format!("{SAVENAME_TXT}: {e}"))?;
        }
        strip_cheat_markers(handler).map_err(|e| format!("globals.xml: {e}"))?;
        Ok((rewarded_fields, flushed_party, warnings))
    }

    fn save_standalone_inner(
        &mut self,
        backup_keep_count: usize,
    ) -> Result<Option<String>, String> {
        let path = self
            .current_file_path
            .clone()
            .ok_or("No character file loaded")?;
        let character = self.character.as_mut().ok_or("No character loaded")?;

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
        character.mark_saved();
        self.standalone_backup_done = true;
        self.clear_history();
        Ok(None)
    }

    /// Drop everything derived from the previous save and invalidate builds
    /// still running against it.
    fn reset_save_caches(&mut self) {
        self.on_character_changed();
        self.module_info_cache = None;
        self.quest_graph_cache = None;
        self.load_generation += 1;
    }

    /// Drop caches derived from the active character and invalidate builds
    /// still running against it. Every path that changes the active character
    /// goes through here.
    fn on_character_changed(&mut self) {
        self.feat_cache = None;
        self.character_revision += 1;
    }

    pub fn close_character(&mut self) {
        self.character = None;
        self.savegame_handler = None;
        self.current_file_path = None;
        self.save_dir = None;
        self.selected_player_index = 0;
        self.primary_player_index = None;
        self.reset_save_caches();
        self.character_source = CharacterSource::Player;
        self.standalone_backup_done = false;
        self.parked.clear();
        self.staged_party.clear();
        self.staged_save_name = None;
        self.quest_edits.clear();
        self.clear_history();
    }

    /// Run `edit` on the loaded character as one undo entry. A failed edit is
    /// rolled back and records nothing.
    pub fn record_edit<T>(
        &mut self,
        label: impl Into<String>,
        edit: impl FnOnce(&mut Character) -> Result<T, CommandError>,
    ) -> Result<T, CommandError> {
        let character = self
            .character
            .as_mut()
            .ok_or(CommandError::NoCharacterLoaded)?;
        let gff_snapshot = character.clone_gff();
        let modified_snapshot = character.is_modified();
        match edit(character) {
            Ok(value) => {
                self.push_history(HistoryEntry {
                    label: label.into(),
                    gff_snapshot,
                    modified_snapshot,
                    quest: None,
                });
                Ok(value)
            }
            Err(e) => {
                character.restore_snapshot(gff_snapshot, modified_snapshot);
                Err(e)
            }
        }
    }

    fn push_history(&mut self, entry: HistoryEntry) {
        self.change_log
            .push((self.character_source.clone(), entry.label.clone()));
        self.history.push(entry);
        self.on_character_changed();
    }

    /// Stage a quest edit and apply its player changes (gold, XP, alignment, feats,
    /// items) to the loaded player as one undo entry: undo unstages the edit, redo
    /// stages it again. Edits it replaces are taken back first.
    pub fn stage_quest_edit(
        &mut self,
        preview: QuestPreview,
        graph: &SaveGraph,
        selected: &[usize],
        replace: bool,
        game_data: &GameData,
    ) -> Result<(), String> {
        let tag = preview.tag.clone();
        let replaced = self.quest_edits.stage(preview, graph, selected, replace)?;
        // Nothing changes on the player until both the take-back of the replaced edits
        // and the new edit's changes worked out.
        let staged = self.plan_quest_revert(&replaced).and_then(|revert| {
            let applied = self.apply_staged_quest_edit(revert.as_ref(), game_data)?;
            Ok((revert, applied))
        });
        let (revert, applied) = match staged {
            Ok(staged) => staged,
            Err(e) => {
                self.quest_edits.discard(Some(&tag));
                self.quest_edits.restore_all(replaced);
                return Err(e);
            }
        };
        if let Some(revert) = revert {
            self.commit_quest_revert(revert, &replaced);
        }
        if let Some((mut edit, changes, player)) = applied
            && let Some((character, history)) = self.player_with_history_mut()
        {
            let name = graph
                .quests
                .iter()
                .find(|q| q.tag == edit.tag)
                .map(|q| strip_toolset_prefix(&q.category.name))
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| edit.tag.clone());
            let label = format!("Quest {name}: entry {}", edit.target);
            edit.applied = Some(changes);
            history.push(HistoryEntry {
                label: label.clone(),
                gff_snapshot: character.clone_gff(),
                modified_snapshot: character.is_modified(),
                quest: Some(edit.clone()),
            });
            character.restore_snapshot(player.into_gff(), true);
            self.change_log.push((CharacterSource::Player, label));
            self.quest_edits.restore(edit);
            self.on_character_changed();
        }
        // A redo would restore an edit this stage replaced, past its conflict check.
        if let Some((_, history)) = self.player_with_history_mut() {
            history.redo_stack.clear();
        }
        Ok(())
    }

    /// The newest pending edit applied to a copy of the loaded player (as it is after
    /// `revert`, when given): the edit, its changes and the changed player. `None`
    /// when the edit has no player writes or the loaded player is not its target.
    fn apply_staged_quest_edit(
        &mut self,
        revert: Option<&QuestRevert>,
        game_data: &GameData,
    ) -> Result<Option<(PendingQuestEdit, Vec<editing::MemoryChange>, Character)>, String> {
        // The player loaded in memory must be the one the quest edits write to.
        if self
            .primary_player_index
            .unwrap_or(self.selected_player_index)
            != self.selected_player_index
        {
            return Ok(None);
        }
        let Some(edit) = self.quest_edits.pending.last().cloned() else {
            return Ok(None);
        };
        if !edit
            .writes
            .iter()
            .any(|w| editing::applies_to_player(&w.key.value_type))
        {
            return Ok(None);
        }
        let Some((character, _)) = self.player_with_history_mut() else {
            return Ok(None);
        };
        let base = revert.map_or_else(|| character.clone_gff(), |r| r.player.clone_gff());
        let mut player = Character::from_gff(base).map_err(|e| e.to_string())?;
        let changes = editing::apply_to_player(&edit, &mut player, game_data)?;
        Ok(Some((edit, changes, player)))
    }

    /// Unstage the pending quest edit of `tag` (all when `None`) and take its changes
    /// back off the loaded player.
    pub fn discard_quest_edits(&mut self, tag: Option<&str>) -> Result<(), String> {
        let removed = self.quest_edits.discard(tag);
        match self.plan_quest_revert(&removed) {
            Ok(Some(revert)) => self.commit_quest_revert(revert, &removed),
            Ok(None) => {}
            Err(e) => {
                self.quest_edits.restore_all(removed);
                return Err(e);
            }
        }
        self.on_character_changed();
        Ok(())
    }

    /// The player and its undo snapshots with the changes of removed edits taken back,
    /// built on copies; `None` when none of them changed the loaded player.
    fn plan_quest_revert(
        &mut self,
        edits: &[PendingQuestEdit],
    ) -> Result<Option<QuestRevert>, String> {
        if edits.iter().all(|edit| edit.applied.is_none()) {
            return Ok(None);
        }
        let Some((character, history)) = self.player_with_history_mut() else {
            return Ok(None);
        };
        let mut player = Character::from_gff(character.clone_gff()).map_err(|e| e.to_string())?;
        let mut snapshots: Vec<Option<Character>> = std::iter::repeat_with(|| None)
            .take(history.undo_stack.len())
            .collect();
        let is_removed = |entry: &HistoryEntry| staged_one_of(entry, edits);
        // Still modified unless the earliest removed edit was staged on an unmodified
        // player and nothing but removed edits came after it.
        let modified = history
            .undo_stack
            .iter()
            .position(is_removed)
            .is_none_or(|first| {
                history.undo_stack[first].modified_snapshot
                    || history
                        .undo_stack
                        .iter()
                        .skip(first)
                        .any(|e| !is_removed(e))
            });
        for edit in edits {
            let Some(changes) = &edit.applied else {
                continue;
            };
            editing::revert_on_player(changes, &mut player)?;
            let marker = history
                .undo_stack
                .iter()
                .position(|e| e.quest.as_ref().is_some_and(|q| q.tag == edit.tag));
            for (entry, snapshot) in history
                .undo_stack
                .iter()
                .zip(snapshots.iter_mut())
                .skip(marker.map_or(0, |i| i + 1))
            {
                let snapshot = match snapshot {
                    Some(snapshot) => snapshot,
                    None => snapshot.insert(
                        Character::from_gff(entry.gff_snapshot.clone())
                            .map_err(|e| e.to_string())?,
                    ),
                };
                editing::revert_on_player(changes, snapshot)?;
            }
        }
        Ok(Some(QuestRevert {
            player,
            snapshots,
            modified,
        }))
    }

    /// Put a planned revert in place: the player and the later undo snapshots lose the
    /// removed edits' changes, their own undo entries go away.
    fn commit_quest_revert(&mut self, revert: QuestRevert, edits: &[PendingQuestEdit]) {
        let Some((character, history)) = self.player_with_history_mut() else {
            return;
        };
        character.restore_snapshot(revert.player.into_gff(), revert.modified);
        for (entry, snapshot) in history.undo_stack.iter_mut().zip(revert.snapshots) {
            if let Some(snapshot) = snapshot {
                entry.gff_snapshot = snapshot.into_gff();
            }
        }
        history
            .undo_stack
            .retain(|entry| !staged_one_of(entry, edits));
        history.redo_stack.clear();
        self.on_character_changed();
    }

    /// The player and its undo history, whether active or parked.
    fn player_with_history_mut(&mut self) -> Option<(&mut Character, &mut History)> {
        if self.character_source == CharacterSource::Player {
            return Some((self.character.as_mut()?, &mut self.history));
        }
        self.parked
            .iter_mut()
            .find(|(source, _, _)| *source == CharacterSource::Player)
            .map(|(_, character, history)| (character, history))
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
            variables: Vec::new(),
        }
    }

    /// Game-state variables are written to disk immediately, so they are
    /// recorded on the load backup right away instead of on save.
    pub fn record_variable_changes(&self, variables: impl IntoIterator<Item = VariableChange>) {
        self.append_to_load_backup(&SaveRecord {
            saved_at: chrono::Local::now().to_rfc3339(),
            variables: variables.into_iter().collect(),
            ..Default::default()
        });
    }

    /// Describe changes written after the load backup in its manifest.
    fn append_to_load_backup(&self, record: &SaveRecord) {
        if record.is_empty() {
            return;
        }
        if let Some(dir) = self
            .savegame_handler
            .as_ref()
            .and_then(|h| h.load_backup_path())
        {
            backup_manifest::append_save(dir, record);
        }
    }

    pub fn undo(&mut self) -> Option<String> {
        let entry = self.history.undo_stack.pop_back()?;
        let label = entry.label.clone();
        self.change_log
            .push((self.character_source.clone(), format!("Undo: {label}")));
        // Undoing a quest entry unstages the quest edit; the snapshot takes its
        // changes back off the player.
        let quest = entry
            .quest
            .and_then(|edit| self.quest_edits.discard(Some(&edit.tag)).pop());
        let redo_entry = self.character.as_ref().map(|ch| HistoryEntry {
            label: label.clone(),
            gff_snapshot: ch.clone_gff(),
            modified_snapshot: ch.is_modified(),
            quest,
        });
        if let Some(character) = self.character.as_mut() {
            character.restore_snapshot(entry.gff_snapshot, entry.modified_snapshot);
        }
        if let Some(redo) = redo_entry {
            self.history.redo_stack.push(redo);
        }
        self.on_character_changed();
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let entry = self.history.redo_stack.pop()?;
        let label = entry.label.clone();
        self.change_log
            .push((self.character_source.clone(), format!("Redo: {label}")));
        if let Some(edit) = &entry.quest {
            self.quest_edits.restore(edit.clone());
        }
        let undo_entry = self.character.as_ref().map(|ch| HistoryEntry {
            label: label.clone(),
            gff_snapshot: ch.clone_gff(),
            modified_snapshot: ch.is_modified(),
            quest: entry.quest,
        });
        if let Some(character) = self.character.as_mut() {
            character.restore_snapshot(entry.gff_snapshot, entry.modified_snapshot);
        }
        if let Some(undo) = undo_entry {
            self.history.undo_stack.push_back(undo);
        }
        self.on_character_changed();
        Some(label)
    }

    /// Drop the undo history of every character of this session.
    pub fn clear_history(&mut self) {
        self.history = History::default();
        for (_, _, history) in &mut self.parked {
            *history = History::default();
        }
        self.change_log.clear();
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
                    undo_labels: history.undo_labels(),
                    redo_labels: history.redo_labels(),
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

    pub fn invalidate_module_info_cache(&mut self) {
        self.module_info_cache = None;
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.character.as_ref().is_some_and(Character::is_modified)
            || self.parked.iter().any(|(_, c, _)| c.is_modified())
            || !self.staged_party.is_empty()
            || !self.quest_edits.pending.is_empty()
            || self.staged_save_name.is_some()
    }

    /// The save's display name from `savename.txt`, staged edits included. `None`
    /// for original-format saves, which keep the name only in the folder name.
    pub fn save_name(&self) -> Option<String> {
        if let Some(name) = &self.staged_save_name {
            return Some(name.clone());
        }
        let handler = self.savegame_handler.as_ref()?;
        if handler.format() != SaveFormat::Ee {
            return None;
        }
        Some(saved_save_name(handler))
    }

    pub fn stage_save_name(&mut self, name: &str) -> Result<(), String> {
        let handler = self.savegame_handler.as_ref().ok_or("No save loaded")?;
        if handler.format() != SaveFormat::Ee {
            return Err("Original-format saves have no savename.txt".into());
        }
        let name = name.trim();
        if name.is_empty() {
            return Err("Save name cannot be empty".into());
        }
        self.staged_save_name = (name != saved_save_name(handler)).then(|| name.to_string());
        Ok(())
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
        let fields = gff
            .read_struct_fields(0)
            .map_err(|e| format!("Failed to read companion fields: {e}"))?;

        let character = Character::from_gff(fields).map_err(|e| e.to_string())?;
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
            return Character::from_gff(character.clone_gff()).map_err(|e| e.to_string());
        }
        if let Some((_, character, _)) = self.parked.iter().find(|(s, _, _)| *s == target) {
            return Character::from_gff(character.clone_gff()).map_err(|e| e.to_string());
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
        let fields = gff
            .read_struct_fields(0)
            .map_err(|e| format!("Failed to read companion fields: {e}"))?;
        Character::from_gff(fields).map_err(|e| e.to_string())
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
        let character = Character::from_gff(fields).map_err(|e| e.to_string())?;
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
        self.on_character_changed();
        self.history = history;
    }

    pub fn list_roster(&self) -> Result<Vec<RosterListing>, String> {
        let handler = self
            .savegame_handler
            .as_ref()
            .ok_or("No active save handler")?;
        Ok(crate::services::roster::companion_members(handler)?
            .into_iter()
            .map(|m| RosterListing {
                // Quest edits are written after the party tab's changes, so theirs win.
                staged_in_party: self
                    .quest_edits
                    .pending
                    .iter()
                    .flat_map(|p| &p.writes)
                    .rfind(|w| {
                        w.key.value_type == editing::PARTY_TYPE
                            && w.key.name.eq_ignore_ascii_case(&m.ros_name)
                    })
                    .map(|w| w.after.as_i64().unwrap_or(0) != 0)
                    .or_else(|| {
                        self.staged_party
                            .get(&m.ros_name.to_ascii_lowercase())
                            .copied()
                    }),
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
        let found = desired
            .iter()
            .map(|(name, in_party)| {
                members
                    .iter()
                    .find(|m| m.ros_name.eq_ignore_ascii_case(name))
                    .map(|member| (member, in_party))
                    .ok_or_else(|| format!("roster member not found: {name}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (member, in_party) in found {
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
    /// caller clears `staged_party` once the write is committed. Returns the
    /// warnings when the roster was written but the faction mirror partially
    /// failed.
    fn flush_staged_party(&mut self) -> Result<Vec<String>, String> {
        if self.staged_party.is_empty() {
            return Ok(Vec::new());
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
    ) -> Result<Vec<String>, String> {
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

        let saved_bic = handler
            .extract_player_bic()
            .map_err(|e| format!("Failed to extract player.bic: {e}"))?
            .ok_or("No player.bic found in save")?;
        // The character as the editor shows it, unsaved edits included.
        let player_bic_data = crate::parsers::gff::merge_fields_into_gff(
            Some(&saved_bic),
            character.gff(),
            "BIC ",
            false,
        )?;

        let first_name = character.first_name();
        let last_name = character.last_name();
        let filename = if last_name.is_empty() {
            format!("{first_name}.bic")
        } else {
            format!("{first_name} {last_name}.bic")
        };

        let mut sanitized_filename = filename
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '.' || *c == '-' || *c == '_')
            .collect::<String>();
        if !sanitized_filename
            .trim_end_matches(".bic")
            .chars()
            .any(char::is_alphanumeric)
        {
            sanitized_filename = "Character.bic".into();
        }

        let dest_path = vault_path.join(&sanitized_filename);

        std::fs::write(&dest_path, &player_bic_data)
            .map_err(|e| format!("Failed to write character to vault: {e}"))?;

        info!("Exported character to vault: {}", dest_path.display());

        Ok(dest_path.to_string_lossy().to_string())
    }
}

/// Where the player's files of the loaded save go.
struct PlayerSlot {
    selected_index: usize,
    primary_index: Option<usize>,
}

fn save_one(
    handler: &mut SaveGameHandler,
    slot: &PlayerSlot,
    source: &CharacterSource,
    character: &Character,
    game_data: &GameData,
) -> Result<Option<String>, String> {
    if !character.is_modified() {
        return Ok(None);
    }
    let companion_prefix = crate::character::pets::companion_blueprint_prefix(
        character.animal_companion_type(),
        game_data,
    );
    match source {
        CharacterSource::Companion { ros_name } => {
            save_companion(handler, character, ros_name, companion_prefix.as_deref())
        }
        CharacterSource::Player => save_player(
            handler,
            slot,
            character,
            companion_prefix.as_deref(),
            game_data,
        ),
        CharacterSource::Standalone => Err("Standalone character cannot be parked".into()),
    }
}

fn save_player(
    handler: &mut SaveGameHandler,
    slot: &PlayerSlot,
    character: &Character,
    companion_prefix: Option<&str>,
    game_data: &GameData,
) -> Result<Option<String>, String> {
    let char_fields = character.gff();
    let selected_player_index = slot.selected_index;
    let update_primary_player_files = slot.primary_index == Some(selected_player_index);

    handler
        .rewrite_player_files(|src| {
            let playerlist = serialize_playerlist_bytes(
                src.playerlist,
                char_fields,
                selected_player_index,
                companion_prefix,
            )?;
            let player_bic = if update_primary_player_files {
                Some(crate::parsers::gff::merge_fields_into_gff(
                    src.player_bic,
                    char_fields,
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
        let mut bytes = Vec::new();
        PlayerInfo::from_character(character, game_data)
            .write(&mut bytes)
            .map_err(|e| e.to_string())?;
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

fn save_companion(
    handler: &mut SaveGameHandler,
    character: &Character,
    ros_name: &str,
    companion_prefix: Option<&str>,
) -> Result<Option<String>, String> {
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
    let mut warnings = Vec::new();
    // roster.rst RosAvailable is the party-membership record; the engine
    // mirrors it into the .ros FactionID. Re-assert it from the roster so
    // a membership change made after this companion was parsed isn't
    // clobbered by the stale in-memory value.
    match companion_faction_from_roster(handler, ros_name) {
        Ok(Some(faction)) => crate::services::roster::set_faction_in_fields(&mut fields, faction),
        Ok(None) => {}
        Err(e) => {
            warn!("Companion faction not updated: {e}");
            warnings.push(format!(
                "Companion saved, but its party faction could not be updated: {e}"
            ));
        }
    }
    let bytes = crate::parsers::gff::merge_fields_into_gff(Some(&original), &fields, "ROS ", true)?;
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
    if let Err(e) = sync_roster(handler, ros_name, &classes, identity) {
        warn!("Roster sync failed after companion save: {e}");
        warnings.push(format!(
            "Companion saved, but updating the party roster failed: {e}"
        ));
    }

    info!("Companion saved: {stored_name}");
    Ok((!warnings.is_empty()).then(|| warnings.join("\n")))
}

fn describe_source(source: &CharacterSource) -> String {
    match source {
        CharacterSource::Player => "player".into(),
        CharacterSource::Companion { ros_name } => ros_name.clone(),
        CharacterSource::Standalone => "standalone character".into(),
    }
}

fn saved_save_name(handler: &SaveGameHandler) -> String {
    std::fs::read_to_string(handler.save_dir().join(SAVENAME_TXT))
        .map(|name| name.trim().to_string())
        .unwrap_or_default()
}

/// A journal category name without the toolset's `{1000}`-style prefixes.
fn strip_toolset_prefix(name: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in name.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// Every save clears the engine's cheat flag, so a save that ran quest console lines
/// earns achievements again once it passes through the editor.
fn strip_cheat_markers(handler: &mut SaveGameHandler) -> Result<(), String> {
    let Ok(xml) = handler.extract_globals_xml() else {
        return Ok(());
    };
    let parser = crate::parsers::xml::RustXmlParser::from_string(&xml)?;
    if !parser.has_cheat_markers() {
        return Ok(());
    }
    handler
        .update_file("globals.xml", parser.to_xml_string()?.as_bytes())
        .map_err(|e| e.to_string())
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

    let mut root_fields = gff
        .read_owned_struct(0)
        .map_err(|e| format!("Failed to read playerlist.ifo root struct: {e}"))?;

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
    if primary_index == Some(selected_index) {
        match handler.extract_player_bic() {
            Ok(Some(bic)) => match read_player_bic_entry(bic) {
                Ok(fields) => return Ok(fields),
                Err(e) => warn!("Failed to parse player.bic, using the playerlist slot: {e}"),
            },
            Ok(None) => {}
            Err(e) => warn!("Failed to extract player.bic, using the playerlist slot: {e}"),
        }
    }
    read_playerlist_entry_from_entries(&entries, selected_index)
}

/// FactionID a companion's `.ros` should carry per the roster's membership
/// record; `None` when the save has no roster or the roster has no such member.
fn companion_faction_from_roster(
    handler: &SaveGameHandler,
    ros_name: &str,
) -> Result<Option<u16>, String> {
    let Some((_, bytes)) = handler
        .extract_roster()
        .map_err(|e| format!("Failed to read roster: {e}"))?
    else {
        return Ok(None);
    };
    let members = crate::services::roster::parse_roster_members(bytes)?;
    let Some(member) = members
        .iter()
        .find(|m| m.ros_name.eq_ignore_ascii_case(ros_name))
    else {
        return Ok(None);
    };
    if member.available {
        Ok(Some(crate::services::roster::BENCHED_FACTION))
    } else {
        crate::services::roster::pc_party_faction(handler)
            .map(Some)
            .ok_or_else(|| "the player's party faction could not be read".into())
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

        lazy_structs
            .iter()
            .map(|entry| entry.force_load().map_err(|e| e.to_string()))
            .collect()
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

fn fatal(
    report: &mut LoadReport,
    stage: LoadStage,
    message: String,
    context: serde_json::Value,
) -> LoadError {
    warn!("{message}");
    report.set_fatal(stage, message.clone(), context.clone());
    LoadError {
        stage,
        message,
        context,
    }
}

pub(crate) fn read_player_bic_entry(
    player_bic_data: Vec<u8>,
) -> Result<IndexMap<String, GffValue<'static>>, String> {
    let gff = GffParser::from_bytes(player_bic_data).map_err(|e| {
        warn!("Failed to parse player.bic: {}", e);
        format!("Failed to parse player.bic: {e}")
    })?;

    gff.read_owned_struct(0).map_err(|e| {
        warn!("Failed to read player.bic root struct: {}", e);
        format!("Failed to read player.bic root struct: {e}")
    })
}

pub(crate) fn resolve_primary_player_index(
    player_entries: &[IndexMap<String, GffValue<'static>>],
    player_bic_fields: Option<&IndexMap<String, GffValue<'static>>>,
) -> Option<usize> {
    if player_entries.len() == 1 {
        return Some(0);
    }

    let player_bic_fields = player_bic_fields?;

    let player_bic_name = Character::from_gff(player_bic_fields.clone())
        .ok()?
        .full_name();
    if player_bic_name.trim().is_empty() {
        warn!("player.bic has no character name; refusing to infer a primary multiplayer slot");
        return None;
    }

    let matching_indices = player_entries
        .iter()
        .enumerate()
        .filter_map(|(index, fields)| {
            Character::from_gff(fields.clone())
                .is_ok_and(|c| c.full_name() == player_bic_name)
                .then_some(index)
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
        let mut session = SessionState::new();
        session.character = Some(Character::from_gff(gff).unwrap());
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
        session.savegame_handler = Some(SaveGameHandler::new(save).unwrap());
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
                applied: None,
            });
        let data = GameData::new(Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::new(),
        )));
        (dir, session, data)
    }

    #[test]
    fn save_name_writes_savename_txt_and_keeps_folder() {
        use indexmap::indexmap;
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let save = dir.path().join("000007 - Old Name");
        std::fs::create_dir(&save).unwrap();
        std::fs::write(save.join("savename.txt"), "Old Name").unwrap();
        let fields = indexmap! { "Str".into() => GffValue::Byte(10) };
        let root =
            indexmap! { "Mod_PlayerList".into() => GffValue::ListOwned(vec![fields.clone()]) };
        let mut zip = zip::ZipWriter::new(std::fs::File::create(save.join("resgff.zip")).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("playerlist.ifo", options).unwrap();
        zip.write_all(&GffWriter::new("IFO ", "V3.2").write(root).unwrap())
            .unwrap();
        zip.start_file("player.bic", options).unwrap();
        zip.write_all(
            &GffWriter::new("BIC ", "V3.2")
                .write(fields.clone())
                .unwrap(),
        )
        .unwrap();
        zip.finish().unwrap();

        let mut session = make_session_with_character(fields);
        session.save_dir = Some(save.clone());
        session.savegame_handler = Some(SaveGameHandler::new(save.clone()).unwrap());
        session.primary_player_index = Some(0);
        let data = GameData::new(Arc::new(std::sync::RwLock::new(
            crate::parsers::tlk::TLKParser::new(),
        )));

        assert_eq!(session.save_name().as_deref(), Some("Old Name"));
        assert!(session.stage_save_name("   ").is_err());
        session.stage_save_name("  Old Name ").unwrap();
        assert!(!session.has_unsaved_changes());
        session.stage_save_name("Jörg's Save").unwrap();
        assert!(session.has_unsaved_changes());
        assert_eq!(
            std::fs::read_to_string(save.join("savename.txt")).unwrap(),
            "Old Name"
        );

        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
        assert!(!session.has_unsaved_changes());
        assert_eq!(
            std::fs::read_to_string(save.join("savename.txt")).unwrap(),
            "Jörg's Save"
        );
        assert!(save.exists());
        assert_eq!(session.save_name().as_deref(), Some("Jörg's Save"));
    }

    #[test]
    fn quest_save_survives_later_character_saves_and_undo() {
        let (_dir, mut session, data) = quest_save_session();
        session
            .record_edit("Before character edit", |_| Ok(()))
            .unwrap();
        session
            .character
            .as_mut()
            .unwrap()
            .gff_mut()
            .insert("Str".into(), GffValue::Byte(12));
        assert!(session.has_unsaved_changes());
        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
        assert!(!session.has_unsaved_changes());
        assert!(session.quest_edits.pending.is_empty());
        assert!(!session.can_undo(), "a save clears the undo history");
        assert!(session.undo().is_none());
        session
            .record_edit("After save", |c| {
                c.gff_mut().insert("Str".into(), GffValue::Byte(14));
                Ok(())
            })
            .unwrap();
        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
        let journal = crate::services::save_graph::read_live_journal(
            session.savegame_handler.as_ref().unwrap(),
            0,
        )
        .unwrap();
        assert_eq!(journal[0].state, 20);
    }

    #[test]
    fn staged_quest_rewards_live_in_memory_follow_undo_and_are_saved_once() {
        use crate::services::save_graph::editing::{CHARACTER_TYPE, VariableKey, VariableWrite};
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = crate::services::save_graph::build_without_bridge(
            session.savegame_handler.as_ref().unwrap(),
            &ModuleInfo::default(),
            &ModuleVariables::default(),
            crate::services::save_graph::OrphanKind::PlatformUnsupported,
            String::new(),
        );
        let preview = || QuestPreview {
            revision: String::new(),
            tag: "test".into(),
            target: 20,
            saved_state: Some(10),
            before_state: Some(10),
            route: None,
            writes: vec![VariableWrite {
                key: VariableKey {
                    module: None,
                    name: "Gold".into(),
                    value_type: CHARACTER_TYPE.into(),
                },
                saved: None,
                before: None,
                after: serde_json::json!(50),
                action_index: 0,
                label: None,
                label_strref: None,
                derived_from: None,
                line: None,
            }],
            effects: Vec::new(),
            followed: Vec::new(),
            follow_stop: None,
            next_steps: Vec::new(),
            blueprints: std::collections::BTreeMap::new(),
        };
        let gold = |session: &SessionState| {
            session
                .character
                .as_ref()
                .unwrap()
                .gff()
                .get("Gold")
                .cloned()
        };

        session
            .stage_quest_edit(preview(), &graph, &[0], false, &data)
            .unwrap();
        assert!(matches!(gold(&session), Some(GffValue::Dword(50))));
        assert_eq!(session.undo_label(), Some("Quest test: entry 20"));

        session.undo();
        assert!(session.quest_edits.pending.is_empty());
        assert!(matches!(gold(&session), None | Some(GffValue::Dword(0))));
        session.redo();
        assert_eq!(session.quest_edits.pending.len(), 1);
        assert!(matches!(gold(&session), Some(GffValue::Dword(50))));

        // A tab edit after staging, then discarding the quest: undoing the tab edit
        // must not bring the quest's gold back.
        session.record_edit("Tab edit", |_| Ok(())).unwrap();
        session
            .character
            .as_mut()
            .unwrap()
            .gff_mut()
            .insert("Str".into(), GffValue::Byte(14));
        session.discard_quest_edits(Some("test")).unwrap();
        assert!(matches!(gold(&session), Some(GffValue::Dword(0))));
        session.undo();
        assert!(matches!(gold(&session), Some(GffValue::Dword(0))));
        assert!(!session.can_undo(), "the quest's own entry is gone");

        session
            .stage_quest_edit(preview(), &graph, &[0], false, &data)
            .unwrap();
        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
        let bytes = session
            .savegame_handler
            .as_ref()
            .unwrap()
            .extract_player_data()
            .unwrap();
        let player = GffParser::from_bytes(bytes)
            .unwrap()
            .read_struct_fields(0)
            .unwrap();
        let Some(GffValue::List(players)) = player.get("Mod_PlayerList") else {
            panic!("Mod_PlayerList missing");
        };
        assert!(matches!(
            players[0].force_load().unwrap().get("Gold"),
            Some(GffValue::Dword(50))
        ));
        assert!(session.quest_edits.pending.is_empty());
        assert!(
            !session.can_undo(),
            "undo cannot reach past the saved quest"
        );
    }

    #[test]
    fn undo_after_save_restores_the_saved_state_as_clean() {
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let set_str = |value: u8| {
            move |c: &mut Character| {
                c.gff_mut().insert("Str".into(), GffValue::Byte(value));
                Ok(())
            }
        };
        session.record_edit("Str 12", set_str(12)).unwrap();
        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
        session.record_edit("Str 14", set_str(14)).unwrap();
        assert!(session.has_unsaved_changes());
        session.undo();
        assert_eq!(read_str(session.character.as_ref().unwrap().gff()), 12);
        assert!(
            !session.has_unsaved_changes(),
            "the undone state is the saved one"
        );
    }

    #[test]
    fn failed_quest_discard_leaves_player_history_and_pending_untouched() {
        use crate::services::save_graph::editing::{CHARACTER_TYPE, VariableKey, VariableWrite};
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = crate::services::save_graph::build_without_bridge(
            session.savegame_handler.as_ref().unwrap(),
            &ModuleInfo::default(),
            &ModuleVariables::default(),
            crate::services::save_graph::OrphanKind::PlatformUnsupported,
            String::new(),
        );
        let preview = QuestPreview {
            revision: String::new(),
            tag: "test".into(),
            target: 20,
            saved_state: Some(10),
            before_state: Some(10),
            route: None,
            writes: vec![VariableWrite {
                key: VariableKey {
                    module: None,
                    name: "Gold".into(),
                    value_type: CHARACTER_TYPE.into(),
                },
                saved: None,
                before: None,
                after: serde_json::json!(50),
                action_index: 0,
                label: None,
                label_strref: None,
                derived_from: None,
                line: None,
            }],
            effects: Vec::new(),
            followed: Vec::new(),
            follow_stop: None,
            next_steps: Vec::new(),
            blueprints: std::collections::BTreeMap::new(),
        };
        session
            .stage_quest_edit(preview, &graph, &[0], false, &data)
            .unwrap();
        session.record_edit("Tab edit", |_| Ok(())).unwrap();
        // An undo snapshot the quest's gold cannot be taken out of.
        session
            .history
            .undo_stack
            .back_mut()
            .unwrap()
            .gff_snapshot
            .insert("Gold".into(), GffValue::String("broken".into()));

        assert!(session.discard_quest_edits(Some("test")).is_err());
        assert_eq!(session.quest_edits.pending.len(), 1);
        assert!(matches!(
            session.character.as_ref().unwrap().gff().get("Gold"),
            Some(GffValue::Dword(50))
        ));
        assert_eq!(session.history.undo_stack.len(), 2);
        assert!(matches!(
            session
                .history
                .undo_stack
                .back()
                .unwrap()
                .gff_snapshot
                .get("Gold"),
            Some(GffValue::String(_))
        ));
    }

    fn quest_write(
        value_type: &str,
        name: &str,
        after: i64,
    ) -> crate::services::save_graph::editing::VariableWrite {
        crate::services::save_graph::editing::VariableWrite {
            key: crate::services::save_graph::editing::VariableKey {
                module: None,
                name: name.into(),
                value_type: value_type.into(),
            },
            saved: None,
            before: None,
            after: serde_json::json!(after),
            action_index: 0,
            label: None,
            label_strref: None,
            derived_from: None,
            line: None,
        }
    }

    fn quest_preview(
        tag: &str,
        writes: Vec<crate::services::save_graph::editing::VariableWrite>,
    ) -> QuestPreview {
        QuestPreview {
            revision: String::new(),
            tag: tag.into(),
            target: 20,
            saved_state: None,
            before_state: None,
            route: None,
            writes,
            effects: Vec::new(),
            followed: Vec::new(),
            follow_stop: None,
            next_steps: Vec::new(),
            blueprints: std::collections::BTreeMap::new(),
        }
    }

    fn empty_graph(session: &SessionState) -> SaveGraph {
        crate::services::save_graph::build_without_bridge(
            session.savegame_handler.as_ref().unwrap(),
            &ModuleInfo::default(),
            &ModuleVariables::default(),
            crate::services::save_graph::OrphanKind::PlatformUnsupported,
            String::new(),
        )
    }

    fn gold(session: &SessionState) -> Option<i64> {
        session
            .character
            .as_ref()
            .unwrap()
            .gff()
            .get("Gold")
            .and_then(crate::parsers::gff::gff_integer)
    }

    /// The replacing edit cannot be applied (its item blueprint is missing): the edit it
    /// would replace stays staged, applied and undoable.
    #[test]
    fn failed_replace_keeps_the_replaced_edit_applied() {
        use crate::services::save_graph::editing::{CHARACTER_TYPE, ITEM_TYPE};
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = empty_graph(&session);
        let gold_quest = quest_preview("test", vec![quest_write(CHARACTER_TYPE, "Gold", 50)]);
        session
            .stage_quest_edit(gold_quest, &graph, &[0], false, &data)
            .unwrap();
        session.feat_cache = Some(Vec::new());

        let broken = quest_preview("test", vec![quest_write(ITEM_TYPE, "missing", 1)]);
        assert!(
            session
                .stage_quest_edit(broken, &graph, &[0], true, &data)
                .is_err()
        );
        assert_eq!(gold(&session), Some(50));
        assert_eq!(session.quest_edits.pending.len(), 1);
        assert!(session.quest_edits.pending[0].applied.is_some());
        assert_eq!(session.undo_label(), Some("Quest test: entry 20"));
        assert!(
            session.feat_cache.is_some(),
            "nothing changed on the player"
        );
        session.undo();
        assert!(session.quest_edits.pending.is_empty());
        assert_eq!(gold(&session).unwrap_or(0), 0);
    }

    /// Replacing an applied edit with one that has no player writes takes the gold back,
    /// drops the cached feat list and leaves nothing to redo.
    #[test]
    fn replacing_with_an_edit_without_player_writes_takes_the_old_one_back() {
        use crate::services::save_graph::editing::CHARACTER_TYPE;
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = empty_graph(&session);
        let gold_quest = || quest_preview("test", vec![quest_write(CHARACTER_TYPE, "Gold", 50)]);
        session
            .stage_quest_edit(gold_quest(), &graph, &[0], false, &data)
            .unwrap();
        session.undo();
        session.redo();
        session.undo();
        assert!(session.can_redo());
        session
            .stage_quest_edit(gold_quest(), &graph, &[0], false, &data)
            .unwrap();
        session.feat_cache = Some(Vec::new());

        let flags_only = quest_preview("test", vec![quest_write("int", "SomeGlobal", 1)]);
        session
            .stage_quest_edit(flags_only, &graph, &[0], true, &data)
            .unwrap();
        assert_eq!(gold(&session), Some(0));
        assert!(session.feat_cache.is_none());
        assert!(!session.can_redo());
        assert_eq!(session.quest_edits.pending.len(), 1);
        assert!(session.quest_edits.pending[0].applied.is_none());
    }

    #[test]
    fn staging_clears_redo() {
        use crate::services::save_graph::editing::CHARACTER_TYPE;
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = empty_graph(&session);
        session
            .stage_quest_edit(
                quest_preview("gold", vec![quest_write(CHARACTER_TYPE, "Gold", 50)]),
                &graph,
                &[0],
                false,
                &data,
            )
            .unwrap();
        session.undo();
        assert!(session.can_redo());
        session
            .stage_quest_edit(
                quest_preview("flags", vec![quest_write("int", "SomeGlobal", 1)]),
                &graph,
                &[0],
                false,
                &data,
            )
            .unwrap();
        assert!(!session.can_redo(), "a redo would skip the stage's checks");
    }

    #[test]
    fn discarding_the_only_quest_edit_leaves_the_player_unmodified() {
        use crate::services::save_graph::editing::CHARACTER_TYPE;
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = empty_graph(&session);
        let gold_quest = || quest_preview("test", vec![quest_write(CHARACTER_TYPE, "Gold", 50)]);
        session
            .stage_quest_edit(gold_quest(), &graph, &[0], false, &data)
            .unwrap();
        assert!(session.character.as_ref().unwrap().is_modified());
        session.discard_quest_edits(Some("test")).unwrap();
        assert!(!session.character.as_ref().unwrap().is_modified());

        // A tab edit after staging keeps the player modified.
        session
            .stage_quest_edit(gold_quest(), &graph, &[0], false, &data)
            .unwrap();
        session
            .record_edit("Str 12", |c| {
                c.gff_mut().insert("Str".into(), GffValue::Byte(12));
                Ok(())
            })
            .unwrap();
        session.discard_quest_edits(Some("test")).unwrap();
        assert!(session.character.as_ref().unwrap().is_modified());
    }

    /// Locals a quest writes (the paid quest XP share) reach the loaded player, so its
    /// next save does not put the old ones back.
    #[test]
    fn quest_locals_survive_the_next_player_save() {
        use crate::services::save_graph::editing::PC_LOCAL_TYPE;
        let (_dir, mut session, data) = quest_save_session();
        session.quest_edits.pending.clear();
        let graph = empty_graph(&session);
        let mut preview =
            quest_preview("test", vec![quest_write(PC_LOCAL_TYPE, "test_XP_QPR", 100)]);
        preview.saved_state = Some(10);
        session
            .stage_quest_edit(preview, &graph, &[0], false, &data)
            .unwrap();
        let paths = crate::config::NWN2Paths::new();
        session.save_character(&data, &paths, 5).unwrap();
        session
            .record_edit("Str 12", |c| {
                c.gff_mut().insert("Str".into(), GffValue::Byte(12));
                Ok(())
            })
            .unwrap();
        session.save_character(&data, &paths, 5).unwrap();
        let bytes = session
            .savegame_handler
            .as_ref()
            .unwrap()
            .extract_player_data()
            .unwrap();
        let root = GffParser::from_bytes(bytes)
            .unwrap()
            .read_owned_struct(0)
            .unwrap();
        let GffValue::ListOwned(players) = &root["Mod_PlayerList"] else {
            panic!("Mod_PlayerList missing");
        };
        let local = |name: &str| editing::pc_int_local(&players[0], name);
        assert_eq!(local("test_XP_QPR"), Some(serde_json::json!(100)));
        assert_eq!(local("NW_JOURNAL_ENTRYtest"), Some(serde_json::json!(20)));
    }

    #[test]
    fn quest_save_targets_primary_player_when_companion_is_selected() {
        let (_dir, mut session, data) = quest_save_session();
        session.character_source = CharacterSource::Companion {
            ros_name: "companion".into(),
        };
        session.character = Some(Character::from_gff(make_gff_with_str(10)).unwrap());
        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
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
        assert!(
            session
                .save_character(&data, &crate::config::NWN2Paths::new(), 5)
                .is_err()
        );
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
        session
            .save_character(&data, &crate::config::NWN2Paths::new(), 5)
            .unwrap();
        assert!(!session.has_unsaved_changes());
    }

    #[test]
    fn undo_redo_round_trips_attribute_change() {
        let initial_gff = make_gff_with_str(10);
        let mut session = make_session_with_character(initial_gff);

        session.record_edit("Set STR to 18", |_| Ok(())).unwrap();

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
    fn failed_edit_rolls_back_and_records_nothing() {
        let mut session = make_session_with_character(make_gff_with_str(10));

        let result = session.record_edit("Set STR to 18", |c| {
            c.gff_mut().insert("Str".into(), GffValue::Byte(18));
            Err::<(), _>(CommandError::Internal("fail".into()))
        });

        assert!(result.is_err());
        assert_eq!(read_str(session.character.as_ref().unwrap().gff()), 10);
        assert!(!session.can_undo());
    }

    #[test]
    fn redo_cleared_on_new_history() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_edit("first", |_| Ok(())).unwrap();
        session.undo();
        assert_eq!(session.history.redo_stack.len(), 1);

        session.record_edit("second", |_| Ok(())).unwrap();
        assert_eq!(session.history.redo_stack.len(), 0);
    }

    #[test]
    fn clear_history_empties_both_stacks() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_edit("op1", |_| Ok(())).unwrap();
        session.record_edit("op2", |_| Ok(())).unwrap();
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
            session.record_edit(format!("op{i}"), |_| Ok(())).unwrap();
        }

        assert_eq!(session.history.undo_stack.len(), HISTORY_LIMIT);
        assert_eq!(session.history.undo_stack.front().unwrap().label, "op1");
    }
}
