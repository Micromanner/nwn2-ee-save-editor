import os
from django.db import models
from django.core.validators import MinValueValidator, MaxValueValidator
from django.core.exceptions import ValidationError
from django.db.models import Q, Sum
from django.db.models.signals import post_save, pre_save
from django.dispatch import receiver


class CharacterManager(models.Manager):
    """Custom manager for Character model with common queries"""
    
    def owned_by(self, user):
        """Get all characters owned by a user"""
        return self.filter(owner=user)
    
    def player_characters(self):
        """Get all player characters (not companions)"""
        return self.filter(is_companion=False)
    
    def companions(self):
        """Get all companion characters"""
        return self.filter(is_companion=True)
    
    def from_campaign(self, campaign_name):
        """Get all characters from a specific campaign"""
        return self.filter(campaign_name=campaign_name)
    
    def with_custom_content(self):
        """Get all characters using custom content"""
        return self.filter(uses_custom_content=True)
    
    def high_level(self, min_level=20):
        """Get high-level characters"""
        return self.filter(character_level__gte=min_level)
    
    def by_class(self, class_id):
        """Get all characters with a specific class"""
        return self.filter(classes__class_id=class_id).distinct()
    
    def with_related(self):
        """Prefetch all related objects for performance"""
        return self.prefetch_related(
            'classes', 'feats', 'skills', 'items', 'spells'
        )


