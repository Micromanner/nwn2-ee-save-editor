"""
Auto-generated Django models for NWN2 game data
Generated from all 2DA files to create a complete game database
"""

from django.db import models
from django.core.validators import MinValueValidator, MaxValueValidator


class GameDataBase(models.Model):
    """Base model for all game data"""
    row_id = models.IntegerField(primary_key=True, help_text='2DA row index')
    label = models.CharField(max_length=64, db_index=True, help_text='2DA label')
    
    class Meta:
        abstract = True



class GameClass(GameDataBase):
    """GameClass from classes.2da"""
    name = models.IntegerField(default=0, help_text='Name')
    plural = models.IntegerField(default=0, help_text='Plural')
    lower = models.IntegerField(default=0, help_text='Lower')
    description = models.IntegerField(default=0, help_text='Description')
    icon = models.CharField(max_length=32, blank=True, default='', help_text='Icon')
    borderedicon = models.CharField(max_length=32, blank=True, default='', help_text='BorderedIcon')
    hit_die = models.IntegerField(default=0, help_text='HitDie')
    attack_bonus_table = models.CharField(max_length=32, blank=True, default='', help_text='AttackBonusTable')
    featstable = models.CharField(max_length=32, blank=True, default='', help_text='FeatsTable')
    saving_throw_table = models.CharField(max_length=32, blank=True, default='', help_text='SavingThrowTable')
    skillstable = models.CharField(max_length=32, blank=True, default='', help_text='SkillsTable')
    bonusfeatstable = models.CharField(max_length=32, blank=True, default='', help_text='BonusFeatsTable')
    skill_points = models.IntegerField(default=0, help_text='SkillPointBase')
    spellgaintable = models.CharField(max_length=32, blank=True, default='', help_text='SpellGainTable')
    spellknowntable = models.CharField(max_length=32, blank=True, default='', help_text='SpellKnownTable')
    is_player_class = models.BooleanField(default=False, help_text='PlayerClass')
    is_spellcaster = models.IntegerField(default=0, help_text='SpellCaster')
    metamagicallowed = models.IntegerField(default=0, help_text='MetaMagicAllowed')
    memorizesspells = models.IntegerField(default=0, help_text='MemorizesSpells')
    hasarcane = models.BooleanField(default=False, help_text='HasArcane')
    hasdivine = models.BooleanField(default=False, help_text='HasDivine')
    hasspontaneousspells = models.BooleanField(default=False, help_text='HasSpontaneousSpells')
    spontaneousconversiontable = models.CharField(max_length=32, blank=True, default='', help_text='SpontaneousConversionTable')
    spellswapminlvl = models.IntegerField(default=0, help_text='SpellSwapMinLvl')
    spellswaplvlinterval = models.IntegerField(default=0, help_text='SpellSwapLvlInterval')
    spellswaplvldiff = models.IntegerField(default=0, help_text='SpellSwapLvlDiff')
    allspellsknown = models.IntegerField(default=0, help_text='AllSpellsKnown')
    hasinfinitespells = models.BooleanField(default=False, help_text='HasInfiniteSpells')
    hasdomains = models.BooleanField(default=False, help_text='HasDomains')
    hasschool = models.BooleanField(default=False, help_text='HasSchool')
    hasfamiliar = models.BooleanField(default=False, help_text='HasFamiliar')
    hasanimalcompanion = models.BooleanField(default=False, help_text='HasAnimalCompanion')
    str = models.IntegerField(default=0, help_text='Str')
    dex = models.IntegerField(default=0, help_text='Dex')
    con = models.IntegerField(default=0, help_text='Con')
    wis = models.IntegerField(default=0, help_text='Wis')
    int_value = models.IntegerField(default=0, help_text='Int', db_column='int')
    cha = models.IntegerField(default=0, help_text='Cha')
    primary_ability = models.CharField(max_length=32, blank=True, default='', help_text='PrimaryAbil')
    spell_ability = models.CharField(max_length=32, blank=True, default='', help_text='SpellAbil')
    alignment_restrict = models.CharField(max_length=32, blank=True, default='', help_text='AlignRestrict')
    alignrstrcttype = models.CharField(max_length=32, blank=True, default='', help_text='AlignRstrctType')
    invertrestrict = models.IntegerField(default=0, help_text='InvertRestrict')
    constant = models.CharField(max_length=40, blank=True, default='', help_text='Constant')
    effcrlvl01 = models.IntegerField(default=0, help_text='EffCRLvl01')
    effcrlvl02 = models.IntegerField(default=0, help_text='EffCRLvl02')
    effcrlvl03 = models.IntegerField(default=0, help_text='EffCRLvl03')
    effcrlvl04 = models.IntegerField(default=0, help_text='EffCRLvl04')
    effcrlvl05 = models.IntegerField(default=0, help_text='EffCRLvl05')
    effcrlvl06 = models.IntegerField(default=0, help_text='EffCRLvl06')
    effcrlvl07 = models.IntegerField(default=0, help_text='EffCRLvl07')
    effcrlvl08 = models.IntegerField(default=0, help_text='EffCRLvl08')
    effcrlvl09 = models.IntegerField(default=0, help_text='EffCRLvl09')
    effcrlvl10 = models.IntegerField(default=0, help_text='EffCRLvl10')
    effcrlvl11 = models.IntegerField(default=0, help_text='EffCRLvl11')
    effcrlvl12 = models.IntegerField(default=0, help_text='EffCRLvl12')
    effcrlvl13 = models.IntegerField(default=0, help_text='EffCRLvl13')
    effcrlvl14 = models.IntegerField(default=0, help_text='EffCRLvl14')
    effcrlvl15 = models.IntegerField(default=0, help_text='EffCRLvl15')
    effcrlvl16 = models.IntegerField(default=0, help_text='EffCRLvl16')
    effcrlvl17 = models.IntegerField(default=0, help_text='EffCRLvl17')
    effcrlvl18 = models.IntegerField(default=0, help_text='EffCRLvl18')
    effcrlvl19 = models.IntegerField(default=0, help_text='EffCRLvl19')
    effcrlvl20 = models.IntegerField(default=0, help_text='EffCRLvl20')
    prereqtable = models.CharField(max_length=64, blank=True, default='', help_text='PreReqTable')
    max_level = models.IntegerField(default=0, help_text='MaxLevel')
    xppenalty = models.IntegerField(default=0, help_text='XPPenalty')
    bonusspellcasterleveltable = models.CharField(max_length=64, blank=True, default='', help_text='BonusSpellcasterLevelTable')
    bonuscasterfeatbyclassmap = models.CharField(max_length=64, blank=True, default='', help_text='BonusCasterFeatByClassMap')
    arcspelllvlmod = models.IntegerField(default=0, help_text='ArcSpellLvlMod')
    divspelllvlmod = models.IntegerField(default=0, help_text='DivSpellLvlMod')
    epiclevel = models.BooleanField(default=False, help_text='EpicLevel')
    package = models.IntegerField(default=0, help_text='Package')
    featpracticedspellcaster = models.IntegerField(default=0, help_text='FEATPracticedSpellcaster')
    featextraslot = models.IntegerField(default=0, help_text='FEATExtraSlot')
    featarmoredcaster = models.IntegerField(default=0, help_text='FEATArmoredCaster')
    favoredweaponproficiency = models.IntegerField(default=0, help_text='FavoredWeaponProficiency')
    favoredweaponfocus = models.IntegerField(default=0, help_text='FavoredWeaponFocus')
    favoredweaponspecialization = models.IntegerField(default=0, help_text='FavoredWeaponSpecialization')
    chargen_chest = models.CharField(max_length=32, blank=True, default='', help_text='CharGen_Chest')
    chargen_feet = models.CharField(max_length=64, blank=True, default='', help_text='CharGen_Feet')
    chargen_hands = models.CharField(max_length=64, blank=True, default='', help_text='CharGen_Hands')
    chargen_cloak = models.CharField(max_length=64, blank=True, default='', help_text='CharGen_Cloak')
    chargen_head = models.CharField(max_length=64, blank=True, default='', help_text='CharGen_Head')
    
    class Meta:
        db_table = 'gamedata_classes'
        ordering = ['row_id']
        verbose_name = 'GameClass'
        verbose_name_plural = 'GameClasss'


