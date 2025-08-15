"""
Character State Manager - handles character state manipulation and persistence
Manages reset, clone, import/export operations and file I/O
"""

from typing import Dict, List, Any, Optional, Tuple, TYPE_CHECKING
import copy
import logging
from pathlib import Path

from parsers import gff
from ..events import EventEmitter, EventType, EventData

if TYPE_CHECKING:
    from ..character_manager import CharacterManager

logger = logging.getLogger(__name__)


class CharacterStateManager(EventEmitter):
    """Manages character state manipulation, cloning, and persistence operations"""
    
    def __init__(self, character_manager: 'CharacterManager'):
        """
        Initialize the CharacterStateManager
        
        Args:
            character_manager: Reference to parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_data_loader = character_manager.game_data_loader
    
    def reset_character(self) -> None:
        """Reset character to a default state (level 1, base attributes)"""
        logger.info("Resetting character to default state")
        
        # Begin transaction for atomic reset
        txn = self.character_manager.begin_transaction()
        
        try:
            # Reset basic info
            self.gff.set('FirstName', {'substrings': [{'string': 'New Character'}]})
            self.gff.set('LastName', {'substrings': [{'string': ''}]})
            
            # Reset to level 1 with first class
            class_list = self.gff.get('ClassList', [])
            if class_list:
                # Keep only first class at level 1
                first_class = class_list[0]
                first_class['ClassLevel'] = 1
                self.gff.set('ClassList', [first_class])
            
            # Reset attributes to base 10
            for ability_field in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                self.gff.set(ability_field, 10)
            
            # Reset alignment to true neutral
            self.gff.set('LawfulChaotic', 50)
            self.gff.set('GoodEvil', 50)
            
            # Clear feats except racial/epithet
            feat_mgr = self.character_manager.get_manager('feat')
            if feat_mgr:
                # Try to get racial/special feats that should be preserved
                racial_feats = []
                if hasattr(feat_mgr, 'detect_epithet_feats'):
                    racial_feats = feat_mgr.detect_epithet_feats()
                elif hasattr(feat_mgr, 'get_racial_feats'):
                    racial_feats = feat_mgr.get_racial_feats()
                elif hasattr(feat_mgr, 'get_automatic_feats'):
                    racial_feats = feat_mgr.get_automatic_feats()
                
                feat_list = self.gff.get('FeatList', [])
                if racial_feats:
                    preserved_feats = [f for f in feat_list if f.get('Feat', -1) in racial_feats]
                    self.gff.set('FeatList', preserved_feats)
                else:
                    # No racial feats detected, clear all feats
                    self.gff.set('FeatList', [])
            else:
                # Fallback: clear all feats
                self.gff.set('FeatList', [])
            
            # Clear skills
            self.gff.set('SkillList', [])
            
            # Reset HP to base
            self.gff.set('HitPoints', 6)  # Base HP for level 1
            self.gff.set('CurrentHitPoints', 6)
            
            # Clear spells
            for level in range(10):
                self.gff.set(f'KnownList{level}', [])
                self.gff.set(f'MemorizedList{level}', [])
            
            # Notify managers
            self.character_manager._notify_managers('character_reset', {})
            
            # Commit transaction
            self.character_manager.commit_transaction()
            
            logger.info("Character reset completed successfully")
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            logger.error(f"Failed to reset character: {e}")
            raise
    
    def clone_character(self) -> 'CharacterManager':
        """
        Create a copy of the current character
        
        Returns:
            New CharacterManager instance with cloned character data
        """
        cloned_data = copy.deepcopy(self.character_manager.character_data)
        
        # Create new manager with cloned data
        from ..character_manager import CharacterManager
        cloned_manager = CharacterManager(
            cloned_data,
            game_data_loader=self.character_manager.game_data_loader,
            rules_service=self.character_manager.rules_service
        )
        
        # Register same managers
        for name, manager_class in self.character_manager._manager_classes.items():
            cloned_manager.register_manager(name, manager_class)
        
        logger.info(f"Created clone of character {self.character_manager._get_character_name()}")
        return cloned_manager
    
    def import_character(self, character_data: Dict[str, Any]) -> None:
        """
        Import character data from exported format
        
        Args:
            character_data: Character data to import
        """
        if 'summary' not in character_data:
            raise ValueError("Invalid character data format - missing summary")
        
        logger.info("Importing character data")
        
        # Begin transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            # Import core character data
            if 'gff_data' in character_data:
                self.character_manager.character_data = character_data['gff_data']
                from ..character_manager import GFFDataWrapper
                self.character_manager.gff = GFFDataWrapper(self.character_manager.character_data)
                self.gff = self.character_manager.gff  # Update our reference
            
            # Re-detect custom content through ContentManager
            content_manager = self.character_manager.get_manager('content')
            if content_manager:
                content_manager._detect_custom_content_dynamic()
                self.character_manager.custom_content = content_manager.custom_content
            
            # Notify managers of import
            self.character_manager._notify_managers('character_imported', character_data)
            
            # Validate imported data
            is_valid, errors = self.character_manager.validate_changes()
            if not is_valid:
                raise ValueError(f"Imported character has validation errors: {errors}")
            
            self.character_manager.commit_transaction()
            logger.info("Character import completed successfully")
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            logger.error(f"Failed to import character: {e}")
            raise
    
    def save_to_file(self, filepath: str) -> None:
        """
        Save character to a file
        
        Args:
            filepath: Path to save the character file
        """
        try:
            # If we have a gff_element, use it for direct write
            if hasattr(self.character_manager, 'gff_element') and self.character_manager.gff_element:
                gff.write_gff(self.character_manager.gff_element, filepath)
            else:
                # Convert dict data back to GFF format
                gff_element = gff.dict_to_gff(self.character_manager.character_data)
                gff.write_gff(gff_element, filepath)
            
            logger.info(f"Character saved to {filepath}")
            
        except Exception as e:
            logger.error(f"Failed to save character to {filepath}: {e}")
            raise
    
    def load_from_file(self, filepath: str) -> None:
        """
        Load character from a file
        
        Args:
            filepath: Path to the character file
        """
        try:
            # Parse the GFF file
            gff_element = gff.parse_gff(filepath)
            character_data = gff.gff_to_dict(gff_element)
            
            # Import the loaded data
            self.import_character({'gff_data': character_data, 'summary': {}})
            
            # Store the gff_element for direct updates
            self.character_manager.gff_element = gff_element
            
            # Try to use DirectGFFWrapper if available, otherwise keep existing wrapper
            try:
                from ..gff_direct_wrapper import DirectGFFWrapper
                self.character_manager.gff = DirectGFFWrapper(gff_element)
                self.gff = self.character_manager.gff  # Update our reference
            except ImportError:
                logger.warning("DirectGFFWrapper not available, keeping existing GFF wrapper")
                # Update existing wrapper with new data
                from ..character_manager import GFFDataWrapper
                self.character_manager.gff = GFFDataWrapper(character_data)
                self.gff = self.character_manager.gff
            
            logger.info(f"Character loaded from {filepath}")
            
        except Exception as e:
            logger.error(f"Failed to load character from {filepath}: {e}")
            raise
    
    def validate(self) -> Tuple[bool, List[str]]:
        """
        Validate character state manager
        
        Returns:
            (is_valid, list_of_errors)
        """
        errors = []
        
        # Validate we have access to required resources
        if not self.character_manager:
            errors.append("Missing character_manager reference")
        
        if not self.gff:
            errors.append("Missing gff reference")
            
        if not self.game_data_loader:
            errors.append("Missing game_data_loader reference")
        
        return len(errors) == 0, errors
    
    def get_alignment(self) -> Dict[str, Any]:
        """Get character alignment"""
        law_chaos = self.gff.get('LawfulChaotic', 50)
        good_evil = self.gff.get('GoodEvil', 50)
        
        return {
            'lawChaos': law_chaos,
            'goodEvil': good_evil,
            'alignment_string': self._get_alignment_string(law_chaos, good_evil)
        }
    
    def set_alignment(self, law_chaos: int = None, good_evil: int = None) -> Dict[str, Any]:
        """Set character alignment"""
        if law_chaos is not None:
            if not (0 <= law_chaos <= 100):
                raise ValueError("lawChaos must be between 0 and 100")
            self.gff.set('LawfulChaotic', law_chaos)
        else:
            law_chaos = self.gff.get('LawfulChaotic', 50)
            
        if good_evil is not None:
            if not (0 <= good_evil <= 100):
                raise ValueError("goodEvil must be between 0 and 100")
            self.gff.set('GoodEvil', good_evil)
        else:
            good_evil = self.gff.get('GoodEvil', 50)
            
        return self.get_alignment()
    
    def shift_alignment(self, law_chaos_shift: int = 0, good_evil_shift: int = 0) -> Dict[str, Any]:
        """Shift alignment by relative amounts"""
        current_law_chaos = self.gff.get('LawfulChaotic', 50)
        current_good_evil = self.gff.get('GoodEvil', 50)
        
        new_law_chaos = max(0, min(100, current_law_chaos + law_chaos_shift))
        new_good_evil = max(0, min(100, current_good_evil + good_evil_shift))
        
        self.gff.set('LawfulChaotic', new_law_chaos)
        self.gff.set('GoodEvil', new_good_evil)
        
        result = self.get_alignment()
        result['shifted'] = {
            'lawChaos': law_chaos_shift,
            'goodEvil': good_evil_shift
        }
        return result
    
    def _get_alignment_string(self, law_chaos: int, good_evil: int) -> str:
        """Convert numeric alignment values to D&D alignment string"""
        # Determine Law/Chaos axis
        if law_chaos <= 30:
            law_axis = "Chaotic"
        elif law_chaos >= 70:
            law_axis = "Lawful"
        else:
            law_axis = "Neutral"
        
        # Determine Good/Evil axis
        if good_evil <= 30:
            evil_axis = "Evil"
        elif good_evil >= 70:
            evil_axis = "Good"
        else:
            evil_axis = "Neutral"
        
        # Handle True Neutral case
        if law_axis == "Neutral" and evil_axis == "Neutral":
            return "True Neutral"
        
        return f"{law_axis} {evil_axis}"