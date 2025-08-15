"""
Pydantic models for CombatManager
Handles combat statistics, attack bonuses, armor class, and damage
"""

from typing import Dict, Any, Optional, List, Literal
from pydantic import BaseModel, Field


class ArmorClassBreakdown(BaseModel):
    """Detailed armor class calculation - matches CombatManager output"""
    # Main AC values (matching manager field names)
    total: int = Field(..., description="Total AC")
    total_ac: int = Field(..., description="Total AC (duplicate for compatibility)")
    touch_ac: int = Field(..., description="AC vs touch attacks")
    flatfooted_ac: int = Field(..., description="AC when flat-footed")
    
    # Component breakdown (matching manager's components dict)
    components: Dict[str, int] = Field(default_factory=dict, description="AC component breakdown")
    
    # Additional fields from manager
    dex_bonus: int = Field(0, description="Character's dex bonus (before armor max)")
    max_dex_from_armor: int = Field(999, description="Max dex bonus allowed by armor")
    armor_check_penalty: int = Field(0, description="Armor check penalty")
    
    # Optional individual component fields for easy access
    base: Optional[int] = Field(None, description="Base AC (always 10)")
    dex_modifier: Optional[int] = Field(None, description="Applied dexterity modifier to AC")
    armor_bonus: Optional[int] = Field(None, description="Armor bonus from equipment")
    shield_bonus: Optional[int] = Field(None, description="Shield bonus from equipment")
    natural_armor: Optional[int] = Field(None, description="Natural armor bonus")
    deflection_bonus: Optional[int] = Field(None, description="Deflection bonus from magic")
    dodge_bonus: Optional[int] = Field(None, description="Dodge bonus from feats/abilities")
    size_modifier: Optional[int] = Field(None, description="Size modifier to AC")
    
    # Deprecated fields (kept for compatibility)
    monk_bonus: Optional[int] = Field(None, description="Wisdom bonus for monks")
    misc_bonus: Optional[int] = Field(None, description="Other miscellaneous bonuses")
    flat_footed_ac: Optional[int] = Field(None, description="AC when flat-footed (alternative name)")
    equipment_total: Optional[int] = Field(None, description="Total from all equipment")
    spell_total: Optional[int] = Field(None, description="Total from active spells")
    feat_total: Optional[int] = Field(None, description="Total from feats")


class BaseAttackBonusInfo(BaseModel):
    """Base Attack Bonus progression and breakdown - matches CombatManager output"""
    # Main BAB info
    base_attack_bonus: int = Field(0, description="Total base attack bonus")
    total_bab: Optional[int] = Field(None, description="Total BAB (alternative name)")
    
    # Attack sequence info
    attack_sequence: Optional[List[int]] = Field(None, description="Full attack sequence")
    iterative_attacks: Optional[int] = Field(None, description="Number of iterative attacks")
    progression_type: Optional[str] = Field(None, description="Good/Average/Poor progression")
    
    # Class breakdown (optional since not always provided)
    class_breakdown: Optional[Dict[str, int]] = Field(None, description="BAB from each class")
    
    # Additional fields that manager might provide
    melee_attack_bonus: Optional[int] = Field(None, description="Total melee attack bonus")
    ranged_attack_bonus: Optional[int] = Field(None, description="Total ranged attack bonus")
    str_modifier: Optional[int] = Field(None, description="Strength modifier")
    dex_modifier: Optional[int] = Field(None, description="Dexterity modifier")
    
    # Detailed breakdown (from get_attack_bonuses)
    melee: Optional[Dict[str, Any]] = Field(None, description="Melee attack breakdown")
    ranged: Optional[Dict[str, Any]] = Field(None, description="Ranged attack breakdown")


class AttackBonusBreakdown(BaseModel):
    """Complete attack bonus calculation"""
    total: int = Field(..., description="Total attack bonus")
    base_attack_bonus: int = Field(..., description="Base attack bonus")
    ability_modifier: int = Field(..., description="STR or DEX modifier")
    size_modifier: int = Field(0, description="Size modifier to attack")
    weapon_enhancement: int = Field(0, description="Weapon enhancement bonus")
    weapon_focus: int = Field(0, description="Weapon Focus feat bonus")
    weapon_mastery: int = Field(0, description="Weapon mastery bonuses")
    spell_bonuses: int = Field(0, description="Bonuses from spells")
    feat_bonuses: int = Field(0, description="Other feat bonuses")
    misc_bonuses: int = Field(0, description="Miscellaneous bonuses")
    penalties: int = Field(0, description="Total penalties")
    
    # Conditional modifiers
    flanking_bonus: int = Field(0, description="Bonus when flanking")
    charging_bonus: int = Field(0, description="Bonus when charging")
    two_weapon_penalty: int = Field(0, description="Two-weapon fighting penalty")


