"""In-memory save file manager for loading, editing, and saving NWN2 save files."""

from typing import Dict, Any, Optional
from datetime import datetime

from loguru import logger
from nwn2_rust import GffParser

from services.core.savegame_handler import SaveGameHandler
from .character_manager import CharacterManager


class InMemorySaveManager:
    """Wrapper around SaveGameHandler for in-memory editing with change tracking."""

    def __init__(self, save_path: str):
        """Initialize with save game directory path."""
        self.save_path = save_path  # Store the save path
        self.save_handler = SaveGameHandler(save_path)

        # In-memory character data (from player.bic) - plain dict with __struct_id__ metadata
        self.character_data: Optional[Dict[str, Any]] = None

        # Track changes
        self.is_dirty = False
        self.last_loaded = None

        # Character manager instance
        self._character_manager: Optional[CharacterManager] = None

        logger.info(f"Initialized InMemorySaveManager for {save_path}")
    
    def load_save_files(self) -> bool:
        """Load player.bic into memory for editing."""
        try:
            logger.info("Loading save files into memory")

            # Use SaveGameHandler to extract player.bic
            playerbic_data = self.save_handler.extract_player_bic()
            if not playerbic_data:
                logger.error("No player.bic data found in save")
                return False

            # Parse player.bic into plain dict with __struct_id__ metadata
            self.character_data = GffParser.from_bytes(playerbic_data).to_dict()

            # Mark as loaded
            self.last_loaded = datetime.now()
            self.is_dirty = False

            logger.info(f"Successfully loaded player.bic - {len(self.character_data)} fields")
            return True

        except Exception as e:
            logger.error(f"Failed to load save files: {e}", exc_info=True)
            return False
    
    def get_character_manager(self, game_data_loader=None, rules_service=None) -> Optional[CharacterManager]:
        """Get CharacterManager instance working with in-memory data."""
        if not self.is_loaded():
            logger.error("Save files not loaded - call load_save_files() first")
            return None

        if self._character_manager is None:
            try:
                # Use factory to create CharacterManager with all managers registered
                from .factory import create_character_manager

                self._character_manager = create_character_manager(
                    self.character_data,
                    game_data_loader=game_data_loader,
                    rules_service=rules_service,
                    lazy=True,
                    save_path=self.save_path
                )

                # Track changes by wrapping the gff.set method
                original_set = self._character_manager.gff.set
                def tracked_set(path: str, value: Any):
                    result = original_set(path, value)
                    self.is_dirty = True
                    return result
                self._character_manager.gff.set = tracked_set

                logger.info("Created CharacterManager with factory (all managers registered)")

            except Exception as e:
                logger.error(f"Failed to create CharacterManager: {e}", exc_info=True)
                return None

        return self._character_manager
    
    def is_loaded(self) -> bool:
        """Check if save files are loaded in memory."""
        return self.character_data is not None
    
    def has_unsaved_changes(self) -> bool:
        """Check if there are unsaved changes."""
        return self.is_dirty
    
    def save_to_disk(self, create_backup: bool = True) -> bool:
        """Save in-memory changes back to disk."""
        if not self.is_loaded():
            logger.error("No save files loaded")
            return False

        if not self.has_unsaved_changes():
            logger.info("No changes to save")
            return True

        try:
            logger.info(f"Saving changes to disk (backup={create_backup})")

            if self._character_manager:
                from nwn2_rust import GffWriter

                # Get the current character data dict from the manager
                char_data = self._character_manager.gff.raw_data

                # Write player.bic using plain dict (writer handles __struct_id__)
                bic_writer = GffWriter('BIC ', 'V3.2')
                playerbic_bytes = bic_writer.dump(char_data)

                # Update playerlist.ifo with relevant changes
                playerlist_data = self.save_handler.extract_player_data()
                if playerlist_data:
                    player_list_dict = GffParser.from_bytes(playerlist_data).to_dict()

                    # Get player struct from Mod_PlayerList
                    mod_player_list = player_list_dict.get('Mod_PlayerList', [])
                    if mod_player_list:
                        player_struct = mod_player_list[0]

                        # Update fields that exist in both
                        for key in list(player_struct.keys()):
                            if key.startswith('__'):
                                continue
                            if key in char_data:
                                player_struct[key] = char_data[key]

                        # Write updated playerlist.ifo
                        ifo_writer = GffWriter('IFO ', 'V3.2')
                        playerlist_bytes = ifo_writer.dump(player_list_dict)

                        # Get base stats and summary from managers for playerinfo.bin sync
                        base_stats = None
                        char_summary = None
                        
                        ability_manager = self._character_manager.get_manager('ability')
                        if ability_manager:
                            attrs = ability_manager.get_attributes(include_equipment=False)
                            base_stats = {
                                'str': attrs.get('Str', 10),
                                'dex': attrs.get('Dex', 10),
                                'con': attrs.get('Con', 10),
                                'int': attrs.get('Int', 10),
                                'wis': attrs.get('Wis', 10),
                                'cha': attrs.get('Cha', 10)
                            }
                        
                        char_summary = self._character_manager.get_character_summary()

                        self.save_handler.update_player_complete(
                            playerlist_bytes,
                            playerbic_bytes,
                            base_stats=base_stats,
                            char_summary=char_summary
                        )
                    else:
                        self.save_handler.update_player_bic(
                            playerbic_bytes
                        )
                else:
                    self.save_handler.update_player_bic(
                        playerbic_bytes
                    )

                self.is_dirty = False
                logger.info("Successfully saved changes to disk")
                return True

        except Exception as e:
            logger.error(f"Failed to save to disk: {e}", exc_info=True)
            return False
    
    def reload_from_disk(self) -> bool:
        """Reload save files from disk, discarding in-memory changes."""
        logger.warning("Reloading from disk - discarding unsaved changes")
        
        # Reset state
        self.character_data = None
        self._character_manager = None
        self.is_dirty = False
        
        # Reload from disk
        return self.load_save_files()
    
    def get_character_summary(self) -> Dict[str, Any]:
        """Get character summary from in-memory data."""
        if not self.is_loaded() or not self._character_manager:
            return {}
        
        try:
            return self._character_manager.get_character_summary()
        except Exception as e:
            logger.error(f"Failed to get character summary: {e}", exc_info=True)
            return {}
    
    def close(self):
        """Clean up resources."""
        if self.has_unsaved_changes():
            logger.warning("Closing InMemorySaveManager with unsaved changes")
        
        self._character_manager = None
        self.character_data = None
        
        logger.info("Closed InMemorySaveManager")
    
    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()