class Race(GameDataBase):
    """Race from racialtypes.2da"""
    abrev = models.CharField(max_length=32, blank=True, default='', help_text='Abrev')
    name = models.IntegerField(default=0, help_text='Name')
    nameplural = models.IntegerField(default=0, help_text='NamePlural')
    namelower = models.IntegerField(default=0, help_text='NameLower')
    namelowerplural = models.IntegerField(default=0, help_text='NameLowerPlural')
    convername = models.IntegerField(default=0, help_text='ConverName')
    convernamelower = models.IntegerField(default=0, help_text='ConverNameLower')
    description = models.IntegerField(default=0, help_text='Description')
    appearance = models.CharField(max_length=64, blank=True, default='', help_text='Appearance')
    str_adjust = models.IntegerField(default=0, help_text='StrAdjust')
    dex_adjust = models.IntegerField(default=0, help_text='DexAdjust')
    int_adjust = models.IntegerField(default=0, help_text='IntAdjust')
    cha_adjust = models.IntegerField(default=0, help_text='ChaAdjust')
    wis_adjust = models.IntegerField(default=0, help_text='WisAdjust')
    con_adjust = models.IntegerField(default=0, help_text='ConAdjust')
    endurance = models.IntegerField(default=0, help_text='Endurance')
    favored = models.IntegerField(default=0, help_text='Favored')
    featstable = models.CharField(max_length=32, blank=True, default='', help_text='FeatsTable')
    biography = models.IntegerField(default=0, help_text='Biography')
    playerrace = models.IntegerField(default=0, help_text='PlayerRace')
    constant = models.CharField(max_length=34, blank=True, default='', help_text='Constant')
    age = models.IntegerField(default=0, help_text='AGE')
    crmodifier = models.FloatField(default=0.0, help_text='CRModifier')
    ishumanoid = models.BooleanField(default=False, help_text='IsHumanoid')
    defaultsubrace = models.IntegerField(default=0, help_text='DefaultSubRace')
    female_race_icon = models.CharField(max_length=32, blank=True, default='', help_text='female_race_icon')
    male_race_icon = models.CharField(max_length=32, blank=True, default='', help_text='male_race_icon')
    featfavoredenemy = models.IntegerField(default=0, help_text='FEATFavoredEnemy')
    featimprovedfavoredenemy = models.IntegerField(default=0, help_text='FEATImprovedFavoredEnemy')
    featfavoredpowerattack = models.IntegerField(default=0, help_text='FEATFavoredPowerAttack')
    featignorecritimmunity = models.IntegerField(default=0, help_text='FEATIgnoreCritImmunity')
    
    class Meta:
        db_table = 'gamedata_racialtypes'
        ordering = ['row_id']
        verbose_name = 'Race'
        verbose_name_plural = 'Races'