class DamageBonusBreakdown(BaseModel):
    """Damage bonus calculation"""
    total: int = Field(..., description="Total damage bonus")
    ability_modifier: int = Field(..., description="STR modifier (or DEX with finesse)")
    weapon_enhancement: int = Field(0, description="Weapon enhancement bonus")
    weapon_specialization: int = Field(0, description="Weapon Specialization bonus")
    weapon_mastery: int = Field(0, description="Weapon mastery bonuses")
    power_attack: int = Field(0, description="Power Attack bonus damage")
    sneak_attack_dice: int = Field(0, description="Sneak attack dice")
    spell_bonuses: int = Field(0, description="Damage from spells")
    feat_bonuses: int = Field(0, description="Other feat bonuses")
    misc_bonuses: int = Field(0, description="Miscellaneous bonuses")
    
    # Special damage
    elemental_damage: Dict[str, int] = Field(default_factory=dict, description="Elemental damage by type")
    critical_multiplier: int = Field(2, description="Critical hit multiplier")
    threat_range: str = Field("20", description="Critical threat range")


class WeaponInfo(BaseModel):
    """Detailed weapon information"""
    name: str = Field(..., description="Weapon name")
    base_item_id: int = Field(..., description="Base item type ID")
    damage_dice: str = Field(..., description="Damage dice (e.g., '1d8')")
    threat_range: str = Field(..., description="Critical threat range")
    critical_multiplier: int = Field(..., description="Critical multiplier")
    enhancement_bonus: int = Field(0, description="Enhancement bonus")
    weapon_type: str = Field(..., description="Simple/Martial/Exotic")
    damage_type: str = Field(..., description="Slashing/Piercing/Bludgeoning")
    size: str = Field("Medium", description="Weapon size")
    weight: float = Field(0.0, description="Weapon weight")
    two_handed: bool = Field(False, description="Requires two hands")
    finesseable: bool = Field(False, description="Can use Weapon Finesse")
    properties: List[Dict[str, Any]] = Field(default_factory=list, description="Special properties")


class EquippedWeapons(BaseModel):
    """Currently equipped weapons"""
    main_hand: Optional[WeaponInfo] = None
    off_hand: Optional[WeaponInfo] = None
    ranged: Optional[WeaponInfo] = None
    ammunition: Optional[Dict[str, Any]] = None
    unarmed_strike: WeaponInfo
    
    # Combat style flags
    two_weapon_fighting: bool = False
    weapon_finesse_active: bool = False
    power_attack_active: bool = False
    combat_expertise_active: bool = False


class DefensiveAbilities(BaseModel):
    """Defensive combat abilities"""
    damage_reduction: List[Dict[str, Any]] = Field(default_factory=list, description="DR types and amounts")
    energy_resistance: Dict[str, int] = Field(default_factory=dict, description="Energy resistance by type")
    damage_immunity: List[str] = Field(default_factory=list, description="Damage immunities")
    spell_resistance: int = Field(0, description="Spell resistance")
    concealment: int = Field(0, description="Concealment percentage")
    fortification: int = Field(0, description="Fortification vs criticals/sneak attacks")
    evasion: bool = Field(False, description="Has Evasion ability")
    improved_evasion: bool = Field(False, description="Has Improved Evasion")
    uncanny_dodge: bool = Field(False, description="Has Uncanny Dodge")
    improved_uncanny_dodge: bool = Field(False, description="Has Improved Uncanny Dodge")


class CombatManeuvers(BaseModel):
    """Special combat maneuver bonuses"""
    bull_rush: int = 0
    disarm: int = 0
    grapple: int = 0
    overrun: int = 0
    sunder: int = 0
    trip: int = 0
    
    # Defensive maneuvers
    combat_maneuver_defense: int = 0


class InitiativeInfo(BaseModel):
    """Initiative calculation - matches CombatManager output"""
    total: int = Field(..., description="Total initiative bonus")
    dex_modifier: int = Field(..., description="Dexterity modifier")
    improved_initiative: int = Field(0, description="Improved Initiative feat bonus")
    misc_bonus: int = Field(0, description="Other bonuses (manager field name)")
    
    # Legacy field for compatibility
    misc_bonuses: Optional[int] = Field(None, description="Other bonuses (deprecated name)")
    
    # Special abilities (optional since manager may not provide)
    acts_in_surprise_round: Optional[bool] = Field(None, description="Acts in surprise round")
    always_acts_first: Optional[bool] = Field(None, description="Always acts first")


