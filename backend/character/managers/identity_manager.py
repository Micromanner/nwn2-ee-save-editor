"""Identity Manager - handles character identity data (name, age, background, XP, alignment)."""

from typing import Dict, Any, Optional
from loguru import logger


class IdentityManager:
    """Manages character identity, biography, and alignment data."""

    def __init__(self, character_manager):
        """Initialize IdentityManager with parent CharacterManager."""
        self.character_manager = character_manager
        self.gff = character_manager.gff

    def get_character_name(self) -> str:
        """Get character's full name from localized string structure."""
        first_name = self.gff.get('FirstName')
        last_name = self.gff.get('LastName')

        first = self._extract_localized_string(first_name) if first_name else ''
        last = self._extract_localized_string(last_name) if last_name else ''

        full_name = f"{first} {last}".strip()
        return full_name if full_name else ''

    def _extract_localized_string(self, value: Any) -> str:
        """Extract string from NWN2 localized string structure."""
        if isinstance(value, dict) and 'substrings' in value:
            substrings = value.get('substrings', [])
            if substrings and isinstance(substrings[0], dict):
                return substrings[0].get('string', '')
        elif isinstance(value, str):
            return value
        return str(value) if value else ''

    def get_character_age(self) -> int:
        """Get character's age in years."""
        age = self.gff.get('Age')
        if age is None:
            raise ValueError("Age field missing from GFF")
        return int(age)

    def get_character_background(self) -> str:
        """Get character's background/biography text."""
        bio = self.gff.get('Description')
        if bio is None:
            return ''
        return self._extract_localized_string(bio)

    def get_experience_points(self) -> int:
        """Get character's current experience points."""
        xp = self.gff.get('Experience')
        if xp is None:
            raise ValueError("Experience field missing from GFF")
        return int(xp)

    def get_biography(self) -> Dict[str, Any]:
        """Get complete biography data for display."""
        return {
            'name': self.get_character_name(),
            'age': self.get_character_age(),
            'background': self.get_character_background(),
            'experience_points': self.get_experience_points()
        }

    def get_alignment(self) -> Dict[str, Any]:
        """Get character alignment values and string representation."""
        law_chaos = self.gff.get('LawfulChaotic')
        good_evil = self.gff.get('GoodEvil')
        if law_chaos is None or good_evil is None:
            raise ValueError("Character missing alignment data (LawfulChaotic/GoodEvil)")
        return {
            'lawChaos': law_chaos,
            'goodEvil': good_evil,
            'alignment_string': self._get_alignment_string(law_chaos, good_evil)
        }

    def set_alignment(self, law_chaos: Optional[int] = None, good_evil: Optional[int] = None) -> Dict[str, Any]:
        """Set character alignment values (0-100 scale)."""
        if law_chaos is not None:
            if not (0 <= law_chaos <= 100):
                raise ValueError("lawChaos must be between 0 and 100")
            self.gff.set('LawfulChaotic', law_chaos)
        if good_evil is not None:
            if not (0 <= good_evil <= 100):
                raise ValueError("goodEvil must be between 0 and 100")
            self.gff.set('GoodEvil', good_evil)
        return self.get_alignment()

    def shift_alignment(self, law_chaos_shift: int = 0, good_evil_shift: int = 0) -> Dict[str, Any]:
        """Shift alignment by relative amounts, clamped to 0-100."""
        current_law_chaos = self.gff.get('LawfulChaotic')
        current_good_evil = self.gff.get('GoodEvil')
        if current_law_chaos is None or current_good_evil is None:
            raise ValueError("Character missing alignment data (LawfulChaotic/GoodEvil)")

        new_law_chaos = max(0, min(100, current_law_chaos + law_chaos_shift))
        new_good_evil = max(0, min(100, current_good_evil + good_evil_shift))

        self.gff.set('LawfulChaotic', new_law_chaos)
        self.gff.set('GoodEvil', new_good_evil)

        result = self.get_alignment()
        result['shifted'] = {'lawChaos': law_chaos_shift, 'goodEvil': good_evil_shift}
        return result

    def _get_alignment_string(self, law_chaos: int, good_evil: int) -> str:
        """Convert numeric alignment values to D&D alignment string."""
        if law_chaos <= 30:
            law_axis = "Chaotic"
        elif law_chaos >= 70:
            law_axis = "Lawful"
        else:
            law_axis = "Neutral"

        if good_evil <= 30:
            evil_axis = "Evil"
        elif good_evil >= 70:
            evil_axis = "Good"
        else:
            evil_axis = "Neutral"

        if law_axis == "Neutral" and evil_axis == "Neutral":
            return "True Neutral"
        return f"{law_axis} {evil_axis}"
