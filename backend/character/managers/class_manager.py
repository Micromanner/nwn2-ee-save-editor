"""Class Manager - handles class changes, multiclassing, and level progression."""

from typing import Dict, List, Tuple, Optional, Any
from loguru import logger
import time

from ..events import EventEmitter, EventType, ClassChangedEvent, LevelGainedEvent, SpellChangedEvent
from ..xp_utils import get_xp_table, xp_to_level, level_to_xp
from ..xp_utils import get_xp_table, xp_to_level, level_to_xp
from gamedata.dynamic_loader.field_mapping_utility import field_mapper
from services.gamedata.class_categorizer import ClassCategorizer, ClassType
from gamedata.dynamic_loader.singleton import get_dynamic_game_data_loader

LVL_STAT_LIST = "LvlStatList"
LVL_STAT_CLASS = "LvlStatClass"
LVL_STAT_HITDIE = "LvlStatHitDie"
LVL_STAT_ABILITY = "LvlStatAbility"
LVL_STAT_SKILL_LIST = "SkillList"
LVL_STAT_SKILL_POINTS = "SkillPoints"
LVL_STAT_FEAT_LIST = "FeatList"
LVL_STAT_KNOWN_LIST = "KnownList"
LVL_STAT_KNOWN_REMOVE_LIST = "KnownRemoveList"