class Subrace(GameDataBase):
    """Subrace from racialsubtypes.2da"""
    baserace = models.IntegerField(default=0, help_text='BaseRace')
    ecl = models.IntegerField(default=0, help_text='ECL')
    abrev = models.CharField(max_length=32, blank=True, default='', help_text='Abrev')
    name = models.IntegerField(default=0, help_text='Name')
    nameplural = models.IntegerField(default=0, help_text='NamePlural')
    namelower = models.IntegerField(default=0, help_text='NameLower')
    namelowerplural = models.IntegerField(default=0, help_text='NameLowerPlural')
    convername = models.IntegerField(default=0, help_text='ConverName')
    convernamelower = models.IntegerField(default=0, help_text='ConverNameLower')
    description = models.IntegerField(default=0, help_text='Description')
    appearance = models.CharField(max_length=64, blank=True, default='', help_text='Appearance')
    str_adjust = models.IntegerField(default=0, help_text='StrAdjust')
    dex_adjust = models.IntegerField(default=0, help_text='DexAdjust')
    int_adjust = models.IntegerField(default=0, help_text='IntAdjust')
    cha_adjust = models.IntegerField(default=0, help_text='ChaAdjust')
    wis_adjust = models.IntegerField(default=0, help_text='WisAdjust')
    con_adjust = models.IntegerField(default=0, help_text='ConAdjust')
    endurance = models.IntegerField(default=0, help_text='Endurance')
    favored = models.IntegerField(default=0, help_text='Favored')
    hasfavoredclass = models.BooleanField(default=False, help_text='HasFavoredClass')
    featstable = models.CharField(max_length=32, blank=True, default='', help_text='FeatsTable')
    biography = models.IntegerField(default=0, help_text='Biography')
    playerrace = models.IntegerField(default=0, help_text='PlayerRace')
    constant = models.CharField(max_length=54, blank=True, default='', help_text='Constant')
    age = models.IntegerField(default=0, help_text='AGE')
    crmodifier = models.IntegerField(default=0, help_text='CRModifier')
    color2da = models.CharField(max_length=34, blank=True, default='', help_text='Color2DA')
    appearanceindex = models.IntegerField(default=0, help_text='AppearanceIndex')
    female_race_icon = models.CharField(max_length=32, blank=True, default='', help_text='female_race_icon')
    male_race_icon = models.CharField(max_length=32, blank=True, default='', help_text='male_race_icon')
    racial_banner = models.CharField(max_length=32, blank=True, default='', help_text='racial_banner')
    
    class Meta:
        db_table = 'gamedata_racialsubtypes'
        ordering = ['row_id']
        verbose_name = 'Subrace'
        verbose_name_plural = 'Subraces'


class Feat(GameDataBase):
    """Feat from feat.2da"""
    feat = models.IntegerField(default=0, help_text='FEAT')
    description = models.IntegerField(default=0, help_text='DESCRIPTION')
    icon = models.CharField(max_length=32, blank=True, default='', help_text='ICON')
    minattackbonus = models.IntegerField(default=0, help_text='MINATTACKBONUS')
    minstr = models.IntegerField(default=0, help_text='MINSTR')
    mindex = models.IntegerField(default=0, help_text='MINDEX')
    minint = models.IntegerField(default=0, help_text='MININT')
    minwis = models.CharField(max_length=64, blank=True, default='', help_text='MINWIS')
    mincon = models.CharField(max_length=64, blank=True, default='', help_text='MINCON')
    mincha = models.CharField(max_length=64, blank=True, default='', help_text='MINCHA')
    maxstr = models.CharField(max_length=64, blank=True, default='', help_text='MAXSTR')
    maxdex = models.CharField(max_length=64, blank=True, default='', help_text='MAXDEX')
    maxint = models.CharField(max_length=64, blank=True, default='', help_text='MAXINT')
    maxwis = models.CharField(max_length=64, blank=True, default='', help_text='MAXWIS')
    maxcon = models.CharField(max_length=64, blank=True, default='', help_text='MAXCON')
    maxcha = models.CharField(max_length=64, blank=True, default='', help_text='MAXCHA')
    minspelllvl = models.IntegerField(default=0, help_text='MINSPELLLVL')
    mincasterlvl = models.CharField(max_length=64, blank=True, default='', help_text='MINCASTERLVL')
    prereq_feat1 = models.IntegerField(default=0, help_text='PREREQFEAT1')
    prereq_feat2 = models.IntegerField(default=0, help_text='PREREQFEAT2')
    gainmultiple = models.IntegerField(default=0, help_text='GAINMULTIPLE')
    effectsstack = models.IntegerField(default=0, help_text='EFFECTSSTACK')
    allclassescanuse = models.BooleanField(default=False, help_text='ALLCLASSESCANUSE')
    category = models.IntegerField(default=0, help_text='CATEGORY')
    maxcr = models.IntegerField(default=0, help_text='MAXCR')
    spellid = models.CharField(max_length=64, blank=True, default='', help_text='SPELLID')
    successor = models.IntegerField(default=0, help_text='SUCCESSOR')
    crvalue = models.IntegerField(default=0, help_text='CRValue')
    usesperday = models.CharField(max_length=64, blank=True, default='', help_text='USESPERDAY')
    usesmapfeat = models.CharField(max_length=64, blank=True, default='', help_text='USESMAPFEAT')
    masterfeat = models.CharField(max_length=64, blank=True, default='', help_text='MASTERFEAT')
    targetself = models.CharField(max_length=64, blank=True, default='', help_text='TARGETSELF')
    orreqfeat0 = models.CharField(max_length=64, blank=True, default='', help_text='OrReqFeat0')
    orreqfeat1 = models.CharField(max_length=64, blank=True, default='', help_text='OrReqFeat1')
    orreqfeat2 = models.CharField(max_length=64, blank=True, default='', help_text='OrReqFeat2')
    orreqfeat3 = models.CharField(max_length=64, blank=True, default='', help_text='OrReqFeat3')
    orreqfeat4 = models.CharField(max_length=64, blank=True, default='', help_text='OrReqFeat4')
    orreqfeat5 = models.CharField(max_length=64, blank=True, default='', help_text='OrReqFeat5')
    reqskill = models.CharField(max_length=64, blank=True, default='', help_text='REQSKILL')
    reqskillmaxranks = models.CharField(max_length=64, blank=True, default='', help_text='ReqSkillMaxRanks')
    reqskillminranks = models.CharField(max_length=64, blank=True, default='', help_text='ReqSkillMinRanks')
    reqskill2 = models.CharField(max_length=64, blank=True, default='', help_text='REQSKILL2')
    reqskillmaxranks2 = models.CharField(max_length=64, blank=True, default='', help_text='ReqSkillMaxRanks2')
    reqskillminranks2 = models.CharField(max_length=64, blank=True, default='', help_text='ReqSkillMinRanks2')
    constant = models.CharField(max_length=56, blank=True, default='', help_text='Constant')
    toolscategories = models.IntegerField(default=0, help_text='TOOLSCATEGORIES')
    hostilefeat = models.IntegerField(default=0, help_text='HostileFeat')
    min_level = models.CharField(max_length=64, blank=True, default='', help_text='MinLevel')
    minlevelclass = models.CharField(max_length=64, blank=True, default='', help_text='MinLevelClass')
    max_level = models.CharField(max_length=64, blank=True, default='', help_text='MaxLevel')
    minfortsave = models.CharField(max_length=64, blank=True, default='', help_text='MinFortSave')
    prereqepic = models.BooleanField(default=False, help_text='PreReqEpic')
    featcategory = models.CharField(max_length=36, blank=True, default='', help_text='FeatCategory')
    isactive = models.BooleanField(default=False, help_text='IsActive')
    ispersistent = models.BooleanField(default=False, help_text='IsPersistent')
    togglemode = models.CharField(max_length=64, blank=True, default='', help_text='ToggleMode')
    cooldown = models.CharField(max_length=64, blank=True, default='', help_text='Cooldown')
    dmfeat = models.IntegerField(default=0, help_text='DMFeat')
    removed = models.IntegerField(default=0, help_text='REMOVED')
    alignment_restrict = models.CharField(max_length=64, blank=True, default='', help_text='AlignRestrict')
    immunitytype = models.CharField(max_length=64, blank=True, default='', help_text='ImmunityType')
    instant = models.CharField(max_length=64, blank=True, default='', help_text='Instant')
    
    class Meta:
        db_table = 'gamedata_feat'
        ordering = ['row_id']
        verbose_name = 'Feat'
        verbose_name_plural = 'Feats'


