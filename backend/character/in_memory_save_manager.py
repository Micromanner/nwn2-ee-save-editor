"""
In-Memory Save File Manager

Simple approach: Load entire save files (.ifo and .bic) into memory,
allow real-time editing, then save back to disk when user clicks save.

This is just a thin wrapper around SaveGameHandler to provide the
in-memory editing concept - SaveGameHandler already has all the loading/saving logic.
"""

import logging
from typing import Dict, Any, Optional
from datetime import datetime
from io import BytesIO

from parsers.gff import GFFParser
from parsers.savegame_handler import SaveGameHandler
from .character_manager import CharacterManager

logger = logging.getLogger(__name__)


class InMemorySaveManager:
    """
    Thin wrapper around SaveGameHandler for in-memory editing concept
    
    Simply loads save files into memory and tracks if changes were made.
    SaveGameHandler handles all the actual file operations.
    """
    
    def __init__(self, save_path: str):
        """
        Initialize with save game directory path
        
        Args:
            save_path: Path to save game directory containing resgff.zip
        """
        self.save_handler = SaveGameHandler(save_path)
        
        # In-memory character data (from player.bic)
        self.character_data: Optional[Dict[str, Any]] = None
        
        # Track changes
        self.is_dirty = False
        self.last_loaded = None
        
        # Character manager instance
        self._character_manager: Optional[CharacterManager] = None
        
        logger.info(f"Initialized InMemorySaveManager for {save_path}")
    
    def load_save_files(self) -> bool:
        """
        Load player.bic into memory for editing
        
        Returns:
            True if successfully loaded, False otherwise
        """
        try:
            logger.info("Loading save files into memory")
            
            # Use SaveGameHandler to extract player.bic
            playerbic_data = self.save_handler.extract_player_bic()
            if not playerbic_data:
                logger.error("No player.bic data found in save")
                return False
            
            # Parse player.bic into dict for CharacterManager
            parser = GFFParser()
            bic_gff = parser.load(BytesIO(playerbic_data))
            self.character_data = bic_gff.to_dict()
            
            # Mark as loaded
            self.last_loaded = datetime.now()
            self.is_dirty = False
            
            logger.info(f"Successfully loaded player.bic - {len(self.character_data)} fields")
            return True
            
        except Exception as e:
            logger.error(f"Failed to load save files: {e}", exc_info=True)
            return False
    
    def get_character_manager(self, game_data_loader=None, rules_service=None) -> Optional[CharacterManager]:
        """
        Get CharacterManager instance working with in-memory data using factory
        
        Args:
            game_data_loader: Optional DynamicGameDataLoader instance
            rules_service: Optional GameRulesService instance
            
        Returns:
            CharacterManager instance or None if not loaded
        """
        if not self.is_loaded():
            logger.error("Save files not loaded - call load_save_files() first")
            return None
        
        if self._character_manager is None:
            try:
                # Use factory to create CharacterManager with all managers registered
                from .factory import create_character_manager
                
                self._character_manager = create_character_manager(
                    self.character_data,
                    gff_element=None,  # In-memory doesn't use direct GFF element
                    game_data_loader=game_data_loader,
                    rules_service=rules_service,
                    lazy=True  # Use lazy loading for better performance
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
        """Check if save files are loaded in memory"""
        return self.character_data is not None
    
    def has_unsaved_changes(self) -> bool:
        """Check if there are unsaved changes"""
        return self.is_dirty
    
    def save_to_disk(self, create_backup: bool = True) -> bool:
        """
        Save in-memory changes back to disk using SaveGameHandler
        
        Args:
            create_backup: Whether to create backup before saving
            
        Returns:
            True if saved successfully, False otherwise
        """
        if not self.is_loaded():
            logger.error("No save files loaded")
            return False
        
        if not self.has_unsaved_changes():
            logger.info("No changes to save")
            return True
        
        try:
            logger.info(f"Saving changes to disk (backup={create_backup})")
            
            if self._character_manager:
                # Get updated character data
                updated_data = self._character_manager.character_data
                
                # Convert back to GFF format for saving
                from parsers.gff import GFFWriter, dict_to_gff
                
                # Create player.bic content
                bic_gff = dict_to_gff(updated_data, 'BIC ')
                playerbic_output = BytesIO()
                bic_writer = GFFWriter('BIC ')
                bic_writer.save(playerbic_output, bic_gff)
                
                # We need to also sync changes to playerlist.ifo
                # First, load the current playerlist.ifo to update it
                playerlist_data = self.save_handler.extract_player_data()
                if playerlist_data:
                    # Parse playerlist.ifo
                    parser = GFFParser()
                    player_list = parser.load(BytesIO(playerlist_data))
                    
                    # Get player struct from playerlist.ifo
                    mod_player_list = player_list.get_field('Mod_PlayerList')
                    if mod_player_list and mod_player_list.value:
                        player_struct = mod_player_list.value[0]
                        
                        # Update player struct with changes from character data
                        # Only update fields that exist in playerlist.ifo
                        player_dict = player_struct.to_dict()
                        for key in player_dict:
                            if key in updated_data:
                                player_struct.set_field(key, updated_data[key])
                        
                        # Write updated playerlist.ifo
                        playerlist_output = BytesIO()
                        writer = GFFWriter.from_parser(parser)  # Preserve IFO file type
                        writer.save(playerlist_output, player_list)
                        
                        # Update both files together
                        self.save_handler.update_player_complete(
                            playerlist_output.getvalue(),
                            playerbic_output.getvalue(),
                            backup=create_backup
                        )
                    else:
                        # Fallback if no playerlist data found
                        self.save_handler.update_player_bic(
                            playerbic_output.getvalue(),
                            backup=create_backup
                        )
                else:
                    # No playerlist.ifo found, just update player.bic
                    self.save_handler.update_player_bic(
                        playerbic_output.getvalue(),
                        backup=create_backup
                    )
                
                # Mark as clean
                self.is_dirty = False
                
                logger.info("Successfully saved changes to disk")
                return True
                
        except Exception as e:
            logger.error(f"Failed to save to disk: {e}", exc_info=True)
            return False
    
    def reload_from_disk(self) -> bool:
        """
        Reload save files from disk, discarding in-memory changes
        
        Returns:
            True if reloaded successfully, False otherwise
        """
        logger.warning("Reloading from disk - discarding unsaved changes")
        
        # Reset state
        self.character_data = None
        self._character_manager = None
        self.is_dirty = False
        
        # Reload from disk
        return self.load_save_files()
    
    def get_character_summary(self) -> Dict[str, Any]:
        """
        Get character summary from in-memory data
        
        Returns:
            Character summary dict or empty dict if not loaded
        """
        if not self.is_loaded() or not self._character_manager:
            return {}
        
        try:
            return self._character_manager.get_character_summary()
        except Exception as e:
            logger.error(f"Failed to get character summary: {e}", exc_info=True)
            return {}
    
    def close(self):
        """Clean up resources"""
        if self.has_unsaved_changes():
            logger.warning("Closing InMemorySaveManager with unsaved changes")
        
        self._character_manager = None
        self.character_data = None
        
        logger.info("Closed InMemorySaveManager")
    
    def __enter__(self):
        """Context manager entry"""
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit"""
        self.close()


class InMemoryCharacterSession:
    """
    High-level session manager for character editing
    
    Handles the complete workflow:
    1. Load save files
    2. Create managers
    3. Handle editing session
    4. Save changes
    """
    
    def __init__(self, save_path: str, auto_load: bool = True):
        """
        Initialize character editing session
        
        Args:
            save_path: Path to save game directory
            auto_load: Whether to automatically load save files
        """
        self.save_manager = InMemorySaveManager(save_path)
        self.character_manager: Optional[CharacterManager] = None
        
        if auto_load:
            self.load()
    
    def load(self) -> bool:
        """Load save files and initialize character manager"""
        if not self.save_manager.load_save_files():
            return False
        
        # Initialize game dependencies
        from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader
        from gamedata.services.game_rules_service import GameRulesService
        
        try:
            game_data_loader = get_dynamic_game_data_loader()
            rules_service = GameRulesService()
            
            self.character_manager = self.save_manager.get_character_manager(
                game_data_loader=game_data_loader,
                rules_service=rules_service
            )
            
            if self.character_manager:
                # Factory already registered all managers, just log success
                logger.info("Character editing session ready with all managers")
                return True
        except Exception as e:
            logger.error(f"Failed to initialize character session: {e}", exc_info=True)
        
        return False
    
    def save(self, create_backup: bool = True) -> bool:
        """Save changes to disk"""
        return self.save_manager.save_to_disk(create_backup=create_backup)
    
    def has_unsaved_changes(self) -> bool:
        """Check for unsaved changes"""
        return self.save_manager.has_unsaved_changes()
    
    def get_info(self) -> Dict[str, Any]:
        """Get session information"""
        info = self.save_manager.get_save_info()
        info['character_summary'] = self.save_manager.get_character_summary()
        return info
    
    def close(self):
        """Close the session"""
        self.save_manager.close()
        self.character_manager = None
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()