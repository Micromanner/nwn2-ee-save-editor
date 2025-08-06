"""
API Views Package - One ViewSet per Character Manager

Each manager gets its own dedicated viewset:
- CharacterViewSet: Base CRUD operations, import/export
- AttributeViewSet: Ability scores, modifiers
- SkillViewSet: Skills, ranks, modifiers
- FeatViewSet: Feats, prerequisites, selection
- SpellViewSet: Spells, spellbooks, memorization
- CombatViewSet: Combat stats, BAB, AC
- InventoryViewSet: Items, equipment, encumbrance
- ClassViewSet: Class changes, levels, prestige
- RaceViewSet: Race changes, subraces
- SaveViewSet: Saving throws, resistances
"""

from .character_views import CharacterViewSet
from .attribute_views import AttributeViewSet
from .skill_views import SkillViewSet
from .feat_views import FeatViewSet
from .spell_views import SpellViewSet
from .combat_views import CombatViewSet
from .inventory_views import InventoryViewSet
from .class_views import ClassViewSet
from .race_views import RaceViewSet
from .save_views import SaveViewSet

__all__ = [
    'CharacterViewSet',
    'AttributeViewSet',
    'SkillViewSet',
    'FeatViewSet',
    'SpellViewSet',
    'CombatViewSet',
    'InventoryViewSet',
    'ClassViewSet',
    'RaceViewSet',
    'SaveViewSet',
]