class Skill(GameDataBase):
    """Skill from skills.2da"""
    name = models.CharField(max_length=32, blank=True, default='', help_text='Name')
    description = models.CharField(max_length=32, blank=True, default='', help_text='Description')
    icon = models.CharField(max_length=32, blank=True, default='', help_text='Icon')
    untrained = models.BooleanField(default=False, help_text='Untrained')
    key_ability = models.CharField(max_length=32, blank=True, default='', help_text='KeyAbility')
    armor_check_penalty = models.IntegerField(default=0, help_text='ArmorCheckPenalty')
    allclassescanuse = models.BooleanField(default=False, help_text='AllClassesCanUse')
    category = models.CharField(max_length=64, blank=True, default='', help_text='Category')
    maxcr = models.IntegerField(default=0, help_text='MaxCR')
    constant = models.CharField(max_length=40, blank=True, default='', help_text='Constant')
    hostileskill = models.IntegerField(default=0, help_text='HostileSkill')
    cosmopolitanfeat = models.IntegerField(default=0, help_text='CosmopolitanFeat')
    isanactiveskill = models.BooleanField(default=False, help_text='IsAnActiveSkill')
    togglemode = models.IntegerField(default=0, help_text='ToggleMode')
    playeronly = models.IntegerField(default=0, help_text='PlayerOnly')
    removed = models.IntegerField(default=0, help_text='REMOVED')
    
    class Meta:
        db_table = 'gamedata_skills'
        ordering = ['row_id']
        verbose_name = 'Skill'
        verbose_name_plural = 'Skills'


class Spell(GameDataBase):
    """Spell from spells.2da"""
    name = models.IntegerField(default=0, help_text='Name')
    iconresref = models.CharField(max_length=32, blank=True, default='', help_text='IconResRef')
    school = models.CharField(max_length=32, blank=True, default='', help_text='School')
    range = models.CharField(max_length=32, blank=True, default='', help_text='Range')
    vs = models.CharField(max_length=32, blank=True, default='', help_text='VS')
    metamagic = models.CharField(max_length=32, blank=True, default='', help_text='MetaMagic')
    targettype = models.CharField(max_length=32, blank=True, default='', help_text='TargetType')
    impactscript = models.CharField(max_length=32, blank=True, default='', help_text='ImpactScript')
    bard = models.IntegerField(default=0, help_text='Bard')
    cleric = models.IntegerField(default=0, help_text='Cleric')
    druid = models.IntegerField(default=0, help_text='Druid')
    paladin = models.IntegerField(default=0, help_text='Paladin')
    ranger = models.IntegerField(default=0, help_text='Ranger')
    wiz_sorc = models.IntegerField(default=0, help_text='Wiz_Sorc')
    warlock = models.CharField(max_length=64, blank=True, default='', help_text='Warlock')
    innate = models.IntegerField(default=0, help_text='Innate')
    conjtime = models.IntegerField(default=0, help_text='ConjTime')
    conjanim = models.CharField(max_length=32, blank=True, default='', help_text='ConjAnim')
    conjvisual0 = models.CharField(max_length=52, blank=True, default='', help_text='ConjVisual0')
    lowconjvisual0 = models.CharField(max_length=40, blank=True, default='', help_text='LowConjVisual0')
    conjvisual1 = models.CharField(max_length=64, blank=True, default='', help_text='ConjVisual1')
    conjvisual2 = models.CharField(max_length=64, blank=True, default='', help_text='ConjVisual2')
    conjsoundvfx = models.CharField(max_length=64, blank=True, default='', help_text='ConjSoundVFX')
    conjsoundmale = models.CharField(max_length=32, blank=True, default='', help_text='ConjSoundMale')
    conjsoundfemale = models.CharField(max_length=32, blank=True, default='', help_text='ConjSoundFemale')
    conjsoundoverride = models.CharField(max_length=64, blank=True, default='', help_text='ConjSoundOverride')
    castanim = models.CharField(max_length=32, blank=True, default='', help_text='CastAnim')
    cast_time = models.IntegerField(default=0, help_text='CastTime')
    castvisual0 = models.CharField(max_length=46, blank=True, default='', help_text='CastVisual0')
    lowcastvisual0 = models.CharField(max_length=46, blank=True, default='', help_text='LowCastVisual0')
    castvisual1 = models.CharField(max_length=64, blank=True, default='', help_text='CastVisual1')
    castvisual2 = models.CharField(max_length=64, blank=True, default='', help_text='CastVisual2')
    castsound = models.CharField(max_length=64, blank=True, default='', help_text='CastSound')
    proj = models.IntegerField(default=0, help_text='Proj')
    projmodel = models.CharField(max_length=64, blank=True, default='', help_text='ProjModel')
    projsef = models.CharField(max_length=48, blank=True, default='', help_text='ProjSEF')
    lowprojsef = models.CharField(max_length=48, blank=True, default='', help_text='LowProjSEF')
    projtype = models.CharField(max_length=32, blank=True, default='', help_text='ProjType')
    projspwnpoint = models.CharField(max_length=32, blank=True, default='', help_text='ProjSpwnPoint')
    projsound = models.CharField(max_length=64, blank=True, default='', help_text='ProjSound')
    projorientation = models.CharField(max_length=32, blank=True, default='', help_text='ProjOrientation')
    impactsef = models.CharField(max_length=44, blank=True, default='', help_text='ImpactSEF')
    lowimpactsef = models.CharField(max_length=44, blank=True, default='', help_text='LowImpactSEF')
    immunitytype = models.CharField(max_length=32, blank=True, default='', help_text='ImmunityType')
    itemimmunity = models.IntegerField(default=0, help_text='ItemImmunity')
    subradspell1 = models.IntegerField(default=0, help_text='SubRadSpell1')
    subradspell2 = models.IntegerField(default=0, help_text='SubRadSpell2')
    subradspell3 = models.CharField(max_length=64, blank=True, default='', help_text='SubRadSpell3')
    subradspell4 = models.CharField(max_length=64, blank=True, default='', help_text='SubRadSpell4')
    subradspell5 = models.CharField(max_length=64, blank=True, default='', help_text='SubRadSpell5')
    category = models.IntegerField(default=0, help_text='Category')
    master = models.CharField(max_length=64, blank=True, default='', help_text='Master')
    usertype = models.IntegerField(default=0, help_text='UserType')
    spelldesc = models.IntegerField(default=0, help_text='SpellDesc')
    useconcentration = models.IntegerField(default=0, help_text='UseConcentration')
    spontaneouslycast = models.IntegerField(default=0, help_text='SpontaneouslyCast')
    spontcastclassreq = models.IntegerField(default=0, help_text='SpontCastClassReq')
    altmessage = models.CharField(max_length=64, blank=True, default='', help_text='AltMessage')
    hostilesetting = models.IntegerField(default=0, help_text='HostileSetting')
    featid = models.CharField(max_length=64, blank=True, default='', help_text='FeatID')
    counter1 = models.IntegerField(default=0, help_text='Counter1')
    counter2 = models.CharField(max_length=64, blank=True, default='', help_text='Counter2')
    hasprojectile = models.BooleanField(default=False, help_text='HasProjectile')
    asmetamagic = models.CharField(max_length=64, blank=True, default='', help_text='AsMetaMagic')
    targetingui = models.IntegerField(default=0, help_text='TargetingUI')
    castableondead = models.IntegerField(default=0, help_text='CastableOnDead')
    removed = models.IntegerField(default=0, help_text='REMOVED')
    
    class Meta:
        db_table = 'gamedata_spells'
        ordering = ['row_id']
        verbose_name = 'Spell'
        verbose_name_plural = 'Spells'