class Character(models.Model):
    """
    Complete NWN2 character data model with all fields from GFF files.
    Generated from analysis of actual character files.
    """

    # === Core Identity Fields ===
    # (These are the primary fields already in the model)
    owner = models.ForeignKey('auth.User', on_delete=models.CASCADE, null=True, blank=True)
    file_name = models.CharField(max_length=255)
    file_path = models.CharField(max_length=500)
    is_savegame = models.BooleanField(default=False, help_text='Whether this is from a save game (vs standalone .bic)')
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    is_companion = models.BooleanField(default=False)
    
    first_name = models.CharField(max_length=100, blank=True)
    last_name = models.CharField(max_length=100, blank=True)
    age = models.IntegerField(default=20)
    gender = models.IntegerField(default=0)  # 0=Male, 1=Female
    deity = models.CharField(max_length=100, blank=True)
    race_id = models.IntegerField(default=0)
    race_name = models.CharField(max_length=100)
    subrace_id = models.IntegerField(null=True, blank=True)
    subrace_name = models.CharField(max_length=100, blank=True)

    # === Appearance Fields ===
    appearance_sef = models.CharField(max_length=32, blank=True, default='', help_text='AppearanceSEF')
    appearance_f_hair = models.IntegerField(default=0, help_text='Appearance_FHair')
    appearance_hair = models.IntegerField(default=0, help_text='Appearance_Hair')
    appearance_head = models.IntegerField(default=0, help_text='Appearance_Head')
    appearance_type = models.IntegerField(default=0, help_text='Appearance_Type')
    armor_tint = models.JSONField(default=dict, blank=True, help_text='ArmorTint')
    body_bag = models.IntegerField(default=0, help_text='BodyBag')
    body_bag_id = models.IntegerField(default=2130706432, help_text='BodyBagId')
    color_tattoo1 = models.IntegerField(default=0, help_text='Color_Tattoo1')
    color_tattoo2 = models.IntegerField(default=0, help_text='Color_Tattoo2')
    custom_portrait = models.CharField(max_length=32, blank=True, default='', help_text='CustomPortrait')
    model_scale = models.JSONField(default=dict, blank=True, help_text='ModelScale')
    portrait = models.CharField(max_length=32, blank=True, default='', help_text='Portrait')
    tail = models.IntegerField(default=0, help_text='Tail')
    tint_hair = models.JSONField(default=dict, blank=True, help_text='Tint_Hair')
    tint_head = models.JSONField(default=dict, blank=True, help_text='Tint_Head')
    tintable = models.JSONField(default=dict, blank=True, help_text='Tintable')
    wings = models.IntegerField(default=0, help_text='Wings')

    # === Combat & Defense Fields ===
    ac_bk_hip = models.JSONField(default=dict, blank=True, help_text='ACBkHip')
    ac_ft_hip = models.JSONField(default=dict, blank=True, help_text='ACFtHip')
    ac_lt_ankle = models.JSONField(default=dict, blank=True, help_text='ACLtAnkle')
    ac_lt_arm = models.JSONField(default=dict, blank=True, help_text='ACLtArm')
    ac_lt_bracer = models.JSONField(default=dict, blank=True, help_text='ACLtBracer')
    ac_lt_elbow = models.JSONField(default=dict, blank=True, help_text='ACLtElbow')
    ac_lt_foot = models.JSONField(default=dict, blank=True, help_text='ACLtFoot')
    ac_lt_hip = models.JSONField(default=dict, blank=True, help_text='ACLtHip')
    ac_lt_knee = models.JSONField(default=dict, blank=True, help_text='ACLtKnee')
    ac_lt_leg = models.JSONField(default=dict, blank=True, help_text='ACLtLeg')
    ac_lt_shin = models.JSONField(default=dict, blank=True, help_text='ACLtShin')
    ac_lt_shoulder = models.JSONField(default=dict, blank=True, help_text='ACLtShoulder')
    ac_rt_ankle = models.JSONField(default=dict, blank=True, help_text='ACRtAnkle')
    ac_rt_arm = models.JSONField(default=dict, blank=True, help_text='ACRtArm')
    ac_rt_bracer = models.JSONField(default=dict, blank=True, help_text='ACRtBracer')
    ac_rt_elbow = models.JSONField(default=dict, blank=True, help_text='ACRtElbow')
    ac_rt_foot = models.JSONField(default=dict, blank=True, help_text='ACRtFoot')
    ac_rt_hip = models.JSONField(default=dict, blank=True, help_text='ACRtHip')
    ac_rt_knee = models.JSONField(default=dict, blank=True, help_text='ACRtKnee')
    ac_rt_leg = models.JSONField(default=dict, blank=True, help_text='ACRtLeg')
    ac_rt_shin = models.JSONField(default=dict, blank=True, help_text='ACRtShin')
    ac_rt_shoulder = models.JSONField(default=dict, blank=True, help_text='ACRtShoulder')
    action_list = models.JSONField(default=dict, blank=True, help_text='ActionList')
    armor_visual_type = models.IntegerField(default=0, help_text='ArmorVisualType')
    attack_result = models.IntegerField(default=0, help_text='AttackResult')
    base_attack_bonus = models.IntegerField(default=0, help_text='BaseAttackBonus')
    block_combat = models.IntegerField(default=0, help_text='BlockCombat')
    challenge_rating = models.FloatField(default=0.0, help_text='ChallengeRating')
    char_background = models.IntegerField(default=0, help_text='CharBackground')
    combat_info = models.JSONField(default=dict, blank=True, help_text='CombatInfo')
    combat_mode = models.IntegerField(default=0, help_text='CombatMode')
    combat_round_data = models.JSONField(default=dict, blank=True, help_text='CombatRoundData')
    current_hit_points = models.IntegerField(default=0, help_text='CurrentHitPoints')
    damage_max = models.IntegerField(default=0, help_text='DamageMax')
    damage_min = models.IntegerField(default=0, help_text='DamageMin')
    faction_id = models.IntegerField(default=0, help_text='FactionID')
    natural_ac = models.IntegerField(default=0, help_text='NaturalAC')
    never_show_armor = models.IntegerField(default=0, help_text='NeverShowArmor')
    off_hand_attacks = models.IntegerField(default=0, help_text='OffHandAttacks')
    on_hand_attacks = models.IntegerField(default=0, help_text='OnHandAttacks')
    origin_attacked = models.CharField(max_length=32, blank=True, default='', help_text='OriginAttacked')
    origin_damaged = models.CharField(max_length=32, blank=True, default='', help_text='OriginDamaged')
    script_attacked = models.CharField(max_length=32, blank=True, default='', help_text='ScriptAttacked')
    script_damaged = models.CharField(max_length=32, blank=True, default='', help_text='ScriptDamaged')
    starting_package = models.IntegerField(default=0, help_text='StartingPackage')
    tracking_mode = models.IntegerField(default=0, help_text='TrackingMode')

    # === Abilities & Skills Fields ===
    conjure_sound_tag = models.CharField(max_length=32, blank=True, default='', help_text='ConjureSoundTag')
    conversation = models.CharField(max_length=32, blank=True, default='', help_text='Conversation')
    domain1 = models.IntegerField(default=0, help_text='Domain1')
    domain2 = models.IntegerField(default=0, help_text='Domain2')
    interruptable = models.IntegerField(default=0, help_text='Interruptable')
    is_destroyable = models.IntegerField(default=0, help_text='IsDestroyable')
    origin_spell_at = models.CharField(max_length=32, blank=True, default='', help_text='OriginSpellAt')
    script_spell_at = models.CharField(max_length=32, blank=True, default='', help_text='ScriptSpellAt')
    skill_points = models.IntegerField(default=0, help_text='SkillPoints')
    template_res_ref = models.CharField(max_length=32, blank=True, default='', help_text='TemplateResRef')
    unrestrict_lu = models.IntegerField(default=0, help_text='UnrestrictLU')
    fortbonus = models.IntegerField(default=0, help_text='fortbonus')
    refbonus = models.IntegerField(default=0, help_text='refbonus')
    willbonus = models.IntegerField(default=0, help_text='willbonus')

    # === AI & Behavior Fields ===
    area_id = models.IntegerField(default=5012, help_text='AreaId')
    associate_list = models.JSONField(default=dict, blank=True, help_text='AssociateList')
    detect_mode = models.IntegerField(default=0, help_text='DetectMode')
    disable_ai_hidden = models.IntegerField(default=0, help_text='DisableAIHidden')
    is_commandable = models.IntegerField(default=0, help_text='IsCommandable')
    is_raiseable = models.IntegerField(default=0, help_text='IsRaiseable')
    perception_list = models.JSONField(default=dict, blank=True, help_text='PerceptionList')
    perception_range = models.IntegerField(default=0, help_text='PerceptionRange')

    # === Animation & Visual Fields ===
    ambient_anim_state = models.IntegerField(default=0, help_text='AmbientAnimState')
    animation_day = models.IntegerField(default=461152, help_text='AnimationDay')
    animation_time = models.IntegerField(default=1148714, help_text='AnimationTime')
    crtr_casts_shadow = models.IntegerField(default=0, help_text='CrtrCastsShadow')
    crtr_rcv_shadow = models.IntegerField(default=0, help_text='CrtrRcvShadow')
    enh_vision_mode = models.IntegerField(default=0, help_text='EnhVisionMode')

    # === Scripts & Events Fields ===
    description = models.JSONField(default=dict, blank=True, help_text='Description')
    origin_death = models.CharField(max_length=32, blank=True, default='', help_text='OriginDeath')
    origin_dialogue = models.CharField(max_length=32, blank=True, default='', help_text='OriginDialogue')
    origin_disturbed = models.CharField(max_length=32, blank=True, default='', help_text='OriginDisturbed')
    origin_end_round = models.CharField(max_length=32, blank=True, default='', help_text='OriginEndRound')
    origin_heartbeat = models.CharField(max_length=32, blank=True, default='', help_text='OriginHeartbeat')
    origin_on_blocked = models.CharField(max_length=32, blank=True, default='', help_text='OriginOnBlocked')
    origin_on_notice = models.CharField(max_length=32, blank=True, default='', help_text='OriginOnNotice')
    origin_rested = models.CharField(max_length=32, blank=True, default='', help_text='OriginRested')
    origin_spawn = models.CharField(max_length=32, blank=True, default='', help_text='OriginSpawn')
    origin_user_define = models.CharField(max_length=32, blank=True, default='', help_text='OriginUserDefine')
    script_death = models.CharField(max_length=32, blank=True, default='', help_text='ScriptDeath')
    script_dialogue = models.CharField(max_length=32, blank=True, default='', help_text='ScriptDialogue')
    script_disturbed = models.CharField(max_length=32, blank=True, default='', help_text='ScriptDisturbed')
    script_end_round = models.CharField(max_length=32, blank=True, default='', help_text='ScriptEndRound')
    script_heartbeat = models.CharField(max_length=32, blank=True, default='', help_text='ScriptHeartbeat')
    script_hidden = models.IntegerField(default=0, help_text='ScriptHidden')
    script_on_blocked = models.CharField(max_length=32, blank=True, default='', help_text='ScriptOnBlocked')
    script_on_notice = models.CharField(max_length=32, blank=True, default='', help_text='ScriptOnNotice')
    script_rested = models.CharField(max_length=32, blank=True, default='', help_text='ScriptRested')
    script_spawn = models.CharField(max_length=32, blank=True, default='', help_text='ScriptSpawn')
    script_user_define = models.CharField(max_length=32, blank=True, default='', help_text='ScriptUserDefine')
    scripts_bckd_up = models.IntegerField(default=0, help_text='ScriptsBckdUp')

    # === Module & Campaign Fields ===
    mod_commnty_id = models.CharField(max_length=32, blank=True, default='', help_text='Mod_CommntyId')
    mod_commnty_name = models.CharField(max_length=32, blank=True, default='', help_text='Mod_CommntyName')
    mod_commnty_platf = models.IntegerField(default=0, help_text='Mod_CommntyPlatf (stored as ASCII value)')
    mod_is_primary_plr = models.IntegerField(default=0, help_text='Mod_IsPrimaryPlr')
    mod_last_mod_id = models.CharField(max_length=128, blank=True, default='', help_text='Mod_LastModId')
    mod_module_list = models.JSONField(default=dict, blank=True, help_text='Mod_ModuleList')
    talk_player_own = models.IntegerField(default=0, help_text='TalkPlayerOwn')

    # === Miscellaneous Fields ===
    alwys_prcvbl = models.IntegerField(default=0, help_text='AlwysPrcvbl')
    block_broadcast = models.IntegerField(default=0, help_text='BlockBroadcast')
    block_respond = models.IntegerField(default=0, help_text='BlockRespond')
    boots = models.JSONField(default=dict, blank=True, help_text='Boots')
    bump_state = models.IntegerField(default=0, help_text='BumpState')
    character_class = models.IntegerField(default=0, help_text='Class')
    class_level = models.IntegerField(default=0, help_text='ClassLevel')
    companion_name = models.CharField(max_length=32, blank=True, default='', help_text='CompanionName')
    companion_type = models.IntegerField(default=0, help_text='CompanionType')
    creatn_scrpt_fird = models.IntegerField(default=0, help_text='CreatnScrptFird')
    creature_size = models.IntegerField(default=0, help_text='CreatureSize')
    creature_version = models.IntegerField(default=0, help_text='CreatureVersion')
    custom_heartbeat = models.IntegerField(default=0, help_text='CustomHeartbeat')
    dead_selectable = models.IntegerField(default=0, help_text='DeadSelectable')
    decay_time = models.IntegerField(default=5000, help_text='DecayTime')
    def_cast_mode = models.IntegerField(default=0, help_text='DefCastMode')
    disarmable = models.IntegerField(default=0, help_text='Disarmable')
    dmg_reduction = models.JSONField(default=dict, blank=True, help_text='DmgReduction')
    effect_list = models.JSONField(default=dict, blank=True, help_text='EffectList')
    expression_list = models.JSONField(default=dict, blank=True, help_text='ExpressionList')
    familiar_name = models.CharField(max_length=32, blank=True, default='', help_text='FamiliarName')
    familiar_type = models.IntegerField(default=0, help_text='FamiliarType')
    hlfr_blst_mode = models.IntegerField(default=0, help_text='HlfrBlstMode')
    hlfr_shld_mode = models.IntegerField(default=0, help_text='HlfrShldMode')
    hotbar_list = models.JSONField(default=dict, blank=True, help_text='HotbarList')
    ignore_target = models.IntegerField(default=0, help_text='IgnoreTarget')
    is_dm = models.IntegerField(default=0, help_text='IsDM')
    is_immortal = models.IntegerField(default=0, help_text='IsImmortal')
    is_pc = models.IntegerField(default=0, help_text='IsPC')
    listening = models.IntegerField(default=0, help_text='Listening')
    lootable = models.IntegerField(default=0, help_text='Lootable')
    m_class_lev_up_in = models.IntegerField(default=0, help_text='MClassLevUpIn')
    master_id = models.IntegerField(default=2130706432, help_text='MasterID')
    movement_rate = models.IntegerField(default=0, help_text='MovementRate')
    never_draw_helmet = models.IntegerField(default=0, help_text='NeverDrawHelmet')
    no_perm_death = models.IntegerField(default=0, help_text='NoPermDeath')
    object_id = models.IntegerField(default=2147483647, help_text='ObjectId')
    orient_on_dialog = models.IntegerField(default=0, help_text='OrientOnDialog')
    override_bab = models.IntegerField(default=0, help_text='OverrideBAB')
    override_bab_min = models.IntegerField(default=0, help_text='OverrideBABMin')
    p_m_is_polymorphed = models.IntegerField(default=0, help_text='PM_IsPolymorphed')
    personal_rep_list = models.JSONField(default=dict, blank=True, help_text='PersonalRepList')
    plot = models.IntegerField(default=0, help_text='Plot')
    poss_blocked = models.IntegerField(default=0, help_text='PossBlocked')
    pregame_current = models.IntegerField(default=0, help_text='PregameCurrent')
    roster_member = models.IntegerField(default=0, help_text='RosterMember')
    roster_tag = models.CharField(max_length=32, blank=True, default='', help_text='RosterTag')
    sit_object = models.IntegerField(default=2130706432, help_text='SitObject')
    sound_set_file = models.IntegerField(default=0, help_text='SoundSetFile')
    spirit_override = models.IntegerField(default=0, help_text='SpiritOverride')
    stealth_mode = models.IntegerField(default=0, help_text='StealthMode')
    tag = models.CharField(max_length=32, blank=True, default='', help_text='Tag')
    uv_scroll = models.JSONField(default=dict, blank=True, help_text='UVScroll')
    var_table = models.JSONField(default=dict, blank=True, help_text='VarTable')
    variation = models.IntegerField(default=0, help_text='Variation')
    x_orientation = models.FloatField(default=-0.1, help_text='XOrientation')
    x_position = models.FloatField(default=98.7, help_text='XPosition')
    xp_mod = models.FloatField(default=1.0, help_text='XpMod')
    y_orientation = models.FloatField(default=1.0, help_text='YOrientation')
    y_position = models.FloatField(default=135.1, help_text='YPosition')
    z_orientation = models.FloatField(default=0.0, help_text='ZOrientation')
    z_position = models.FloatField(default=-0.0, help_text='ZPosition')
    a = models.IntegerField(default=0, help_text='a')
    b = models.IntegerField(default=0, help_text='b')
    g = models.IntegerField(default=0, help_text='g')
    oid_target = models.IntegerField(default=2130706432, help_text='oidTarget')
    r = models.IntegerField(default=0, help_text='r')

    # Keep the existing critical fields with their validators
    law_chaos = models.IntegerField(default=50)
    good_evil = models.IntegerField(default=50)
    experience = models.IntegerField(default=0, validators=[MinValueValidator(0)])  # Keep this - negative XP would be weird
    character_level = models.IntegerField(default=1)
    strength = models.IntegerField(default=10)
    dexterity = models.IntegerField(default=10)
    constitution = models.IntegerField(default=10)
    intelligence = models.IntegerField(default=10)
    wisdom = models.IntegerField(default=10)
    charisma = models.IntegerField(default=10)
    hit_points = models.IntegerField(default=1, validators=[MinValueValidator(1)])
    max_hit_points = models.IntegerField(default=1, validators=[MinValueValidator(1)])
    armor_class = models.IntegerField(default=10)
    fortitude_save = models.IntegerField(default=0)
    reflex_save = models.IntegerField(default=0)
    will_save = models.IntegerField(default=0)
    gold = models.IntegerField(default=0, validators=[MinValueValidator(0)])
    
    # Module support fields
    module_name = models.CharField(max_length=255, blank=True, 
                                  help_text='Module this save is from (e.g., MotB_Campaign)')
    uses_custom_content = models.BooleanField(default=False,
                                            help_text='Whether this character uses module-specific content')
    module_hakpaks = models.JSONField(default=list, blank=True,
                                     help_text='List of hakpaks from module.ifo')
    custom_content_ids = models.JSONField(default=dict, blank=True,
                                        help_text='IDs of custom content used by this character')
    
    # Campaign support fields
    campaign_name = models.CharField(max_length=255, blank=True,
                                   help_text='Campaign this character is from (e.g., Original Campaign, MotB)')
    campaign_path = models.CharField(max_length=500, blank=True,
                                   help_text='Path to the campaign directory')
    campaign_modules = models.JSONField(default=list, blank=True,
                                      help_text='List of modules in the campaign')
    campaign_level_cap = models.IntegerField(null=True, blank=True,
                                           help_text='Campaign level cap')
    
    # Quest and story progress (from globals.xml)
    completed_quests_count = models.IntegerField(default=0, 
                                               help_text='Number of completed quests')
    active_quests_count = models.IntegerField(default=0,
                                            help_text='Number of active quests')
    companion_influence = models.JSONField(default=dict, blank=True,
                                         help_text='Companion influence levels')
    unlocked_locations = models.JSONField(default=list, blank=True,
                                        help_text='Unlocked world map locations')
    current_area = models.CharField(max_length=255, blank=True,
                                  help_text='Current area/module from currentmodule.txt')
    
    # Enhanced campaign overview data
    game_act = models.IntegerField(null=True, blank=True,
                                 help_text='Current game act (1, 2, 3, etc.)')
    difficulty_level = models.IntegerField(null=True, blank=True,
                                         help_text='Game difficulty level')
    last_saved_timestamp = models.BigIntegerField(null=True, blank=True,
                                                help_text='Unix timestamp of last save')
    
    # Comprehensive companion data (JSON fields for rich storage)
    companion_status = models.JSONField(default=dict, blank=True,
                                      help_text='Detailed companion status: influence, joined, met')
    hidden_statistics = models.JSONField(default=dict, blank=True,
                                       help_text='Hidden gameplay stats: dialogue choices, etc.')
    story_milestones = models.JSONField(default=dict, blank=True,
                                      help_text='Major story progression milestones')
    quest_details = models.JSONField(default=dict, blank=True,
                                   help_text='Enhanced quest information with categories and progress')
    
    # Cache for module-specific game rules
    _module_game_rules = None
    
    # Custom manager
    objects = CharacterManager()

    class Meta:
        ordering = ['-updated_at']
        indexes = [
            models.Index(fields=['owner', '-updated_at']),
        ]
        constraints = [
            # No constraints - let people play however they want!
        ]

    def __str__(self):
        name = f"{self.first_name} {self.last_name}".strip()
        return f"{name or 'Unnamed'} (Level {self.character_level})"
    
    def clean(self):
        """Model-level validation - permissive approach, no game rule enforcement"""
        super().clean()
        
        # Only sync data if the instance has been saved
        if self.pk:
            # Sync character level with class levels (but don't enforce rules)
            if hasattr(self, 'classes'):
                total_level = sum(c.class_level for c in self.classes.all())
                if total_level > 0 and total_level != self.character_level:
                    self.character_level = total_level  # Auto-fix level sync
    
    def get_ability_modifier(self, ability: str) -> int:
        """Get ability modifier (e.g., STR 16 = +3)"""
        ability_map = {
            'STR': 'strength', 'STRENGTH': 'strength',
            'DEX': 'dexterity', 'DEXTERITY': 'dexterity',
            'CON': 'constitution', 'CONSTITUTION': 'constitution',
            'INT': 'intelligence', 'INTELLIGENCE': 'intelligence',
            'WIS': 'wisdom', 'WISDOM': 'wisdom',
            'CHA': 'charisma', 'CHARISMA': 'charisma'
        }
        
        attr_name = ability_map.get(ability.upper(), ability.lower())
        value = getattr(self, attr_name, 10)
        return (value - 10) // 2
    
    def calculate_total_level(self) -> int:
        """Calculate total character level from all classes"""
        if self.pk and hasattr(self, 'classes'):
            total = sum(c.class_level for c in self.classes.all())
            if total > 0:
                return total
        return self.character_level
    
    def calculate_saves(self) -> dict:
        """Calculate all saves including base + ability + magic bonuses"""
        # Base saves from fields
        saves = {
            'fortitude': self.fortitude_save,
            'reflex': self.reflex_save,
            'will': self.will_save
        }
        
        # Add ability modifiers
        saves['fortitude'] += self.get_ability_modifier('CON')
        saves['reflex'] += self.get_ability_modifier('DEX')
        saves['will'] += self.get_ability_modifier('WIS')
        
        # Add any bonuses from fields
        saves['fortitude'] += self.fortbonus
        saves['reflex'] += self.refbonus
        saves['will'] += self.willbonus
        
        return saves
    
    def can_take_feat(self, feat_id: int) -> tuple[bool, list[str]]:
        """Check if character meets feat prerequisites"""
        rules = self.get_game_rules()
        errors = []
        
        feat_data = rules.feats.get(feat_id)
        if not feat_data:
            return False, [f"Unknown feat ID: {feat_id}"]
        
        # Check if already has the feat
        if self.feats.filter(feat_id=feat_id).exists():
            return False, ["Character already has this feat"]
        
        # Check prerequisites (this would need feat prerequisite data)
        # For now, return True as a placeholder
        return True, []
    
    def get_available_skill_points(self) -> int:
        """Calculate unspent skill points"""
        # Total skill points from skill_points field
        total_points = self.skill_points
        
        # Subtract spent points
        if hasattr(self, 'skills'):
            spent_points = sum(s.rank for s in self.skills.all())
            return total_points - spent_points
        
        return total_points
    
    def get_spell_slots(self, class_index: int = 0) -> dict:
        """Get available spell slots by level for a class"""
        # This would need to be implemented based on class spell progression
        # For now, return a placeholder
        slots = {}
        for level in range(10):
            slots[level] = 0
        return slots
    
    @property
    def alignment(self):
        """Get alignment as string"""
        if self.law_chaos >= 70:
            law = "Lawful"
        elif self.law_chaos <= 30:
            law = "Chaotic"
        else:
            law = "Neutral"
            
        if self.good_evil >= 70:
            good = "Good"
        elif self.good_evil <= 30:
            good = "Evil"
        else:
            good = "Neutral"
            
        if law == "Neutral" and good == "Neutral":
            return "True Neutral"
        return f"{law} {good}"
    
    def get_game_rules(self):
        """Get appropriate game rules for this character (base or module-specific)"""
        from gamedata.services.game_rules_service import GameRulesService
        from parsers.resource_manager import ResourceManager
        
        # If no custom content, return base game rules (cached)
        if not self.uses_custom_content:
            if not hasattr(self, '_base_game_rules'):
                self._base_game_rules = GameRulesService()
            return self._base_game_rules
        
        # Load module-specific rules (cached)
        if self._module_game_rules is None:
            # Create resource manager and set module context
            rm = ResourceManager(suppress_warnings=True)
            
            # Find and load the module
            if self.module_name:
                module_path = rm.find_module(self.module_name)
                if module_path:
                    rm.set_module(module_path)
            
            # Create game rules service with module-aware resource manager
            self._module_game_rules = GameRulesService(resource_manager=rm)
        
        return self._module_game_rules
    
    def get_primary_class_name(self):
        """Get the character's primary class name using appropriate rules"""
        rules = self.get_game_rules()
        
        # Find the highest level class
        highest_class = self.classes.order_by('-class_level').first()
        if highest_class:
            class_data = rules.classes.get(highest_class.class_id)
            if class_data:
                return class_data.name
        
        return 'Unknown'
    
    def validate_character_data(self):
        """Validate character using appropriate game rules"""
        rules = self.get_game_rules()
        errors = []
        
        # Validate classes exist
        for char_class in self.classes.all():
            if char_class.class_id not in rules.classes:
                errors.append(f"Unknown class ID: {char_class.class_id} ({char_class.class_name})")
        
        # Validate feats exist
        for feat in self.feats.all():
            if feat.feat_id not in rules.feats:
                errors.append(f"Unknown feat ID: {feat.feat_id} ({feat.feat_name})")
        
        # Validate race
        if self.race_id not in rules.races:
            errors.append(f"Unknown race ID: {self.race_id}")
        
        # Validate skills
        for skill in self.skills.all():
            if skill.skill_id not in rules.skills:
                errors.append(f"Unknown skill ID: {skill.skill_id} ({skill.skill_name})")
        
        # Validate spells
        for spell in self.spells.all():
            if spell.spell_id not in rules.spells:
                errors.append(f"Unknown spell ID: {spell.spell_id} ({spell.spell_name})")
        
        # Validate items
        for item in self.items.all():
            if item.base_item_id not in rules.base_items:
                errors.append(f"Unknown base item ID: {item.base_item_id} ({item.base_item_name})")
        
        return errors
    
    @classmethod
    def create_from_file(cls, file_path: str, owner=None):
        """
        Create character from save file, detecting module context
        This is a convenience method that uses CharacterImportService
        
        Args:
            file_path: Path to .bic/.ros file or save game directory
            owner: User object (optional)
            
        Returns:
            Character instance
        """
        from .services import CharacterImportService
        from parsers.resource_manager import ResourceManager
        
        # Create resource manager
        rm = ResourceManager(suppress_warnings=True)
        
        # Create import service with resource manager
        import_service = CharacterImportService(resource_manager=rm)
        
        # Import character (will handle module detection internally)
        if str(file_path).endswith('.zip') or os.path.isdir(str(file_path)):
            # Save game import
            return import_service.import_character(str(file_path), owner=owner, is_savegame=True)
        else:
            # Standard .bic/.ros import
            return import_service.import_character(str(file_path), owner=owner, is_savegame=False)
    
    def is_from_campaign(self):
        """Check if this character is from an official campaign"""
        return bool(self.campaign_name)
    
    def get_campaign_display_name(self):
        """Get a user-friendly campaign name"""
        campaign_map = {
            'Neverwinter Nights 2 Campaign': 'Original Campaign',
            'NWN2 Mask of the Betrayer Campaign': 'Mask of the Betrayer',
            'Neverwinter Nights 2 Campaign_X2': 'Storm of Zehir',
            'Neverwinter Nights 2 Campaign_X3': 'Mysteries of Westgate'
        }
        return campaign_map.get(self.campaign_name, self.campaign_name)
    
    def get_campaign_progress(self):
        """Get character's progress through the campaign"""
        if not self.campaign_modules or not self.module_name:
            return None
            
        try:
            current_index = self.campaign_modules.index(self.module_name)
            total_modules = len(self.campaign_modules)
            progress_percent = int((current_index + 1) / total_modules * 100)
            
            return {
                'current_module': self.module_name,
                'current_index': current_index + 1,
                'total_modules': total_modules,
                'progress_percent': progress_percent,
                'modules_completed': current_index,
                'modules_remaining': total_modules - current_index - 1
            }
        except ValueError:
            # Module not found in campaign list
            return None
    
    def export_to_gff(self, output_path: str = None):
        """Export character data back to GFF format"""
        from parsers.gff import GFFWriter, GFFElement, GFFFieldType
        
        # Create GFF structure
        root = GFFElement()
        root.struct_id = 0xFFFFFFFF  # Top level struct ID
        
        # Add basic fields
        root.add_field('FirstName', GFFFieldType.LOCSTRING, {
            'substrings': [{'string': self.first_name, 'language': 0, 'gender': 0}]
        })
        root.add_field('LastName', GFFFieldType.LOCSTRING, {
            'substrings': [{'string': self.last_name, 'language': 0, 'gender': 0}]
        })
        root.add_field('Race', GFFFieldType.BYTE, self.race_id)
        root.add_field('LawfulChaotic', GFFFieldType.BYTE, self.law_chaos)
        root.add_field('GoodEvil', GFFFieldType.BYTE, self.good_evil)
        root.add_field('Str', GFFFieldType.BYTE, self.strength)
        root.add_field('Dex', GFFFieldType.BYTE, self.dexterity)
        root.add_field('Con', GFFFieldType.BYTE, self.constitution)
        root.add_field('Int', GFFFieldType.BYTE, self.intelligence)
        root.add_field('Wis', GFFFieldType.BYTE, self.wisdom)
        root.add_field('Cha', GFFFieldType.BYTE, self.charisma)
        root.add_field('HitPoints', GFFFieldType.SHORT, self.hit_points)
        root.add_field('MaxHitPoints', GFFFieldType.SHORT, self.max_hit_points)
        root.add_field('Gold', GFFFieldType.INT, self.gold)
        root.add_field('Age', GFFFieldType.INT, self.age)
        root.add_field('Gender', GFFFieldType.BYTE, self.gender)
        
        # Add class list
        class_list = []
        for char_class in self.classes.all():
            class_struct = GFFElement()
            class_struct.add_field('Class', GFFFieldType.INT, char_class.class_id)
            class_struct.add_field('ClassLevel', GFFFieldType.SHORT, char_class.class_level)
            if char_class.domain1_id is not None:
                class_struct.add_field('Domain1', GFFFieldType.BYTE, char_class.domain1_id)
            if char_class.domain2_id is not None:
                class_struct.add_field('Domain2', GFFFieldType.BYTE, char_class.domain2_id)
            class_list.append(class_struct)
        root.add_field('ClassList', GFFFieldType.LIST, class_list)
        
        # Add feat list
        feat_list = []
        for feat in self.feats.all():
            feat_struct = GFFElement()
            feat_struct.add_field('Feat', GFFFieldType.WORD, feat.feat_id)
            feat_list.append(feat_struct)
        root.add_field('FeatList', GFFFieldType.LIST, feat_list)
        
        # Add skill list
        skill_list = []
        for skill in self.skills.all():
            skill_struct = GFFElement()
            skill_struct.add_field('Skill', GFFFieldType.BYTE, skill.skill_id)
            skill_struct.add_field('Rank', GFFFieldType.BYTE, skill.rank)
            skill_list.append(skill_struct)
        root.add_field('SkillList', GFFFieldType.LIST, skill_list)
        
        # Write to file
        if output_path:
            writer = GFFWriter()
            writer.write(output_path, root, 'BIC V3.2')
            return output_path
        
        return root  # Return GFF structure if no output path


