"""Manages character state manipulation operations."""

from typing import List, Tuple, TYPE_CHECKING
from loguru import logger

from ..events import EventEmitter

if TYPE_CHECKING:
    from ..character_manager import CharacterManager


class CharacterStateManager(EventEmitter):
    """Manages character state manipulation operations."""

    def __init__(self, character_manager: 'CharacterManager'):
        """Initialize with reference to parent CharacterManager."""
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
    
    def reset_character(self) -> None:
        """Reset character to default state (level 1, base attributes)."""
        logger.info("Resetting character to default state")

        txn = self.character_manager.begin_transaction()

        try:
            self.gff.set('FirstName', {'substrings': [{'string': 'New Character'}]})
            self.gff.set('LastName', {'substrings': [{'string': ''}]})

            class_list = self.gff.get('ClassList')
            if not class_list:
                raise ValueError("Character has no ClassList - cannot reset")
            first_class = class_list[0]
            first_class['ClassLevel'] = 1
            self.gff.set('ClassList', [first_class])

            for ability_field in ['Str', 'Dex', 'Con', 'Int', 'Wis', 'Cha']:
                self.gff.set(ability_field, 10)

            self.gff.set('LawfulChaotic', 50)
            self.gff.set('GoodEvil', 50)

            feat_mgr = self.character_manager.get_manager('feat')
            if not feat_mgr:
                raise ValueError("FeatManager not available - cannot reset")
            epithet_feats = feat_mgr.detect_epithet_feats()
            feat_list = self.gff.get('FeatList') or []
            preserved_feats = [f for f in feat_list if f.get('Feat') in epithet_feats]
            self.gff.set('FeatList', preserved_feats)

            self.gff.set('SkillList', [])

            self.gff.set('HitPoints', 6)
            self.gff.set('CurrentHitPoints', 6)

            for level in range(10):
                self.gff.set(f'KnownList{level}', [])
                self.gff.set(f'MemorizedList{level}', [])

            self.character_manager._notify_managers('character_reset', {})
            self.character_manager.commit_transaction()
            logger.info("Character reset completed successfully")

        except Exception as e:
            self.character_manager.rollback_transaction()
            logger.error(f"Failed to reset character: {e}")
            raise
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate character state manager has required references."""
        errors = []
        if not self.character_manager:
            errors.append("Missing character_manager reference")
        if not self.gff:
            errors.append("Missing gff reference")
        return len(errors) == 0, errors