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
use crate::services::resource_manager::ResourceManager;
use crate::services::savegame_handler::SaveGameHandler;

pub struct SessionState {
    pub current_file_path: Option<PathBuf>,
    pub savegame_handler: Option<SaveGameHandler>,
    pub character: Option<Character>,
    pub selected_player_index: usize,
    pub primary_player_index: Option<usize>,
    pub item_property_decoder: ItemPropertyDecoder,
    pub feat_cache: Option<Vec<FeatInfo>>,
    pub module_info_cache: Option<(ModuleInfo, ModuleVariables)>,
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
            savegame_handler: None,
            character: None,
            selected_player_index: 0,
            primary_player_index: None,
            item_property_decoder,
            feat_cache: None,
            module_info_cache: None,
        }
    }

    #[instrument(name = "SessionState::load_character", skip(self), fields(file_path = %file_path))]
    pub fn load_character(
        &mut self,
        file_path: &str,
        player_index: Option<usize>,
    ) -> Result<(), String> {
        info!("Loading character from save file");
        let path = PathBuf::from(file_path);

        crate::services::savegame_handler::backup::clear_backup_tracking();

        debug!("Creating SaveGameHandler");
        let handler = SaveGameHandler::new(&path, false, true).map_err(|e| {
            warn!("Failed to create save handler: {}", e);
            format!("Failed to create save handler: {e}")
        })?;
        debug!("SaveGameHandler created");

        debug!("Extracting playerlist.ifo from save archive");
        let playerlist_data = handler.extract_player_data().map_err(|e| {
            warn!("Failed to extract playerlist.ifo: {}", e);
            format!("Failed to extract playerlist.ifo: {e}")
        })?;
        info!("playerlist.ifo extracted ({} bytes)", playerlist_data.len());

        debug!("Parsing playerlist.ifo GFF data");
        let gff = GffParser::from_bytes(playerlist_data).map_err(|e| {
            warn!("GFF parse error: {}", e);
            format!("GFF Parse error: {e}")
        })?;
        debug!("playerlist.ifo parsed successfully");

        let player_entries = read_playerlist_entries(gff)?;
        let player_bic_fields = match handler.extract_player_bic() {
            Ok(Some(player_bic_data)) => match read_player_bic_entry(player_bic_data) {
                Ok(fields) => Some(fields),
                Err(err) => {
                    warn!("Failed to parse player.bic while loading save: {}", err);
                    None
                }
            },
            Ok(None) => None,
            Err(err) => {
                warn!("Failed to extract player.bic while loading save: {}", err);
                None
            }
        };
        let primary_player_index =
            resolve_primary_player_index(&player_entries, player_bic_fields.as_ref());
        let selected_player_index = player_index.unwrap_or(primary_player_index.unwrap_or(0));

        let fields = if primary_player_index == Some(selected_player_index) {
            if let Some(fields) = player_bic_fields {
                debug!(
                    "Using player.bic as authoritative source for playerlist slot {}",
                    selected_player_index
                );
                fields
            } else {
                read_playerlist_entry_from_entries(&player_entries, selected_player_index)?
            }
        } else {
            read_playerlist_entry_from_entries(&player_entries, selected_player_index)?
        };
        info!("Character data extracted ({} fields)", fields.len());

        debug!("Creating Character from GFF fields");
        let character = Character::from_gff(fields);
        info!(
            "Character created: {} (Level {})",
            character.full_name(),
            character.total_level()
        );

        self.character = Some(character);
        self.savegame_handler = Some(handler);
        self.current_file_path = Some(path);
        self.module_info_cache = None;
        self.selected_player_index = selected_player_index;
        self.primary_player_index = primary_player_index;

        info!("Character loaded successfully");
        Ok(())
    }

    pub fn normalize_loaded_skill_points(&mut self, game_data: &GameData) {
        let Some(character) = self.character.as_mut() else {
            return;
        };

        let summary = character.get_skill_points_summary(game_data);
        if summary.mismatch == 0 {
            return;
        }

        let was_modified = character.is_modified();
        character.normalize_skill_points(game_data);

        if !was_modified {
            character.mark_saved();
        }
    }

    pub fn save_character(&mut self, game_data: &GameData) -> Result<(), String> {
        if self.savegame_handler.is_none() {
            return Err("No active save handler".to_string());
        }
        if self.character.is_none() {
            return Err("No character loaded".to_string());
        }

        let char_fields = {
            let character = self.character.as_mut().unwrap();
            character.normalize_skill_points(game_data);
            character
                .recalculate_stats(game_data)
                .map_err(|e| format!("Failed to recalculate class-derived stats: {e}"))?;
            character.clone_gff()
        };

        let (playerlist_data, player_bic_data) = {
            let handler = self.savegame_handler.as_ref().unwrap();
            let playerlist_data = handler
                .extract_player_data()
                .map_err(|e| format!("Failed to read playerlist.ifo: {e}"))?;
            let player_bic_data = handler
                .extract_player_bic()
                .map_err(|e| format!("Failed to read player.bic: {e}"))?;
            (playerlist_data, player_bic_data)
        };

        let playerlist_bytes =
            serialize_playerlist_bytes(playerlist_data, &char_fields, self.selected_player_index)?;
        let update_primary_player_files =
            self.primary_player_index == Some(self.selected_player_index);
        let player_bic_bytes = if update_primary_player_files {
            Some(serialize_player_bic_bytes(player_bic_data, &char_fields)?)
        } else {
            None
        };

        self.savegame_handler
            .as_mut()
            .unwrap()
            .update_player_complete(&playerlist_bytes, player_bic_bytes.as_deref(), None, None)
            .map_err(|e| format!("Failed to write save file: {e}"))?;

        if update_primary_player_files {
            self.write_playerinfo(game_data)?;
        }

        self.character.as_mut().unwrap().mark_saved();

        info!(
            "Character saved successfully (playerlist={} bytes, player.bic_updated={})",
            playerlist_bytes.len(),
            update_primary_player_files
        );
        Ok(())
    }

    pub fn close_character(&mut self) {
        self.character = None;
        self.savegame_handler = None;
        self.current_file_path = None;
        self.selected_player_index = 0;
        self.primary_player_index = None;
        self.feat_cache = None;
        self.module_info_cache = None;
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

    fn current_save_dir(&self) -> Result<PathBuf, String> {
        let current_path = self
            .current_file_path
            .as_ref()
            .ok_or("No current save path")?;

        if current_path.is_dir() {
            Ok(current_path.clone())
        } else {
            current_path
                .parent()
                .map(PathBuf::from)
                .ok_or_else(|| "Failed to determine save directory".to_string())
        }
    }

    fn write_playerinfo(&self, game_data: &GameData) -> Result<(), String> {
        let character = self.character.as_ref().ok_or("No character loaded")?;
        let save_dir = self.current_save_dir()?;
        write_playerinfo_for_character(&save_dir, character, game_data)
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

    let subrace_label = character.subrace_name(game_data).unwrap_or_default();
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
    playerlist_data: Vec<u8>,
    character_fields: &IndexMap<String, GffValue<'static>>,
    player_index: usize,
) -> Result<Vec<u8>, String> {
    let gff = GffParser::from_bytes(playerlist_data)
        .map_err(|e| format!("playerlist.ifo parse error: {e}"))?;

    let file_type = gff.file_type.clone();
    let file_version = gff.file_version.clone();

    let root_fields_raw = gff
        .read_struct_fields(0)
        .map_err(|e| format!("Failed to read playerlist.ifo root struct: {e}"))?;

    let mut root_fields: IndexMap<String, GffValue<'static>> = root_fields_raw
        .into_iter()
        .map(|(k, v)| (k, v.force_owned()))
        .collect();

    fn merge_player_entry_fields(
        existing: &IndexMap<String, GffValue<'static>>,
        updated_character_fields: &IndexMap<String, GffValue<'static>>,
    ) -> IndexMap<String, GffValue<'static>> {
        let mut merged = existing.clone();
        for (key, value) in updated_character_fields {
            merged.insert(key.clone(), value.clone());
        }
        merged
    }

    match root_fields.get_mut("Mod_PlayerList") {
        Some(GffValue::ListOwned(players)) => {
            if players.is_empty() {
                if player_index == 0 {
                    players.push(character_fields.clone());
                } else {
                    return Err(format!(
                        "Selected player index {player_index} is out of range for an empty Mod_PlayerList"
                    ));
                }
            } else {
                let Some(player_entry) = players.get_mut(player_index) else {
                    return Err(format!(
                        "Selected player index {player_index} is out of range for Mod_PlayerList with {} entries",
                        players.len()
                    ));
                };
                *player_entry = merge_player_entry_fields(player_entry, character_fields);
            }
        }
        Some(_) => {
            return Err("Mod_PlayerList in playerlist.ifo is not a list".to_string());
        }
        None => {
            if player_index == 0 {
                root_fields.insert(
                    "Mod_PlayerList".to_string(),
                    GffValue::ListOwned(vec![character_fields.clone()]),
                );
            } else {
                return Err(format!(
                    "Selected player index {player_index} is out of range because Mod_PlayerList is missing"
                ));
            }
        }
    }

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
    player_bic_data: Option<Vec<u8>>,
    character_fields: &IndexMap<String, GffValue<'static>>,
) -> Result<Vec<u8>, String> {
    let (file_type, file_version) = if let Some(player_bic_data) = player_bic_data {
        let gff = GffParser::from_bytes(player_bic_data)
            .map_err(|e| format!("player.bic parse error: {e}"))?;
        let file_type = gff.file_type.clone();
        let file_version = gff.file_version.clone();
        (file_type, file_version)
    } else {
        ("BIC ".to_string(), "V3.2".to_string())
    };

    GffWriter::new(&file_type, &file_version)
        .write(character_fields.clone())
        .map_err(|e| format!("player.bic serialization error: {e}"))
}
