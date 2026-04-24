use indexmap::IndexMap;
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

        info!("Character loaded successfully");
        Ok(())
    }

    pub fn save_character(&mut self, game_data: &GameData) -> Result<(), String> {
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
            return Ok(());
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
                    Some(serialize_player_bic_bytes(src.player_bic, &char_fields)?)
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
        Ok(())
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
        crate::services::savegame_handler::backup::clear_backup_tracking();
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

fn serialize_player_bic_bytes(
    player_bic_data: Option<&[u8]>,
    character_fields: &IndexMap<String, GffValue<'static>>,
) -> Result<Vec<u8>, String> {
    let Some(player_bic_data) = player_bic_data else {
        return GffWriter::new("BIC ", "V3.2")
            .write(character_fields.clone())
            .map_err(|e| format!("player.bic serialization error: {e}"));
    };

    let gff = GffParser::from_bytes(player_bic_data.to_vec())
        .map_err(|e| format!("player.bic parse error: {e}"))?;
    let file_type = gff.file_type.clone();
    let file_version = gff.file_version.clone();
    let root_struct_id = gff
        .get_struct_id(0)
        .map_err(|e| format!("Failed to read player.bic root struct_id: {e}"))?;
    let bic_fields = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read player.bic fields: {e}"))?;

    // Merge character_fields into existing BIC: overwrite keys that BIC already has,
    // preserve BIC-only keys, skip GFF-internal metadata.
    let mut merged: IndexMap<String, GffValue<'static>> = bic_fields
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();
    for (key, value) in character_fields {
        if key.starts_with("__") {
            continue;
        }
        if merged.contains_key(key) {
            merged.insert(key.clone(), value.clone());
        }
    }

    GffWriter::new(&file_type, &file_version)
        .write_with_struct_id(merged, root_struct_id)
        .map_err(|e| format!("player.bic serialization error: {e}"))
}