class InMemoryCharacterSession:
    """High-level session manager for character editing workflow."""

    def __init__(self, save_path: str, auto_load: bool = True):
        """Initialize character editing session."""
        self.save_manager = InMemorySaveManager(save_path)
        self.character_manager: Optional[CharacterManager] = None
        
        if auto_load:
            self.load()
    
    def load(self) -> bool:
        """Load save files and initialize character manager."""
        if not self.save_manager.load_save_files():
            return False

        from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
        from services.gamedata.game_rules_service import GameRulesService

        try:
            from services.fastapi.shared_services import get_shared_resource_manager

            shared_rm = get_shared_resource_manager()
            if shared_rm is not None:
                logger.info("Using shared ResourceManager from FastAPI (via independent registry)")
            else:
                logger.warning("No shared ResourceManager available, DynamicGameDataLoader will create one")

            game_data_loader = get_dynamic_game_data_loader(resource_manager=shared_rm)
            rules_service = GameRulesService(resource_manager=shared_rm)

            self.character_manager = self.save_manager.get_character_manager(
                game_data_loader=game_data_loader,
                rules_service=rules_service
            )

            if self.character_manager:
                content_manager = self.character_manager.get_manager('content')
                if content_manager and content_manager.module_info and shared_rm:
                    hak_list = content_manager.module_info.get('hak_list', [])
                    custom_tlk = content_manager.module_info.get('custom_tlk', '')
                    campaign_id = content_manager.module_info.get('campaign_id', '')

                    if hak_list or campaign_id:
                        shared_rm.load_haks_for_save(hak_list, custom_tlk, campaign_id)

                logger.info("Character editing session ready with all managers")
                return True

            logger.error("Character manager is None after creation")
            return False
        except Exception as e:
            logger.error(f"Failed to initialize character session: {e}", exc_info=True)
            return False
    
    def save(self, create_backup: bool = True) -> bool:
        """Save changes to disk."""
        return self.save_manager.save_to_disk(create_backup=create_backup)

    def has_unsaved_changes(self) -> bool:
        """Check for unsaved changes."""
        return self.save_manager.has_unsaved_changes()

    def close(self):
        """Close the session."""
        self.save_manager.close()
        self.character_manager = None

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()