class BaseItem(GameDataBase):
    """BaseItem from baseitems.2da"""
    name = models.IntegerField(default=0, help_text='Name')
    invslotwidth = models.IntegerField(default=0, help_text='InvSlotWidth')
    invslotheight = models.IntegerField(default=0, help_text='InvSlotHeight')
    equipableslots = models.CharField(max_length=32, blank=True, default='', help_text='EquipableSlots')
    canrotateicon = models.BooleanField(default=False, help_text='CanRotateIcon')
    modeltype = models.IntegerField(default=0, help_text='ModelType')
    nwn2_anim = models.IntegerField(default=0, help_text='NWN2_Anim')
    itemclass = models.CharField(max_length=32, blank=True, default='', help_text='ItemClass')
    genderspecific = models.IntegerField(default=0, help_text='GenderSpecific')
    part1envmap = models.IntegerField(default=0, help_text='Part1EnvMap')
    part2envmap = models.IntegerField(default=0, help_text='Part2EnvMap')
    part3envmap = models.IntegerField(default=0, help_text='Part3EnvMap')
    defaultmodel = models.CharField(max_length=32, blank=True, default='', help_text='DefaultModel')
    nwn2_defaulticon = models.IntegerField(default=0, help_text='NWN2_DefaultIcon')
    defaulticon = models.CharField(max_length=64, blank=True, default='', help_text='DefaultIcon')
    container = models.IntegerField(default=0, help_text='Container')
    weaponwield = models.IntegerField(default=0, help_text='WeaponWield')
    weapontype = models.IntegerField(default=0, help_text='WeaponType')
    weaponsize = models.IntegerField(default=0, help_text='WeaponSize')
    rangedweapon = models.IntegerField(default=0, help_text='RangedWeapon')
    prefattackdist = models.FloatField(default=0.0, help_text='PrefAttackDist')
    minrange = models.IntegerField(default=0, help_text='MinRange')
    maxrange = models.IntegerField(default=0, help_text='MaxRange')
    numdice = models.IntegerField(default=0, help_text='NumDice')
    dietoroll = models.IntegerField(default=0, help_text='DieToRoll')
    critthreat = models.IntegerField(default=0, help_text='CritThreat')
    crithitmult = models.IntegerField(default=0, help_text='CritHitMult')
    category = models.IntegerField(default=0, help_text='Category')
    basecost = models.IntegerField(default=0, help_text='BaseCost')
    stacking = models.IntegerField(default=0, help_text='Stacking')
    itemmultiplier = models.IntegerField(default=0, help_text='ItemMultiplier')
    description = models.IntegerField(default=0, help_text='Description')
    invsoundtype = models.IntegerField(default=0, help_text='InvSoundType')
    maxprops = models.IntegerField(default=0, help_text='MaxProps')
    minprops = models.IntegerField(default=0, help_text='MinProps')
    propcolumn = models.IntegerField(default=0, help_text='PropColumn')
    storepanel = models.IntegerField(default=0, help_text='StorePanel')
    reqfeat0 = models.IntegerField(default=0, help_text='ReqFeat0')
    reqfeat1 = models.IntegerField(default=0, help_text='ReqFeat1')
    reqfeat2 = models.IntegerField(default=0, help_text='ReqFeat2')
    reqfeat3 = models.IntegerField(default=0, help_text='ReqFeat3')
    reqfeat4 = models.IntegerField(default=0, help_text='ReqFeat4')
    reqfeat5 = models.CharField(max_length=64, blank=True, default='', help_text='ReqFeat5')
    ac_enchant = models.IntegerField(default=0, help_text='AC_Enchant')
    base_ac = models.IntegerField(default=0, help_text='BaseAC')
    armorcheckpen = models.IntegerField(default=0, help_text='ArmorCheckPen')
    baseitemstatref = models.IntegerField(default=0, help_text='BaseItemStatRef')
    chargesstarting = models.IntegerField(default=0, help_text='ChargesStarting')
    rotateonground = models.IntegerField(default=0, help_text='RotateOnGround')
    tenthlbs = models.IntegerField(default=0, help_text='TenthLBS')
    weaponmattype = models.IntegerField(default=0, help_text='WeaponMatType')
    ammunitiontype = models.IntegerField(default=0, help_text='AmmunitionType')
    qbbehaviour = models.CharField(max_length=64, blank=True, default='', help_text='QBBehaviour')
    arcanespellfailure = models.CharField(max_length=64, blank=True, default='', help_text='ArcaneSpellFailure')
    percent_animslashl = models.IntegerField(default=0, help_text='%AnimSlashL')
    percent_animslashr = models.IntegerField(default=0, help_text='%AnimSlashR')
    percent_animslashs = models.IntegerField(default=0, help_text='%AnimSlashS')
    storepanelsort = models.IntegerField(default=0, help_text='StorePanelSort')
    ilrstacksize = models.IntegerField(default=0, help_text='ILRStackSize')
    featimprcrit = models.IntegerField(default=0, help_text='FEATImprCrit')
    featwpnfocus = models.IntegerField(default=0, help_text='FEATWpnFocus')
    featwpnspec = models.IntegerField(default=0, help_text='FEATWpnSpec')
    featepicdevcrit = models.BooleanField(default=False, help_text='FEATEpicDevCrit')
    featepicwpnfocus = models.BooleanField(default=False, help_text='FEATEpicWpnFocus')
    featepicwpnspec = models.BooleanField(default=False, help_text='FEATEpicWpnSpec')
    featoverwhcrit = models.IntegerField(default=0, help_text='FEATOverWhCrit')
    featwpnofchoice = models.IntegerField(default=0, help_text='FEATWpnOfChoice')
    featgrtrwpnfocus = models.IntegerField(default=0, help_text='FEATGrtrWpnFocus')
    featgrtrwpnspec = models.IntegerField(default=0, help_text='FEATGrtrWpnSpec')
    featpowercrit = models.IntegerField(default=0, help_text='FEATPowerCrit')
    gmaterialtype = models.IntegerField(default=0, help_text='GMaterialType')
    baseitemsortorder = models.IntegerField(default=0, help_text='BaseItemSortOrder')
    
    class Meta:
        db_table = 'gamedata_baseitems'
        ordering = ['row_id']
        verbose_name = 'BaseItem'
        verbose_name_plural = 'BaseItems'