class ClassManager(EventEmitter):
    """Manages character class changes and progression."""

    def __init__(self, character_manager):
        """Initialize ClassManager with parent CharacterManager."""
        super().__init__()
        self.character_manager = character_manager
        self.gff = character_manager.gff
        self.rules_service = character_manager.rules_service

        self._class_cache = {}
        self._bab_table_cache = {}
        self._save_table_cache = {}

        self._register_event_handlers()
    
    def _register_event_handlers(self):
        """Register handlers for relevant events."""
        self.character_manager.on(EventType.SKILL_POINTS_AWARDED, self.on_skill_points_awarded)
        self.character_manager.on(EventType.SPELL_LEARNED, self._on_spell_changed)
        self.character_manager.on(EventType.SPELL_FORGOTTEN, self._on_spell_changed)

    def _on_spell_changed(self, event: SpellChangedEvent):
        """Handle spell learned/forgotten events to sync to level history."""
        added = event.action == 'learned'
        self.record_spell_change(event.spell_level, event.spell_id, added)
    
    def _get_class_name(self, class_id: int, class_data=None) -> str:
        """Get class name, resolving TLK strref for proper localized name."""
        if class_data is None:
            class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            raise ValueError(f"Class ID {class_id} not found in classes.2da")

        name_value = field_mapper.get_field_value(class_data, 'name')
        if name_value is not None:
            if isinstance(name_value, str) and name_value.strip() and not name_value.isdigit():
                return name_value
            strref = field_mapper._safe_int(name_value, 0)
            if strref > 0:
                resolved_name = self.rules_service._loader.get_string(strref)
                if resolved_name and not resolved_name.startswith('{StrRef:'):
                    return resolved_name

        label = field_mapper.get_field_value(class_data, 'label')
        if label and str(label).strip():
            return str(label)

        raise ValueError(f"Class ID {class_id} has no name or label in classes.2da")
    
    def change_class(self, new_class_id: int, preserve_level: bool = True) -> Dict[str, Any]:
        """Change character's primary class."""
        logger.info(f"Changing class to {new_class_id}")

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

        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {new_class_id}")

        class_list = self.gff.get('ClassList', [])
        old_class_id = class_list[0].get('Class', 0) if class_list else None
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) or 1

        changes['class_change']['old_class'] = old_class_id
        changes['class_change']['level'] = total_level

        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True

        try:
            self._update_class_list(new_class_id, total_level)
            stat_changes = self._update_class_stats(new_class, total_level)
            changes['stats_updated'] = stat_changes

            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                timestamp=time.time(),
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                level=total_level,
                preserve_feats=self._get_preserved_feats()
            )
            self.character_manager.emit(event)

            changes['skills_reset'] = True

            if transaction_started:
                self.character_manager.commit_transaction()

        except Exception as e:
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during class change: {e}")
            raise

        return changes
    
    def add_class_level(self, class_id: int) -> Dict[str, Any]:
        """Add a level in a specific class."""
        logger.info(f"Adding level in class {class_id}")

        new_class = self.rules_service.get_by_id('classes', class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {class_id}")

        class_list = self.gff.get('ClassList', [])
        total_level = sum(c.get('ClassLevel', 0) for c in class_list) + 1

        current_xp = self.get_experience()
        min_xp = level_to_xp(total_level, self.rules_service)
        if current_xp < min_xp:
            logger.info(f"Auto-adjusting XP from {current_xp} to {min_xp} for level {total_level}")
            self.set_experience(min_xp)

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

        current_class_lvl = 0
        for c in class_list:
            if c.get('Class') == class_id:
                current_class_lvl = c.get('ClassLevel', 1)
                break

        modifiers = self._calculate_ability_modifiers()
        hp_gained = self._calculate_hit_points(new_class, 1, modifiers['CON'])
        self._record_level_up(class_id, hp_gained)

        event = LevelGainedEvent(
            event_type=EventType.LEVEL_GAINED,
            source_manager='class',
            timestamp=time.time(),
            class_id=class_id,
            new_level=total_level,
            total_level=total_level,
            class_level_gained=current_class_lvl
        )
        self.character_manager.emit(event)

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        points_gained = 0
        if lvl_stat_list:
            last_entry = lvl_stat_list[-1]
            points_gained = last_entry.get(LVL_STAT_SKILL_POINTS, 0)

        gains = {
            'skill_points': points_gained,
            'total_skill_points': self.gff.get('SkillPoints', 0),
            'feats': 0,
            'ability_score': False,
            'new_spells': False
        }

        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            raise RuntimeError("FeatManager required for feat slot calculation")
        feat_slots = feat_manager.get_feat_slots_for_level(total_level, class_id, current_class_lvl)

        gains['feats'] = feat_slots['general']
        if feat_slots['bonus'] > 0:
            gains['bonus_feats'] = feat_slots['bonus']

        if total_level % 4 == 0:
            gains['ability_score'] = True

        is_spellcaster = field_mapper.get_field_value(new_class, 'spellcaster', '0') == '1'

        # Prestige casters have casting but no spell table - they advance base class casting
        spell_gain_table = field_mapper.get_field_value(new_class, 'spell_gain_table', '')
        is_prestige_caster = is_spellcaster and (not spell_gain_table or spell_gain_table == '****')

        if is_prestige_caster:
            best_base_class_entry = None
            max_lvl = -1

            for c_entry in self.gff.get('ClassList', []):
                c_id = c_entry.get('Class')
                c_data = self.rules_service.get_by_id('classes', c_id)
                if c_data:
                    c_table = field_mapper.get_field_value(c_data, 'spell_gain_table', '')
                    if c_table and c_table != '****':
                        lvl = c_entry.get('ClassLevel', 0)
                        if lvl > max_lvl:
                            max_lvl = lvl
                            best_base_class_entry = c_entry

            if best_base_class_entry:
                current_cl = best_base_class_entry.get('ClassLevel', 0)
                current_scl = best_base_class_entry.get('SpellCasterLevel')

                effective_level = current_cl
                if current_scl is not None:
                    try:
                        effective_level = int(current_scl)
                    except (ValueError, TypeError):
                        pass
                new_scl = effective_level + 1
                best_base_class_entry['SpellCasterLevel'] = new_scl
                logger.info(f"Prestige Class Advancement: Increased effective caster level of Class {best_base_class_entry.get('Class')} to {new_scl}")

        gains['new_spells'] = is_spellcaster

        return {
            'class_id': class_id,
            'new_total_level': total_level,
            'multiclass': not class_found,
            'gains': gains
        }
    
    def adjust_class_level(self, class_id: int, level_change: int) -> Dict[str, Any]:
        """Adjust levels in a specific class (add or remove)."""
        if level_change == 0:
            return {}

        if level_change > 0:
            changes = {}
            accumulated_gains = {
                'skill_points': 0,
                'feats': 0,
                'bonus_feats': 0,
                'ability_score': False,
                'new_spells': False
            }

            for _ in range(level_change):
                result = self.add_class_level(class_id)
                changes = result

                if 'gains' in result:
                    g = result['gains']
                    accumulated_gains['skill_points'] += g.get('skill_points', 0)
                    accumulated_gains['total_skill_points'] = g.get('total_skill_points', 0)
                    accumulated_gains['feats'] += g.get('feats', 0)
                    accumulated_gains['bonus_feats'] += g.get('bonus_feats', 0)
                    accumulated_gains['ability_score'] = accumulated_gains['ability_score'] or g.get('ability_score', False)
                    accumulated_gains['new_spells'] = accumulated_gains['new_spells'] or g.get('new_spells', False)

            changes['gains'] = accumulated_gains
            return changes

        class_list = self.gff.get('ClassList', [])

        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True

        try:
            for class_entry in class_list:
                if class_entry.get('Class') == class_id:
                    current_level = class_entry.get('ClassLevel', 0)
                    new_level = max(0, current_level + level_change)

                    if new_level == 0:
                        if transaction_started:
                            self.character_manager.rollback_transaction()
                        return self.remove_class(class_id)

                    levels_removed = current_level - new_level
                    class_entry['ClassLevel'] = new_level
                    self.gff.set('ClassList', class_list)

                    self._remove_level_history_for_class(class_id, levels_removed)

                    lvl_stat_list = self.gff.get('LvlStatList', [])
                    total_level = len(lvl_stat_list)

                    min_xp_current = level_to_xp(total_level, self.rules_service)
                    next_level_xp = level_to_xp(total_level + 1, self.rules_service)
                    current_xp = self.get_experience()

                    # Adjust XP to prevent "XP Level differs from Class Level" warning
                    if current_xp >= next_level_xp:
                        logger.info(f"Auto-adjusting XP down from {current_xp} to {min_xp_current} for level {total_level}")
                        self.set_experience(min_xp_current)

                    event = ClassChangedEvent(
                        event_type=EventType.CLASS_CHANGED,
                        source_manager='class',
                        timestamp=time.time(),
                        old_class_id=class_id,
                        new_class_id=class_id,
                        level=total_level,
                        preserve_feats=self._get_preserved_feats(),
                        is_level_adjustment=True
                    )
                    self.character_manager.emit(event)

                    if transaction_started:
                        self.character_manager.commit_transaction()

                    return {
                        'class_id': class_id,
                        'level_change': level_change,
                        'new_class_level': new_level,
                        'new_total_level': total_level
                    }

            if transaction_started:
                self.character_manager.rollback_transaction()

        except Exception as e:
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during level adjustment: {e}")
            raise

        raise ValueError(f"Character does not have class {class_id}")
    
    def _update_class_list(self, new_class_id: int, total_level: int):
        """Update class list for single-class characters only."""
        class_list = self.gff.get('ClassList', [])

        if len(class_list) > 1:
            raise ValueError("Cannot use _update_class_list for multiclass characters. Use change_specific_class() instead.")

        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if new_class:
            max_level_raw = field_mapper.get_field_value(new_class, 'max_level', '0')
            try:
                max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
            except (ValueError, TypeError):
                max_level = 0

            if max_level > 0 and total_level > max_level:
                logger.info(f"Capping level from {total_level} to {max_level} for prestige class {new_class_id}")
                total_level = max_level

        self.gff.set('ClassList', [{
            'Class': new_class_id,
            'ClassLevel': total_level
        }])
        self.gff.set('Class', new_class_id)

    def change_specific_class(self, old_class_id: int, new_class_id: int, preserve_level: bool = True) -> Dict[str, Any]:
        """Swap a specific class in multiclass character (resets to level 1)."""
        logger.info(f"Changing specific class from {old_class_id} to {new_class_id} (Clean Swap)")

        new_class = self.rules_service.get_by_id('classes', new_class_id)
        if not new_class:
            raise ValueError(f"Invalid class ID: {new_class_id}")

        class_list = self.gff.get('ClassList', [])

        class_index = -1
        for i, class_entry in enumerate(class_list):
            if class_entry.get('Class') == old_class_id:
                class_index = i
                break

        if class_index == -1:
            raise ValueError(f"Character does not have class {old_class_id}")

        transaction_started = False
        if not self.character_manager._current_transaction:
            self.character_manager.begin_transaction()
            transaction_started = True

        try:
            self._remove_class_features_and_feats(old_class_id)
            self._remove_class_from_history(old_class_id)

            # Re-get and recreate entry to clear leftover data (spells, domain, etc.)
            class_list = self.gff.get('ClassList', [])
            class_list[class_index] = {
                'Class': new_class_id,
                'ClassLevel': 0
            }
            self.gff.set('ClassList', class_list)

            current_primary = self.gff.get('Class', 0)
            if current_primary == old_class_id:
                self.gff.set('Class', new_class_id)

            result = self.add_class_level(new_class_id)
            self._recalculate_all_stats()

            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                timestamp=time.time(),
                old_class_id=old_class_id,
                new_class_id=new_class_id,
                level=1,
                preserve_feats=self._get_preserved_feats()
            )
            self.character_manager.emit(event)

            if transaction_started:
                self.character_manager.commit_transaction()

            return {
                'class_change': {
                    'old_class': old_class_id,
                    'new_class': new_class_id,
                    'level': 1
                },
                'multiclass_preserved': True,
                'total_level': sum(c.get('ClassLevel', 0) for c in self.gff.get('ClassList', [])),
                'stats_updated': True,
                'details': result
            }

        except Exception as e:
            if transaction_started:
                self.character_manager.rollback_transaction()
            logger.error(f"Error during specific class change: {e}")
            raise

    def _update_class_stats(self, new_class, total_level: int) -> Dict[str, Any]:
        """Update HP, BAB, and saves based on new class."""
        changes = {}

        modifiers = self._calculate_ability_modifiers()

        old_hp = self.gff.get('HitPoints', 0)
        new_hp = self._calculate_hit_points(new_class, total_level, modifiers['CON'])
        self.gff.set('HitPoints', new_hp)
        self.gff.set('MaxHitPoints', new_hp)
        self.gff.set('CurrentHitPoints', new_hp)
        changes['hit_points'] = {'old': old_hp, 'new': new_hp}

        old_bab = self.gff.get('BaseAttackBonus', 0)
        combat_manager = self.character_manager.get_manager('combat')
        if not combat_manager:
            raise RuntimeError("CombatManager required for BAB calculation")
        combat_manager.invalidate_bab_cache()
        new_bab = combat_manager.calculate_base_attack_bonus()
        self.gff.set('BaseAttackBonus', new_bab)
        changes['bab'] = {'old': old_bab, 'new': new_bab}

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
    
    def _calculate_historical_skill_points(self) -> Dict[int, int]:
        """Reconstruct skill points per class from history entries."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list:
            return {}

        class_points = {}
        for entry in lvl_stat_list:
            class_id = entry.get(LVL_STAT_CLASS, -1)
            if class_id == -1:
                continue
            skill_list = entry.get(LVL_STAT_SKILL_LIST, [])
            points_this_level = sum(s.get("Rank", 0) for s in skill_list if isinstance(s, dict))
            class_points[class_id] = class_points.get(class_id, 0) + points_this_level

        return class_points

    def _calculate_ability_modifiers(self) -> Dict[str, int]:
        """Get ability modifiers from AbilityManager."""
        ability_manager = self.character_manager.get_manager('ability')
        if not ability_manager:
            raise RuntimeError("AbilityManager required for modifier calculation")
        return ability_manager.get_all_modifiers()
    
    def _calculate_hit_points(self, class_data, level: int, con_modifier: int) -> int:
        """Calculate total hit points for class and level."""
        hit_die_raw = field_mapper.get_field_value(class_data, 'hit_die')
        if hit_die_raw is None:
            raise ValueError("Class data missing hit_die field")
        hit_die = field_mapper._safe_int(hit_die_raw, None)
        if hit_die is None:
            raise ValueError(f"Invalid hit_die value: {hit_die_raw}")

        base_hp = hit_die
        if level > 1:
            avg_roll = (hit_die + 1) // 2
            base_hp += avg_roll * (level - 1)

        total_hp = base_hp + (con_modifier * level)
        return max(1, total_hp)
    
    def _calculate_bab(self, class_data, level: int) -> int:
        """Calculate BAB for a single class and level."""
        if level <= 0:
            return 0

        bab_table_name = field_mapper.get_field_value(class_data, 'attack_bonus_table')
        if not bab_table_name:
            class_label = field_mapper.get_field_value(class_data, 'label', 'Unknown')
            raise ValueError(f"Class {class_label} missing attack_bonus_table field")

        bab_table_name_lower = bab_table_name.lower()
        if bab_table_name_lower not in self._bab_table_cache:
            bab_table = self.rules_service.get_table(bab_table_name_lower)
            if not bab_table:
                raise ValueError(f"BAB table '{bab_table_name}' not found")
            self._bab_table_cache[bab_table_name_lower] = bab_table

        bab_table = self._bab_table_cache[bab_table_name_lower]
        level_idx = min(level - 1, 19)
        if level_idx < 0 or level_idx >= len(bab_table):
            raise ValueError(f"Level {level} out of range for BAB table")

        bab_row = bab_table[level_idx]
        bab_value = field_mapper.get_field_value(bab_row, 'bab')
        if bab_value is None:
            raise ValueError(f"BAB table missing bab field at level {level}")
        return field_mapper._safe_int(bab_value, 0)

    def calculate_total_saves(self) -> Dict[str, int]:
        """Delegate save calculation to SaveManager."""
        save_manager = self.character_manager.get_manager('save')
        if not save_manager:
            raise RuntimeError("SaveManager required for save calculation")
        saves = save_manager.calculate_saving_throws()
        return {
            'fortitude': saves['fortitude']['total'],
            'reflex': saves['reflex']['total'],
            'will': saves['will']['total'],
            'base_fortitude': saves['fortitude']['base'],
            'base_reflex': saves['reflex']['base'],
            'base_will': saves['will']['base']
        }
    
    def _get_preserved_feats(self) -> List[int]:
        """Get list of feat IDs that should be preserved during class change."""
        preserved = set()

        feat_manager = self.character_manager.get_manager('feat')
        if feat_manager:
            epithet_feats = feat_manager.detect_epithet_feats()
            preserved.update(epithet_feats)

        for content_id, info in self.character_manager.custom_content.items():
            if info['type'] == 'feat' and not info.get('removable', True):
                preserved.add(info['id'])

        race_id = self.gff.get('Race', 0)
        racial_feats = self._get_racial_feats(race_id)
        preserved.update(racial_feats)

        feat_list = self.gff.get('FeatList', [])
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)

            # Domain feats in vanilla NWN2 use IDs 4000-4999
            if 4000 <= feat_id <= 4999:
                preserved.add(feat_id)

            if self._is_background_feat(feat_id):
                preserved.add(feat_id)

        logger.info(f"Preserving {len(preserved)} feats during class change")
        return list(preserved)
    
    def get_class_summary(self) -> Dict[str, Any]:
        """Get summary of character's classes."""
        class_list = self.gff.get('ClassList', [])
        historical_points = self._calculate_historical_skill_points()

        classes = []
        for c in class_list:
            class_id = c.get('Class', 0)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                raise ValueError(f"Class ID {class_id} not found in classes.2da")

            class_name = self._get_class_name(class_id, class_data)
            skill_points_display = historical_points.get(class_id, 0)
            class_level = c.get('ClassLevel', 0)

            class_bab = 0
            if class_level > 0:
                class_bab = self._calculate_bab(class_data, class_level)

            class_fort = 0
            class_ref = 0
            class_will = 0
            if class_level > 0:
                save_manager = self.character_manager.get_manager('save')
                if save_manager:
                    class_saves = save_manager._calculate_base_save_delta(class_data, class_level)
                    class_fort = class_saves.get('fortitude', 0)
                    class_ref = class_saves.get('reflex', 0)
                    class_will = class_saves.get('will', 0)

            hit_die_raw = field_mapper.get_field_value(class_data, 'hit_die')
            if hit_die_raw is None:
                raise ValueError(f"Class {class_id} missing hit_die field")
            hit_die = field_mapper._safe_int(hit_die_raw, None)
            if hit_die is None:
                raise ValueError(f"Class {class_id} has invalid hit_die: {hit_die_raw}")

            classes.append({
                'id': class_id,
                'level': class_level,
                'name': class_name,
                'skill_points': skill_points_display,
                'bab': class_bab,
                'fort_save': class_fort,
                'ref_save': class_ref,
                'will_save': class_will,
                'hit_die': hit_die,
                '_raw_calc': skill_points_display
            })

        # Normalize skill points to match actual total (handles manual overrides/discrepancies)
        skill_manager = self.character_manager.get_manager('skill')
        if skill_manager:
            available_pts = self.gff.get('SkillPoints', 0)
            spent_pts = skill_manager._calculate_spent_skill_points()
            actual_total = available_pts + spent_pts

            calculated_total = sum(c['_raw_calc'] for c in classes)

            if calculated_total > 0 and actual_total != calculated_total:
                ratio = actual_total / calculated_total
                current_sum = 0
                max_points = -1
                max_idx = 0

                for idx, c in enumerate(classes):
                    scaled = int(round(c['_raw_calc'] * ratio))
                    c['skill_points'] = scaled
                    current_sum += scaled

                    if c['_raw_calc'] > max_points:
                        max_points = c['_raw_calc']
                        max_idx = idx

                diff = actual_total - current_sum
                if diff != 0:
                    classes[max_idx]['skill_points'] += diff

        return {
            'classes': classes,
            'total_level': sum(c.get('ClassLevel', 0) for c in class_list),
            'multiclass': len(class_list) > 1,
            'can_multiclass': len(class_list) < 3
        }
    
    def get_attack_bonuses(self) -> Dict[str, Any]:
        """Get all attack bonuses including BAB and ability modifiers."""
        combat_manager = self.character_manager.get_manager('combat')
        if not combat_manager:
            raise RuntimeError("CombatManager required for BAB calculation")
        bab = combat_manager.calculate_base_attack_bonus()

        modifiers = self._calculate_ability_modifiers()
        str_mod = modifiers['STR']
        dex_mod = modifiers['DEX']

        has_weapon_finesse = self._has_feat_by_name('WeaponFinesse')

        melee_bonus = bab + str_mod
        finesse_bonus = bab + dex_mod if has_weapon_finesse else None
        ranged_bonus = bab + dex_mod
        touch_bonus = bab

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
        """Check if character has a feat by its label."""
        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            raise RuntimeError("FeatManager required for feat lookup")
        return feat_manager.has_feat_by_name(feat_label)

    def _get_racial_feats(self, race_id: int) -> List[int]:
        """Get list of racial feats for a given race."""
        racial_feats = []

        race_data = self.rules_service.get_by_id('racialtypes', race_id)
        if not race_data:
            return racial_feats

        feat_table_name = getattr(race_data, 'feat_table', None)
        if feat_table_name:
            feat_table = self.rules_service.get_table(feat_table_name.lower())
            if feat_table:
                for feat_entry in feat_table:
                    feat_id = getattr(feat_entry, 'feat_index', -1)
                    if feat_id >= 0:
                        racial_feats.append(feat_id)

        return racial_feats

    def _is_background_feat(self, feat_id: int) -> bool:
        """Check if a feat is a background/history feat that should be preserved."""
        feat_data = self.rules_service.get_by_id('feat', feat_id)
        if not feat_data:
            return False

        label = getattr(feat_data, 'label', '').lower()
        category = getattr(feat_data, 'categories', '').lower()

        background_patterns = [
            'background', 'history', 'past', 'origin',
            'blessing', 'curse', 'gift', 'legacy',
            'shard', 'silver', 'influence', 'touched'
        ]

        for pattern in background_patterns:
            if pattern in label or pattern in category:
                return True

        removable = getattr(feat_data, 'removable', 1)
        if removable == 0:
            return True

        return False

    def _record_level_up(self, class_id: int, hp_gained: int):
        """Create a new LvlStatList entry for level up."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list):
            lvl_stat_list = []
        
        new_entry = {
            LVL_STAT_CLASS: class_id,
            LVL_STAT_HITDIE: hp_gained,
            LVL_STAT_SKILL_POINTS: 0,
            LVL_STAT_FEAT_LIST: [],
            LVL_STAT_SKILL_LIST: []
        }
        
        for i in range(10):
            new_entry[f"{LVL_STAT_KNOWN_LIST}{i}"] = []
            new_entry[f"{LVL_STAT_KNOWN_REMOVE_LIST}{i}"] = []
        
        lvl_stat_list.append(new_entry)
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
        logger.info(f"Recorded level up history: Class {class_id}, HP {hp_gained}")
            
    def get_categorized_classes(self, search: str = None, type_filter: str = None, include_unplayable: bool = False) -> Dict[str, Any]:
        """Get all classes organized by type and focus."""
        game_data_loader = get_dynamic_game_data_loader()
        if not game_data_loader:
             raise RuntimeError("Game data not available")

        categorizer = ClassCategorizer(game_data_loader)
        
        # Search mode
        if search:
            search_filter = None
            if type_filter == 'base':
                search_filter = ClassType.BASE
            elif type_filter == 'prestige':
                 search_filter = ClassType.PRESTIGE
            
            search_results = categorizer.search_classes(search, search_filter)
            
            return {
                'categories': {},
                'focus_info': {},
                'total_classes': 0,
                'include_unplayable': include_unplayable,
                'search_results': [class_info.to_dict() for class_info in search_results],
                'query': search,
                'total_results': len(search_results)
            }
        
        # Categorized mode
        categories = categorizer.get_categorized_classes(include_unplayable)
        
        if type_filter in ['base', 'prestige']:
            filtered_categories = {type_filter: categories[type_filter]}
        else:
             filtered_categories = categories
        
        serialized_categories = {}
        for class_type, focus_groups in filtered_categories.items():
            serialized_categories[class_type] = {}
            for focus, class_list in focus_groups.items():
                if class_list:
                    serialized_categories[class_type][focus] = [
                        class_info.to_dict() for class_info in class_list
                    ]
        
        focus_info = categorizer.get_focus_display_info()
        
        total_classes = sum(
            len(class_list) 
            for focus_groups in filtered_categories.values() 
            for class_list in focus_groups.values()
        )
        
        return {
            'categories': serialized_categories,
            'focus_info': focus_info,
            'total_classes': total_classes,
            'include_unplayable': include_unplayable
        }

    def get_class_features_detail(self, class_id: int, max_level: int = 20) -> Dict[str, Any]:
        """Get detailed class features and progression."""
        game_data_loader = get_dynamic_game_data_loader()
        if not game_data_loader:
             raise RuntimeError("Game data not available")
             
        class_data = game_data_loader.get_by_id('classes', class_id)
        if not class_data:
             raise ValueError(f"Class with ID {class_id} not found")
        
        categorizer = ClassCategorizer(game_data_loader)
        class_info = categorizer._create_simple_class_info(class_data, class_id)
        
        progression_data = {
            'class_id': class_id,
            'class_name': class_info.name if class_info else 'Unknown Class',
            'basic_info': {
                'hit_die': class_info.hit_die if class_info else 8,
                'skill_points_per_level': class_info.skill_points if class_info else 2,
                'is_spellcaster': class_info.is_spellcaster if class_info else False,
                'spell_type': 'arcane' if class_info and class_info.has_arcane else ('divine' if class_info and class_info.has_divine else 'none')
            },
            'description': {},
            'max_level_shown': max_level
        }
        
        if class_info and class_info.parsed_description:
            progression_data['description'] = {
                'title': getattr(class_info.parsed_description, 'title', ''),
                'class_type': getattr(class_info.parsed_description, 'class_type', ''),
                'summary': getattr(class_info.parsed_description, 'summary', ''),
                'features': getattr(class_info.parsed_description, 'features', '')
            }
            
        return progression_data

    def on_skill_points_awarded(self, event: Any):
        """Handle SKILL_POINTS_AWARDED event by updating the latest level history entry."""
        if not hasattr(event, 'points'):
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            return

        last_entry = lvl_stat_list[-1]
        if last_entry.get(LVL_STAT_CLASS) == event.class_id:
            last_entry[LVL_STAT_SKILL_POINTS] = event.points
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.info(f"Updated history entry with awarded skill points: {event.points}")

    def _remove_level_history_for_class(self, class_id: int, count: int = 1):
        """Remove the last N entries for a class from LvlStatList."""
        if count <= 0:
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return

        feat_manager = self.character_manager.get_manager('feat')
        skill_manager = self.character_manager.get_manager('skill')

        racial_feats = set()
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            racial_feats = set(race_manager.get_all_racial_feats())

        removed_count = 0
        feats_removed = 0
        skills_refunded = 0
        ability_removed = False

        for i in range(len(lvl_stat_list) - 1, -1, -1):
            if removed_count >= count:
                break

            entry = lvl_stat_list[i]
            if entry.get(LVL_STAT_CLASS) != class_id:
                continue

            logger.info(f"Removing level history entry at index {i} for class {class_id}")

            feat_list = entry.get(LVL_STAT_FEAT_LIST, [])
            if isinstance(feat_list, list) and feat_manager:
                for feat_entry in feat_list:
                    feat_id = feat_entry.get('Feat')
                    if feat_id is None:
                        continue

                    if feat_id in racial_feats:
                        logger.debug(f"Skipping removal of racial feat {feat_id}")
                        continue
                    if self._is_background_feat(feat_id):
                        logger.debug(f"Skipping removal of background feat {feat_id}")
                        continue

                    if feat_manager.remove_feat(feat_id, force=True):
                        feats_removed += 1
                        logger.debug(f"Removed feat {feat_id} from level {i+1}")

            skill_list_entry = entry.get('SkillList', [])
            if isinstance(skill_list_entry, list) and skill_manager:
                for skill_idx, skill_entry in enumerate(skill_list_entry):
                    if isinstance(skill_entry, dict):
                        ranks_spent = skill_entry.get('Rank', 0)
                        if ranks_spent > 0:
                            current_ranks = skill_manager.get_skill_ranks(skill_idx)
                            new_ranks = max(0, current_ranks - ranks_spent)

                            char_skill_list = self.gff.get('SkillList', [])
                            if skill_idx < len(char_skill_list):
                                if isinstance(char_skill_list[skill_idx], dict):
                                    char_skill_list[skill_idx]['Rank'] = new_ranks
                                    self.gff.set('SkillList', char_skill_list)
                                    skills_refunded += ranks_spent
                                    logger.debug(f"Removed {ranks_spent} ranks from skill {skill_idx}")

            ability_idx = entry.get('LvlStatAbility')
            if ability_idx is not None and ability_idx >= 0:
                ability_map = {0: 'Str', 1: 'Dex', 2: 'Con', 3: 'Int', 4: 'Wis', 5: 'Cha'}
                attr_name = ability_map.get(ability_idx)
                if attr_name:
                    current_val = self.gff.get(attr_name, 10)
                    new_val = max(3, current_val - 1)
                    self.gff.set(attr_name, new_val)
                    ability_removed = True
                    logger.info(f"Removed ability increase: {attr_name} {current_val} -> {new_val}")

            hit_die_roll = entry.get('LvlStatHitDie', 0)
            if hit_die_roll > 0:
                con_score = self.gff.get('Con', 10)
                con_mod = (con_score - 10) // 2
                hp_reduction = max(1, hit_die_roll + con_mod)

                current_max_hp = self.gff.get('MaxHitPoints', 0)
                current_hp = self.gff.get('CurrentHitPoints', 0)

                new_max_hp = max(1, current_max_hp - hp_reduction)
                new_current_hp = max(1, min(current_hp - hp_reduction, new_max_hp))

                self.gff.set('MaxHitPoints', new_max_hp)
                self.gff.set('CurrentHitPoints', new_current_hp)
                self.gff.set('HitPoints', new_max_hp)

                logger.info(f"Reduced HP by {hp_reduction} (die roll {hit_die_roll} + CON mod {con_mod}): "
                           f"Max {current_max_hp} -> {new_max_hp}, Current {current_hp} -> {new_current_hp}")

            spells_removed = 0
            for spell_level in range(10):
                known_list_key = f'KnownList{spell_level}'
                known_list_from_history = entry.get(known_list_key, [])
                if not isinstance(known_list_from_history, list) or not known_list_from_history:
                    continue

                class_list = self.gff.get('ClassList', [])
                for class_entry in class_list:
                    if class_entry.get('Class') != class_id:
                        continue

                    class_known_list = class_entry.get(known_list_key, [])
                    if not isinstance(class_known_list, list):
                        continue

                    for spell_entry in known_list_from_history:
                        spell_id = spell_entry.get('Spell')
                        if spell_id is None:
                            continue

                        for j in range(len(class_known_list) - 1, -1, -1):
                            if class_known_list[j].get('Spell') == spell_id:
                                class_known_list.pop(j)
                                spells_removed += 1
                                logger.debug(f"Removed spell {spell_id} (level {spell_level})")
                                break

                    class_entry[known_list_key] = class_known_list

                self.gff.set('ClassList', class_list)

            if spells_removed > 0:
                logger.info(f"Removed {spells_removed} spells from level {i+1}")

            lvl_stat_list.pop(i)
            removed_count += 1

        self.gff.set(LVL_STAT_LIST, lvl_stat_list)

        if removed_count > 0:
            logger.info(f"Level-down cleanup for class {class_id}: Removed {removed_count} levels, "
                       f"{feats_removed} feats, refunded {skills_refunded} skill ranks, "
                       f"ability removed: {ability_removed}")

    def _remove_class_features_and_feats(self, class_id: int):
        """Remove feats and features associated with a class."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return

        feat_manager = self.character_manager.get_manager('feat')
        if not feat_manager:
            logger.warning("FeatManager not available for class removal cleanup")
            return

        racial_feats = set()
        race_manager = self.character_manager.get_manager('race')
        if race_manager:
            racial_feats = set(race_manager.get_all_racial_feats())
            logger.debug(f"Protected racial feats: {racial_feats}")
        else:
            logger.warning("RaceManager not available for feat protection")

        count_removed = 0
        count_protected = 0
        for entry in lvl_stat_list:
            if entry.get(LVL_STAT_CLASS) == class_id:
                feat_list = entry.get(LVL_STAT_FEAT_LIST, [])
                if isinstance(feat_list, list):
                    for feat_entry in feat_list:
                        feat_id = feat_entry.get('Feat')
                        if feat_id is not None:
                            if feat_id in racial_feats:
                                logger.debug(f"Skipping removal of racial feat {feat_id}")
                                count_protected += 1
                                continue

                            if self._is_background_feat(feat_id):
                                logger.debug(f"Skipping removal of background feat {feat_id}")
                                count_protected += 1
                                continue

                            feat_manager.remove_feat(feat_id)
                            count_removed += 1

        if count_removed > 0 or count_protected > 0:
            logger.info(f"Cleanup for class {class_id}: Removed {count_removed} feats, Protected {count_protected} racial/background feats")
            
    def _remove_class_from_history(self, class_id: int) -> int:
        """Remove all LvlStatList entries for a specific class."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return 0

        original_count = len(lvl_stat_list)
        lvl_stat_list = [entry for entry in lvl_stat_list if entry.get(LVL_STAT_CLASS) != class_id]
        removed_count = original_count - len(lvl_stat_list)

        if removed_count > 0:
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.info(f"Removed {removed_count} level history entries for class {class_id}")

        return removed_count

    def _update_class_in_history(self, old_class_id: int, new_class_id: int) -> int:
        """Replace class ID occurrences in LvlStatList."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not isinstance(lvl_stat_list, list) or not lvl_stat_list:
            return 0

        updated_count = 0
        for entry in lvl_stat_list:
            if entry.get(LVL_STAT_CLASS) == old_class_id:
                entry[LVL_STAT_CLASS] = new_class_id
                updated_count += 1

        if updated_count > 0:
            self.gff.set(LVL_STAT_LIST, lvl_stat_list)
            logger.info(f"Updated {updated_count} level history entries from class {old_class_id} to {new_class_id}")

        return updated_count

    def record_feat_change(self, feat_id: int, added: bool):
        """Sync a feat change to current level history."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync feat change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        feat_list = current_level_entry.get(LVL_STAT_FEAT_LIST, [])
        if not isinstance(feat_list, list):
            feat_list = []

        if added:
            if not any(f.get('Feat') == feat_id for f in feat_list):
                feat_list.append({'Feat': feat_id})
                logger.info(f"Synced feat {feat_id} addition to level history.")
        else:
            original_len = len(feat_list)
            feat_list = [f for f in feat_list if f.get('Feat') != feat_id]
            if len(feat_list) < original_len:
                logger.info(f"Synced feat {feat_id} removal from level history.")
        
        current_level_entry[LVL_STAT_FEAT_LIST] = feat_list
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)

    def record_skill_change(self, skill_id: int, rank_delta: int):
        """Sync a skill rank change to current level history."""
        if rank_delta == 0:
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync skill change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        skill_list = current_level_entry.get(LVL_STAT_SKILL_LIST, [])
        if not isinstance(skill_list, list):
            skill_list = []

        current_count = len(skill_list)
        if skill_id >= current_count:
            for _ in range(skill_id - current_count + 1):
                skill_list.append({'Rank': 0})

        current_rank_entry = skill_list[skill_id]
        if isinstance(current_rank_entry, dict):
            new_rank = current_rank_entry.get('Rank', 0) + rank_delta
            current_rank_entry['Rank'] = new_rank
            logger.debug(f"Synced skill {skill_id} rank change ({rank_delta}) to history. Level delta: {new_rank}")
        else:
            logger.warning(f"Invalid skill history entry format at index {skill_id}")
            
        current_level_entry[LVL_STAT_SKILL_LIST] = skill_list
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
    
    def record_ability_change(self, ability_index: int):
        """Record ability score increase in current level history (0-5 index)."""
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync ability change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        # Only save first ability increase per level to preserve history consistency
        if current_level_entry.get(LVL_STAT_ABILITY) is not None:
            logger.info("Ability increase already recorded for this level, ignoring.")
            return

        current_level_entry[LVL_STAT_ABILITY] = ability_index

        logger.info(f"Recorded ability increase (Index {ability_index}) to level history index {current_level_idx}")
        self.gff.set(LVL_STAT_LIST, lvl_stat_list)

    def record_spell_change(self, spell_level: int, spell_id: int, added: bool):
        """Sync a spell change to the current level history."""
        if spell_level < 0 or spell_level > 9:
            logger.warning(f"Invalid spell level {spell_level} for history sync.")
            return

        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list or not isinstance(lvl_stat_list, list):
            logger.warning("No level history found to sync spell change.")
            return

        current_level_idx = len(lvl_stat_list) - 1
        current_level_entry = lvl_stat_list[current_level_idx]

        if added:
            list_key = f"{LVL_STAT_KNOWN_LIST}{spell_level}"
            spell_list = current_level_entry.get(list_key, [])
            if not isinstance(spell_list, list):
                spell_list = []

            if not any(s.get('Spell') == spell_id for s in spell_list):
                spell_list.append({'Spell': spell_id})
                logger.debug(f"Synced spell {spell_id} (level {spell_level}) addition to level history.")

            current_level_entry[list_key] = spell_list
        else:
            list_key = f"{LVL_STAT_KNOWN_REMOVE_LIST}{spell_level}"
            remove_list = current_level_entry.get(list_key, [])
            if not isinstance(remove_list, list):
                remove_list = []

            if not any(s.get('Spell') == spell_id for s in remove_list):
                remove_list.append({'Spell': spell_id})
                logger.debug(f"Synced spell {spell_id} (level {spell_level}) removal to level history.")

            current_level_entry[list_key] = remove_list

        self.gff.set(LVL_STAT_LIST, lvl_stat_list)
    
    def get_level_history(self) -> List[Dict[str, Any]]:
        """Get the full level up history for display."""
        history_data = []
        lvl_stat_list = self.gff.get(LVL_STAT_LIST, [])
        if not lvl_stat_list:
            return []

        class_level_counts: Dict[int, int] = {}

        for level_idx, entry in enumerate(lvl_stat_list):
            class_id = entry.get(LVL_STAT_CLASS, -1)
            class_level_counts[class_id] = class_level_counts.get(class_id, 0) + 1

            level_info = {
                'level': level_idx + 1,
                'class': 'Unknown',
                'class_level': class_level_counts[class_id],
                'hp_gained': entry.get(LVL_STAT_HITDIE, 0),
                'skill_points_remaining': entry.get(LVL_STAT_SKILL_POINTS, 0),
                'ability_increase': None,
                'skills_gained': [],
                'feats_gained': [],
                'spells_learned': [],
                'spells_removed': [],
            }

            level_info['class'] = self._get_rule_label('classes', class_id, f"Class {class_id}")
            
            ability_id = entry.get(LVL_STAT_ABILITY)
            if ability_id is not None:
                abilities = ['Strength', 'Dexterity', 'Constitution', 'Intelligence', 'Wisdom', 'Charisma']
                if 0 <= ability_id < len(abilities):
                    level_info['ability_increase'] = abilities[ability_id]

            skill_list = entry.get(LVL_STAT_SKILL_LIST, [])
            for skill_id, skill_entry in enumerate(skill_list):
                rank = skill_entry.get('Rank', 0)
                if rank != 0:
                    skill_name = self._get_rule_label('skills', skill_id, f"Skill {skill_id}")
                    level_info['skills_gained'].append({'name': skill_name, 'rank': rank})

            feat_list = entry.get(LVL_STAT_FEAT_LIST, [])
            feat_manager = self.character_manager.get_manager('feat')
            for feat_entry in feat_list:
                feat_id = feat_entry.get('Feat', -1)
                if feat_id >= 0 and feat_manager:
                    feat_info = feat_manager.get_feat_info_display(feat_id)
                    feat_name = feat_info.get('name', f"Feat {feat_id}") if feat_info else f"Feat {feat_id}"
                else:
                    feat_name = f"Feat {feat_id}"
                level_info['feats_gained'].append({'name': feat_name})

            spell_manager = self.character_manager.get_manager('spell')
            for i in range(10):
                known_list = entry.get(f"{LVL_STAT_KNOWN_LIST}{i}", [])
                for spell_entry in known_list:
                    spell_id = spell_entry.get('Spell', -1)
                    if spell_id >= 0 and spell_manager:
                        spell_info = spell_manager.get_spell_details(spell_id)
                        spell_name = spell_info.get('name', f"Spell {spell_id}")
                    else:
                        spell_name = f"Spell {spell_id}"
                    level_info['spells_learned'].append({'name': spell_name, 'level': i})

                remove_list = entry.get(f"{LVL_STAT_KNOWN_REMOVE_LIST}{i}", [])
                for spell_entry in remove_list:
                    spell_id = spell_entry.get('Spell', -1)
                    if spell_id >= 0 and spell_manager:
                        spell_info = spell_manager.get_spell_details(spell_id)
                        spell_name = spell_info.get('name', f"Spell {spell_id}")
                    else:
                        spell_name = f"Spell {spell_id}"
                    level_info['spells_removed'].append({'name': spell_name, 'level': i})

            history_data.append(level_info)
            
        return history_data

    def _get_rule_label(self, rule_type: str, rule_id: int, default: str) -> str:
        """Get a human-readable label from game rules."""
        if rule_id is None or rule_id < 0:
            return default

        try:
            data = self.rules_service.get_by_id(rule_type, rule_id)
            if data:
                return field_mapper.get_field_value(data, 'label',
                       field_mapper.get_field_value(data, 'name', default))
        except Exception:
            pass
        return default

    def validate(self) -> Tuple[bool, List[str]]:
        """Validate current class configuration for corruption prevention."""
        errors = []

        class_list = self.gff.get('ClassList', [])

        for class_entry in class_list:
            class_id = class_entry.get('Class', 0)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if not class_data:
                errors.append(f"Invalid class ID: {class_id}")

        total_level = sum(c.get('ClassLevel', 0) for c in class_list)
        if total_level > 60:
            errors.append(f"Total level {total_level} exceeds maximum of 60")
        if total_level < 1:
            errors.append("Total level must be at least 1")

        return len(errors) == 0, errors

    def get_class_level_info(self, class_id: int) -> Dict[str, Any]:
        """Get level information for a class."""
        current_level = 0
        class_list = self.gff.get('ClassList', [])
        for class_entry in class_list:
            if class_entry.get('Class') == class_id:
                current_level = class_entry.get('ClassLevel', 0)
                break

        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return {
                'current_level': current_level,
                'max_level': None,
                'remaining_levels': None,
                'is_prestige': False,
                'can_level_up': True
            }

        max_level_raw = field_mapper.get_field_value(class_data, 'max_level', '0')
        try:
            max_level = int(max_level_raw) if max_level_raw not in ['****', ''] else 0
        except (ValueError, TypeError):
            max_level = 0

        is_prestige = max_level > 0
        remaining_levels = max_level - current_level if is_prestige else None
        can_level_up = not is_prestige or remaining_levels > 0

        return {
            'current_level': current_level,
            'max_level': max_level if is_prestige else None,
            'remaining_levels': remaining_levels,
            'is_prestige': is_prestige,
            'can_level_up': can_level_up
        }

    def remove_class(self, class_id: int) -> Dict[str, Any]:
        """Remove a class from multiclass character."""
        class_list = self.gff.get('ClassList', [])

        class_to_remove = None
        for i, class_entry in enumerate(class_list):
            if class_entry.get('Class') == class_id:
                class_to_remove = i
                break

        if class_to_remove is None:
            raise ValueError(f"Character does not have class {class_id}")

        if len(class_list) <= 1:
            raise ValueError("Cannot remove last remaining class")

        txn = self.character_manager.begin_transaction()

        try:
            removed_class = class_list.pop(class_to_remove)
            self.gff.set('ClassList', class_list)

            self._remove_class_features_and_feats(class_id)
            self._remove_class_from_history(class_id)
            self._recalculate_all_stats()

            event = ClassChangedEvent(
                event_type=EventType.CLASS_CHANGED,
                source_manager='class',
                timestamp=time.time(),
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
        """Get available prestige classes based on current character."""
        available_prestige = []
        classes_table = self.rules_service.get_table('classes')
        if not classes_table:
            return available_prestige

        for class_data in classes_table:
            is_prestige = getattr(class_data, 'is_prestige', 0)
            if not is_prestige:
                continue

            class_id = getattr(class_data, 'id', -1)
            if class_id < 0:
                continue

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
        """Check if character meets prestige class requirements."""
        can_take, errors = self.character_manager.check_prerequisites('class', prestige_id)

        if can_take:
            return True, "All requirements met"
        return False, "; ".join(errors)

    def _get_prestige_requirements(self, class_data) -> Dict[str, Any]:
        """Extract prestige class requirements from class data."""
        requirements = {}

        min_bab = getattr(class_data, 'min_attack_bonus', 0)
        if min_bab > 0:
            requirements['base_attack_bonus'] = min_bab

        skill_req = getattr(class_data, 'required_skill', '')
        skill_rank = getattr(class_data, 'required_skill_rank', 0)
        if skill_req and skill_rank > 0:
            requirements['skills'] = {skill_req: skill_rank}

        req_feat = getattr(class_data, 'required_feat', '')
        if req_feat:
            requirements['feats'] = [req_feat]

        alignment_restrict = getattr(class_data, 'alignment_restrict', 0)
        if alignment_restrict > 0:
            requirements['alignment'] = self._decode_alignment_restriction(alignment_restrict)

        return requirements

    def _decode_alignment_restriction(self, restriction: int) -> str:
        """Decode alignment restriction bitmask."""
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

    def _recalculate_all_stats(self):
        """Recalculate all class-dependent stats after class change."""
        combat_manager = self.character_manager.get_manager('combat')
        if not combat_manager:
            raise RuntimeError("CombatManager required for BAB calculation")
        combat_manager.invalidate_bab_cache()
        total_bab = combat_manager.calculate_base_attack_bonus()
        self.gff.set('BaseAttackBonus', total_bab)

        saves = self.calculate_total_saves()
        self.gff.set('FortSaveBase', saves['base_fortitude'])
        self.gff.set('RefSaveBase', saves['base_reflex'])
        self.gff.set('WillSaveBase', saves['base_will'])

    def get_class_feats_for_level(self, class_data: Any, level: int) -> List[Dict[str, Any]]:
        """Get feats granted by a class at a specific level."""
        feats_for_level = []

        feat_table_name = field_mapper.get_field_value(class_data, 'feats_table')
        if not feat_table_name:
            return feats_for_level

        feat_table = self.rules_service.get_table(feat_table_name.lower())
        if not feat_table:
            return feats_for_level

        for feat_entry in feat_table:
            granted_level_raw = field_mapper.get_field_value(feat_entry, 'granted_on_level', -1)

            try:
                if isinstance(granted_level_raw, str):
                    if granted_level_raw == '****':
                        granted_level = -1
                    else:
                        granted_level = int(granted_level_raw)
                else:
                    granted_level = int(granted_level_raw)
            except ValueError:
                granted_level = -1

            if granted_level == level:
                feat_id = field_mapper.get_field_value(feat_entry, 'feat_index', -1)
                try:
                    feat_id = int(feat_id)
                except (ValueError, TypeError):
                    feat_id = -1

                list_type = field_mapper.get_field_value(feat_entry, 'list', 3)
                try:
                    list_type = int(list_type)
                except (ValueError, TypeError):
                    list_type = 3

                if feat_id >= 0:
                    feats_for_level.append({
                        'feat_id': feat_id,
                        'list_type': list_type,
                        'granted_on_level': granted_level
                    })

        return feats_for_level

    def has_class_by_name(self, class_name: str) -> bool:
        """Check if character has levels in a class by name."""
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if class_data:
                label = getattr(class_data, 'label', '')
                if label.lower() == class_name.lower():
                    return True
        
        return False
    
    def get_class_level_by_name(self, class_name: str) -> int:
        """Get level in a specific class by name."""
        class_list = self.gff.get('ClassList', [])
        
        for class_info in class_list:
            class_id = class_info.get('Class', -1)
            class_data = self.rules_service.get_by_id('classes', class_id)
            if class_data:
                label = getattr(class_data, 'label', '')
                if label.lower() == class_name.lower():
                    return class_info.get('ClassLevel', 0)
        
        return 0
    
    def get_class_name(self, class_id: int) -> str:
        """Get class name for character summary."""
        return self._get_class_name(class_id)

    def _get_content_name(self, table_name: str, content_id: int) -> str:
        """Get content name from dynamic data."""
        content_data = self.rules_service.get_by_id(table_name, content_id)
        if content_data:
            for field in ['label', 'name', 'Label', 'Name']:
                name = getattr(content_data, field, '')
                if name and str(name).strip() and str(name) != '****':
                    return str(name)
        return f'{table_name.title()}_{content_id}'

    def get_total_level(self) -> int:
        """Get total character level from all classes."""
        return sum(
            c.get('ClassLevel', 0)
            for c in self.gff.get('ClassList', [])
            if isinstance(c, dict)
        )

    def get_class_label(self, class_id: int) -> Optional[str]:
        """Get class label or name."""
        class_data = self.rules_service.get_by_id('classes', class_id)
        if not class_data:
            return None

        class_label = field_mapper.get_field_value(class_data, 'label', '')
        if not class_label:
            class_label = field_mapper.get_field_value(class_data, 'name', '')

        return class_label

    def get_experience(self) -> int:
        """Get current experience points."""
        return self.gff.get('Experience', 0)

    def set_experience(self, xp: int) -> Dict[str, Any]:
        """Set experience points."""
        if xp < 0:
            raise ValueError("XP cannot be negative")

        old_xp = self.get_experience()
        old_level = xp_to_level(old_xp, self.rules_service)
        new_level = xp_to_level(xp, self.rules_service)

        self.gff.set('Experience', xp)
        logger.info(f"Set XP from {old_xp} to {xp} (level {old_level} -> {new_level})")

        return {
            'old_xp': old_xp,
            'new_xp': xp,
            'old_level': old_level,
            'new_level': new_level,
            'level_changed': old_level != new_level
        }

    def get_xp_progress(self) -> Dict[str, Any]:
        """Get XP progress info for UI."""
        xp_table = get_xp_table(self.rules_service)
        current_xp = self.get_experience()
        current_level = xp_to_level(current_xp, self.rules_service)
        total_level = self.get_total_level()
        max_level = len(xp_table)

        if current_level >= max_level:
            return {
                'current_xp': current_xp,
                'current_xp_level': current_level,
                'total_class_level': total_level,
                'next_level_xp': None,
                'xp_to_next': None,
                'current_level_min_xp': xp_table[max_level - 1],
                'next_level_min_xp': xp_table[max_level - 1],
                'progress_percent': 100
            }

        current_threshold = xp_table[current_level - 1] if current_level > 0 else 0
        next_threshold = xp_table[current_level]
        xp_in_level = current_xp - current_threshold
        xp_needed = next_threshold - current_threshold
        progress = (xp_in_level / xp_needed * 100) if xp_needed > 0 else 100

        return {
            'current_xp': current_xp,
            'current_xp_level': current_level,
            'total_class_level': total_level,
            'next_level_xp': next_threshold,
            'xp_to_next': next_threshold - current_xp,
            'current_level_min_xp': current_threshold,
            'next_level_min_xp': next_threshold,
            'progress_percent': round(progress, 1)
        }