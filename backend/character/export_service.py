"""
Enhanced character export service that exports all 200+ fields
"""
import os
from typing import Optional, Dict, Any, List
from django.db import transaction

from .models import Character, CharacterClass, CharacterFeat, CharacterSkill, CharacterSpell, CharacterItem
from parsers.gff import GFFParser, GFFWriter, GFFElement, GFFFieldType, LocalizedString, LocalizedSubstring
from parsers.resource_manager import ResourceManager


class CharacterExportService:
    """Service to export character data to GFF files with all fields"""
    
    def __init__(self, resource_manager: ResourceManager, file_type: str = "BIC "):
        self.rm = resource_manager
        self.file_type = file_type  # BIC for characters, ROS for companions
        
    def export_character(self, character: Character, output_path: str) -> str:
        """Export a character to a .bic or .ros file"""
        
        # Create the GFF structure
        gff_data = self._create_gff_structure(character)
        
        # Write to file
        writer = GFFWriter(self.file_type)
        writer.write(output_path, gff_data)
        
        return output_path
        
    def _create_gff_structure(self, character: Character) -> GFFElement:
        """Create the complete GFF structure for a character with all fields"""
        
        fields = []
        
        # Map model fields back to GFF fields
        field_mapping = [
            # Core fields
            ("FirstName", GFFFieldType.LOCSTRING, character.first_name, self._create_localized_string),
            ("LastName", GFFFieldType.LOCSTRING, character.last_name, self._create_localized_string),
            ("Age", GFFFieldType.INT, character.age, None),
            ("Gender", GFFFieldType.BYTE, character.gender, None),
            ("Deity", GFFFieldType.STRING, character.deity or "", None),
            ("Race", GFFFieldType.BYTE, character.race_id, None),
            ("Subrace", GFFFieldType.WORD, character.subrace_id, None),
            ("LawfulChaotic", GFFFieldType.BYTE, character.law_chaos, None),
            ("GoodEvil", GFFFieldType.BYTE, character.good_evil, None),
            ("Experience", GFFFieldType.DWORD, character.experience, None),
            ("Str", GFFFieldType.BYTE, character.strength, None),
            ("Dex", GFFFieldType.BYTE, character.dexterity, None),
            ("Con", GFFFieldType.BYTE, character.constitution, None),
            ("Int", GFFFieldType.BYTE, character.intelligence, None),
            ("Wis", GFFFieldType.BYTE, character.wisdom, None),
            ("Cha", GFFFieldType.BYTE, character.charisma, None),
            ("HitPoints", GFFFieldType.SHORT, character.hit_points, None),
            ("MaxHitPoints", GFFFieldType.SHORT, character.max_hit_points, None),
            ("CurrentHitPoints", GFFFieldType.SHORT, character.current_hit_points, None),
            ("ArmorClass", GFFFieldType.SHORT, character.armor_class, None),
            ("FortSaveThrow", GFFFieldType.CHAR, character.fortitude_save, None),
            ("RefSaveThrow", GFFFieldType.CHAR, character.reflex_save, None),
            ("WillSaveThrow", GFFFieldType.CHAR, character.will_save, None),
            ("Gold", GFFFieldType.DWORD, character.gold, None),
            
            # Appearance fields
            ("AppearanceSEF", GFFFieldType.STRING, character.appearance_sef, None),
            ("Appearance_FHair", GFFFieldType.INT, character.appearance_f_hair, None),
            ("Appearance_Hair", GFFFieldType.INT, character.appearance_hair, None),
            ("Appearance_Head", GFFFieldType.INT, character.appearance_head, None),
            ("Appearance_Type", GFFFieldType.INT, character.appearance_type, None),
            ("ArmorTint", GFFFieldType.STRUCT, character.armor_tint, self._create_struct_from_dict),
            ("BodyBag", GFFFieldType.BYTE, character.body_bag, None),
            ("BodyBagId", GFFFieldType.DWORD, character.body_bag_id, None),
            ("Color_Tattoo1", GFFFieldType.INT, character.color_tattoo1, None),
            ("Color_Tattoo2", GFFFieldType.INT, character.color_tattoo2, None),
            ("CustomPortrait", GFFFieldType.STRING, character.custom_portrait, None),
            ("ModelScale", GFFFieldType.STRUCT, character.model_scale, self._create_struct_from_dict),
            ("Portrait", GFFFieldType.STRING, character.portrait, None),
            ("Tail", GFFFieldType.INT, character.tail, None),
            ("Tint_Hair", GFFFieldType.STRUCT, character.tint_hair, self._create_struct_from_dict),
            ("Tint_Head", GFFFieldType.STRUCT, character.tint_head, self._create_struct_from_dict),
            ("Tintable", GFFFieldType.STRUCT, character.tintable, self._create_struct_from_dict),
            ("Wings", GFFFieldType.INT, character.wings, None),
            
            # Combat & Defense fields
            ("ACBkHip", GFFFieldType.STRUCT, character.ac_bk_hip, self._create_struct_from_dict),
            ("ACFtHip", GFFFieldType.STRUCT, character.ac_ft_hip, self._create_struct_from_dict),
            ("ACLtAnkle", GFFFieldType.STRUCT, character.ac_lt_ankle, self._create_struct_from_dict),
            ("ACLtArm", GFFFieldType.STRUCT, character.ac_lt_arm, self._create_struct_from_dict),
            ("ACLtBracer", GFFFieldType.STRUCT, character.ac_lt_bracer, self._create_struct_from_dict),
            ("ACLtElbow", GFFFieldType.STRUCT, character.ac_lt_elbow, self._create_struct_from_dict),
            ("ACLtFoot", GFFFieldType.STRUCT, character.ac_lt_foot, self._create_struct_from_dict),
            ("ACLtHip", GFFFieldType.STRUCT, character.ac_lt_hip, self._create_struct_from_dict),
            ("ACLtKnee", GFFFieldType.STRUCT, character.ac_lt_knee, self._create_struct_from_dict),
            ("ACLtLeg", GFFFieldType.STRUCT, character.ac_lt_leg, self._create_struct_from_dict),
            ("ACLtShin", GFFFieldType.STRUCT, character.ac_lt_shin, self._create_struct_from_dict),
            ("ACLtShoulder", GFFFieldType.STRUCT, character.ac_lt_shoulder, self._create_struct_from_dict),
            ("ACRtAnkle", GFFFieldType.STRUCT, character.ac_rt_ankle, self._create_struct_from_dict),
            ("ACRtArm", GFFFieldType.STRUCT, character.ac_rt_arm, self._create_struct_from_dict),
            ("ACRtBracer", GFFFieldType.STRUCT, character.ac_rt_bracer, self._create_struct_from_dict),
            ("ACRtElbow", GFFFieldType.STRUCT, character.ac_rt_elbow, self._create_struct_from_dict),
            ("ACRtFoot", GFFFieldType.STRUCT, character.ac_rt_foot, self._create_struct_from_dict),
            ("ACRtHip", GFFFieldType.STRUCT, character.ac_rt_hip, self._create_struct_from_dict),
            ("ACRtKnee", GFFFieldType.STRUCT, character.ac_rt_knee, self._create_struct_from_dict),
            ("ACRtLeg", GFFFieldType.STRUCT, character.ac_rt_leg, self._create_struct_from_dict),
            ("ACRtShin", GFFFieldType.STRUCT, character.ac_rt_shin, self._create_struct_from_dict),
            ("ACRtShoulder", GFFFieldType.STRUCT, character.ac_rt_shoulder, self._create_struct_from_dict),
            ("ActionList", GFFFieldType.LIST, character.action_list, self._create_list_from_array),
            ("ArmorVisualType", GFFFieldType.BYTE, character.armor_visual_type, None),
            ("AttackResult", GFFFieldType.BYTE, character.attack_result, None),
            ("BaseAttackBonus", GFFFieldType.BYTE, character.base_attack_bonus, None),
            ("BlockCombat", GFFFieldType.BYTE, character.block_combat, None),
            ("ChallengeRating", GFFFieldType.FLOAT, character.challenge_rating, None),
            ("CharBackground", GFFFieldType.INT, character.char_background, None),
            ("CombatInfo", GFFFieldType.STRUCT, character.combat_info, self._create_struct_from_dict),
            ("CombatMode", GFFFieldType.BYTE, character.combat_mode, None),
            ("CombatRoundData", GFFFieldType.STRUCT, character.combat_round_data, self._create_struct_from_dict),
            ("DamageMax", GFFFieldType.INT, character.damage_max, None),
            ("DamageMin", GFFFieldType.INT, character.damage_min, None),
            ("FactionID", GFFFieldType.WORD, character.faction_id, None),
            ("NaturalAC", GFFFieldType.BYTE, character.natural_ac, None),
            ("NeverShowArmor", GFFFieldType.BYTE, character.never_show_armor, None),
            ("OffHandAttacks", GFFFieldType.BYTE, character.off_hand_attacks, None),
            ("OnHandAttacks", GFFFieldType.BYTE, character.on_hand_attacks, None),
            ("OriginAttacked", GFFFieldType.STRING, character.origin_attacked, None),
            ("OriginDamaged", GFFFieldType.STRING, character.origin_damaged, None),
            ("ScriptAttacked", GFFFieldType.STRING, character.script_attacked, None),
            ("ScriptDamaged", GFFFieldType.STRING, character.script_damaged, None),
            ("StartingPackage", GFFFieldType.BYTE, character.starting_package, None),
            ("TrackingMode", GFFFieldType.BYTE, character.tracking_mode, None),
            
            # Abilities & Skills fields
            ("ConjureSoundTag", GFFFieldType.STRING, character.conjure_sound_tag, None),
            ("Conversation", GFFFieldType.STRING, character.conversation, None),
            ("Domain1", GFFFieldType.BYTE, character.domain1, None),
            ("Domain2", GFFFieldType.BYTE, character.domain2, None),
            ("Interruptable", GFFFieldType.BYTE, character.interruptable, None),
            ("IsDestroyable", GFFFieldType.BYTE, character.is_destroyable, None),
            ("OriginSpellAt", GFFFieldType.STRING, character.origin_spell_at, None),
            ("ScriptSpellAt", GFFFieldType.STRING, character.script_spell_at, None),
            ("SkillPoints", GFFFieldType.SHORT, character.skill_points, None),
            ("TemplateResRef", GFFFieldType.STRING, character.template_res_ref, None),
            ("UnrestrictLU", GFFFieldType.BYTE, character.unrestrict_lu, None),
            ("fortbonus", GFFFieldType.SHORT, character.fortbonus, None),
            ("refbonus", GFFFieldType.SHORT, character.refbonus, None),
            ("willbonus", GFFFieldType.SHORT, character.willbonus, None),
            
            # AI & Behavior fields
            ("AreaId", GFFFieldType.DWORD, character.area_id, None),
            ("AssociateList", GFFFieldType.LIST, character.associate_list, self._create_list_from_array),
            ("DetectMode", GFFFieldType.BYTE, character.detect_mode, None),
            ("DisableAIHidden", GFFFieldType.BYTE, character.disable_ai_hidden, None),
            ("IsCommandable", GFFFieldType.BYTE, character.is_commandable, None),
            ("IsRaiseable", GFFFieldType.BYTE, character.is_raiseable, None),
            ("PerceptionList", GFFFieldType.LIST, character.perception_list, self._create_list_from_array),
            ("PerceptionRange", GFFFieldType.FLOAT, character.perception_range, None),
            
            # Animation & Visual fields
            ("AmbientAnimState", GFFFieldType.BYTE, character.ambient_anim_state, None),
            ("AnimationDay", GFFFieldType.DWORD, character.animation_day, None),
            ("AnimationTime", GFFFieldType.DWORD, character.animation_time, None),
            ("CrtrCastsShadow", GFFFieldType.BYTE, character.crtr_casts_shadow, None),
            ("CrtrRcvShadow", GFFFieldType.BYTE, character.crtr_rcv_shadow, None),
            ("EnhVisionMode", GFFFieldType.BYTE, character.enh_vision_mode, None),
            
            # Scripts & Events fields
            ("Description", GFFFieldType.LOCSTRING, character.description, self._create_localized_from_dict),
            ("OriginDeath", GFFFieldType.STRING, character.origin_death, None),
            ("OriginDialogue", GFFFieldType.STRING, character.origin_dialogue, None),
            ("OriginDisturbed", GFFFieldType.STRING, character.origin_disturbed, None),
            ("OriginEndRound", GFFFieldType.STRING, character.origin_end_round, None),
            ("OriginHeartbeat", GFFFieldType.STRING, character.origin_heartbeat, None),
            ("OriginOnBlocked", GFFFieldType.STRING, character.origin_on_blocked, None),
            ("OriginOnNotice", GFFFieldType.STRING, character.origin_on_notice, None),
            ("OriginRested", GFFFieldType.STRING, character.origin_rested, None),
            ("OriginSpawn", GFFFieldType.STRING, character.origin_spawn, None),
            ("OriginUserDefine", GFFFieldType.STRING, character.origin_user_define, None),
            ("ScriptDeath", GFFFieldType.STRING, character.script_death, None),
            ("ScriptDialogue", GFFFieldType.STRING, character.script_dialogue, None),
            ("ScriptDisturbed", GFFFieldType.STRING, character.script_disturbed, None),
            ("ScriptEndRound", GFFFieldType.STRING, character.script_end_round, None),
            ("ScriptHeartbeat", GFFFieldType.STRING, character.script_heartbeat, None),
            ("ScriptHidden", GFFFieldType.BYTE, character.script_hidden, None),
            ("ScriptOnBlocked", GFFFieldType.STRING, character.script_on_blocked, None),
            ("ScriptOnNotice", GFFFieldType.STRING, character.script_on_notice, None),
            ("ScriptRested", GFFFieldType.STRING, character.script_rested, None),
            ("ScriptSpawn", GFFFieldType.STRING, character.script_spawn, None),
            ("ScriptUserDefine", GFFFieldType.STRING, character.script_user_define, None),
            ("ScriptsBckdUp", GFFFieldType.BYTE, character.scripts_bckd_up, None),
            
            # Module & Campaign fields
            ("Mod_CommntyId", GFFFieldType.STRING, character.mod_commnty_id, None),
            ("Mod_CommntyName", GFFFieldType.STRING, character.mod_commnty_name, None),
            ("Mod_CommntyPlatf", GFFFieldType.CHAR, character.mod_commnty_platf, None),
            ("Mod_IsPrimaryPlr", GFFFieldType.BYTE, character.mod_is_primary_plr, None),
            ("Mod_LastModId", GFFFieldType.STRING, character.mod_last_mod_id, None),
            ("Mod_ModuleList", GFFFieldType.LIST, character.mod_module_list, self._create_list_from_array),
            ("TalkPlayerOwn", GFFFieldType.BYTE, character.talk_player_own, None),
            
            # Miscellaneous fields
            ("AlwysPrcvbl", GFFFieldType.BYTE, character.alwys_prcvbl, None),
            ("BlockBroadcast", GFFFieldType.BYTE, character.block_broadcast, None),
            ("BlockRespond", GFFFieldType.BYTE, character.block_respond, None),
            ("Boots", GFFFieldType.STRUCT, character.boots, self._create_struct_from_dict),
            ("BumpState", GFFFieldType.BYTE, character.bump_state, None),
            ("Class", GFFFieldType.INT, character.character_class, None),
            ("ClassLevel", GFFFieldType.SHORT, character.class_level, None),
            ("CompanionName", GFFFieldType.STRING, character.companion_name, None),
            ("CompanionType", GFFFieldType.INT, character.companion_type, None),
            ("CreatnScrptFird", GFFFieldType.BYTE, character.creatn_scrpt_fird, None),
            ("CreatureSize", GFFFieldType.INT, character.creature_size, None),
            ("CreatureVersion", GFFFieldType.WORD, character.creature_version, None),
            ("CustomHeartbeat", GFFFieldType.DWORD, character.custom_heartbeat, None),
            ("DeadSelectable", GFFFieldType.BYTE, character.dead_selectable, None),
            ("DecayTime", GFFFieldType.DWORD, character.decay_time, None),
            ("DefCastMode", GFFFieldType.BYTE, character.def_cast_mode, None),
            ("Disarmable", GFFFieldType.BYTE, character.disarmable, None),
            ("DmgReduction", GFFFieldType.LIST, character.dmg_reduction, self._create_list_from_array),
            ("EffectList", GFFFieldType.LIST, character.effect_list, self._create_list_from_array),
            ("ExpressionList", GFFFieldType.LIST, character.expression_list, self._create_list_from_array),
            ("FamiliarName", GFFFieldType.STRING, character.familiar_name, None),
            ("FamiliarType", GFFFieldType.INT, character.familiar_type, None),
            ("HlfrBlstMode", GFFFieldType.BYTE, character.hlfr_blst_mode, None),
            ("HlfrShldMode", GFFFieldType.BYTE, character.hlfr_shld_mode, None),
            ("HotbarList", GFFFieldType.LIST, character.hotbar_list, self._create_list_from_array),
            ("IgnoreTarget", GFFFieldType.INT, character.ignore_target, None),
            ("IsDM", GFFFieldType.BYTE, character.is_dm, None),
            ("IsImmortal", GFFFieldType.BYTE, character.is_immortal, None),
            ("IsPC", GFFFieldType.BYTE, character.is_pc, None),
            ("Listening", GFFFieldType.BYTE, character.listening, None),
            ("Lootable", GFFFieldType.BYTE, character.lootable, None),
            ("MClassLevUpIn", GFFFieldType.SHORT, character.m_class_lev_up_in, None),
            ("MasterID", GFFFieldType.DWORD, character.master_id, None),
            ("MovementRate", GFFFieldType.BYTE, character.movement_rate, None),
            ("NeverDrawHelmet", GFFFieldType.BYTE, character.never_draw_helmet, None),
            ("NoPermDeath", GFFFieldType.BYTE, character.no_perm_death, None),
            ("ObjectId", GFFFieldType.DWORD, character.object_id, None),
            ("OrientOnDialog", GFFFieldType.BYTE, character.orient_on_dialog, None),
            ("OverrideBAB", GFFFieldType.INT, character.override_bab, None),
            ("OverrideBABMin", GFFFieldType.INT, character.override_bab_min, None),
            ("PM_IsPolymorphed", GFFFieldType.BYTE, character.p_m_is_polymorphed, None),
            ("PersonalRepList", GFFFieldType.LIST, character.personal_rep_list, self._create_list_from_array),
            ("Plot", GFFFieldType.BYTE, character.plot, None),
            ("PossBlocked", GFFFieldType.BYTE, character.poss_blocked, None),
            ("PregameCurrent", GFFFieldType.BYTE, character.pregame_current, None),
            ("RosterMember", GFFFieldType.BYTE, character.roster_member, None),
            ("RosterTag", GFFFieldType.STRING, character.roster_tag, None),
            ("SitObject", GFFFieldType.DWORD, character.sit_object, None),
            ("SoundSetFile", GFFFieldType.WORD, character.sound_set_file, None),
            ("SpiritOverride", GFFFieldType.BYTE, character.spirit_override, None),
            ("StealthMode", GFFFieldType.BYTE, character.stealth_mode, None),
            ("Tag", GFFFieldType.STRING, character.tag, None),
            ("UVScroll", GFFFieldType.STRUCT, character.uv_scroll, self._create_struct_from_dict),
            ("VarTable", GFFFieldType.LIST, character.var_table, self._create_list_from_array),
            ("Variation", GFFFieldType.INT, character.variation, None),
            ("XOrientation", GFFFieldType.FLOAT, character.x_orientation, None),
            ("XPosition", GFFFieldType.FLOAT, character.x_position, None),
            ("XpMod", GFFFieldType.FLOAT, character.xp_mod, None),
            ("YOrientation", GFFFieldType.FLOAT, character.y_orientation, None),
            ("YPosition", GFFFieldType.FLOAT, character.y_position, None),
            ("ZOrientation", GFFFieldType.FLOAT, character.z_orientation, None),
            ("ZPosition", GFFFieldType.FLOAT, character.z_position, None),
            ("a", GFFFieldType.BYTE, character.a, None),
            ("b", GFFFieldType.BYTE, character.b, None),
            ("g", GFFFieldType.BYTE, character.g, None),
            ("oidTarget", GFFFieldType.DWORD, character.oid_target, None),
            ("r", GFFFieldType.BYTE, character.r, None),
        ]
        
        # Add all fields
        for label, field_type, value, converter in field_mapping:
            if value is None and field_type not in [GFFFieldType.STRING, GFFFieldType.LOCSTRING]:
                continue  # Skip null values except for strings
                
            if converter:
                converted_value = converter(value)
                if converted_value is not None:
                    # For STRUCT types, use the fields directly
                    if field_type == GFFFieldType.STRUCT and isinstance(converted_value, GFFElement):
                        fields.append(self._create_field(label, field_type, converted_value.value))
                    else:
                        fields.append(self._create_field(label, field_type, converted_value))
            else:
                fields.append(self._create_field(label, field_type, value))
                
        # Add complex fields (classes, feats, skills, etc.)
        class_list = self._create_class_list(character)
        if class_list:
            fields.append(self._create_field("ClassList", GFFFieldType.LIST, class_list))
            
        feat_list = self._create_feat_list(character)
        if feat_list:
            fields.append(self._create_field("FeatList", GFFFieldType.LIST, feat_list))
            
        skill_list = self._create_skill_list(character)
        if skill_list:
            fields.append(self._create_field("SkillList", GFFFieldType.LIST, skill_list))
            
        spell_fields = self._create_spell_fields(character)
        fields.extend(spell_fields)
        
        equip_list, item_list = self._create_item_lists(character)
        if equip_list:
            fields.append(self._create_field("Equip_ItemList", GFFFieldType.LIST, equip_list))
        if item_list:
            fields.append(self._create_field("ItemList", GFFFieldType.LIST, item_list))
            
        # Create top-level struct
        return GFFElement(GFFFieldType.STRUCT, 0, "", fields)
        
    def _create_field(self, label: str, field_type: GFFFieldType, value: Any) -> GFFElement:
        """Create a GFF field element"""
        return GFFElement(field_type, 0, label, value)
        
    def _create_localized_string(self, text: str) -> LocalizedString:
        """Create a localized string with English text"""
        if not text:
            return LocalizedString(-1, [])
        substrings = [LocalizedSubstring(text, 0, 0)]  # Language 0 = English, Gender 0 = Male
        return LocalizedString(-1, substrings)  # -1 = no string ref
        
    def _create_localized_from_dict(self, data: dict) -> LocalizedString:
        """Create localized string from dict"""
        if not data or not isinstance(data, dict):
            return LocalizedString(-1, [])
            
        string_ref = data.get('string_ref', -1)
        substrings = []
        
        if 'substrings' in data:
            for sub in data['substrings']:
                if isinstance(sub, dict):
                    text = sub.get('string', '')
                    lang = sub.get('language', 0)
                    gender = sub.get('gender', 0)
                    substrings.append(LocalizedSubstring(text, lang, gender))
                    
        return LocalizedString(string_ref, substrings)
        
    def _create_struct_from_dict(self, data: dict) -> GFFElement:
        """Create struct from dict"""
        if not data or not isinstance(data, dict):
            return None  # Don't create empty structs
            
        fields = []
        
        # Handle common struct patterns
        for key, value in data.items():
            if key == 'a' and isinstance(value, int):
                fields.append(self._create_field("a", GFFFieldType.BYTE, value))
            elif key == 'b' and isinstance(value, int):
                fields.append(self._create_field("b", GFFFieldType.BYTE, value))
            elif key == 'g' and isinstance(value, int):
                fields.append(self._create_field("g", GFFFieldType.BYTE, value))
            elif key == 'r' and isinstance(value, int):
                fields.append(self._create_field("r", GFFFieldType.BYTE, value))
            elif key == 'x' and isinstance(value, (int, float)):
                fields.append(self._create_field("x", GFFFieldType.FLOAT, float(value)))
            elif key == 'y' and isinstance(value, (int, float)):
                fields.append(self._create_field("y", GFFFieldType.FLOAT, float(value)))
            elif key == 'z' and isinstance(value, (int, float)):
                fields.append(self._create_field("z", GFFFieldType.FLOAT, float(value)))
                
        # Get struct_id from data if present
        struct_id = data.get('__struct_id', 0)
        return GFFElement(GFFFieldType.STRUCT, struct_id, "", fields)
        
    def _create_list_from_array(self, data: list) -> List[GFFElement]:
        """Create list from array"""
        if not data or not isinstance(data, list):
            return None  # Don't create empty lists
            
        structs = []
        for item in data:
            if isinstance(item, dict):
                # Convert dict to struct
                structs.append(self._create_struct_from_dict(item))
            elif isinstance(item, (int, float, str)):
                # Create simple struct with value
                fields = [self._create_field("Value", GFFFieldType.INT, int(item))]
                structs.append(GFFElement(GFFFieldType.STRUCT, 0, "", fields))
                
        return structs
        
    def _create_class_list(self, character: Character) -> List[GFFElement]:
        """Create the class list"""
        class_structs = []
        
        for char_class in character.classes.all().order_by('id'):
            fields = [
                self._create_field("Class", GFFFieldType.INT, char_class.class_id),
                self._create_field("ClassLevel", GFFFieldType.SHORT, char_class.class_level)
            ]
            
            # Add domains if present
            if char_class.domain1_id is not None:
                fields.append(self._create_field("Domain1", GFFFieldType.BYTE, char_class.domain1_id))
            if char_class.domain2_id is not None:
                fields.append(self._create_field("Domain2", GFFFieldType.BYTE, char_class.domain2_id))
                
            class_structs.append(GFFElement(GFFFieldType.STRUCT, 0, "", fields))
            
        return class_structs
        
    def _create_feat_list(self, character: Character) -> List[GFFElement]:
        """Create the feat list"""
        feat_structs = []
        
        for feat in character.feats.all().order_by('id'):
            fields = [
                self._create_field("Feat", GFFFieldType.WORD, feat.feat_id)
            ]
            feat_structs.append(GFFElement(GFFFieldType.STRUCT, 1, "", fields))
            
        return feat_structs
        
    def _create_skill_list(self, character: Character) -> List[GFFElement]:
        """Create the skill list"""
        skill_structs = []
        
        # Get all skills from 2DA
        skills_2da = self.rm.get_2da('skills')
        if not skills_2da:
            return skill_structs
            
        # Create a map of skill_id -> rank
        skill_ranks = {skill.skill_id: skill.rank for skill in character.skills.all()}
        
        # Create struct for each skill (including 0 ranks)
        for i in range(skills_2da.get_resource_count()):
            rank = skill_ranks.get(i, 0)
            fields = [
                self._create_field("Rank", GFFFieldType.BYTE, rank)
            ]
            skill_structs.append(GFFElement(GFFFieldType.STRUCT, 0, "", fields))
            
        return skill_structs
        
    def _create_spell_fields(self, character: Character) -> List[GFFElement]:
        """Create spell-related fields"""
        fields = []
        
        # Group spells by level and type
        known_spells = {}
        memorized_spells = {}
        
        for spell in character.spells.all():
            level = spell.spell_level
            if spell.is_memorized:
                if level not in memorized_spells:
                    memorized_spells[level] = []
                memorized_spells[level].append(spell)
            else:
                if level not in known_spells:
                    known_spells[level] = []
                known_spells[level].append(spell)
                
        # Create known spell lists
        for level in range(10):
            if level in known_spells:
                spell_structs = []
                for spell in known_spells[level]:
                    spell_fields = [
                        self._create_field("Spell", GFFFieldType.DWORD, spell.spell_id),
                        self._create_field("SpellMetaMagic", GFFFieldType.BYTE, 0),
                        self._create_field("SpellFlags", GFFFieldType.BYTE, 1)
                    ]
                    spell_structs.append(GFFElement(GFFFieldType.STRUCT, 3, "", spell_fields))
                fields.append(self._create_field(f"KnownList{level}", GFFFieldType.LIST, spell_structs))
                
        # Create memorized spell lists
        for level in range(10):
            if level in memorized_spells:
                spell_structs = []
                for i, spell in enumerate(memorized_spells[level]):
                    spell_fields = [
                        self._create_field("Spell", GFFFieldType.DWORD, spell.spell_id),
                        self._create_field("SpellMetaMagic", GFFFieldType.BYTE, 0),
                        self._create_field("SpellFlags", GFFFieldType.BYTE, 1),
                        self._create_field("SpellIndex", GFFFieldType.BYTE, i)
                    ]
                    spell_structs.append(GFFElement(GFFFieldType.STRUCT, 17234, "", spell_fields))
                fields.append(self._create_field(f"MemorizedList{level}", GFFFieldType.LIST, spell_structs))
                
        return fields
        
    def _create_item_lists(self, character: Character) -> tuple[List[GFFElement], List[GFFElement]]:
        """Create equipped and inventory item lists"""
        equip_structs = []
        inv_structs = []
        
        # Equipment slots order
        equipment_slots = [
            'HEAD', 'CHEST', 'BOOTS', 'ARMS', 'RIGHT_HAND', 'LEFT_HAND',
            'CLOAK', 'LEFT_RING', 'RIGHT_RING', 'NECK', 'BELT',
            'ARROWS', 'BULLETS', 'BOLTS', 'CWEAPON_L', 'CWEAPON_R', 
            'CWEAPON_B', 'CARMOUR'
        ]
        
        # Create empty slots
        for i in range(18):  # 18 equipment slots total
            equip_structs.append(None)
            
        # Place equipped items
        for item in character.items.filter(location__in=equipment_slots):
            try:
                slot_index = equipment_slots.index(item.location)
                equip_structs[slot_index] = self._create_item_struct(item)
            except ValueError:
                pass
                
        # Fill empty slots with empty struct
        for i in range(len(equip_structs)):
            if equip_structs[i] is None:
                equip_structs[i] = GFFElement(GFFFieldType.STRUCT, 0, "", [])
                
        # Add inventory items
        for item in character.items.filter(location='INVENTORY').order_by('inventory_slot'):
            inv_structs.append(self._create_item_struct(item))
            
        return equip_structs, inv_structs
        
    def _create_item_struct(self, item: CharacterItem) -> GFFElement:
        """Create an item struct"""
        fields = [
            self._create_field("BaseItem", GFFFieldType.INT, item.base_item_id),
            self._create_field("StackSize", GFFFieldType.WORD, item.stack_size)
        ]
        
        # Add localized name if present
        if item.localized_name:
            fields.append(self._create_field("LocalizedName", GFFFieldType.LOCSTRING,
                                           self._create_localized_string(item.localized_name)))
            
        # Add properties if present
        if item.properties and isinstance(item.properties, list):
            prop_structs = []
            for prop in item.properties:
                if isinstance(prop, dict):
                    prop_fields = []
                    if 'PropertyName' in prop:
                        prop_fields.append(self._create_field("PropertyName", GFFFieldType.WORD, prop['PropertyName']))
                    if 'Subtype' in prop:
                        prop_fields.append(self._create_field("Subtype", GFFFieldType.WORD, prop['Subtype']))
                    if 'CostTable' in prop:
                        prop_fields.append(self._create_field("CostTable", GFFFieldType.BYTE, prop['CostTable']))
                    if 'CostValue' in prop:
                        prop_fields.append(self._create_field("CostValue", GFFFieldType.WORD, prop['CostValue']))
                    if 'Param1' in prop:
                        prop_fields.append(self._create_field("Param1", GFFFieldType.BYTE, prop['Param1']))
                    if 'Param1Value' in prop:
                        prop_fields.append(self._create_field("Param1Value", GFFFieldType.BYTE, prop['Param1Value']))
                    prop_structs.append(GFFElement(GFFFieldType.STRUCT, 0, "", prop_fields))
                    
            if prop_structs:
                fields.append(self._create_field("PropertiesList", GFFFieldType.LIST, prop_structs))
        
        return GFFElement(GFFFieldType.STRUCT, 0, "", fields)