class Domain(GameDataBase):
    """Domain from domains.2da"""
    name = models.IntegerField(default=0, help_text='Name')
    description = models.IntegerField(default=0, help_text='Description')
    icon = models.CharField(max_length=32, blank=True, default='', help_text='Icon')
    level_1 = models.IntegerField(default=0, help_text='Level_1')
    level_2 = models.IntegerField(default=0, help_text='Level_2')
    level_3 = models.IntegerField(default=0, help_text='Level_3')
    level_4 = models.IntegerField(default=0, help_text='Level_4')
    level_5 = models.IntegerField(default=0, help_text='Level_5')
    level_6 = models.IntegerField(default=0, help_text='Level_6')
    level_7 = models.CharField(max_length=64, blank=True, default='', help_text='Level_7')
    level_8 = models.CharField(max_length=64, blank=True, default='', help_text='Level_8')
    level_9 = models.IntegerField(default=0, help_text='Level_9')
    grantedfeat = models.IntegerField(default=0, help_text='GrantedFeat')
    castablefeat = models.IntegerField(default=0, help_text='CastableFeat')
    epithetfeat = models.IntegerField(default=0, help_text='EpithetFeat')
    
    class Meta:
        db_table = 'gamedata_domains'
        ordering = ['row_id']
        verbose_name = 'Domain'
        verbose_name_plural = 'Domains'


class SpellSchool(GameDataBase):
    """SpellSchool from spellschools.2da"""
    letter = models.CharField(max_length=32, blank=True, default='', help_text='Letter')
    stringref = models.CharField(max_length=32, blank=True, default='', help_text='StringRef')
    icon = models.IntegerField(default=0, help_text='Icon')
    opposition = models.CharField(max_length=32, blank=True, default='', help_text='Opposition')
    description = models.IntegerField(default=0, help_text='Description')
    secondopposition = models.IntegerField(default=0, help_text='SecondOpposition')
    
    class Meta:
        db_table = 'gamedata_spellschools'
        ordering = ['row_id']
        verbose_name = 'SpellSchool'
        verbose_name_plural = 'SpellSchools'


