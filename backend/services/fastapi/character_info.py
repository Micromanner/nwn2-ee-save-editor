"""Simple character information for session management."""

from pathlib import Path


class CharacterInfo:
    """Simple character information for session management."""
    
    def __init__(self, file_path: str):
        self.file_path = file_path
        self.path_obj = Path(file_path)
        
    @property
    def is_savegame(self) -> bool:
        """Check if this is a savegame directory (contains resgff.zip)."""
        if not self.path_obj.is_dir():
            return False
        resgff_path = self.path_obj / 'resgff.zip'
        return resgff_path.exists()
    
    @property
    def exists(self) -> bool:
        """Check if the character file/directory exists."""
        return self.path_obj.exists()
    
    @property
    def name(self) -> str:
        """Get character name (directory name for savegames)."""
        return self.path_obj.name


def get_character_info(character_id: str) -> CharacterInfo:
    """Get character information from character ID (file path)."""
    character_info = CharacterInfo(character_id)
    
    if not character_info.exists:
        raise FileNotFoundError(f"Character not found: {character_id}")
    
    return character_info