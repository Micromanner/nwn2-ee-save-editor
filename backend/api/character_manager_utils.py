"""
Utility functions for character management in API views
"""

from parsers.gff import GFFParser, GFFWriter
from character.character_manager import CharacterManager


def create_character_manager(character_file_path):
    """
    Create a CharacterManager with direct GFF updates enabled
    
    Args:
        character_file_path: Path to the character file
        
    Returns:
        tuple: (manager, gff_element, raw_data)
    """
    parser = GFFParser()
    gff_element = parser.read(character_file_path)
    raw_data = gff_element.to_dict()
    
    # Create manager with gff_element for direct updates
    manager = CharacterManager(raw_data, gff_element=gff_element)
    
    return manager, gff_element, raw_data


def save_character_changes(manager, character_file_path):
    """
    Save character changes without using update_from_dict
    
    Args:
        manager: CharacterManager instance
        character_file_path: Path to save the character file
        
    Returns:
        bool: True if save was successful
    """
    # If manager was created with gff_element, it's already updated
    if hasattr(manager, 'gff_element') and manager.gff_element:
        writer = GFFWriter()
        writer.write(character_file_path, manager.gff_element)
        return True
    else:
        # Fallback to update_from_dict if no direct updates
        # This shouldn't happen with our new approach
        raise ValueError("CharacterManager must be created with gff_element for direct updates")