class Appearance(GameDataBase):
    """Appearance from appearance.2da"""
    string_ref = models.CharField(max_length=32, blank=True, default='', help_text='STRING_REF')
    bodytype = models.IntegerField(default=0, help_text='BodyType')
    segments = models.IntegerField(default=0, help_text='Segments')
    nwn2_scale_x = models.IntegerField(default=0, help_text='NWN2_Scale_X')
    nwn2_scale_y = models.FloatField(default=0.0, help_text='NWN2_Scale_Y')
    nwn2_scale_z = models.FloatField(default=0.0, help_text='NWN2_Scale_Z')
    animationspeed = models.FloatField(default=0.0, help_text='AnimationSpeed')
    nwn2_model_body = models.IntegerField(default=0, help_text='NWN2_Model_Body')
    nwn2_model_helm = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_Model_Helm')
    nwn2_model_head = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_Model_Head')
    nwn2_model_hair = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_Model_Hair')
    nwn2_head_skeleton = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_Head_Skeleton')
    nwn2_skeleton_file = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_Skeleton_File')
    nwn2_accessorysize = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_AccessorySize')
    nwn2_accessorytype = models.IntegerField(default=0, help_text='NWN2_AccessoryType')
    toolsetusestubmodel = models.CharField(max_length=32, blank=True, default='', help_text='ToolsetUseStubModel')
    mount = models.IntegerField(default=0, help_text='Mount')
    name = models.IntegerField(default=0, help_text='NAME')
    race = models.CharField(max_length=32, blank=True, default='', help_text='RACE')
    envmap = models.CharField(max_length=32, blank=True, default='', help_text='ENVMAP')
    nwn2_bloodtype = models.CharField(max_length=32, blank=True, default='', help_text='NWN2_BLOODTYPE')
    modeltype = models.IntegerField(default=0, help_text='MODELTYPE')
    weaponvisualscale = models.CharField(max_length=32, blank=True, default='', help_text='WEAPONVISUALSCALE')
    weaponattackdistancescale = models.BooleanField(default=False, help_text='WEAPONATTACKDISTANCESCALE')
    wing_tail_scale = models.IntegerField(default=0, help_text='WING_TAIL_SCALE')
    helmet_scale_m = models.IntegerField(default=0, help_text='HELMET_SCALE_M')
    helmet_scale_f = models.FloatField(default=0.0, help_text='HELMET_SCALE_F')
    moverate = models.FloatField(default=0.0, help_text='MOVERATE')
    walkdist = models.CharField(max_length=32, blank=True, default='', help_text='WALKDIST')
    rundist = models.FloatField(default=0.0, help_text='RUNDIST')
    perspace = models.FloatField(default=0.0, help_text='PERSPACE')
    creperspace = models.FloatField(default=0.0, help_text='CREPERSPACE')
    height = models.FloatField(default=0.0, help_text='HEIGHT')
    hitdist = models.FloatField(default=0.0, help_text='HITDIST')
    prefatckdist = models.FloatField(default=0.0, help_text='PREFATCKDIST')
    targetheight = models.FloatField(default=0.0, help_text='TARGETHEIGHT')
    abortonparry = models.CharField(max_length=32, blank=True, default='', help_text='ABORTONPARRY')
    racialtype = models.IntegerField(default=0, help_text='RACIALTYPE')
    haslegs = models.BooleanField(default=False, help_text='HASLEGS')
    hasarms = models.BooleanField(default=False, help_text='HASARMS')
    portrait = models.IntegerField(default=0, help_text='PORTRAIT')
    sizecategory = models.CharField(max_length=32, blank=True, default='', help_text='SIZECATEGORY')
    perceptiondist = models.BooleanField(default=False, help_text='PERCEPTIONDIST')
    footsteptype = models.IntegerField(default=0, help_text='FOOTSTEPTYPE')
    soundapptype = models.IntegerField(default=0, help_text='SOUNDAPPTYPE')
    headtrack = models.IntegerField(default=0, help_text='HEADTRACK')
    head_arc_h = models.IntegerField(default=0, help_text='HEAD_ARC_H')
    head_arc_v = models.IntegerField(default=0, help_text='HEAD_ARC_V')
    head_name = models.IntegerField(default=0, help_text='HEAD_NAME')
    body_bag = models.CharField(max_length=32, blank=True, default='', help_text='BODY_BAG')
    targetable = models.IntegerField(default=0, help_text='TARGETABLE')
    selection_capsule = models.IntegerField(default=0, help_text='SELECTION_CAPSULE')
    selection_size = models.IntegerField(default=0, help_text='SELECTION_SIZE')
    sef = models.CharField(max_length=64, blank=True, default='', help_text='SEF')
    
    class Meta:
        db_table = 'gamedata_appearance'
        ordering = ['row_id']
        verbose_name = 'Appearance'
        verbose_name_plural = 'Appearances'


class StartingPackage(GameDataBase):
    """StartingPackage from packages.2da"""
    name = models.CharField(max_length=32, blank=True, default='', help_text='Name')
    description = models.IntegerField(default=0, help_text='Description')
    classid = models.IntegerField(default=0, help_text='ClassID')
    attribute = models.IntegerField(default=0, help_text='Attribute')
    gold = models.CharField(max_length=32, blank=True, default='', help_text='Gold')
    school = models.IntegerField(default=0, help_text='School')
    domain1 = models.IntegerField(default=0, help_text='Domain1')
    domain2 = models.IntegerField(default=0, help_text='Domain2')
    associate = models.IntegerField(default=0, help_text='Associate')
    spellpref2da = models.IntegerField(default=0, help_text='SpellPref2DA')
    featpref2da = models.CharField(max_length=32, blank=True, default='', help_text='FeatPref2DA')
    skillpref2da = models.CharField(max_length=32, blank=True, default='', help_text='SkillPref2DA')
    equip2da = models.CharField(max_length=32, blank=True, default='', help_text='Equip2DA')
    soundset = models.CharField(max_length=32, blank=True, default='', help_text='Soundset')
    is_player_class = models.BooleanField(default=False, help_text='PlayerClass')
    
    class Meta:
        db_table = 'gamedata_packages'
        ordering = ['row_id']
        verbose_name = 'StartingPackage'
        verbose_name_plural = 'StartingPackages'


class Disease(GameDataBase):
    """Disease from disease.2da"""
    name = models.CharField(max_length=34, blank=True, default='', help_text='Name')
    first_save = models.IntegerField(default=0, help_text='First_Save')
    subs_save = models.IntegerField(default=0, help_text='Subs_Save')
    incu_hours = models.IntegerField(default=0, help_text='Incu_Hours')
    dice_1 = models.IntegerField(default=0, help_text='Dice_1')
    dam_1 = models.IntegerField(default=0, help_text='Dam_1')
    type_1 = models.IntegerField(default=0, help_text='Type_1')
    dice_2 = models.IntegerField(default=0, help_text='Dice_2')
    dam_2 = models.IntegerField(default=0, help_text='Dam_2')
    type_2 = models.IntegerField(default=0, help_text='Type_2')
    dice_3 = models.IntegerField(default=0, help_text='Dice_3')
    dam_3 = models.CharField(max_length=64, blank=True, default='', help_text='Dam_3')
    type_3 = models.CharField(max_length=64, blank=True, default='', help_text='Type_3')
    type = models.CharField(max_length=64, blank=True, default='', help_text='Type')
    end_incu_script = models.CharField(max_length=32, blank=True, default='', help_text='End_Incu_Script')
    field_24_hour_script = models.CharField(max_length=64, blank=True, default='', help_text='24_Hour_Script')
    
    class Meta:
        db_table = 'gamedata_disease'
        ordering = ['row_id']
        verbose_name = 'Disease'
        verbose_name_plural = 'Diseases'