class CharacterClass(models.Model):
    """Character class levels"""
    character = models.ForeignKey(Character, on_delete=models.CASCADE, related_name='classes')
    
    class_id = models.IntegerField()
    class_name = models.CharField(max_length=100)
    class_level = models.IntegerField(validators=[MinValueValidator(1), MaxValueValidator(60)])
    
    # For divine casters
    domain1_id = models.IntegerField(null=True, blank=True)
    domain1_name = models.CharField(max_length=100, blank=True)
    domain2_id = models.IntegerField(null=True, blank=True) 
    domain2_name = models.CharField(max_length=100, blank=True)
    
    class Meta:
        ordering = ['-class_level', 'class_name']
        verbose_name_plural = "Character classes"
    
    def __str__(self):
        return f"{self.class_name} {self.class_level}"
    
    def save(self, *args, **kwargs):
        """Auto-populate class name from appropriate game rules if not set"""
        if not self.class_name and self.character_id:
            rules = self.character.get_game_rules()
            class_data = rules.classes.get(self.class_id)
            if class_data:
                self.class_name = class_data.name
            else:
                self.class_name = f'Unknown_{self.class_id}'
        super().save(*args, **kwargs)


class CharacterFeat(models.Model):
    """Character feats"""
    character = models.ForeignKey(Character, on_delete=models.CASCADE, related_name='feats')
    
    feat_id = models.IntegerField()
    feat_name = models.CharField(max_length=200)
    
    class Meta:
        ordering = ['feat_name']
    
    def __str__(self):
        return self.feat_name


