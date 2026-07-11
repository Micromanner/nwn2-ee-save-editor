use indexmap::IndexMap;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
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

const HISTORY_LIMIT: usize = 100;
const COALESCE_WINDOW: Duration = Duration::from_millis(500);

pub struct HistoryEntry {
    pub label: String,
    pub coalesce_key: Option<String>,
    pub timestamp: Instant,
    gff_snapshot: IndexMap<String, GffValue<'static>>,
    modified_snapshot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterSource {
    Player,
    Companion { ros_name: String },
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
    pub undo_stack: VecDeque<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
    pub character_source: CharacterSource,
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
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            character_source: CharacterSource::Player,
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
        self.clear_history();

        info!("Character loaded successfully");
        Ok(())
    }

    pub fn save_character(&mut self, game_data: &GameData) -> Result<Option<String>, String> {
        if let CharacterSource::Companion { ros_name } = &self.character_source {
            let ros_name = ros_name.clone();
            return self.save_companion_inner(&ros_name);
        }

        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or_else(|| "No active save handler".to_string())?;
        let character = self
            .character
            .as_ref()
            .ok_or_else(|| "No character loaded".to_string())?;

        if !character.is_modified() {
            info!("No changes to save");
            return Ok(None);
        }

        let char_fields = character.clone_gff();
        let selected_player_index = self.selected_player_index;
        let update_primary_player_files = self.primary_player_index == Some(selected_player_index);

        handler
            .rewrite_player_files(|src| {
                let playerlist = serialize_playerlist_bytes(
                    src.playerlist,
                    &char_fields,
                    selected_player_index,
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
            self.write_playerinfo(game_data)?;
        }

        self.character.as_mut().unwrap().mark_saved();

        info!(
            "Character saved successfully (player.bic_updated={})",
            update_primary_player_files
        );
        Ok(None)
    }

    pub fn save_companion(&mut self) -> Result<Option<String>, String> {
        let CharacterSource::Companion { ros_name } = &self.character_source else {
            return Err("Active character is not a companion".into());
        };
        let ros_name = ros_name.clone();
        self.save_companion_inner(&ros_name)
    }

    fn save_companion_inner(&mut self, ros_name: &str) -> Result<Option<String>, String> {
        let handler = self
            .savegame_handler
            .as_mut()
            .ok_or("No active save handler")?;
        let character = self.character.as_ref().ok_or("No character loaded")?;

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
        let bytes = crate::parsers::gff::merge_fields_into_gff(
            Some(&original),
            &character.clone_gff(),
            "ROS ",
            true,
        )?;
        handler
            .update_file(&stored_name, &bytes)
            .map_err(|e| format!("Failed to write companion file: {e}"))?;

        let classes: Vec<(i32, i32)> = character
            .class_entries()
            .into_iter()
            .map(|e| (e.class_id.0, e.level))
            .collect();
        let warning = match sync_roster(handler, ros_name, &classes) {
            Ok(()) => None,
            Err(e) => {
                warn!("Roster sync failed after companion save: {e}");
                Some(format!(
                    "Companion saved, but updating the party roster failed: {e}"
                ))
            }
        };

        self.character.as_mut().unwrap().mark_saved();
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
        self.clear_history();
        crate::services::savegame_handler::backup::clear_backup_tracking();
    }

    pub fn record_history(&mut self, label: impl Into<String>, coalesce_key: Option<&str>) {
        let Some(character) = self.character.as_ref() else {
            return;
        };
        let now = Instant::now();
        if let Some(coalesce_key) = coalesce_key
            && let Some(last) = self.undo_stack.back()
            && last.coalesce_key.as_deref() == Some(coalesce_key)
            && now.duration_since(last.timestamp) < COALESCE_WINDOW
        {
            return;
        }
        let entry = HistoryEntry {
            label: label.into(),
            coalesce_key: coalesce_key.map(str::to_owned),
            timestamp: now,
            gff_snapshot: character.clone_gff(),
            modified_snapshot: character.is_modified(),
        };
        self.undo_stack.push_back(entry);
        if self.undo_stack.len() > HISTORY_LIMIT {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> Option<String> {
        let entry = self.undo_stack.pop_back()?;
        let label = entry.label.clone();
        let redo_entry = self.character.as_ref().map(|ch| HistoryEntry {
            label: label.clone(),
            coalesce_key: entry.coalesce_key.clone(),
            timestamp: Instant::now(),
            gff_snapshot: ch.clone_gff(),
            modified_snapshot: ch.is_modified(),
        });
        if let Some(character) = self.character.as_mut() {
            character.restore_snapshot(entry.gff_snapshot, entry.modified_snapshot);
        }
        if let Some(redo) = redo_entry {
            self.redo_stack.push(redo);
        }
        Some(label)
    }

    pub fn redo(&mut self) -> Option<String> {
        let entry = self.redo_stack.pop()?;
        let label = entry.label.clone();
        let undo_entry = self.character.as_ref().map(|ch| HistoryEntry {
            label: label.clone(),
            coalesce_key: entry.coalesce_key.clone(),
            timestamp: Instant::now(),
            gff_snapshot: ch.clone_gff(),
            modified_snapshot: ch.is_modified(),
        });
        if let Some(character) = self.character.as_mut() {
            character.restore_snapshot(entry.gff_snapshot, entry.modified_snapshot);
        }
        if let Some(undo) = undo_entry {
            self.undo_stack.push_back(undo);
        }
        Some(label)
    }

    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo_stack.back().map(|e| e.label.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo_stack.last().map(|e| e.label.as_str())
    }

    pub fn invalidate_feat_cache(&mut self) {
        self.feat_cache = None;
    }

    pub fn invalidate_module_info_cache(&mut self) {
        self.module_info_cache = None;
    }

    pub fn has_unsaved_changes(&self) -> bool {
        self.character
            .as_ref()
            .is_some_and(super::super::character::Character::is_modified)
    }

    pub fn character(&self) -> Option<&Character> {
        self.character.as_ref()
    }

    pub fn character_mut(&mut self) -> Option<&mut Character> {
        self.character.as_mut()
    }

    pub fn load_companion(&mut self, ros_name: &str, force: bool) -> Result<(), String> {
        if !force && self.has_unsaved_changes() {
            return Err(
                "Unsaved changes present; save or discard before switching characters".into(),
            );
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
        self.character = Some(character);
        self.character_source = CharacterSource::Companion {
            ros_name: ros_name.to_string(),
        };
        // Same save, so module/quest caches stay; character-scoped cache does not.
        self.feat_cache = None;
        self.clear_history();
        Ok(())
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

        Ok(members
            .into_iter()
            .filter(|m| {
                (m.available || m.campaign_npc)
                    && companion_files
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

    fn write_playerinfo(&self, game_data: &GameData) -> Result<(), String> {
        let character = self.character.as_ref().ok_or("No character loaded")?;
        let save_dir = self.save_dir.as_ref().ok_or("No current save path")?;
        write_playerinfo_for_character(save_dir, character, game_data)
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
) -> Result<(), String> {
    let Some((stored_name, bytes)) = handler.extract_roster().map_err(|e| e.to_string())? else {
        return Err("no roster file in save".into());
    };
    match crate::services::roster::sync_member_classes(bytes, ros_name, classes)? {
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
    *player_entry = merged;

    GffWriter::new(&file_type, &file_version)
        .write(root_fields)
        .map_err(|e| format!("playerlist.ifo serialization error: {e}"))
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

        session.record_history("Set STR to 18", None);

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
    fn coalesce_skips_second_push_within_window() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_history("Set STR to 15", Some("ability:STR"));
        session.record_history("Set STR to 16", Some("ability:STR"));

        assert_eq!(session.undo_stack.len(), 1);
        assert_eq!(session.undo_stack.back().unwrap().label, "Set STR to 15");
    }

    #[test]
    fn redo_cleared_on_new_history() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_history("first", None);
        session.undo();
        assert_eq!(session.redo_stack.len(), 1);

        session.record_history("second", None);
        assert_eq!(session.redo_stack.len(), 0);
    }

    #[test]
    fn clear_history_empties_both_stacks() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        session.record_history("op1", None);
        session.record_history("op2", None);
        session.undo();
        session.clear_history();

        assert!(session.undo_stack.is_empty());
        assert!(session.redo_stack.is_empty());
    }

    #[test]
    fn history_limit_enforced() {
        let gff = make_gff_with_str(10);
        let mut session = make_session_with_character(gff);

        for i in 0..=HISTORY_LIMIT {
            session.record_history(format!("op{i}"), None);
        }

        assert_eq!(session.undo_stack.len(), HISTORY_LIMIT);
        assert_eq!(session.undo_stack.front().unwrap().label, "op1");
    }
}