class Poison(GameDataBase):
    """Poison from poison.2da"""
    name = models.CharField(max_length=44, blank=True, default='', help_text='Name')
    save_dc = models.IntegerField(default=0, help_text='Save_DC')
    handle_dc = models.IntegerField(default=0, help_text='Handle_DC')
    dice_1 = models.IntegerField(default=0, help_text='Dice_1')
    dam_1 = models.IntegerField(default=0, help_text='Dam_1')
    default_1 = models.IntegerField(default=0, help_text='Default_1')
    script_1 = models.CharField(max_length=32, blank=True, default='', help_text='Script_1')
    dice_2 = models.CharField(max_length=64, blank=True, default='', help_text='Dice_2')
    dam_2 = models.IntegerField(default=0, help_text='Dam_2')
    default_2 = models.IntegerField(default=0, help_text='Default_2')
    script_2 = models.CharField(max_length=32, blank=True, default='', help_text='Script_2')
    cost = models.CharField(max_length=32, blank=True, default='', help_text='Cost')
    onhitapplied = models.FloatField(default=0.0, help_text='OnHitApplied')
    vfx_impact = models.IntegerField(default=0, help_text='VFX_Impact')
    
    class Meta:
        db_table = 'gamedata_poison'
        ordering = ['row_id']
        verbose_name = 'Poison'
        verbose_name_plural = 'Poisons'



# ===== RELATIONSHIP MODELS =====

class ClassFeat(models.Model):
    """Feats available to specific classes"""
    game_class = models.ForeignKey(GameClass, on_delete=models.CASCADE, related_name='class_feats')
    feat = models.ForeignKey(Feat, on_delete=models.CASCADE)
    level = models.IntegerField(default=1)
    
    class Meta:
        db_table = 'gamedata_class_feat'
        unique_together = [['game_class', 'feat', 'level']]


class ClassSkill(models.Model):
    """Skills that are class skills for each class"""
    game_class = models.ForeignKey(GameClass, on_delete=models.CASCADE, related_name='class_skills')
    skill = models.ForeignKey(Skill, on_delete=models.CASCADE)
    
    class Meta:
        db_table = 'gamedata_class_skill'
        unique_together = [['game_class', 'skill']]


class SpellLevel(models.Model):
    """Spell levels for each class"""
    spell = models.ForeignKey(Spell, on_delete=models.CASCADE, related_name='spell_levels')
    game_class = models.ForeignKey(GameClass, on_delete=models.CASCADE)
    level = models.IntegerField(validators=[MinValueValidator(0), MaxValueValidator(9)])
    
    class Meta:
        db_table = 'gamedata_spell_level'
        unique_together = [['spell', 'game_class']]


class FeatPrerequisite(models.Model):
    """Prerequisites for feats"""
    feat = models.ForeignKey(Feat, on_delete=models.CASCADE, related_name='prerequisites')
    prerequisite_feat = models.ForeignKey(Feat, on_delete=models.CASCADE, 
                                        related_name='required_for', null=True, blank=True)
    min_ability_score = models.CharField(max_length=3, blank=True)  # STR, DEX, etc
    min_ability_value = models.IntegerField(default=0)
    min_skill = models.ForeignKey(Skill, on_delete=models.CASCADE, null=True, blank=True)
    min_skill_rank = models.IntegerField(default=0)
    min_bab = models.IntegerField(default=0)
    min_level = models.IntegerField(default=1)
    
    class Meta:
        db_table = 'gamedata_feat_prerequisite'


# ===== PROGRESSION TABLES =====

class BABProgression(models.Model):
    """Base Attack Bonus progression"""
    table_name = models.CharField(max_length=32, unique=True)  # cls_atk_1, cls_atk_2, cls_atk_3
    level = models.IntegerField()
    bab = models.IntegerField()
    
    class Meta:
        db_table = 'gamedata_bab_progression'
        unique_together = [['table_name', 'level']]
        ordering = ['table_name', 'level']


class SaveProgression(models.Model):
    """Saving throw progression"""
    table_name = models.CharField(max_length=64, db_index=True)  # cls_savthr_barb, etc
    level = models.IntegerField()
    fortitude = models.IntegerField(default=0)
    reflex = models.IntegerField(default=0)
    will = models.IntegerField(default=0)
    
    class Meta:
        db_table = 'gamedata_save_progression'
        unique_together = [['table_name', 'level']]
        ordering = ['table_name', 'level']


class SpellProgression(models.Model):
    """Spells per day progression"""
    table_name = models.CharField(max_length=64, db_index=True)  # cls_spgn_wiz, etc
    level = models.IntegerField()
    spell_level_0 = models.IntegerField(default=0)
    spell_level_1 = models.IntegerField(default=0)
    spell_level_2 = models.IntegerField(default=0)
    spell_level_3 = models.IntegerField(default=0)
    spell_level_4 = models.IntegerField(default=0)
    spell_level_5 = models.IntegerField(default=0)
    spell_level_6 = models.IntegerField(default=0)
    spell_level_7 = models.IntegerField(default=0)
    spell_level_8 = models.IntegerField(default=0)
    spell_level_9 = models.IntegerField(default=0)
    
    class Meta:
        db_table = 'gamedata_spell_progression'
        unique_together = [['table_name', 'level']]
        ordering = ['table_name', 'level']


class ItemProperty(models.Model):
    """Item properties from iprp_* tables"""
    property_type = models.CharField(max_length=64)  # damage, ability, skill, etc
    property_id = models.IntegerField()
    name = models.CharField(max_length=128)
    cost_table = models.IntegerField(default=0)
    param_table = models.IntegerField(default=0)
    game_str_ref = models.IntegerField(default=0)
    description_str_ref = models.IntegerField(default=0)
    
    class Meta:
        db_table = 'gamedata_item_property'
        unique_together = [['property_type', 'property_id']]