class CharacterSkill(models.Model):
    """Character skills and ranks"""
    character = models.ForeignKey(Character, on_delete=models.CASCADE, related_name='skills')
    
    skill_id = models.IntegerField()
    skill_name = models.CharField(max_length=100)
    rank = models.IntegerField(default=0, validators=[MinValueValidator(0), MaxValueValidator(50)])
    
    class Meta:
        ordering = ['skill_name']
    
    def __str__(self):
        return f"{self.skill_name}: {self.rank}"


class CharacterSpell(models.Model):
    """Known/memorized spells"""
    character = models.ForeignKey(Character, on_delete=models.CASCADE, related_name='spells')
    
    spell_id = models.IntegerField()
    spell_name = models.CharField(max_length=200)
    spell_level = models.IntegerField(validators=[MinValueValidator(0), MaxValueValidator(9)])
    class_index = models.IntegerField(default=0)  # Which class this spell belongs to
    is_memorized = models.BooleanField(default=False)
    
    class Meta:
        ordering = ['spell_level', 'spell_name']
    
    def __str__(self):
        return f"L{self.spell_level}: {self.spell_name}"


class CharacterItem(models.Model):
    """Items in inventory or equipped"""
    character = models.ForeignKey(Character, on_delete=models.CASCADE, related_name='items')
    
    # Basic item info
    base_item_id = models.IntegerField()
    base_item_name = models.CharField(max_length=200)
    localized_name = models.CharField(max_length=200, blank=True)
    stack_size = models.IntegerField(default=1, validators=[MinValueValidator(1)])
    
    # Location
    LOCATION_CHOICES = [
        ('INVENTORY', 'Inventory'),
        ('HEAD', 'Head'),
        ('CHEST', 'Chest'), 
        ('BOOTS', 'Boots'),
        ('ARMS', 'Arms'),
        ('RIGHT_HAND', 'Right Hand'),
        ('LEFT_HAND', 'Left Hand'),
        ('CLOAK', 'Cloak'),
        ('LEFT_RING', 'Left Ring'),
        ('RIGHT_RING', 'Right Ring'),
        ('NECK', 'Neck'),
        ('BELT', 'Belt'),
        ('ARROWS', 'Arrows'),
        ('BULLETS', 'Bullets'),
        ('BOLTS', 'Bolts'),
    ]
    location = models.CharField(max_length=20, choices=LOCATION_CHOICES, default='INVENTORY')
    inventory_slot = models.IntegerField(null=True, blank=True)  # Position in inventory
    
    # Item properties (stored as JSON for flexibility)
    properties = models.JSONField(default=list, blank=True)
    
    class Meta:
        ordering = ['location', 'inventory_slot']
    
    def __str__(self):
        name = self.localized_name or self.base_item_name
        if self.stack_size > 1:
            name += f" x{self.stack_size}"
        return name
    
    @property
    def display_name(self):
        """Get best available name for display"""
        return self.localized_name or self.base_item_name


# Signals for data integrity
@receiver(post_save, sender=CharacterClass)
def update_character_level(sender, instance, **kwargs):
    """Keep character level in sync with class levels"""
    character = instance.character
    total = character.classes.aggregate(Sum('class_level'))['class_level__sum'] or 0
    if character.character_level != total and total > 0:
        character.character_level = total
        character.save(update_fields=['character_level'])


@receiver(pre_save, sender=Character)
def validate_character_data_on_save(sender, instance, **kwargs):
    """Validate character data before saving"""
    # Only run clean if the instance has a pk (already saved)
    # This avoids issues with accessing related objects before initial save
    if instance.pk:
        instance.clean()