class CombatSummary(BaseModel):
    """High-level combat statistics summary - matches CombatManager.get_combat_summary()"""
    hit_points: int
    max_hit_points: int
    temporary_hit_points: int = 0
    armor_class: int
    touch_ac: int
    flat_footed_ac: int
    base_attack_bonus: int
    initiative: int
    speed: int = 30
    damage_reduction: Optional[str] = None
    spell_resistance: int = 0
    
    # Quick attack/damage for main weapon
    main_attack_bonus: int
    main_damage: str
    
    # Status flags
    is_flat_footed: bool = False
    is_flanked: bool = False
    is_prone: bool = False
    is_stunned: bool = False
    
    # Additional fields that manager provides
    bab_info: Optional[Dict[str, Any]] = Field(None, description="BAB breakdown info")
    attack_bonuses: Optional[Dict[str, Any]] = Field(None, description="Attack bonuses breakdown")
    damage_bonuses: Optional[Dict[str, Any]] = Field(None, description="Damage bonuses breakdown")
    weapons: Optional[Dict[str, Any]] = Field(None, description="Equipped weapons info")
    defensive_abilities: Optional[Dict[str, Any]] = Field(None, description="Defensive abilities")
    combat_maneuvers: Optional[Dict[str, Any]] = Field(None, description="Combat maneuvers")
    initiative_breakdown: Optional[Dict[str, Any]] = Field(None, description="Initiative breakdown details")


class CombatState(BaseModel):
    """Complete combat state from CombatManager - matches get_combat_summary output"""
    summary: CombatSummary
    armor_class: ArmorClassBreakdown
    base_attack_bonus: BaseAttackBonusInfo
    attack_bonuses: Dict[str, Any] = Field(default_factory=dict, description="Attack bonuses from manager")
    damage_bonuses: Dict[str, Any] = Field(default_factory=dict, description="Damage bonuses from manager")
    equipped_weapons: EquippedWeapons
    defensive_abilities: Dict[str, Any] = Field(default_factory=dict, description="Defensive abilities from manager")
    combat_maneuvers: Dict[str, Any] = Field(default_factory=dict, description="Combat maneuvers from manager")
    initiative: InitiativeInfo
    
    # Combat options currently active (optional)
    active_combat_modes: Optional[List[str]] = Field(None, description="Active combat modes")
    active_stances: Optional[List[str]] = Field(None, description="Active stances")



class CombatUpdateRequest(BaseModel):
    """Request to update combat values"""
    field: str = Field(..., description="Combat field to update")
    value: Any = Field(..., description="New value")
    
    
class CombatUpdateResponse(BaseModel):
    """Response after updating combat values"""
    field: str
    old_value: Any
    new_value: Any
    cascading_changes: List[Dict[str, Any]] = Field(default_factory=list)
    has_unsaved_changes: bool = True


class CombatModeToggleRequest(BaseModel):
    """Request to toggle combat mode"""
    mode: Literal['power_attack', 'combat_expertise', 'fighting_defensively', 'total_defense']
    active: bool


class CombatModeToggleResponse(BaseModel):
    """Response after toggling combat mode"""
    mode: str
    active: bool
    combat_summary: CombatSummary
    modifiers_applied: Dict[str, int]
    has_unsaved_changes: bool = False  # Combat modes are temporary




class DefensiveStats(BaseModel):
    """Defensive combat statistics"""
    armor_class: ArmorClassBreakdown
    defensive_abilities: DefensiveAbilities
    saving_throws: Dict[str, int] = Field(default_factory=dict, description="Base saving throw bonuses")
    hit_points: int = Field(..., description="Current hit points")
    max_hit_points: int = Field(..., description="Maximum hit points")
    temporary_hit_points: int = Field(0, description="Temporary hit points")
    
    # Damage mitigation
    damage_reduction_summary: str = Field("", description="Summary of DR")
    spell_resistance: int = Field(0, description="Spell resistance")
    concealment: int = Field(0, description="Concealment percentage")
    
    # Special defensive abilities
    evasion: bool = Field(False, description="Has Evasion")
    improved_evasion: bool = Field(False, description="Has Improved Evasion")
    uncanny_dodge: bool = Field(False, description="Cannot be caught flat-footed")


class NaturalArmorUpdateRequest(BaseModel):
    """Request to update natural armor value"""
    natural_ac: int = Field(..., ge=0, le=20, description="New natural armor bonus")


class NaturalArmorUpdateResponse(BaseModel):
    """Response after updating natural armor - matches manager output"""
    field: str = "NaturalAC"
    old_value: int
    new_value: int
    new_ac: Dict[str, Any] = Field(default_factory=dict, description="New AC calculation from manager")
    has_unsaved_changes: bool = True
    
    # Legacy field for compatibility
    updated_ac: Optional[Dict[str, Any]] = Field(None, description="Updated AC calculation (deprecated name)")