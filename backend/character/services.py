"""
Character import/export services
"""
import os
import logging
from django.db import transaction
from parsers.gff import GFFParser
from parsers.resource_manager import ResourceManager
from .models import Character, CharacterClass, CharacterFeat, CharacterSkill, CharacterSpell, CharacterItem

logger = logging.getLogger(__name__)


class CharacterImportService:
    """Service to import character data from GFF files with all fields"""
    
    def __init__(self, resource_manager: ResourceManager):
        self.rm = resource_manager
    
    def _extract_string(self, field_value):
        """Extract string from localized string structure"""
        if isinstance(field_value, dict):
            # First try substrings
            if 'substrings' in field_value and field_value['substrings']:
                return field_value['substrings'][0].get('string', '')
            # If no substrings but has string_ref, look it up
            elif 'string_ref' in field_value and field_value['string_ref'] >= 0:
                return self.rm.get_string(field_value['string_ref'])
            elif 'value' in field_value:
                return str(field_value['value'])
        return str(field_value) if field_value else ''
    
    def _get_numeric_value(self, value):
        """Convert value to int, handling character values"""
        if isinstance(value, str) and len(value) == 1:
            return ord(value)
        return int(value) if value else 0
    
    @transaction.atomic
    def import_character(self, file_path: str, owner=None, is_savegame: bool = False) -> Character:
        """
        Import a character from a .bic file or save game
        
        Args:
            file_path: Path to .bic/.ros file or save game directory/zip
            owner: Owner user object
            is_savegame: Whether this is a save game import
            
        Returns:
            Character model instance
        """
        # Check if this is a save game directory or zip
        if os.path.isdir(file_path):
            return self._import_from_savegame(file_path, owner)
        elif file_path.endswith('resgff.zip'):
            # Get the save directory from zip path
            save_dir = os.path.dirname(file_path)
            return self._import_from_savegame(save_dir, owner)
        
        # Standard .bic/.ros file import
        parser = GFFParser()
        parser.read(file_path)
        data = parser.top_level_struct.to_dict()
        
        # Detect module information
        self._detect_module_info(data, file_path)
        
        # Set up ResourceManager context based on detected module info
        if '_module_info' in data:
            self.rm.set_context(data['_module_info'])
        
        # Create character with all fields
        character = self._create_character(data, file_path, owner, is_savegame=False)
        
        # Import related data
        self._import_classes(character, data)
        self._import_feats(character, data)
        self._import_skills(character, data)
        self._import_spells(character, data)
        self._import_items(character, data)
        
        return character
    
    def _import_from_savegame(self, save_path: str, owner) -> Character:
        """Import character from a save game directory or zip with parallel GFF parsing"""
        from parsers.savegame_handler import SaveGameHandler
        from parsers.parallel_gff import extract_and_parse_save_gff_files
        from parsers.xml_parser import XmlParser
        from io import BytesIO
        
        # Create save game handler
        handler = SaveGameHandler(save_path)
        
        # Extract and parse all GFF files in parallel (2.3x speedup!)
        gff_results = extract_and_parse_save_gff_files(handler, max_workers=4)
        
        # Get playerlist.ifo data
        if 'playerlist.ifo' not in gff_results or not gff_results['playerlist.ifo']['success']:
            error = gff_results.get('playerlist.ifo', {}).get('error', 'Unknown error')
            raise ValueError(f"Failed to parse playerlist.ifo: {error}")
        
        player_list_data = gff_results['playerlist.ifo']['data']
        
        # Get the player character data (first entry in Mod_PlayerList)
        mod_player_list = player_list_data.get('Mod_PlayerList')
        if not mod_player_list:
            raise ValueError("No player data found in save game")
        
        # Get the first (and usually only) player
        data = mod_player_list[0] if mod_player_list else {}
        
        # Extract comprehensive campaign data from globals.xml
        try:
            globals_xml = handler.extract_globals_xml()
            if globals_xml:
                globals_parser = XmlParser(globals_xml)
                enhanced_data = globals_parser.get_full_summary()
                
                # Store comprehensive campaign data
                # Map new XML parser data structure to expected backend format
                quest_overview = enhanced_data['quest_overview']
                companion_status = enhanced_data['companion_status']
                
                # Convert companion data from new format to old expected format
                converted_companions = {}
                for comp_id, comp_data in companion_status.items():
                    influence_value = comp_data.get('influence')
                    converted_companions[comp_id] = {
                        'name': comp_data['name'],
                        'influence': influence_value if influence_value is not None else 0,
                        'status': comp_data.get('recruitment', 'not_met'),  # Map recruitment -> status
                        'influence_found': influence_value is not None,
                        'source': comp_data.get('source', 'explicit')
                    }
                
                data['_campaign_data'] = {
                    # Basic quest data (backwards compatibility)
                    'completed_quests_count': quest_overview['completed_count'],
                    'active_quests_count': quest_overview['active_count'],
                    'companion_influence': {k: v.get('influence', 0) for k, v in companion_status.items()},
                    'unlocked_locations': [],  # TODO: implement if needed
                    
                    # Enhanced campaign overview data  
                    'general_info': enhanced_data['general_info'],
                    'companion_status': converted_companions,
                    'hidden_statistics': {},  # Not in new parser yet
                    'story_milestones': {},   # Not in new parser yet
                    'quest_details': {
                        'summary': {
                            'completed_quests': quest_overview['completed_count'],
                            'active_quests': quest_overview['active_count'],
                            'total_quest_variables': quest_overview['total_quest_vars'],
                            'completed_quest_list': [],  # Available in quest_groups if needed
                            'active_quest_list': []     # Available in quest_groups if needed
                        },
                        'categories': quest_overview.get('quest_groups', {}),
                        'progress_stats': {
                            'total_completion_rate': round((quest_overview['completed_count'] / max(quest_overview['total_quest_vars'], 1)) * 100, 1),
                            'main_story_progress': quest_overview['completed_count'],  # Simplified for now
                            'companion_progress': len([c for c in converted_companions.values() if c['status'] in ['recruited', 'joined']]),
                            'exploration_progress': 0  # TODO: implement if needed
                        }
                    }
                }
                
                logger.info(f"Extracted enhanced campaign data:")
                logger.info(f"  - Act: {enhanced_data['general_info'].get('game_act', 'Unknown')}")
                logger.info(f"  - Quests: {quest_overview['completed_count']} completed, {quest_overview['active_count']} active")
                logger.info(f"  - Companions: {len([c for c in converted_companions.values() if c['status'] in ['recruited', 'joined']])} joined")
                logger.info(f"  - Quest Groups: {len(quest_overview.get('quest_groups', {}))} detected")
                
        except Exception as e:
            logger.warning(f"Failed to extract campaign data: {e}")
            import traceback
            traceback.print_exc()
            data['_campaign_data'] = {
                'completed_quests_count': 0,
                'active_quests_count': 0,
                'companion_influence': {},
                'unlocked_locations': [],
                'general_info': {},
                'companion_status': {},
                'hidden_statistics': {},
                'story_milestones': {},
                'quest_details': {'summary': {'completed_quests': 0, 'active_quests': 0, 'total_quest_variables': 0, 'completed_quest_list': [], 'active_quest_list': []}, 'categories': {}, 'progress_stats': {'total_completion_rate': 0, 'main_story_progress': 0, 'companion_progress': 0, 'exploration_progress': 0}}
            }
        
        # Extract current module/area
        try:
            current_module = handler.extract_current_module()
            if current_module:
                data['_current_area'] = current_module
                logger.info(f"Current area: {current_module}")
        except Exception as e:
            logger.warning(f"Failed to extract current module: {e}")
            data['_current_area'] = ''
        
        # Detect module information from save directory
        self._detect_module_info(data, save_path)
        
        # Set up ResourceManager context based on detected module info
        if '_module_info' in data:
            self.rm.set_context(data['_module_info'])
        
        # Store parsed GFF results for potential future use
        # (companions, additional character data, etc.)
        data['_parsed_gff_files'] = gff_results
        
        # Create character - store the save path for later updates
        character = self._create_character(data, save_path, owner, is_savegame=True)
        
        # Import related data
        self._import_classes(character, data)
        self._import_feats(character, data)
        self._import_skills(character, data)
        self._import_spells(character, data)
        self._import_items(character, data)
        
        return character
    
    def _create_character(self, data, file_path: str, owner, is_savegame: bool = False) -> Character:
        """Create the main character record with all fields"""
        
        # Map all GFF fields to model fields
        field_mapping = {
            # Core fields
            'FirstName': ('first_name', self._extract_string),
            'LastName': ('last_name', self._extract_string),
            'Age': ('age', int),
            'Gender': ('gender', int),
            'Deity': ('deity', str),
            'Race': ('race_id', int),
            'Subrace': ('subrace_id', int),
            'LawfulChaotic': ('law_chaos', int),
            'GoodEvil': ('good_evil', int),
            'Experience': ('experience', int),
            'Str': ('strength', int),
            'Dex': ('dexterity', int),
            'Con': ('constitution', int),
            'Int': ('intelligence', int),
            'Wis': ('wisdom', int),
            'Cha': ('charisma', int),
            'HitPoints': ('hit_points', int),
            'MaxHitPoints': ('max_hit_points', int),
            'CurrentHitPoints': ('current_hit_points', int),
            'ArmorClass': ('armor_class', int),
            'FortSaveThrow': ('fortitude_save', self._get_numeric_value),
            'RefSaveThrow': ('reflex_save', self._get_numeric_value),
            'WillSaveThrow': ('will_save', self._get_numeric_value),
            'Gold': ('gold', int),
            
            # Appearance fields
            'AppearanceSEF': ('appearance_sef', str),
            'Appearance_FHair': ('appearance_f_hair', int),
            'Appearance_Hair': ('appearance_hair', int),
            'Appearance_Head': ('appearance_head', int),
            'Appearance_Type': ('appearance_type', int),
            'ArmorTint': ('armor_tint', dict),
            'BodyBag': ('body_bag', int),
            'BodyBagId': ('body_bag_id', int),
            'Color_Tattoo1': ('color_tattoo1', int),
            'Color_Tattoo2': ('color_tattoo2', int),
            'CustomPortrait': ('custom_portrait', str),
            'ModelScale': ('model_scale', dict),
            'Portrait': ('portrait', str),
            'Tail': ('tail', int),
            'Tint_Hair': ('tint_hair', dict),
            'Tint_Head': ('tint_head', dict),
            'Tintable': ('tintable', dict),
            'Wings': ('wings', int),
            
            # Combat & Defense fields
            'ACBkHip': ('ac_bk_hip', dict),
            'ACFtHip': ('ac_ft_hip', dict),
            'ACLtAnkle': ('ac_lt_ankle', dict),
            'ACLtArm': ('ac_lt_arm', dict),
            'ACLtBracer': ('ac_lt_bracer', dict),
            'ACLtElbow': ('ac_lt_elbow', dict),
            'ACLtFoot': ('ac_lt_foot', dict),
            'ACLtHip': ('ac_lt_hip', dict),
            'ACLtKnee': ('ac_lt_knee', dict),
            'ACLtLeg': ('ac_lt_leg', dict),
            'ACLtShin': ('ac_lt_shin', dict),
            'ACLtShoulder': ('ac_lt_shoulder', dict),
            'ACRtAnkle': ('ac_rt_ankle', dict),
            'ACRtArm': ('ac_rt_arm', dict),
            'ACRtBracer': ('ac_rt_bracer', dict),
            'ACRtElbow': ('ac_rt_elbow', dict),
            'ACRtFoot': ('ac_rt_foot', dict),
            'ACRtHip': ('ac_rt_hip', dict),
            'ACRtKnee': ('ac_rt_knee', dict),
            'ACRtLeg': ('ac_rt_leg', dict),
            'ACRtShin': ('ac_rt_shin', dict),
            'ACRtShoulder': ('ac_rt_shoulder', dict),
            'ActionList': ('action_list', list),
            'ArmorVisualType': ('armor_visual_type', int),
            'AttackResult': ('attack_result', int),
            'BaseAttackBonus': ('base_attack_bonus', int),
            'BlockCombat': ('block_combat', int),
            'ChallengeRating': ('challenge_rating', float),
            'CharBackground': ('char_background', int),
            'CombatInfo': ('combat_info', dict),
            'CombatMode': ('combat_mode', int),
            'CombatRoundData': ('combat_round_data', dict),
            'DamageMax': ('damage_max', int),
            'DamageMin': ('damage_min', int),
            'FactionID': ('faction_id', int),
            'NaturalAC': ('natural_ac', int),
            'NeverShowArmor': ('never_show_armor', int),
            'OffHandAttacks': ('off_hand_attacks', int),
            'OnHandAttacks': ('on_hand_attacks', int),
            'OriginAttacked': ('origin_attacked', str),
            'OriginDamaged': ('origin_damaged', str),
            'ScriptAttacked': ('script_attacked', str),
            'ScriptDamaged': ('script_damaged', str),
            'StartingPackage': ('starting_package', int),
            'TrackingMode': ('tracking_mode', int),
            
            # Abilities & Skills fields
            'ConjureSoundTag': ('conjure_sound_tag', str),
            'Conversation': ('conversation', str),
            'Domain1': ('domain1', int),
            'Domain2': ('domain2', int),
            'Interruptable': ('interruptable', int),
            'IsDestroyable': ('is_destroyable', int),
            'OriginSpellAt': ('origin_spell_at', str),
            'ScriptSpellAt': ('script_spell_at', str),
            'SkillPoints': ('skill_points', int),
            'TemplateResRef': ('template_res_ref', str),
            'UnrestrictLU': ('unrestrict_lu', int),
            'fortbonus': ('fortbonus', int),
            'refbonus': ('refbonus', int),
            'willbonus': ('willbonus', int),
            
            # AI & Behavior fields
            'AreaId': ('area_id', int),
            'AssociateList': ('associate_list', list),
            'DetectMode': ('detect_mode', int),
            'DisableAIHidden': ('disable_ai_hidden', int),
            'IsCommandable': ('is_commandable', int),
            'IsRaiseable': ('is_raiseable', int),
            'PerceptionList': ('perception_list', list),
            'PerceptionRange': ('perception_range', int),
            
            # Animation & Visual fields
            'AmbientAnimState': ('ambient_anim_state', int),
            'AnimationDay': ('animation_day', int),
            'AnimationTime': ('animation_time', int),
            'CrtrCastsShadow': ('crtr_casts_shadow', int),
            'CrtrRcvShadow': ('crtr_rcv_shadow', int),
            'EnhVisionMode': ('enh_vision_mode', int),
            
            # Scripts & Events fields
            'Description': ('description', dict),
            'OriginDeath': ('origin_death', str),
            'OriginDialogue': ('origin_dialogue', str),
            'OriginDisturbed': ('origin_disturbed', str),
            'OriginEndRound': ('origin_end_round', str),
            'OriginHeartbeat': ('origin_heartbeat', str),
            'OriginOnBlocked': ('origin_on_blocked', str),
            'OriginOnNotice': ('origin_on_notice', str),
            'OriginRested': ('origin_rested', str),
            'OriginSpawn': ('origin_spawn', str),
            'OriginUserDefine': ('origin_user_define', str),
            'ScriptDeath': ('script_death', str),
            'ScriptDialogue': ('script_dialogue', str),
            'ScriptDisturbed': ('script_disturbed', str),
            'ScriptEndRound': ('script_end_round', str),
            'ScriptHeartbeat': ('script_heartbeat', str),
            'ScriptHidden': ('script_hidden', int),
            'ScriptOnBlocked': ('script_on_blocked', str),
            'ScriptOnNotice': ('script_on_notice', str),
            'ScriptRested': ('script_rested', str),
            'ScriptSpawn': ('script_spawn', str),
            'ScriptUserDefine': ('script_user_define', str),
            'ScriptsBckdUp': ('scripts_bckd_up', int),
            
            # Module & Campaign fields
            'Mod_CommntyId': ('mod_commnty_id', str),
            'Mod_CommntyName': ('mod_commnty_name', str),
            'Mod_CommntyPlatf': ('mod_commnty_platf', self._get_numeric_value),
            'Mod_IsPrimaryPlr': ('mod_is_primary_plr', int),
            'Mod_LastModId': ('mod_last_mod_id', str),
            'Mod_ModuleList': ('mod_module_list', list),
            'TalkPlayerOwn': ('talk_player_own', int),
            
            # Miscellaneous fields
            'AlwysPrcvbl': ('alwys_prcvbl', int),
            'BlockBroadcast': ('block_broadcast', int),
            'BlockRespond': ('block_respond', int),
            'Boots': ('boots', dict),
            'BumpState': ('bump_state', int),
            'Class': ('character_class', int),
            'ClassLevel': ('class_level', int),
            'CompanionName': ('companion_name', str),
            'CompanionType': ('companion_type', int),
            'CreatnScrptFird': ('creatn_scrpt_fird', int),
            'CreatureSize': ('creature_size', int),
            'CreatureVersion': ('creature_version', int),
            'CustomHeartbeat': ('custom_heartbeat', int),
            'DeadSelectable': ('dead_selectable', int),
            'DecayTime': ('decay_time', int),
            'DefCastMode': ('def_cast_mode', int),
            'Disarmable': ('disarmable', int),
            'DmgReduction': ('dmg_reduction', list),
            'EffectList': ('effect_list', list),
            'ExpressionList': ('expression_list', list),
            'FamiliarName': ('familiar_name', str),
            'FamiliarType': ('familiar_type', int),
            'HlfrBlstMode': ('hlfr_blst_mode', int),
            'HlfrShldMode': ('hlfr_shld_mode', int),
            'HotbarList': ('hotbar_list', list),
            'IgnoreTarget': ('ignore_target', int),
            'IsDM': ('is_dm', int),
            'IsImmortal': ('is_immortal', int),
            'IsPC': ('is_pc', int),
            'Listening': ('listening', int),
            'Lootable': ('lootable', int),
            'MClassLevUpIn': ('m_class_lev_up_in', int),
            'MasterID': ('master_id', int),
            'MovementRate': ('movement_rate', int),
            'NeverDrawHelmet': ('never_draw_helmet', int),
            'NoPermDeath': ('no_perm_death', int),
            'ObjectId': ('object_id', int),
            'OrientOnDialog': ('orient_on_dialog', int),
            'OverrideBAB': ('override_bab', int),
            'OverrideBABMin': ('override_bab_min', int),
            'PM_IsPolymorphed': ('p_m_is_polymorphed', int),
            'PersonalRepList': ('personal_rep_list', list),
            'Plot': ('plot', int),
            'PossBlocked': ('poss_blocked', int),
            'PregameCurrent': ('pregame_current', int),
            'RosterMember': ('roster_member', int),
            'RosterTag': ('roster_tag', str),
            'SitObject': ('sit_object', int),
            'SoundSetFile': ('sound_set_file', int),
            'SpiritOverride': ('spirit_override', int),
            'StealthMode': ('stealth_mode', int),
            'Tag': ('tag', str),
            'UVScroll': ('uv_scroll', dict),
            'VarTable': ('var_table', list),
            'Variation': ('variation', int),
            'XOrientation': ('x_orientation', float),
            'XPosition': ('x_position', float),
            'XpMod': ('xp_mod', float),
            'YOrientation': ('y_orientation', float),
            'YPosition': ('y_position', float),
            'ZOrientation': ('z_orientation', float),
            'ZPosition': ('z_position', float),
            'a': ('a', int),
            'b': ('b', int),
            'g': ('g', int),
            'oidTarget': ('oid_target', int),
            'r': ('r', int),
        }
        
        # Build character data
        char_data = {
            'owner': owner,
            'file_name': os.path.basename(file_path),
            'file_path': file_path,
            'is_savegame': is_savegame,
            'is_companion': file_path.endswith('.ros'),
        }
        
        # Map all fields with detailed validation
        validation_errors = []
        processed_fields = []
        skipped_fields = []
        
        for gff_field, (model_field, converter) in field_mapping.items():
            if gff_field in data:
                value = data[gff_field]
                try:
                    if converter == dict or converter == list:
                        # Keep complex types as-is
                        char_data[model_field] = value if isinstance(value, converter) else converter()
                    else:
                        # Convert simple types
                        converted_value = converter(value)
                        
                        # Only validate to prevent crashes - no game rule enforcement
                        if model_field in ['hit_points', 'max_hit_points']:
                            if converted_value < 1:
                                error_msg = f"Invalid {model_field}: {converted_value} (must be >= 1, fixing to 1)"
                                validation_errors.append(error_msg)
                                logger.warning(f"GFF field {gff_field} -> {error_msg}")
                                converted_value = 1
                        elif model_field == 'gold':
                            if converted_value < 0:
                                error_msg = f"Invalid gold: {converted_value} (negative gold not allowed, fixing to 0)"
                                validation_errors.append(error_msg)
                                logger.warning(f"GFF field {gff_field} -> {error_msg}")
                                converted_value = 0
                        
                        char_data[model_field] = converted_value
                        processed_fields.append(f"{gff_field}->{model_field}")
                        
                except (ValueError, TypeError) as e:
                    error_msg = f"Failed to convert GFF field '{gff_field}' (value: {repr(value)}) to {converter.__name__}: {e}"
                    validation_errors.append(error_msg)
                    logger.warning(error_msg)
                    skipped_fields.append(gff_field)
                    # Skip fields that fail conversion entirely
                    continue
            else:
                # Track fields that weren't present in GFF data
                pass
        
        # Log field processing summary
        logger.debug(f"Processed {len(processed_fields)} GFF fields successfully")
        if skipped_fields:
            logger.warning(f"Skipped {len(skipped_fields)} GFF fields due to conversion errors: {', '.join(skipped_fields)}")
        
        # Log validation summary
        if validation_errors:
            logger.warning(f"Character import had {len(validation_errors)} validation issues:")
            for error in validation_errors:
                logger.warning(f"  - {error}")
            logger.info("Invalid values were corrected to defaults, character import will proceed")
        else:
            logger.debug("All field validations passed")
        
        # Get race/subrace names
        race_id = char_data.get('race_id', 0)
        char_data['race_name'] = self.rm.get_race_name(race_id)
        
        subrace_id = char_data.get('subrace_id')
        if subrace_id is not None:
            subraces = self.rm.get_2da('racialsubtypes')
            if subraces and 0 <= subrace_id < subraces.get_resource_count():
                name_ref = subraces.get_int(subrace_id, 'Name')
                if name_ref:
                    char_data['subrace_name'] = self.rm.get_string(name_ref)
        
        # Calculate total character level from classes
        character_level = 0
        class_list = data.get('ClassList', [])
        if isinstance(class_list, list):
            for class_struct in class_list:
                if isinstance(class_struct, dict):
                    character_level += class_struct.get('ClassLevel', 0)
        char_data['character_level'] = character_level
        
        # Add module information
        if '_module_info' in data:
            module_info = data['_module_info']
            char_data['module_name'] = module_info.get('module_name', '')
            char_data['uses_custom_content'] = module_info.get('uses_custom_content', False)
            char_data['custom_content_ids'] = module_info.get('custom_content_ids', {})
            
            # Try to extract hakpaks from VarTable or other fields
            # This is where module.ifo would normally provide the hakpak list
            # For now, we'll leave module_hakpaks empty
            char_data['module_hakpaks'] = []
            
            # Add campaign information
            char_data['campaign_name'] = module_info.get('campaign_name', '')
            char_data['campaign_path'] = module_info.get('campaign_path', '')
            char_data['campaign_modules'] = module_info.get('campaign_modules', [])
            char_data['campaign_level_cap'] = module_info.get('campaign_level_cap')
        
        # Add comprehensive campaign overview data
        if '_campaign_data' in data:
            campaign_data = data['_campaign_data']
            
            # Basic quest data (backwards compatibility)
            char_data['completed_quests_count'] = campaign_data.get('completed_quests_count', 0)
            char_data['active_quests_count'] = campaign_data.get('active_quests_count', 0)
            char_data['companion_influence'] = campaign_data.get('companion_influence', {})
            char_data['unlocked_locations'] = campaign_data.get('unlocked_locations', [])
            
            # Enhanced campaign overview data
            general_info = campaign_data.get('general_info', {})
            char_data['game_act'] = general_info.get('game_act')
            char_data['difficulty_level'] = general_info.get('difficulty_level')
            char_data['last_saved_timestamp'] = general_info.get('last_saved_timestamp')
            
            char_data['companion_status'] = campaign_data.get('companion_status', {})
            char_data['hidden_statistics'] = campaign_data.get('hidden_statistics', {})
            char_data['story_milestones'] = campaign_data.get('story_milestones', {})
            char_data['quest_details'] = campaign_data.get('quest_details', {})
        
        # Add current area/module
        if '_current_area' in data:
            char_data['current_area'] = data['_current_area']
        
        # Log character data summary before database save for debugging
        logger.info(f"Creating character '{char_data.get('first_name', '')} {char_data.get('last_name', '')}' from {file_path}")
        
        # Log critical attributes
        critical_attrs = ['strength', 'dexterity', 'constitution', 'intelligence', 'wisdom', 'charisma', 
                         'character_level', 'law_chaos', 'good_evil', 'hit_points', 'max_hit_points', 'gold']
        attr_summary = []
        for attr in critical_attrs:
            if attr in char_data:
                attr_summary.append(f"{attr}={char_data[attr]}")
        
        if attr_summary:
            logger.info(f"Character attributes: {', '.join(attr_summary)}")
        
        # Log any custom content info
        if char_data.get('uses_custom_content'):
            logger.info(f"Character uses custom content from module: {char_data.get('module_name', 'Unknown')}")
        
        try:
            character = Character.objects.create(**char_data)
            logger.info(f"Successfully created character with ID {character.id}")
            return character
        except Exception as e:
            # Log the full char_data that caused the error for debugging
            logger.error(f"Failed to create character in database: {e}")
            logger.debug(f"Character data that failed: {char_data}")
            raise
    
    def _import_classes(self, character, data):
        """Import character classes"""
        class_list = data.get('ClassList', [])
        if not isinstance(class_list, list):
            return
            
        classes = self.rm.get_2da_with_overrides('classes')
        domains = self.rm.get_2da_with_overrides('domains')
        
        for class_struct in class_list:
            if not isinstance(class_struct, dict):
                continue
                
            class_id = class_struct.get('Class', 0)
            class_level = class_struct.get('ClassLevel', 1)
            
            # Get class name
            class_name = "Unknown"
            if classes and 0 <= class_id < classes.get_resource_count():
                name_ref = classes.get_int(class_id, 'Name')
                if name_ref:
                    class_name = self.rm.get_string(name_ref)
            
            # Get domains for divine casters
            domain1_id = class_struct.get('Domain1')
            domain2_id = class_struct.get('Domain2')
            domain1_name = ""
            domain2_name = ""
            
            if domains:
                if domain1_id is not None and 0 <= domain1_id < domains.get_resource_count():
                    name_ref = domains.get_int(domain1_id, 'Name')
                    if name_ref:
                        domain1_name = self.rm.get_string(name_ref)
                        
                if domain2_id is not None and 0 <= domain2_id < domains.get_resource_count():
                    name_ref = domains.get_int(domain2_id, 'Name')
                    if name_ref:
                        domain2_name = self.rm.get_string(name_ref)
            
            CharacterClass.objects.create(
                character=character,
                class_id=class_id,
                class_name=class_name,
                class_level=class_level,
                domain1_id=domain1_id,
                domain1_name=domain1_name,
                domain2_id=domain2_id,
                domain2_name=domain2_name
            )
    
    def _import_feats(self, character, data):
        """Import character feats"""
        feat_list = data.get('FeatList', [])
        if not isinstance(feat_list, list):
            return
            
        feats = self.rm.get_2da_with_overrides('feat')
        
        for feat_struct in feat_list:
            if not isinstance(feat_struct, dict):
                continue
                
            feat_id = feat_struct.get('Feat', 0)
            
            # Get feat name
            feat_name = "Unknown"
            if feats and 0 <= feat_id < feats.get_resource_count():
                name_ref = feats.get_int(feat_id, 'FEAT')
                if name_ref:
                    feat_name = self.rm.get_string(name_ref)
            
            CharacterFeat.objects.create(
                character=character,
                feat_id=feat_id,
                feat_name=feat_name
            )
    
    def _import_skills(self, character, data):
        """Import character skills"""
        skill_list = data.get('SkillList', [])
        if not isinstance(skill_list, list):
            return
            
        skills = self.rm.get_2da_with_overrides('skills')
        
        # Check if we have positional format (index = skill ID)
        # This is the modern format where each position in the list corresponds to a skill ID
        is_positional = False
        if skill_list and isinstance(skill_list[0], dict):
            is_positional = 'Skill' not in skill_list[0]
        
        if is_positional:
            # Handle positional format - index is the skill ID
            for skill_id, skill_data in enumerate(skill_list):
                if not isinstance(skill_data, dict):
                    continue
                    
                rank = skill_data.get('Rank', 0)
                
                if rank > 0 and skills and skill_id < skills.get_resource_count():
                    # Get skill name
                    name_ref = skills.get_int(skill_id, 'Name')
                    skill_name = self.rm.get_string(name_ref) if name_ref else f"Skill {skill_id}"
                    
                    CharacterSkill.objects.create(
                        character=character,
                        skill_id=skill_id,
                        skill_name=skill_name,
                        rank=rank
                    )
        else:
            # Handle old format with explicit 'Skill' field
            for skill_data in skill_list:
                if not isinstance(skill_data, dict):
                    continue
                    
                skill_id = skill_data.get('Skill')
                rank = skill_data.get('Rank', 0)
                
                if skill_id is None:
                    logger.warning(f"Skill entry missing 'Skill' field in old format: {skill_data}")
                    continue
                    
                if rank > 0 and skills and skill_id < skills.get_resource_count():
                    # Get skill name
                    name_ref = skills.get_int(skill_id, 'Name')
                    skill_name = self.rm.get_string(name_ref) if name_ref else f"Skill {skill_id}"
                    
                    CharacterSkill.objects.create(
                        character=character,
                        skill_id=skill_id,
                        skill_name=skill_name,
                        rank=rank
                    )
    
    def _import_spells(self, character, data):
        """Import character spells"""
        # TODO: Implement spell import
        pass
    
    def _import_items(self, character, data):
        """Import character items"""
        # Import equipped items
        equip_list = data.get('Equip_ItemList', [])
        if isinstance(equip_list, list):
            self._process_item_list(character, equip_list, is_equipped=True)
        
        # Import inventory items
        item_list = data.get('ItemList', [])
        if isinstance(item_list, list):
            self._process_item_list(character, item_list, is_equipped=False)
    
    def _process_item_list(self, character, item_list, is_equipped):
        """Process a list of items"""
        baseitems = self.rm.get_2da_with_overrides('baseitems')
        
        # Equipment slot mapping
        slot_mapping = {
            0: 'HEAD',
            1: 'CHEST',
            2: 'BOOTS',
            3: 'ARMS',
            4: 'RIGHT_HAND',
            5: 'LEFT_HAND',
            6: 'CLOAK',
            7: 'LEFT_RING',
            8: 'RIGHT_RING',
            9: 'NECK',
            10: 'BELT',
            11: 'ARROWS',
            12: 'BULLETS',
            13: 'BOLTS',
            14: 'CWEAPON_L',
            15: 'CWEAPON_R',
            16: 'CWEAPON_B',
            17: 'CARMOUR'
        }
        
        for index, item_data in enumerate(item_list):
            if not isinstance(item_data, dict):
                continue
                
            base_item_id = item_data.get('BaseItem', 0)
            
            # Get base item name
            base_item_name = "Unknown Item"
            if baseitems and 0 <= base_item_id < baseitems.get_resource_count():
                name_ref = baseitems.get_int(base_item_id, 'Name')
                if name_ref:
                    base_item_name = self.rm.get_string(name_ref)
            
            # Get localized name if available
            localized_name = ""
            local_name_field = item_data.get('LocalizedName')
            if local_name_field:
                localized_name = self._extract_string(local_name_field)
            
            # Determine location
            if is_equipped and index in slot_mapping:
                location = slot_mapping[index]
                inventory_slot = None
            else:
                location = 'INVENTORY'
                inventory_slot = index
            
            CharacterItem.objects.create(
                character=character,
                base_item_id=base_item_id,
                base_item_name=base_item_name,
                localized_name=localized_name,
                stack_size=item_data.get('StackSize', 1),
                location=location,
                inventory_slot=inventory_slot,
                properties=item_data.get('PropertiesList', [])
            )
    
    def _detect_module_info(self, data, file_path: str = None):
        """Detect module information and custom content in character data"""
        # Initialize module info in data if not present
        data['_module_info'] = {
            'module_name': '',
            'uses_custom_content': False,
            'custom_content_ids': {
                'classes': [],
                'feats': [],
                'spells': [],
                'skills': [],
                'items': []
            },
            'hakpaks': [],
            'campaign_name': '',
            'campaign_path': '',
            'campaign_modules': [],
            'campaign_level_cap': None
        }
        
        # First try to detect module from file location
        module_detected = False
        if file_path:
            # Check for currentmodule.txt in save directory
            from pathlib import Path
            save_dir = Path(file_path).parent
            module_txt = save_dir / "currentmodule.txt"
            
            if module_txt.exists():
                try:
                    module_name = module_txt.read_text().strip()
                    if module_name:
                        # Just detect and store the module name - don't load it yet
                        module_path = self.rm.find_module(module_name)
                        if module_path:
                            module_detected = True
                            data['_module_info']['module_name'] = module_name
                            data['_module_info']['module_path'] = str(module_path)
                            # Store the module path but don't load it globally yet
                            print(f"Detected module from currentmodule.txt: {module_name}")
                except Exception as e:
                    print(f"Error reading currentmodule.txt: {e}")
        
        # If no module detected from file, try from character data
        if not module_detected:
            # Try to determine module name from Mod_Name field (direct from character)
            mod_name = data.get('Mod_Name', '')
            if mod_name and mod_name != 'OfficialCampaign':
                data['_module_info']['module_name'] = mod_name
                # Just detect and store the module name - don't load it yet
                module_path = self.rm.find_module(mod_name)
                if module_path:
                    data['_module_info']['module_path'] = str(module_path)
                    print(f"Detected module from character data: {mod_name}")
            else:
                # Fall back to Mod_LastModId
                mod_id = data.get('Mod_LastModId', '')
                if mod_id:
                    # The mod ID often contains the module name encoded
                    # For official campaign it might be empty or 'OfficialCampaign'
                    if 'OfficialCampaign' in mod_id or not mod_id.strip():
                        data['_module_info']['module_name'] = 'OfficialCampaign'
                    else:
                        # Try to extract a readable name from the ID
                        # This is a best-effort approach
                        data['_module_info']['module_name'] = mod_id[:32]  # First 32 chars
        
        # Check for custom classes - any class not in base game could be custom
        class_list = data.get('ClassList', [])
        for cls in class_list:
            class_id = cls.get('Class', -1)
            if class_id >= 0:
                # Check if this class exists in base game data
                classes_2da = self.rm.get_2da('classes')
                if classes_2da and class_id >= classes_2da.get_resource_count():
                    data['_module_info']['uses_custom_content'] = True
                    data['_module_info']['custom_content_ids']['classes'].append(class_id)
        
        # Check for custom feats - any feat not in base game could be custom
        feat_list = data.get('FeatList', [])
        for feat in feat_list:
            feat_id = feat.get('Feat', -1)
            if feat_id >= 0:
                # Check if this feat exists in base game data
                feats_2da = self.rm.get_2da('feat')
                if feats_2da and feat_id >= feats_2da.get_resource_count():
                    data['_module_info']['uses_custom_content'] = True
                    data['_module_info']['custom_content_ids']['feats'].append(feat_id)
        
        # Check for custom spells - any spell not in base game could be custom
        spells_2da = self.rm.get_2da('spells')
        if spells_2da:
            max_base_spell_id = spells_2da.get_resource_count()
            
            # Check memorized spells
            for i in range(10):  # Check spell levels 0-9
                spell_class = f'SpellLvlMem{i}'
                if spell_class in data:
                    for spell_level in data[spell_class]:
                        for spell in spell_level.get('MemorizedList', []):
                            spell_id = spell.get('Spell', -1)
                            if spell_id >= max_base_spell_id:
                                data['_module_info']['uses_custom_content'] = True
                                if spell_id not in data['_module_info']['custom_content_ids']['spells']:
                                    data['_module_info']['custom_content_ids']['spells'].append(spell_id)
            
            # Check known spells
            for i in range(10):
                spell_class = f'KnownList{i}'
                if spell_class in data:
                    for spell in data[spell_class]:
                        spell_id = spell.get('Spell', -1)
                        if spell_id >= max_base_spell_id:
                            data['_module_info']['uses_custom_content'] = True
                            if spell_id not in data['_module_info']['custom_content_ids']['spells']:
                                data['_module_info']['custom_content_ids']['spells'].append(spell_id)
        
        # Detect campaign information
        self._detect_campaign_info(data, file_path)
    
    def _detect_campaign_info(self, data, file_path: str = None):
        """Detect campaign information from module name and location"""
        module_name = data['_module_info'].get('module_name', '')
        
        # Known campaign module patterns
        campaign_patterns = {
            # Original Campaign
            '0_tutorial': 'Neverwinter Nights 2 Campaign',
            '1000_neverwinter': 'Neverwinter Nights 2 Campaign',
            '1100_west_harbor': 'Neverwinter Nights 2 Campaign',
            '1200_highcliff': 'Neverwinter Nights 2 Campaign',
            '1300_old_owl_well': 'Neverwinter Nights 2 Campaign',
            '1700_merchant_quarter': 'Neverwinter Nights 2 Campaign',
            # Mask of the Betrayer
            '2000_motb': 'NWN2 Mask of the Betrayer Campaign',
            '2100_mulsantir': 'NWN2 Mask of the Betrayer Campaign',
            '2200_thayred': 'NWN2 Mask of the Betrayer Campaign',
            # Storm of Zehir
            '3000_soz': 'Neverwinter Nights 2 Campaign_X2',
            'n2_black_jungle': 'Neverwinter Nights 2 Campaign_X2',
            'neverwinter_x2': 'Neverwinter Nights 2 Campaign_X2',
            # Mysteries of Westgate
            'westgate': 'Neverwinter Nights 2 Campaign_X3',
            'mow_': 'Neverwinter Nights 2 Campaign_X3',
        }
        
        # Try to match module name to campaign
        campaign_folder = None
        for pattern, campaign_dir in campaign_patterns.items():
            if pattern in module_name.lower():
                campaign_folder = campaign_dir
                break
        
        # If we found a campaign, try to load its information
        if campaign_folder:
            from config.nwn2_settings import nwn2_paths
            campaigns_dir = str(nwn2_paths.campaigns)
            if campaigns_dir:
                campaign_path = os.path.join(campaigns_dir, campaign_folder)
                if os.path.exists(campaign_path):
                    campaign_info = self.rm.find_campaign(campaign_path)
                    if campaign_info:
                        data['_module_info']['campaign_name'] = campaign_info.get('name', '')
                        data['_module_info']['campaign_path'] = campaign_path
                        data['_module_info']['campaign_modules'] = campaign_info.get('modules', [])
                        data['_module_info']['campaign_level_cap'] = campaign_info.get('level_cap')
        
        # If module is in a known campaign list, detect from resource manager's campaign data
        if not data['_module_info']['campaign_name'] and module_name:
            # Check all campaigns for this module
            from config.nwn2_settings import nwn2_paths
            campaigns_dir = str(nwn2_paths.campaigns)
            if campaigns_dir and os.path.exists(campaigns_dir):
                for campaign_folder in os.listdir(campaigns_dir):
                    campaign_path = os.path.join(campaigns_dir, campaign_folder)
                    if os.path.isdir(campaign_path):
                        campaign_info = self.rm.find_campaign(campaign_path)
                        if campaign_info and module_name in campaign_info.get('modules', []):
                            data['_module_info']['campaign_name'] = campaign_info.get('name', '')
                            data['_module_info']['campaign_path'] = campaign_path
                            data['_module_info']['campaign_modules'] = campaign_info.get('modules', [])
                            data['_module_info']['campaign_level_cap'] = campaign_info.get('level_cap')
                            break