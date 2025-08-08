"""
Class Manager - handles class changes, multiclassing, and level progression
Refactored from class_change_service.py to work with event system
"""

from typing import Dict, List, Tuple, Optional, Any
import logging
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent

logger = logging.getLogger(__name__)


class ClassManager(EventEmitter):
    """Manages character class changes and progression"""
    
    def __init__(self, character_manager):
        """
        Initialize the ClassManager
        
        Args:
            character_manager: Reference to the parent CharacterManager
        """
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.game_data_loader = character_manager.game_data_loader
        
        # Cache for performance
        self._class_cache = {}
        self._bab_table_cache = {}
        self._save_table_cache = {}
    
    def change_class(self, new_class_id: int, preserve_level: bool = True, 
                    cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Change character's primary class
        
        Args:
            new_class_id: The new class ID
            preserve_level: Keep the same total level
            cheat_mode: Bypass validation if True
            
        Returns:
            Dict with all changes made
        """
        logger.info(f"Changing class to {new_class_id}")
        
        # Start tracking changes
        changes = {
            'class_change': {
                'old_class': None,
                'new_class': new_class_id,
                'level': 0
            },
            'stats_updated': {},
            'feats_changed': {'removed': [], 'added': []},
            'spells_changed': {},
            'skills_reset': False,
            'custom_content_preserved': []
        }
        
        # Basic validation for save file integrity only
        if not cheat_mode:
            new_class = self.game_data_loader.get_by_id('classes', new_class_id)
            if not new_class:
                raise ValueError(f"Invalid class ID: {new_class_id}")
        
        # Get class info
        new_class = self.game_data_loader.get_by_id('classes', new_class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {new_class_id}")
        
        # Get current state
        class_list = self.gff.get('ClassList', [])
        old_class_id = class_list[0].get('Class', 0) if class_list else None
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) or 1
        
        changes['class_change']['old_class'] = old_class_id
        changes['class_change']['level'] = total_level
        
        # Begin transaction if not already in one
        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True
        
        try:
            # 1. Update class list
            self._update_class_list(new_class_id, total_level)
            
            # 2. Update derived stats
            stat_changes = self._update_class_stats(new_class, total_level)
            changes['stats_updated'] = stat_changes
            
            # 3. Emit class changed event
            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,  # Will be overridden by __post_init__
                source_manager='class',
                timestamp=time.time(),
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                level=total_level,
                preserve_feats=self._get_preserved_feats()
            )
            self.character_manager.emit(event)
            
            # 4. Handle skills (will be updated by SkillManager via event)
            changes['skills_reset'] = True
            
            # Commit transaction if we started it
            if transaction_started:
                self.character_manager.commit_transaction()
            
        except Exception as e:
            # Rollback on error if we started the transaction
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during class change: {e}")
            raise
        
        return changes
    
    def add_class_level(self, class_id: int, cheat_mode: bool = False) -> Dict[str, Any]:
        """
        Add a level in a specific class (multiclassing or leveling up)
        
        Args:
            class_id: Class to add level in
            cheat_mode: Bypass validation
            
        Returns:
            Dict with changes made
        """
        logger.info(f"Adding level in class {class_id}")
        
        # Basic validation for save file integrity only
        if not cheat_mode:
            new_class = self.game_data_loader.get_by_id('classes', class_id)
            if not new_class:
                raise ValueError(f"Invalid class ID: {class_id}")
        
        # Get class info
        new_class = self.game_data_loader.get_by_id('classes', class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {class_id}")
        
        # Update class list
        class_list = self.gff.get('ClassList', [])
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) + 1
        
        # Find existing class or add new
        class_found = False
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                class_entry['ClassLevel'] += 1
                class_found = True
                break
        
        if not class_found:
            class_list.append({
                'Class': class_id,
                'ClassLevel': 1
            })
        
        self.gff.set('ClassList', class_list)
        
        # Emit level gained event
        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,  # Will be overridden by __post_init__
            source_manager='class',
            timestamp=time.time(),
            class_id=class_id,
            new_level=total_level,
            total_level=total_level
        )
        self.emit(event)
        
        return {
            'class_id': class_id,
            'new_total_level': total_level,
            'multiclass': not class_found
        }
    
    
    
    def _update_class_list(self, new_class_id: int, total_level: int):
        """Update the character's class list"""
        # Clear existing classes
        self.gff.set('ClassList', [{
            'Class': new_class_id,
            'ClassLevel': total_level
        }])
        
        # Update primary class field
        self.gff.set('Class', new_class_id)
    
    def _update_class_stats(self, new_class, total_level: int) -> Dict[str, Any]:
        """Update HP, BAB, saves based on new class"""
        changes = {}
        
        # Get ability modifiers
        modifiers = self._calculate_ability_modifiers()
        
        # 1. Hit Points
        old_hp = self.gff.get('HitPoints', 0)
        new_hp = self._calculate_hit_points(new_class, total_level, modifiers['CON'])
        self.gff.set('HitPoints', new_hp)
        self.gff.set('MaxHitPoints', new_hp)
        self.gff.set('CurrentHitPoints', new_hp)
        changes['hit_points'] = {'old': old_hp, 'new': new_hp}
        
        # 2. Base Attack Bonus - use total from all classes for multiclass
        old_bab = self.gff.get('BaseAttackBonus', 0)
        new_bab = self.calculate_total_bab()
        self.gff.set('BaseAttackBonus', new_bab)
        changes['bab'] = {'old': old_bab, 'new': new_bab}
        
        # 3. Saves - use total from all classes for multiclass
        saves = self.calculate_total_saves()
        old_saves = {
            'fortitude': self.gff.get('FortSave', 0),
            'reflex': self.gff.get('RefSave', 0),
            'will': self.gff.get('WillSave', 0)
        }
        
        self.gff.set('FortSave', saves['fortitude'])
        self.gff.set('RefSave', saves['reflex'])
        self.gff.set('WillSave', saves['will'])
        
        changes['saves'] = {
            'fortitude': {'old': old_saves['fortitude'], 'new': saves['fortitude']},
            'reflex': {'old': old_saves['reflex'], 'new': saves['reflex']},
            'will': {'old': old_saves['will'], 'new': saves['will']}
        }
        
        return changes
    
    def _calculate_ability_modifiers(self) -> Dict[str, int]:
        """Calculate ability modifiers"""
        # Use CharacterManager's method if available
        if hasattr(self.character_manager, 'get_ability_scores'):
            scores = self.character_manager.get_ability_scores()
            return {
                'STR': (scores.get('strength', 10) - 10) // 2,
                'DEX': (scores.get('dexterity', 10) - 10) // 2,
                'CON': (scores.get('constitution', 10) - 10) // 2,
                'INT': (scores.get('intelligence', 10) - 10) // 2,
                'WIS': (scores.get('wisdom', 10) - 10) // 2,
                'CHA': (scores.get('charisma', 10) - 10) // 2
            }
        
        # Fallback to direct GFF access
        abilities = {
            'STR': self.gff.get('Str', 10),
            'DEX': self.gff.get('Dex', 10),
            'CON': self.gff.get('Con', 10),
            'INT': self.gff.get('Int', 10),
            'WIS': self.gff.get('Wis', 10),
            'CHA': self.gff.get('Cha', 10)
        }
        
        return {
            ability: (value - 10) // 2
            for ability, value in abilities.items()
        }
    
    def _calculate_hit_points(self, class_data, level: int, con_modifier: int) -> int:
        """Calculate total hit points for class and level"""
        # Max HP at level 1, average for other levels
        # Get the correct attribute name (case-sensitive)
        hit_die = getattr(class_data, 'HitDie', None) or getattr(class_data, 'hit_die', 4)
        if isinstance(hit_die, str):
            hit_die = int(hit_die)  # Convert string to int if necessary
        base_hp = hit_die
        if level > 1:
            avg_roll = (hit_die + 1) // 2
            base_hp += avg_roll * (level - 1)
        
        total_hp = base_hp + (con_modifier * level)
        return max(1, total_hp)  # Minimum 1 HP
    
    def _calculate_bab(self, class_data, level: int) -> int:
        """Calculate BAB for a single class and level"""
        # Get the correct attribute name (case-sensitive)
        bab_table_name = getattr(class_data, 'AttackBonusTable', '') or getattr(class_data, 'attack_bonus_table', '')
        if not bab_table_name:
            logger.warning(f"No BAB table found for class {getattr(class_data, 'Label', 'Unknown')}")
            return 0
            
        bab_table_name = bab_table_name.lower()
        
        # Cache BAB table data
        if bab_table_name not in self._bab_table_cache:
            bab_table = self.game_data_loader.get_table(bab_table_name)
            if bab_table:
                self._bab_table_cache[bab_table_name] = bab_table
            else:
                logger.warning(f"BAB table '{bab_table_name}' not found")
                return 0
        
        bab_table = self._bab_table_cache[bab_table_name]
        
        # Get BAB for level (level - 1 because tables are 0-indexed)
        level_idx = min(level - 1, 19)  # Cap at 20
        if level_idx < len(bab_table):
            bab_row = bab_table[level_idx]
            # The BAB value is in the 'BAB' column (uppercase, values are strings)
            bab_value = getattr(bab_row, 'BAB', '0')
            try:
                return int(bab_value)
            except (ValueError, TypeError):
                logger.warning(f"Invalid BAB value '{bab_value}' in table '{bab_table_name}' at level {level}")
                return 0
        
        return 0
    
    def calculate_total_bab(self) -> int:
        """
        Calculate total BAB from all classes (for multiclass characters)
        
        Returns:
            Total base attack bonus
        """
        class_list = self.gff.get('ClassList', [])
        total_bab = 0
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_level = class_info.get('ClassLevel', 0)
            
            if class_level > 0:
                class_data = self.game_data_loader.get_by_id('classes', class_id)
                if class_data:
                    class_bab = self._calculate_bab(class_data, class_level)
                    total_bab += class_bab
                    class_label = getattr(class_data, 'label', f'Class {class_id}')
                    logger.debug(f"Class {class_label} (lvl {class_level}): BAB +{class_bab}")
        
        logger.info(f"Total BAB: {total_bab}")
        return total_bab
    
    def _calculate_saves(self, class_data, level: int, modifiers: Dict[str, int]) -> Dict[str, int]:
        """Calculate saving throws for a single class"""
        # Get the correct attribute name (case-sensitive)
        save_table_name = getattr(class_data, 'SavingThrowTable', '') or getattr(class_data, 'savingthrowtable', '')
        if not save_table_name:
            logger.warning(f"No save table found for class {getattr(class_data, 'Label', 'Unknown')}")
            return {
                'fortitude': modifiers['CON'],
                'reflex': modifiers['DEX'],
                'will': modifiers['WIS']
            }
            
        save_table_name = save_table_name.lower()
        
        # Cache save table data
        if save_table_name not in self._save_table_cache:
            save_table = self.game_data_loader.get_table(save_table_name)
            if save_table:
                self._save_table_cache[save_table_name] = save_table
            else:
                logger.warning(f"Save table '{save_table_name}' not found")
                return {
                    'fortitude': modifiers['CON'],
                    'reflex': modifiers['DEX'],
                    'will': modifiers['WIS']
                }
        
        save_table = self._save_table_cache[save_table_name]
        
        # Get saves for level (level - 1 because tables are 0-indexed)
        level_idx = min(level - 1, 19)  # Cap at 20
        if level_idx < len(save_table):
            save_row = save_table[level_idx]
            # The save values are in 'FortSave', 'RefSave', 'WillSave' columns (values are strings)
            try:
                fort_base = int(getattr(save_row, 'FortSave', '0'))
                ref_base = int(getattr(save_row, 'RefSave', '0'))
                will_base = int(getattr(save_row, 'WillSave', '0'))
            except (ValueError, TypeError) as e:
                logger.warning(f"Invalid save values in table '{save_table_name}' at level {level}: {e}")
                fort_base = ref_base = will_base = 0
        else:
            fort_base = ref_base = will_base = 0
        
        return {
            'fortitude': fort_base + modifiers['CON'],
            'reflex': ref_base + modifiers['DEX'],
            'will': will_base + modifiers['WIS']
        }
    
    def calculate_total_saves(self) -> Dict[str, int]:
        """
        Calculate total saving throws from all classes (for multiclass)
        
        Returns:
            Dict with fortitude, reflex, and will saves
        """
        class_list = self.gff.get('ClassList', [])
        
        # Get ability modifiers
        modifiers = self._calculate_ability_modifiers()
        con_mod = modifiers['CON']
        dex_mod = modifiers['DEX']
        wis_mod = modifiers['WIS']
        
        # For multiclass, take the best base save from any class
        best_fort = 0
        best_ref = 0
        best_will = 0
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_level = class_info.get('ClassLevel', 0)
            
            if class_level > 0:
                class_data = self.game_data_loader.get_by_id('classes', class_id)
                if class_data:
                    # Get base saves without modifiers
                    saves = self._calculate_saves(class_data, class_level, {'CON': 0, 'DEX': 0, 'WIS': 0})
                    best_fort = max(best_fort, saves['fortitude'])
                    best_ref = max(best_ref, saves['reflex'])
                    best_will = max(best_will, saves['will'])
        
        return {
            'fortitude': best_fort + con_mod,
            'reflex': best_ref + dex_mod,
            'will': best_will + wis_mod,
            'base_fortitude': best_fort,
            'base_reflex': best_ref,
            'base_will': best_will
        }
    
    def _get_preserved_feats(self) -> List[int]:
        """Get list of feat IDs that should be preserved during class change"""
        preserved = set()
        
        # 1. Get epithet/story feats
        epithet_feats = self.character_manager.detect_epithet_feats()
        preserved.update(epithet_feats)
        
        # 2. Get custom content that should be preserved
        for content_id, info in self.character_manager.custom_content.items():
            if info['type'] == 'feat' and not info.get('removable', True):
                preserved.add(info['id'])
        
        # 3. Get racial feats
        race_id = self.gff.get('Race', 0)
        racial_feats = self._get_racial_feats(race_id)
        preserved.update(racial_feats)
        
        # 4. Get background/history feats (typically have specific IDs or patterns)
        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            
            # Preserve domain feats (usually have IDs in specific ranges)
            if 4000 <= feat_id <= 4999:  # Common domain feat range
                preserved.add(feat_id)
            
            # Preserve background feats
            if self._is_background_feat(feat_id):
                preserved.add(feat_id)
        
        logger.info(f"Preserving {len(preserved)} feats during class change")
        return list(preserved)
    
    def get_class_summary(self) -> Dict[str, Any]:
        """Get summary of character's classes"""
        class_list = self.gff.get('ClassList', [])
        
        classes = []
        for c in class_list:
            class_id = c.get('Class', 0)
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            
            # Safely get class name
            if class_data:
                class_name = getattr(class_data, 'label', getattr(class_data, 'name', f"Unknown Class {class_id}"))
            else:
                class_name = f"Unknown Class {class_id}"
            
            classes.append({
                'id': class_id,
                'level': c.get('ClassLevel', 0),
                'name': class_name
            })
        
        return {
            'classes': classes,
            'total_level': sum(c.get('ClassLevel', 0) for c in class_list),
            'multiclass': len(class_list) > 1,
            'can_multiclass': len(class_list) < 3
        }
    
    def get_attack_bonuses(self) -> Dict[str, Any]:
        """
        Get all attack bonuses including BAB and ability modifiers
        
        Returns:
            Dict with melee, ranged, and touch attack bonuses
        """
        bab = self.calculate_total_bab()
        
        # Get ability modifiers
        modifiers = self._calculate_ability_modifiers()
        str_mod = modifiers['STR']
        dex_mod = modifiers['DEX']
        
        # Check for Weapon Finesse
        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')
        
        # Calculate attack bonuses
        melee_bonus = bab + str_mod
        finesse_bonus = bab + dex_mod if has_weapon_finesse else None
        ranged_bonus = bab + dex_mod
        touch_bonus = bab  # Touch attacks ignore armor, use BAB only
        
        # Multiple attacks at higher BAB
        attacks = []
        current_bab = bab
        while current_bab > 0:
            attacks.append(current_bab)
            current_bab -= 5
        
        return {
            'base_attack_bonus': bab,
            'melee_attack_bonus': melee_bonus,
            'finesse_attack_bonus': finesse_bonus,
            'ranged_attack_bonus': ranged_bonus,
            'touch_attack_bonus': touch_bonus,
            'multiple_attacks': attacks,
            'str_modifier': str_mod,
            'dex_modifier': dex_mod,
            'has_weapon_finesse': has_weapon_finesse
        }
    
    def _has_feat_by_name(self, feat_label: str) -> bool:
        """Check if character has a feat by its label"""
        # Use CharacterManager's method if available
        if hasattr(self.character_manager, 'has_feat_by_name'):
            return self.character_manager.has_feat_by_name(feat_label)
        
        # Fallback implementation
        feat_list = self.gff.get('FeatList', [])
        
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            feat_data = self.game_data_loader.get_by_id('feat', feat_id)
            if feat_data:
                label = getattr(feat_data, 'label', '')
                if label == feat_label:
                    return True
        
        return False
    
    def _get_racial_feats(self, race_id: int) -> List[int]:
        """Get racial feats for a specific race"""
        racial_feats = []
        
        # Get race data
        race_data = self.game_data_loader.get_by_id('racialtypes', race_id)
        if not race_data:
            return racial_feats
        
        # Check for racial feat table
        feat_table_name = getattr(race_data, 'feat_table', None)
        if feat_table_name:
            feat_table = self.game_data_loader.get_table(feat_table_name.lower())
            if feat_table:
                for feat_entry in feat_table:
                    feat_id = getattr(feat_entry, 'feat_index', -1)
                    if feat_id >= 0:
                        racial_feats.append(feat_id)
        
        return racial_feats
    
    def _is_background_feat(self, feat_id: int) -> bool:
        """Check if a feat is a background/history feat that should be preserved"""
        # Get feat data
        feat_data = self.game_data_loader.get_by_id('feat', feat_id)
        if not feat_data:
            return False
        
        # Check feat properties
        label = getattr(feat_data, 'label', '').lower()
        category = getattr(feat_data, 'categories', '').lower()
        
        # Background feat patterns
        background_patterns = [
            'background', 'history', 'past', 'origin',
            'blessing', 'curse', 'gift', 'legacy',
            'shard', 'silver', 'influence', 'touched'
        ]
        
        for pattern in background_patterns:
            if pattern in label or pattern in category:
                return True
        
        # Check if feat cannot be removed (indicator of special feat)
        removable = getattr(feat_data, 'removable', 1)
        if removable == 0:
            return True
        
        return False
    
    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current class configuration - corruption prevention only"""
        errors = []
        
        class_list = self.gff.get('ClassList', [])
        
        # Check for valid classes (prevent crashes from invalid class references)
        for class_entry in class_list:
            class_id = class_entry.get('Class', 0)
            class_data = self.game_data_loader.get_by_id('classes', class_id)
            if not class_data:
                errors.append(f"Invalid class ID: {class_id}")
        
        # Check level bounds (prevent GFF corruption)
        total_level = sum(c.get('ClassLevel', 0) for c in class_list)
        if total_level > 60:  # NWN2 max with epic levels - prevent GFF corruption
            errors.append(f"Total level {total_level} exceeds maximum of 60")
        if total_level < 1:
            errors.append("Total level must be at least 1")
        
        return len(errors) == 0, errors
    
    def get_class_by_id(self, class_id: int) -> Optional[Dict[str, Any]]:
        """
        Get class info for a specific class in ClassList
        
        Args:
            class_id: The class ID to look up
            
        Returns:
            Class info dict or None if not found
        """
        class_list = self.gff.get('ClassList', [])
        
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                class_data = self.game_data_loader.get_by_id('classes', class_id)
                return {
                    'id': class_id,
                    'level': class_entry.get('ClassLevel', 0),
                    'name': getattr(class_data, 'label', 'Unknown') if class_data else 'Unknown',
                    'data': class_data
                }
        
        return None
    
    def remove_class(self, class_id: int) -> Dict[str, Any]:
        """
        Remove a class from multiclass (keeping others)
        
        Args:
            class_id: Class ID to remove
            
        Returns:
            Summary of changes
        """
        class_list = self.gff.get('ClassList', [])
        
        # Find the class to remove
        class_to_remove = None
        for i, class_entry in enumerate(class_list):
            if class_entry.get('Class') == class_id:
                class_to_remove = i
                break
        
        if class_to_remove is None:
            raise ValueError(f"Character does not have class {class_id}")
        
        if len(class_list) <= 1:
            raise ValueError("Cannot remove last remaining class")
        
        # Begin transaction
        txn = self.character_manager.begin_transaction()
        
        try:
            # Remove the class
            removed_class = class_list.pop(class_to_remove)
            self.gff.set('ClassList', class_list)
            
            # Recalculate stats
            self._recalculate_all_stats()
            
            # Emit event
            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                old_class_id=class_id,
                new_class_id=-1,  # Indicates removal
                level=removed_class.get('ClassLevel', 0)
            )
            self.character_manager.emit(event)
            
            self.character_manager.commit_transaction()
            
            return {
                'removed_class': class_id,
                'removed_levels': removed_class.get('ClassLevel', 0),
                'remaining_classes': len(class_list)
            }
            
        except Exception as e:
            self.character_manager.rollback_transaction()
            raise
    
    def get_prestige_class_options(self) -> List[Dict[str, Any]]:
        """
        Get available prestige classes based on current character
        
        Returns:
            List of prestige class options with requirements
        """
        available_prestige = []
        
        # Get all classes
        classes_table = self.game_data_loader.get_table('classes')
        if not classes_table:
            return available_prestige
        
        for class_data in classes_table:
            # Check if it's a prestige class
            is_prestige = getattr(class_data, 'is_prestige', 0)
            if not is_prestige:
                continue
            
            class_id = getattr(class_data, 'id', -1)
            if class_id < 0:
                continue
            
            # Check if character can take this prestige class
            can_take, reason = self.can_take_prestige_class(class_id)
            
            available_prestige.append({
                'id': class_id,
                'name': getattr(class_data, 'label', 'Unknown'),
                'can_take': can_take,
                'reason': reason,
                'requirements': self._get_prestige_requirements(class_data)
            })
        
        return available_prestige
    
    def can_take_prestige_class(self, prestige_id: int) -> Tuple[bool, str]:
        """
        Check if character meets prestige class requirements
        
        Args:
            prestige_id: Prestige class ID
            
        Returns:
            (can_take, reason) tuple
        """
        # Use character manager's prerequisite checking
        can_take, errors = self.character_manager.check_prerequisites('class', prestige_id)
        
        if can_take:
            return True, "All requirements met"
        else:
            return False, "; ".join(errors)
    
    def _get_prestige_requirements(self, class_data) -> Dict[str, Any]:
        """Extract prestige class requirements from class data"""
        requirements = {}
        
        # Base attack bonus requirement
        min_bab = getattr(class_data, 'min_attack_bonus', 0)
        if min_bab > 0:
            requirements['base_attack_bonus'] = min_bab
        
        # Skill requirements
        skill_req = getattr(class_data, 'required_skill', '')
        skill_rank = getattr(class_data, 'required_skill_rank', 0)
        if skill_req and skill_rank > 0:
            requirements['skills'] = {skill_req: skill_rank}
        
        # Feat requirements
        req_feat = getattr(class_data, 'required_feat', '')
        if req_feat:
            requirements['feats'] = [req_feat]
        
        # Alignment restrictions
        alignment_restrict = getattr(class_data, 'alignment_restrict', 0)
        if alignment_restrict > 0:
            requirements['alignment'] = self._decode_alignment_restriction(alignment_restrict)
        
        return requirements
    
    def _decode_alignment_restriction(self, restriction: int) -> str:
        """Decode alignment restriction bitmask"""
        # Common alignment restrictions
        restrictions = {
            0x01: "Lawful",
            0x02: "Chaotic", 
            0x04: "Good",
            0x08: "Evil",
            0x10: "Neutral (Law/Chaos)",
            0x20: "Neutral (Good/Evil)"
        }
        
        allowed = []
        for mask, name in restrictions.items():
            if restriction & mask:
                allowed.append(name)
        
        return " or ".join(allowed) if allowed else "Any"
    
    def get_class_features(self, class_id: int, level: int) -> Dict[str, Any]:
        """
        Get features gained at specific level for a class
        
        Args:
            class_id: The class ID
            level: The level to check
            
        Returns:
            Dict of features gained at this level
        """
        features = {
            'feats': [],
            'abilities': [],
            'spells': {},
            'bab_increase': 0,
            'save_increases': {},
            'skill_points': 0
        }
        
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            return features
        
        # Get feats granted at this level
        feats = self.character_manager.get_class_feats_for_level(class_data, level)
        features['feats'] = feats
        
        # Get special abilities
        abilities = self.character_manager.get_class_abilities(class_id, level)
        features['abilities'] = abilities
        
        # Calculate BAB increase
        if level > 1:
            bab_current = self._calculate_bab(class_data, level)
            bab_previous = self._calculate_bab(class_data, level - 1)
            features['bab_increase'] = bab_current - bab_previous
        else:
            features['bab_increase'] = self._calculate_bab(class_data, 1)
        
        # Calculate save increases
        for save_type in ['fortitude', 'reflex', 'will']:
            if level > 1:
                save_current = self._calculate_single_save(class_data, save_type, level)
                save_previous = self._calculate_single_save(class_data, save_type, level - 1)
                increase = save_current - save_previous
            else:
                increase = self._calculate_single_save(class_data, save_type, 1)
            
            if increase > 0:
                features['save_increases'][save_type] = increase
        
        # Get skill points
        skill_points = getattr(class_data, 'skill_point_base', 2)
        features['skill_points'] = skill_points
        
        return features
    
    def _calculate_single_save(self, class_data, save_type: str, level: int) -> int:
        """Calculate a single save value for a class at a level"""
        save_table_name = getattr(class_data, f'{save_type}_save_table', None)
        if save_table_name:
            save_table = self._get_save_table(save_table_name)
            if save_table and level <= len(save_table):
                return save_table[level - 1]
        
        # Fallback to good/poor save progression
        return self._calculate_save_progression(level, save_type in ['fortitude'])
    
    def _calculate_save_progression(self, level: int, is_good_save: bool) -> int:
        """Calculate save bonus based on good/poor progression"""
        if is_good_save:
            return 2 + (level // 2)
        else:
            return level // 3
    
    def _recalculate_all_stats(self):
        """Recalculate all class-dependent stats after class change"""
        # Recalculate BAB
        total_bab = self.calculate_total_bab()
        self.gff.set('BaseAttackBonus', total_bab)
        
        # Recalculate saves
        saves = self.calculate_total_saves()
        self.gff.set('FortSaveBase', saves['base_fortitude'])
        self.gff.set('RefSaveBase', saves['base_reflex'])
        self.gff.set('WillSaveBase', saves['base_will'])
    
    def _get_save_table(self, save_table_name: str) -> Optional[List[int]]:
        """Get save progression table"""
        if save_table_name in self._save_table_cache:
            return self._save_table_cache[save_table_name]
        
        # Load save table
        save_table_data = self.game_data_loader.get_table(save_table_name.lower())
        if not save_table_data:
            return None
        
        # Extract save values
        save_values = []
        for entry in save_table_data:
            save_value = getattr(entry, 'save_throw', 0)
            save_values.append(save_value)
        
        self._save_table_cache[save_table_name] = save_values
        return save_values
    
    def get_class_progression_summary(self, class_id: int, max_level: int = 20) -> Dict[str, Any]:
        """
        Get complete class progression summary for UI display
        
        Args:
            class_id: The class ID to analyze
            max_level: Maximum level to show progression for
            
        Returns:
            Dict with progression data formatted for frontend display
        """
        class_data = self.game_data_loader.get_by_id('classes', class_id)
        if not class_data:
            return {}
        
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        # Get basic class info
        class_name = field_mapper.get_field_value(class_data, 'Name', 'Unknown Class')
        hit_die = int(field_mapper.get_field_value(class_data, 'HitDie', '8'))
        skill_points = int(field_mapper.get_field_value(class_data, 'SkillPointBase', '2'))
        
        # Build level progression
        progression = []
        for level in range(1, min(max_level + 1, 21)):
            level_info = {
                'level': level,
                'hit_die': hit_die,
                'skill_points': skill_points,
                'bab': self._calculate_bab(class_data, level),
                'saves': self._get_saves_for_level_detailed(class_data, level),
                'features': self._get_level_features(class_data, level),
                'new_features': self._get_new_features_at_level(class_data, level)
            }
            progression.append(level_info)
        
        # Get class categories and type
        class_info = self._analyze_class_type(class_data)
        
        return {
            'class_id': class_id,
            'class_name': class_name,
            'class_info': class_info,
            'basic_stats': {
                'hit_die': hit_die,
                'skill_points_per_level': skill_points,
                'is_spellcaster': class_info['is_spellcaster'],
                'spell_type': class_info['spell_type'],
                'primary_ability': class_info['primary_ability']
            },
            'progression': progression,
            'proficiencies': self._get_class_proficiencies_detailed(class_data),
            'special_abilities': self._get_class_special_abilities(class_data)
        }
    
    def _get_saves_for_level_detailed(self, class_data, level: int) -> Dict[str, int]:
        """Get detailed save progression for a specific level"""
        save_table_name = getattr(class_data, 'SavingThrowTable', '')
        if not save_table_name:
            # Use standard progression if no table
            return {
                'fortitude': level // 3,  # Poor save
                'reflex': level // 3,     # Poor save  
                'will': level // 3        # Poor save
            }
        
        save_table_name = save_table_name.lower()
        save_table = self.game_data_loader.get_table(save_table_name)
        
        if not save_table or level > len(save_table):
            return {'fortitude': 0, 'reflex': 0, 'will': 0}
        
        row = save_table[level - 1]
        return {
            'fortitude': int(getattr(row, 'FortSave', '0') or 0),
            'reflex': int(getattr(row, 'RefSave', '0') or 0),
            'will': int(getattr(row, 'WillSave', '0') or 0)
        }
    
    def _get_level_features(self, class_data, level: int) -> List[Dict[str, Any]]:
        """Get all features available at a specific level"""
        features = []
        
        # Level 1 always gets proficiencies
        if level == 1:
            features.append({
                'name': 'Weapon and Armor Proficiencies',
                'type': 'proficiency',
                'description': 'Class weapon and armor proficiencies',
                'icon': 'sword'
            })
        
        # Check for bonus feats (common pattern)
        if self._class_gets_bonus_feats(class_data) and level % 2 == 0:
            features.append({
                'name': 'Bonus Feat',
                'type': 'feat',
                'description': f'Choose a bonus feat at level {level}',
                'icon': 'star'
            })
        
        # Spellcasting progression
        if self._is_spellcaster_class_data(class_data):
            spell_info = self._get_spell_progression_at_level(class_data, level)
            if spell_info:
                features.append({
                    'name': 'Spell Progression',
                    'type': 'spell',
                    'description': f'Gain access to new spell levels',
                    'details': spell_info,
                    'icon': 'magic'
                })
        
        return features
    
    def _get_new_features_at_level(self, class_data, level: int) -> List[Dict[str, Any]]:
        """Get only new features gained specifically at this level"""
        new_features = []
        
        # This is where we'd parse class-specific feature tables
        # For now, implement common patterns
        
        if level == 1:
            new_features.append({
                'name': 'Class Skills',
                'type': 'skill',
                'description': 'Access to class skill list',
                'icon': 'book'
            })
        
        # Every 3rd level for some classes
        if level % 3 == 0 and level > 1:
            class_focus = self._get_class_focus(class_data)
            if class_focus == 'combat':
                new_features.append({
                    'name': 'Combat Improvement',
                    'type': 'combat',
                    'description': f'Enhanced combat abilities at level {level}',
                    'icon': 'sword'
                })
        
        return new_features
    
    def _analyze_class_type(self, class_data) -> Dict[str, Any]:
        """Analyze class characteristics"""
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        has_arcane = field_mapper.get_field_value(class_data, 'HasArcane', '0') == '1'
        has_divine = field_mapper.get_field_value(class_data, 'HasDivine', '0') == '1'
        primary_ability = field_mapper.get_field_value(class_data, 'PrimaryAbil', 'STR')
        
        spell_type = 'none'
        if has_arcane:
            spell_type = 'arcane'
        elif has_divine:
            spell_type = 'divine'
        
        return {
            'is_spellcaster': has_arcane or has_divine,
            'spell_type': spell_type,
            'primary_ability': primary_ability,
            'focus': self._get_class_focus(class_data),
            'alignment_restricted': field_mapper.get_field_value(class_data, 'AlignRestrict', '0') != '0'
        }
    
    def _get_class_focus(self, class_data) -> str:
        """Determine class focus/role"""
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        has_arcane = field_mapper.get_field_value(class_data, 'HasArcane', '0') == '1'
        has_divine = field_mapper.get_field_value(class_data, 'HasDivine', '0') == '1'
        skill_points = int(field_mapper.get_field_value(class_data, 'SkillPointBase', '2'))
        hit_die = int(field_mapper.get_field_value(class_data, 'HitDie', '8'))
        
        if has_arcane:
            return 'arcane_caster'
        elif has_divine:
            return 'divine_caster'
        elif skill_points >= 6:
            return 'skill_specialist'
        elif hit_die >= 10:
            return 'combat'
        else:
            return 'hybrid'
    
    def _class_gets_bonus_feats(self, class_data) -> bool:
        """Check if class gets bonus feats (like Fighter)"""
        class_name = getattr(class_data, 'Name', '').lower()
        return 'fighter' in class_name or 'warrior' in class_name
    
    def _is_spellcaster_class_data(self, class_data) -> bool:
        """Check if class data indicates spellcasting"""
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        has_arcane = field_mapper.get_field_value(class_data, 'HasArcane', '0') == '1'
        has_divine = field_mapper.get_field_value(class_data, 'HasDivine', '0') == '1'
        return has_arcane or has_divine
    
    def _get_spell_progression_at_level(self, class_data, level: int) -> Optional[Dict[str, Any]]:
        """Get spell slot progression for a level (placeholder for now)"""
        if not self._is_spellcaster_class_data(class_data):
            return None
        
        # TODO: Implement actual spell table lookup
        # For now, return basic info
        return {
            'new_spell_level': max(0, (level + 1) // 2),
            'description': f'Spell casting progression at level {level}'
        }
    
    def _get_class_proficiencies_detailed(self, class_data) -> Dict[str, Any]:
        """Get detailed weapon and armor proficiencies"""
        from gamedata.dynamic_loader.field_mapping_utility import FieldMappingUtility
        field_mapper = FieldMappingUtility()
        
        # TODO: Parse actual proficiency data from class tables
        # This is a placeholder implementation
        class_focus = self._get_class_focus(class_data)
        
        proficiencies = {
            'weapons': [],
            'armor': [],
            'shields': False
        }
        
        if class_focus == 'combat':
            proficiencies['weapons'] = ['Simple', 'Martial']
            proficiencies['armor'] = ['Light', 'Medium', 'Heavy']
            proficiencies['shields'] = True
        elif class_focus in ['arcane_caster', 'divine_caster']:
            proficiencies['weapons'] = ['Simple']
            proficiencies['armor'] = ['Light'] if class_focus == 'divine_caster' else []
            proficiencies['shields'] = class_focus == 'divine_caster'
        else:
            proficiencies['weapons'] = ['Simple']
            proficiencies['armor'] = ['Light']
            proficiencies['shields'] = False
        
        return proficiencies
    
    def _get_class_special_abilities(self, class_data) -> List[Dict[str, Any]]:
        """Get class special abilities (placeholder)"""
        # TODO: Parse actual special abilities from class data
        abilities = []
        
        class_name = getattr(class_data, 'Name', '').lower()
        
        if 'rogue' in class_name:
            abilities.append({
                'name': 'Sneak Attack',
                'description': 'Deal extra damage when flanking or attacking flat-footed enemies',
                'progression': 'Increases every 2 levels'
            })
        elif 'barbarian' in class_name:
            abilities.append({
                'name': 'Rage',
                'description': 'Enter a berserker rage for combat bonuses',
                'progression': 'Additional uses per day at higher levels'
            })
        
        return abilities